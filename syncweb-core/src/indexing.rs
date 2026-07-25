//! Opt-in local indexing for synchronized folders.
//!
//! The indexer deliberately owns its database and event consumers. The core
//! synchronization path does not depend on the index being enabled or healthy.

pub mod catalog;
pub mod denylist;
pub mod links;
pub mod parallel;
pub mod reputation;
pub mod resilience;
pub mod wot;
pub use catalog::{Catalog, CatalogMetadata, CatalogRecord, CatalogService};
pub use denylist::{Denied, DenyReason, Denylist, DenylistRule, DenylistService, FilterContext, FilterList};
pub use links::{
    CapabilityLink, ContentLink, ImmutableLink, Link, LinkResolution, LinkResolver, Mirror, MutableLink,
    MutablePointer, NameLink, PrivateLink, ProviderFetch, REVOCATION_GOSSIP_TOPIC, ResolveOptions, ResolvedLink,
    SignedMutablePointer, SyncwebLink, current_epoch_seconds, fetch_from_mirrors, revocation_topic_id,
};
pub use parallel::{ParallelDownloadConfig, TryParallelResult};
pub use reputation::{
    ProviderReputation, ProviderReputationStore, ProviderTrustSignal, ReputationConfig, TrustSignalKind,
    trust_stream_topic,
};
pub use resilience::{
    AvailabilityHealth, BanRecord, BanSource, FailureRecord, FetchFailure, FetchFailureKind, FetchWait, LeaseUpdate,
    ProviderLease, ProviderLeaseTracker, ReplicationBudget, ReplicationResult, ResilienceConfig, ResilienceService,
    consistent_hashing_selection, jitter_delay, resilience_topic, validate_bounded_fetch, validate_bounded_stream,
    validate_fetch_stream, xor_distance,
};
pub use wot::{
    Attestation, AttestationKind, MetadataEntry, ModerationAction, ModerationContext, ModerationDecision,
    ModerationRecord, ModerationScope, ProviderTrustAction, ProviderTrustDecision, ProviderTrustRecord,
    RevocationRecord, TrustDecision, TrustDelegation, TrustPolicy, WotMetadata, WotService,
};

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use iroh::PublicKey;
use iroh_blobs::Hash;
use iroh_docs::{Entry, NamespaceId, engine::LiveEvent};
use n0_future::StreamExt;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tokio::{sync::broadcast, task::JoinHandle};

use crate::{
    error::{Result, SyncwebError},
    folder::SyncwebFolder,
    gossip::SignedGossipMessage,
};

/// Current indexing database schema version.
pub const SCHEMA_VERSION: &str = "3";
const EVENT_CAPACITY: usize = 256;

/// A content entry known to the local indexing service.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct IndexedEntry {
    pub namespace_id: NamespaceId,
    pub key: Vec<u8>,
    pub hash: Hash,
    pub size: u64,
}

/// Core synchronization events that do not add or remove an indexed entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CoreIndexingEvent {
    ContentReady { hash: Hash },
    PendingContentReady,
    NeighborUp,
    NeighborDown,
    SyncFinished,
}

/// Events emitted by an [`IndexingService`].
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum IndexingEvent {
    FolderEnabled {
        namespace_id: NamespaceId,
    },
    FolderDisabled {
        namespace_id: NamespaceId,
    },
    EntryIndexed(IndexedEntry),
    Core {
        namespace_id: NamespaceId,
        event: CoreIndexingEvent,
    },
    Error {
        namespace_id: NamespaceId,
        message: String,
    },
}

/// A folder opt-in returned by [`IndexingService::enable_folder`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct IndexingHandle {
    namespace_id: NamespaceId,
}

impl IndexingHandle {
    #[must_use]
    pub const fn namespace_id(self) -> NamespaceId {
        self.namespace_id
    }
}

/// Thread-safe `SQLite` database used by the indexing service.
#[derive(Clone)]
pub struct IndexingDatabase {
    connection: Arc<Mutex<Connection>>,
    path: Arc<PathBuf>,
}

impl std::fmt::Debug for IndexingDatabase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IndexingDatabase")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl IndexingDatabase {
    /// Open or create an indexing database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let database_path = path.as_ref().to_path_buf();
        if database_path.as_os_str() != ":memory:"
            && let Some(parent) = database_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .map_err(|error| SyncwebError::operation("failed to create indexing database directory", error))?;
        }
        let connection = Connection::open(&database_path)
            .map_err(|error| SyncwebError::operation("failed to open indexing database", error))?;
        initialize_connection(&connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            path: Arc::new(database_path),
        })
    }

    /// Create an isolated in-memory indexing database.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot initialize the schema.
    pub fn in_memory() -> Result<Self> {
        Self::open(":memory:")
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// Return the persisted indexing schema version.
    ///
    /// # Errors
    ///
    /// Returns an error if the schema metadata cannot be read.
    pub fn schema_version(&self) -> Result<String> {
        self.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT value FROM index_metadata WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| database_error("failed to read indexing schema version", error))
        })
    }

    /// Return whether the `SQLite` FTS5 virtual table is available.
    ///
    /// # Errors
    ///
    /// Returns an error if the schema cannot be queried.
    pub fn has_fts5(&self) -> Result<bool> {
        self.has_table("indexed_entries_fts")
    }

    /// Return the names of schema objects owned by the index.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot query its schema.
    pub fn schema_objects(&self) -> Result<Vec<String>> {
        self.with_connection(|connection| {
            let mut statement = connection
                .prepare("SELECT name FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY name")
                .map_err(|error| database_error("failed to prepare schema query", error))?;
            let names = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|error| database_error("failed to query schema objects", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read schema objects", error))?;
            Ok(names)
        })
    }

    /// Return whether a schema table or view exists.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot query its schema.
    pub fn has_table(&self, name: &str) -> Result<bool> {
        self.with_connection(|connection| {
            let exists = connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM sqlite_master
                        WHERE name = ?1 AND type IN ('table', 'view')
                    )",
                    [name],
                    |row| row.get(0),
                )
                .map_err(|error| database_error("failed to query schema object", error))?;
            Ok(exists)
        })
    }

    /// Return the number of opted-in folders.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot query the database.
    pub fn folder_count(&self) -> Result<usize> {
        self.with_connection(|connection| {
            let count = connection
                .query_row("SELECT COUNT(*) FROM indexed_folders", [], |row| row.get::<_, i64>(0))
                .map_err(|error| database_error("failed to count indexed folders", error))?;
            usize::try_from(count).map_err(|error| database_error("indexed folder count is invalid", error))
        })
    }

    /// Return the number of indexed entries.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot query the database.
    pub fn entry_count(&self) -> Result<usize> {
        self.with_connection(|connection| {
            let count = connection
                .query_row("SELECT COUNT(*) FROM indexed_entries", [], |row| row.get::<_, i64>(0))
                .map_err(|error| database_error("failed to count indexed entries", error))?;
            usize::try_from(count).map_err(|error| database_error("indexed entry count is invalid", error))
        })
    }

    /// Return whether a namespace is currently opted into indexing.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot query the database.
    pub fn is_folder_enabled(&self, namespace_id: NamespaceId) -> Result<bool> {
        self.with_connection(|connection| {
            let enabled = connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM indexed_folders WHERE namespace_id = ?1
                    )",
                    [namespace_id.to_string()],
                    |row| row.get(0),
                )
                .map_err(|error| database_error("failed to query indexed folder", error))?;
            Ok(enabled)
        })
    }

    /// Insert or update an entry in the index.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot persist the entry.
    pub fn upsert_entry(
        &self,
        namespace_id: NamespaceId,
        key: impl AsRef<[u8]>,
        hash: Hash,
        size: u64,
    ) -> Result<IndexedEntry> {
        let key_bytes = key.as_ref().to_vec();
        let namespace = namespace_id.to_string();
        let hash_bytes = hash.as_bytes().to_vec();
        let size_value =
            i64::try_from(size).map_err(|error| database_error("indexed entry size is too large", error))?;
        let key_text = String::from_utf8_lossy(&key_bytes).into_owned();
        let title = Path::new(&key_text)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&key_text)
            .to_owned();
        self.with_connection(|connection| {
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| database_error("failed to begin indexing transaction", error))?;
            let existing_id = transaction
                .query_row(
                    "SELECT id FROM indexed_entries WHERE namespace_id = ?1 AND entry_key = ?2",
                    params![namespace, key_bytes],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(|error| database_error("failed to find indexed entry", error))?;
            let id = if let Some(id) = existing_id {
                transaction
                    .execute(
                        "UPDATE indexed_entries
                         SET content_hash = ?1, content_len = ?2, updated_at = ?3
                         WHERE id = ?4",
                        params![hash_bytes, size_value, now_seconds(), id],
                    )
                    .map_err(|error| database_error("failed to update indexed entry", error))?;
                transaction
                    .execute("DELETE FROM indexed_entries_fts WHERE rowid = ?1", [id])
                    .map_err(|error| database_error("failed to refresh indexed search entry", error))?;
                id
            } else {
                transaction
                    .execute(
                        "INSERT INTO indexed_entries
                         (namespace_id, entry_key, content_hash, content_len, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![namespace, key_bytes, hash_bytes, size_value, now_seconds()],
                    )
                    .map_err(|error| database_error("failed to insert indexed entry", error))?;
                transaction.last_insert_rowid()
            };
            transaction
                .execute(
                    "INSERT INTO indexed_entries_fts(rowid, namespace_id, entry_key, title, tags)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, namespace, key_text, title, ""],
                )
                .map_err(|error| database_error("failed to insert indexed search entry", error))?;
            transaction
                .commit()
                .map_err(|error| database_error("failed to commit indexed entry", error))?;
            Ok(IndexedEntry {
                namespace_id,
                key: key_bytes,
                hash,
                size,
            })
        })
    }

    /// Remove an indexed entry if it exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot remove the entry.
    pub fn remove_entry(&self, namespace_id: NamespaceId, key: impl AsRef<[u8]>) -> Result<bool> {
        self.with_connection(|connection| {
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| database_error("failed to begin indexing transaction", error))?;
            let id = transaction
                .query_row(
                    "SELECT id FROM indexed_entries WHERE namespace_id = ?1 AND entry_key = ?2",
                    params![namespace_id.to_string(), key.as_ref()],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(|error| database_error("failed to find indexed entry", error))?;
            let Some(entry_id) = id else {
                transaction
                    .commit()
                    .map_err(|error| database_error("failed to commit indexing transaction", error))?;
                return Ok(false);
            };
            transaction
                .execute("DELETE FROM indexed_entries_fts WHERE rowid = ?1", [entry_id])
                .map_err(|error| database_error("failed to remove indexed search entry", error))?;
            transaction
                .execute("DELETE FROM indexed_entries WHERE id = ?1", [entry_id])
                .map_err(|error| database_error("failed to remove indexed entry", error))?;
            transaction
                .commit()
                .map_err(|error| database_error("failed to commit indexing transaction", error))?;
            Ok(true)
        })
    }

    /// Search indexed paths and metadata using `SQLite` FTS5.
    ///
    /// An empty query returns the most recently updated entries. FTS5 query
    /// syntax is passed through for non-empty queries.
    ///
    /// # Errors
    ///
    /// Returns an error if the query is invalid or `SQLite` cannot read results.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<IndexedEntry>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let limit_value = i64::try_from(limit).map_err(|error| database_error("search limit is invalid", error))?;
        self.with_connection(|connection| {
            let mut output = Vec::new();
            if query.trim().is_empty() {
                let mut statement = connection
                    .prepare(
                        "SELECT namespace_id, entry_key, content_hash, content_len
                         FROM indexed_entries ORDER BY updated_at DESC, id DESC LIMIT ?1",
                    )
                    .map_err(|error| database_error("failed to prepare index query", error))?;
                let rows = statement
                    .query_map([limit_value], indexed_entry_from_row)
                    .map_err(|error| database_error("failed to query indexed entries", error))?;
                for row in rows {
                    output.push(row.map_err(|error| database_error("failed to read indexed entry", error))?);
                }
            } else {
                let mut statement = connection
                    .prepare(
                        "SELECT e.namespace_id, e.entry_key, e.content_hash, e.content_len
                         FROM indexed_entries_fts f
                         JOIN indexed_entries e ON e.id = f.rowid
                         WHERE indexed_entries_fts MATCH ?1
                         ORDER BY bm25(indexed_entries_fts), e.id DESC
                         LIMIT ?2",
                    )
                    .map_err(|error| database_error("failed to prepare full-text query", error))?;
                let rows = statement
                    .query_map(params![query, limit_value], indexed_entry_from_row)
                    .map_err(|error| database_error("failed to query full-text index", error))?;
                for row in rows {
                    output.push(row.map_err(|error| database_error("failed to read full-text result", error))?);
                }
            }
            Ok(output)
        })
    }

    /// Register a catalog that is published or subscribed to locally.
    ///
    /// # Errors
    ///
    /// Returns an error if the catalog metadata cannot be stored.
    pub fn enable_catalog(&self, namespace_id: NamespaceId, label: &str) -> Result<()> {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO indexed_catalogs(namespace_id, label, subscribed_at)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(namespace_id) DO UPDATE SET label = excluded.label",
                    params![namespace_id.to_string(), label, now_seconds()],
                )
                .map_err(|error| database_error("failed to enable indexed catalog", error))?;
            Ok(())
        })
    }

    /// Return the number of known catalogs.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot query the database.
    pub fn catalog_count(&self) -> Result<usize> {
        self.with_connection(|connection| {
            let count = connection
                .query_row("SELECT COUNT(*) FROM indexed_catalogs", [], |row| row.get::<_, i64>(0))
                .map_err(|error| database_error("failed to count indexed catalogs", error))?;
            usize::try_from(count).map_err(|error| database_error("indexed catalog count is invalid", error))
        })
    }

    /// Return all known catalogs with their namespace identifiers and labels.
    ///
    /// # Errors
    ///
    /// Returns an error if the catalog metadata cannot be read.
    pub fn load_catalogs(&self) -> Result<Vec<(NamespaceId, String)>> {
        use std::str::FromStr;
        self.with_connection(|connection| {
            let mut statement = connection
                .prepare("SELECT namespace_id, label FROM indexed_catalogs ORDER BY subscribed_at DESC")
                .map_err(|error| database_error("failed to prepare catalog listing", error))?;
            let rows = statement
                .query_map([], |row| {
                    let namespace: String = row.get(0)?;
                    let label: String = row.get(1)?;
                    Ok((namespace, label))
                })
                .map_err(|error| database_error("failed to read indexed catalogs", error))?;
            let mut catalogs = Vec::new();
            for row in rows {
                let (namespace, label) =
                    row.map_err(|error| database_error("failed to read indexed catalog record", error))?;
                let namespace_id = NamespaceId::from_str(&namespace)
                    .map_err(|error| database_error("indexed catalog namespace is invalid", error))?;
                catalogs.push((namespace_id, label));
            }
            Ok(catalogs)
        })
    }

    /// Return the number of records imported from catalogs.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot query the database.
    pub fn catalog_entry_count(&self) -> Result<usize> {
        self.with_connection(|connection| {
            let count = connection
                .query_row("SELECT COUNT(*) FROM indexed_catalog_entries", [], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(|error| database_error("failed to count indexed catalog entries", error))?;
            usize::try_from(count).map_err(|error| database_error("indexed catalog entry count is invalid", error))
        })
    }

    /// Insert or update a record received from a catalog.
    ///
    /// # Errors
    ///
    /// Returns an error if the catalog is not registered or the record cannot
    /// be persisted.
    pub fn upsert_catalog_record(&self, record: &CatalogRecord) -> Result<CatalogRecord> {
        record.validate()?;
        let catalog_namespace = record.catalog_namespace_id.to_string();
        let folder_namespace = record.folder_namespace_id.to_string();
        let hash_bytes = record.hash.as_bytes().to_vec();
        let size =
            i64::try_from(record.size).map_err(|error| database_error("catalog record size is too large", error))?;
        let tags = serde_json::to_string(&record.tags)
            .map_err(|error| database_error("failed to serialize catalog record tags", error))?;
        let key = record.key.clone();
        let publisher = record.publisher.clone();
        self.with_connection(|connection| {
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| database_error("failed to begin catalog transaction", error))?;
            let catalog_exists = transaction
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM indexed_catalogs WHERE namespace_id = ?1
                    )",
                    [&catalog_namespace],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(|error| database_error("failed to find indexed catalog", error))?;
            if !catalog_exists {
                return Err(SyncwebError::InvalidConfig(format!(
                    "catalog is not enabled: {catalog_namespace}"
                )));
            }
            let existing_id = transaction
                .query_row(
                    "SELECT id FROM indexed_catalog_entries
                     WHERE catalog_namespace_id = ?1
                       AND folder_namespace_id = ?2
                       AND entry_key = ?3",
                    params![catalog_namespace, folder_namespace, key],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(|error| database_error("failed to find indexed catalog record", error))?;
            let id = if let Some(id) = existing_id {
                transaction
                    .execute(
                        "UPDATE indexed_catalog_entries
                         SET content_hash = ?1, content_len = ?2, folder_name = ?3,
                             title = ?4, tags = ?5, publisher = ?6, updated_at = ?7
                         WHERE id = ?8",
                        params![
                            hash_bytes,
                            size,
                            record.folder_name,
                            record.title,
                            tags,
                            publisher,
                            now_seconds(),
                            id
                        ],
                    )
                    .map_err(|error| database_error("failed to update indexed catalog record", error))?;
                transaction
                    .execute("DELETE FROM indexed_catalog_entries_fts WHERE rowid = ?1", [id])
                    .map_err(|error| database_error("failed to refresh catalog search record", error))?;
                id
            } else {
                transaction
                    .execute(
                        "INSERT INTO indexed_catalog_entries
                         (catalog_namespace_id, folder_namespace_id, entry_key, content_hash,
                          content_len, folder_name, title, tags, publisher, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        params![
                            catalog_namespace,
                            folder_namespace,
                            key,
                            hash_bytes,
                            size,
                            record.folder_name,
                            record.title,
                            tags,
                            publisher,
                            now_seconds()
                        ],
                    )
                    .map_err(|error| database_error("failed to insert indexed catalog record", error))?;
                transaction.last_insert_rowid()
            };
            transaction
                .execute(
                    "INSERT INTO indexed_catalog_entries_fts(
                         rowid, catalog_namespace_id, folder_namespace_id, entry_key,
                        folder_name, title, tags, publisher, content_hash
                     )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        id,
                        record.catalog_namespace_id.to_string(),
                        record.folder_namespace_id.to_string(),
                        String::from_utf8_lossy(&record.key),
                        record.folder_name,
                        record.title,
                        tags,
                        publisher,
                        record.hash.to_string()
                    ],
                )
                .map_err(|error| database_error("failed to insert catalog search record", error))?;
            transaction
                .commit()
                .map_err(|error| database_error("failed to commit catalog record", error))?;
            Ok(record.clone())
        })
    }

    /// Search all subscribed catalogs with `SQLite` FTS5.
    ///
    /// An empty query returns the most recently updated catalog records.
    ///
    /// # Errors
    ///
    /// Returns an error if the query is invalid or `SQLite` cannot read results.
    pub fn search_catalogs(&self, query: &str, limit: usize) -> Result<Vec<CatalogRecord>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let limit_value =
            i64::try_from(limit).map_err(|error| database_error("catalog search limit is invalid", error))?;
        self.with_connection(|connection| {
            let sql = if query.trim().is_empty() {
                "SELECT catalog_namespace_id, folder_namespace_id, entry_key, content_hash,
                        content_len, folder_name, title, tags, publisher
                 FROM indexed_catalog_entries
                 ORDER BY updated_at DESC, id DESC LIMIT ?1"
            } else {
                "SELECT e.catalog_namespace_id, e.folder_namespace_id, e.entry_key, e.content_hash,
                        e.content_len, e.folder_name, e.title, e.tags, e.publisher
                 FROM indexed_catalog_entries_fts f
                 JOIN indexed_catalog_entries e ON e.id = f.rowid
                 WHERE indexed_catalog_entries_fts MATCH ?1
                 ORDER BY bm25(indexed_catalog_entries_fts), e.id DESC
                 LIMIT ?2"
            };
            let mut statement = connection
                .prepare(sql)
                .map_err(|error| database_error("failed to prepare catalog search", error))?;
            let rows = if query.trim().is_empty() {
                statement
                    .query_map([rusqlite::types::Value::Integer(limit_value)], catalog_record_from_row)
                    .map_err(|error| database_error("failed to query catalog records", error))?
            } else {
                statement
                    .query_map(params![query, limit_value], catalog_record_from_row)
                    .map_err(|error| database_error("failed to query catalog records", error))?
            };
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read catalog records", error))
        })
    }

    /// Search all known catalogs. This is an alias for [`Self::search_catalogs`].
    ///
    /// # Errors
    ///
    /// Returns an error if the query is invalid or `SQLite` cannot read results.
    pub fn global_search(&self, query: &str, limit: usize) -> Result<Vec<CatalogRecord>> {
        self.search_catalogs(query, limit)
    }

    /// Search all known catalogs. Alias for [`Self::search_catalogs`].
    ///
    /// # Errors
    ///
    /// Returns an error if the query is invalid or `SQLite` cannot read results.
    pub fn catalog_search(&self, query: &str, limit: usize) -> Result<Vec<CatalogRecord>> {
        self.search_catalogs(query, limit)
    }

    /// Append a signed Web-of-Trust metadata entry to the local index.
    ///
    /// # Errors
    ///
    /// Returns an error if the entry is invalid or `SQLite` cannot persist it.
    pub fn append_wot_metadata(&self, entry: &wot::MetadataEntry) -> Result<bool> {
        entry.validate()?;
        let hash_bytes = entry.content.as_bytes().to_vec();
        let sequence =
            i64::try_from(entry.sequence).map_err(|error| database_error("metadata sequence is too large", error))?;
        let created_at = i64::try_from(entry.created_at)
            .map_err(|error| database_error("metadata timestamp is too large", error))?;
        let key = entry.key.clone();
        let value = entry.value.clone();
        let author = entry.author.clone();
        let signature = entry
            .signature
            .clone()
            .ok_or_else(|| SyncwebError::InvalidConfig("Web-of-Trust metadata must be signed".to_owned()))?;
        self.with_connection(|connection| {
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| database_error("failed to begin Web-of-Trust metadata transaction", error))?;
            let inserted = transaction
                .execute(
                    "INSERT INTO wot_metadata
                     (content_hash, metadata_key, metadata_value, author, sequence, created_at, signature)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT(content_hash, metadata_key, author, sequence) DO NOTHING",
                    params![hash_bytes, key, value, author, sequence, created_at, signature],
                )
                .map_err(|error| database_error("failed to append Web-of-Trust metadata", error))?;
            if inserted == 1 {
                let rowid = transaction.last_insert_rowid();
                transaction
                    .execute(
                        "INSERT INTO wot_metadata_fts(rowid, content_hash, metadata_key, metadata_value, author)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![rowid, entry.content.to_string(), entry.key, entry.value, entry.author],
                    )
                    .map_err(|error| database_error("failed to index Web-of-Trust metadata", error))?;
            }
            transaction
                .commit()
                .map_err(|error| database_error("failed to commit Web-of-Trust metadata", error))?;
            Ok(inserted == 1)
        })
    }

    /// Search accepted Web-of-Trust metadata using `SQLite` FTS5.
    ///
    /// # Errors
    ///
    /// Returns an error if the query is invalid or metadata cannot be read.
    pub fn search_wot_metadata(&self, query: &str, limit: usize) -> Result<Vec<wot::MetadataEntry>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let limit_value =
            i64::try_from(limit).map_err(|error| database_error("metadata search limit is invalid", error))?;
        self.with_connection(|connection| {
            let mut statement = if query.trim().is_empty() {
                connection
                    .prepare(
                        "SELECT content_hash, metadata_key, metadata_value, author, sequence, created_at, signature
                         FROM wot_metadata ORDER BY created_at DESC, id DESC LIMIT ?1",
                    )
                    .map_err(|error| database_error("failed to prepare metadata query", error))?
            } else {
                connection
                    .prepare(
                        "SELECT m.content_hash, m.metadata_key, m.metadata_value, m.author,
                                m.sequence, m.created_at, m.signature
                         FROM wot_metadata_fts f
                         JOIN wot_metadata m ON m.id = f.rowid
                         WHERE wot_metadata_fts MATCH ?1
                         ORDER BY bm25(wot_metadata_fts), m.id DESC
                         LIMIT ?2",
                    )
                    .map_err(|error| database_error("failed to prepare metadata full-text query", error))?
            };
            let rows = if query.trim().is_empty() {
                statement
                    .query_map([limit_value], wot_metadata_from_row)
                    .map_err(|error| database_error("failed to query metadata", error))?
            } else {
                statement
                    .query_map(params![query, limit_value], wot_metadata_from_row)
                    .map_err(|error| database_error("failed to query metadata full-text index", error))?
            };
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read metadata search results", error))
        })
    }

    /// Remove a catalog and all records imported from it.
    ///
    /// # Errors
    ///
    /// Returns an error if the catalog cannot be removed.
    pub fn disable_catalog(&self, namespace_id: NamespaceId) -> Result<()> {
        self.with_connection(|connection| {
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| database_error("failed to begin catalog removal", error))?;
            let namespace = namespace_id.to_string();
            let mut statement = transaction
                .prepare("SELECT id FROM indexed_catalog_entries WHERE catalog_namespace_id = ?1")
                .map_err(|error| database_error("failed to find catalog records", error))?;
            let ids = statement
                .query_map([namespace.as_str()], |row| row.get::<_, i64>(0))
                .map_err(|error| database_error("failed to list catalog records", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read catalog record IDs", error))?;
            drop(statement);
            for id in ids {
                transaction
                    .execute("DELETE FROM indexed_catalog_entries_fts WHERE rowid = ?1", [id])
                    .map_err(|error| database_error("failed to remove catalog search record", error))?;
            }
            transaction
                .execute(
                    "DELETE FROM indexed_catalog_entries WHERE catalog_namespace_id = ?1",
                    [namespace.as_str()],
                )
                .map_err(|error| database_error("failed to remove catalog records", error))?;
            transaction
                .execute(
                    "DELETE FROM indexed_catalogs WHERE namespace_id = ?1",
                    [namespace.as_str()],
                )
                .map_err(|error| database_error("failed to disable indexed catalog", error))?;
            transaction
                .commit()
                .map_err(|error| database_error("failed to commit catalog removal", error))?;
            Ok(())
        })
    }

    /// Register a namespace for local indexing.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder metadata cannot be stored.
    pub fn enable_folder(&self, namespace_id: NamespaceId, label: &str) -> Result<()> {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO indexed_folders(namespace_id, label, enabled_at)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(namespace_id) DO UPDATE SET label = excluded.label",
                    params![namespace_id.to_string(), label, now_seconds()],
                )
                .map_err(|error| database_error("failed to enable indexed folder", error))?;
            Ok(())
        })
    }

    /// Remove a namespace and its indexed entries.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder metadata or entries cannot be removed.
    pub fn disable_folder(&self, namespace_id: NamespaceId) -> Result<()> {
        self.with_connection(|connection| {
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| database_error("failed to begin folder removal", error))?;
            let namespace = namespace_id.to_string();
            let mut statement = transaction
                .prepare("SELECT id FROM indexed_entries WHERE namespace_id = ?1")
                .map_err(|error| database_error("failed to find folder entries", error))?;
            let ids = statement
                .query_map([namespace.as_str()], |row| row.get::<_, i64>(0))
                .map_err(|error| database_error("failed to list folder entries", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read folder entries", error))?;
            drop(statement);
            for id in ids {
                transaction
                    .execute("DELETE FROM indexed_entries_fts WHERE rowid = ?1", [id])
                    .map_err(|error| database_error("failed to remove folder search entry", error))?;
            }
            transaction
                .execute(
                    "DELETE FROM indexed_entries WHERE namespace_id = ?1",
                    [namespace.as_str()],
                )
                .map_err(|error| database_error("failed to remove folder entries", error))?;
            transaction
                .execute(
                    "DELETE FROM indexed_folders WHERE namespace_id = ?1",
                    [namespace.as_str()],
                )
                .map_err(|error| database_error("failed to disable indexed folder", error))?;
            transaction
                .commit()
                .map_err(|error| database_error("failed to commit folder removal", error))?;
            Ok(())
        })
    }

    fn with_connection<T>(&self, operation: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("indexing database lock poisoned", error))?;
        operation(&mut connection)
    }

    /// Persist denylist rules to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the rules cannot be saved.
    pub fn save_denylist_rules(&self, rules: &[DenylistRule]) -> Result<()> {
        self.with_connection(|connection| {
            connection
                .execute("DELETE FROM denylist_rules", [])
                .map_err(|error| database_error("failed to clear denylist rules", error))?;
            let now = now_seconds();
            for rule in rules {
                let (rule_type, rule_value, namespace_id) = match rule {
                    DenylistRule::Device(d) => ("device", d.as_bytes().to_vec(), None::<String>),
                    DenylistRule::Hash(h) => ("hash", h.as_bytes().to_vec(), None),
                    DenylistRule::File { namespace_id: ns, key } => ("file", key.clone(), ns.map(|n| n.to_string())),
                };
                connection
                    .execute(
                        "INSERT OR REPLACE INTO denylist_rules(rule_type, rule_value, namespace_id, updated_at)
                     VALUES (?1, ?2, ?3, ?4)",
                        params![rule_type, rule_value, namespace_id, now],
                    )
                    .map_err(|error| database_error("failed to save denylist rule", error))?;
            }
            Ok(())
        })
    }

    /// Load persisted denylist rules from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the rules cannot be read.
    pub fn load_denylist_rules(&self) -> Result<Vec<DenylistRule>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare("SELECT rule_type, rule_value, namespace_id FROM denylist_rules ORDER BY updated_at")
                .map_err(|error| database_error("failed to prepare denylist query", error))?;
            let rules = stmt
                .query_map([], |row| {
                    let rule_type: String = row.get(0)?;
                    let rule_value: Vec<u8> = row.get(1)?;
                    let namespace_id: Option<String> = row.get(2)?;
                    match rule_type.as_str() {
                        "device" => Ok(DenylistRule::Device(String::from_utf8_lossy(&rule_value).to_string())),
                        "hash" => {
                            let arr: [u8; 32] = rule_value.try_into().map_err(|error: Vec<u8>| {
                                rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                                    "invalid hash length",
                                    format!("expected 32 bytes, got {}", error.len()),
                                )))
                            })?;
                            Ok(DenylistRule::Hash(Hash::from(arr)))
                        }
                        "file" => {
                            let ns = namespace_id.and_then(|s| s.parse::<NamespaceId>().ok());
                            Ok(DenylistRule::File {
                                namespace_id: ns,
                                key: rule_value,
                            })
                        }
                        _ => Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                            SyncwebError::operation("unknown rule type", rule_type),
                        ))),
                    }
                })
                .map_err(|error| database_error("failed to query denylist rules", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read denylist rules", error))?;
            Ok(rules)
        })
    }

    /// Load persisted filter list subscriptions from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the filter lists cannot be read.
    pub fn load_filter_lists(&self) -> Result<Vec<denylist::FilterList>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare("SELECT namespace_id, sequence, publisher, payload, updated_at FROM filter_lists ORDER BY updated_at")
                .map_err(|error| database_error("failed to prepare filter lists query", error))?;
            let lists = stmt
                .query_map([], |row| {
                    let payload: Vec<u8> = row.get(3)?;
                    let list: denylist::FilterList = serde_json::from_slice(&payload).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "failed to deserialize filter list",
                            error,
                        )))
                    })?;
                    Ok(list)
                })
                .map_err(|error| database_error("failed to query filter lists", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read filter list rows", error))?;
            Ok(lists)
        })
    }

    /// Upsert a filter list subscription.
    ///
    /// # Errors
    ///
    /// Returns an error if the filter list cannot be persisted.
    pub fn upsert_filter_list(&self, list: &denylist::FilterList) -> Result<()> {
        let payload = list.to_bytes()?;
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT OR REPLACE INTO filter_lists(namespace_id, sequence, publisher, payload, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        list.namespace_id.to_string(),
                        i64::try_from(list.sequence).unwrap_or(i64::MAX),
                        list.publisher,
                        payload,
                        now_seconds(),
                    ],
                )
                .map_err(|error| database_error("failed to upsert filter list", error))?;
            Ok(())
        })
    }

    /// Load stable links from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the links cannot be read.
    pub fn load_links(&self) -> Result<(Vec<links::MutablePointer>, Vec<String>, Vec<links::PrivateLink>)> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare("SELECT link, kind, payload, updated_at FROM stable_links ORDER BY updated_at")
                .map_err(|error| database_error("failed to prepare links query", error))?;
            let mut pointers = Vec::new();
            let rows = stmt
                .query_map([], |row| {
                    let kind: String = row.get(1)?;
                    let payload: Vec<u8> = row.get(2)?;
                    Ok((kind, payload))
                })
                .map_err(|error| database_error("failed to query stable links", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read link rows", error))?;
            for (kind, payload) in rows {
                if kind.as_str() == "mutable"
                    && let Ok(p) = serde_json::from_slice::<links::MutablePointer>(&payload)
                {
                    pointers.push(p);
                }
            }

            let mut mirror_stmt = connection
                .prepare("SELECT ticket FROM link_mirrors ORDER BY priority")
                .map_err(|error| database_error("failed to prepare mirror query", error))?;
            let mirrors = mirror_stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|error| database_error("failed to query mirrors", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read mirror rows", error))?;

            let mut revoked = Vec::new();
            if let Ok(mut rev_stmt) = connection.prepare("SELECT manifest, capability, revoked_at FROM revoked_links")
                && let Ok(rev_rows) = rev_stmt.query_map([], |row| {
                    let manifest_bytes: Vec<u8> = row.get(0)?;
                    let capability: String = row.get(1)?;
                    let manifests: [u8; 32] = manifest_bytes.try_into().unwrap_or([0_u8; 32]);
                    let hash = Hash::from_bytes(manifests);
                    Ok((hash, capability))
                })
            {
                for row in rev_rows.flatten() {
                    if let Ok(link) = PrivateLink::new(row.0, row.1, u64::MAX) {
                        revoked.push(link);
                    }
                }
            }
            Ok((pointers, mirrors, revoked))
        })
    }

    /// Save stable links to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the links cannot be persisted.
    pub fn save_links(
        &self,
        pointers: &[links::MutablePointer],
        mirrors: &[String],
        revoked: &[links::PrivateLink],
    ) -> Result<()> {
        self.with_connection(|connection| {
            connection.execute("DELETE FROM stable_links", [])
                .map_err(|error| database_error("failed to clear stable links", error))?;
            connection.execute("DELETE FROM link_mirrors", [])
                .map_err(|error| database_error("failed to clear link mirrors", error))?;
            connection.execute("DELETE FROM revoked_links", [])
                .map_err(|error| database_error("failed to clear revoked links", error))?;
            let now = now_seconds();
            for pointer in pointers {
                let payload = serde_json::to_vec(pointer)
                    .map_err(|error| database_error("failed to serialize pointer", error))?;
                connection.execute(
                    "INSERT OR REPLACE INTO stable_links(link, kind, publisher, alias, content_hash, sequence, version, payload, updated_at)
                     VALUES (?1, 'mutable', ?2, ?3, NULL, ?4, ?5, ?6, ?7)",
                    params![
                        format!("mutable:{}:{}", pointer.publisher, pointer.alias),
                        pointer.publisher.to_string(),
                        pointer.alias,
                        i64::try_from(pointer.sequence).unwrap_or(i64::MAX),
                         pointer.version.as_deref(),
                        payload,
                        now,
                    ],
                ).map_err(|error| database_error("failed to save pointer", error))?;
            }
            for uri in mirrors {
                connection.execute(
                    "INSERT INTO link_mirrors(link, provider, ticket, priority, updated_at)
                     VALUES ('', 'local', ?1, 0, ?2)",
                    params![uri, now],
                ).map_err(|error| database_error("failed to save mirror", error))?;
            }
            for link in revoked {
                connection.execute(
                    "INSERT OR REPLACE INTO revoked_links(manifest, capability, revoked_at)
                     VALUES (?1, ?2, ?3)",
                    params![link.manifest.as_bytes(), link.capability, now],
                ).map_err(|error| database_error("failed to save revoked link", error))?;
            }
            Ok(())
        })
    }

    /// Upsert a single mutable pointer to the stable links table.
    ///
    /// # Errors
    ///
    /// Returns an error if the pointer cannot be persisted.
    pub fn upsert_link_pointer(&self, pointer: &links::MutablePointer) -> Result<()> {
        let payload =
            serde_json::to_vec(pointer).map_err(|error| database_error("failed to serialize pointer", error))?;
        self.with_connection(|connection| {
            connection.execute(
                "INSERT OR REPLACE INTO stable_links(link, kind, publisher, alias, content_hash, sequence, version, payload, updated_at)
                 VALUES (?1, 'mutable', ?2, ?3, NULL, ?4, ?5, ?6, ?7)",
                params![
                    format!("mutable:{}:{}", pointer.publisher, pointer.alias),
                    pointer.publisher.to_string(),
                    pointer.alias,
                    i64::try_from(pointer.sequence).unwrap_or(i64::MAX),
                    pointer.version.as_deref(),
                    payload,
                    now_seconds(),
                ],
            )
            .map_err(|error| database_error("failed to upsert link pointer", error))?;
            Ok(())
        })
    }

    /// Insert a single mirror ticket.
    ///
    /// # Errors
    ///
    /// Returns an error if the mirror cannot be persisted.
    pub fn upsert_link_mirror(&self, ticket: &str) -> Result<()> {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT OR REPLACE INTO link_mirrors(link, provider, ticket, priority, updated_at)
                 VALUES ('', 'local', ?1, 0, ?2)",
                    params![ticket, now_seconds()],
                )
                .map_err(|error| database_error("failed to upsert link mirror", error))?;
            Ok(())
        })
    }

    /// Save provider leases to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the leases cannot be persisted.
    pub fn save_leases(&self, leases: &[resilience::ProviderLease]) -> Result<()> {
        self.with_connection(|connection| {
            connection.execute("DELETE FROM provider_leases", [])
                .map_err(|error| database_error("failed to clear provider leases", error))?;
            for lease in leases {
                connection.execute(
                    "INSERT INTO provider_leases(provider, content_hash, ticket, sequence, issued_at, expires_at, signature)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        lease.provider.to_string(),
                        lease.hash.as_bytes().to_vec(),
                        lease.ticket,
                        i64::try_from(lease.sequence).unwrap_or(i64::MAX),
                        i64::try_from(lease.issued_at).unwrap_or(i64::MAX),
                        i64::try_from(lease.expires_at).unwrap_or(i64::MAX),
                        lease.signature,
                    ],
                ).map_err(|error| database_error("failed to save lease", error))?;
            }
            Ok(())
        })
    }

    /// Load provider leases from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the leases cannot be read.
    pub fn load_leases(&self) -> Result<Vec<resilience::ProviderLease>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare(
                    "SELECT provider, content_hash, ticket, sequence, issued_at, expires_at, signature
                 FROM provider_leases ORDER BY issued_at",
                )
                .map_err(|error| database_error("failed to prepare leases query", error))?;
            let leases = stmt
                .query_map([], |row| {
                    let provider_str: String = row.get(0)?;
                    let hash_bytes: Vec<u8> = row.get(1)?;
                    let arr: [u8; 32] = hash_bytes.try_into().map_err(|error: Vec<u8>| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid hash length",
                            format!("expected 32 bytes, got {}", error.len()),
                        )))
                    })?;
                    let provider = provider_str.parse::<PublicKey>().map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid provider",
                            error,
                        )))
                    })?;
                    Ok(resilience::ProviderLease {
                        hash: Hash::from(arr),
                        provider,
                        ticket: row.get(2)?,
                        sequence: u64::try_from(row.get::<_, i64>(3)?).unwrap_or_default(),
                        issued_at: u64::try_from(row.get::<_, i64>(4)?).unwrap_or_default(),
                        expires_at: u64::try_from(row.get::<_, i64>(5)?).unwrap_or_default(),
                        signature: row.get(6)?,
                    })
                })
                .map_err(|error| database_error("failed to query leases", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read lease rows", error))?;
            Ok(leases)
        })
    }

    /// Save trust delegations to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the delegations cannot be persisted.
    pub fn save_trust_delegations(&self, delegations: &[wot::TrustDelegation]) -> Result<()> {
        self.with_connection(|connection| {
            connection.execute("DELETE FROM trust_delegations", [])
                .map_err(|error| database_error("failed to clear trust delegations", error))?;
            for d in delegations {
                connection.execute(
                    "INSERT INTO trust_delegations(delegator, delegate, scope, max_depth, sequence, issued_at, expires_at, revoked_at, signature)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        d.delegator, d.delegate,
                        d.scope.map(|h| h.as_bytes().to_vec()),
                        d.max_depth.map(i64::from),
                        i64::try_from(d.sequence).unwrap_or(i64::MAX),
                        i64::try_from(d.issued_at).unwrap_or(i64::MAX),
                        i64::try_from(d.expires_at).unwrap_or(i64::MAX),
                        d.revoked_at.map(|v| i64::try_from(v).unwrap_or(i64::MAX)),
                        d.signature,
                    ],
                ).map_err(|error| database_error("failed to save delegation", error))?;
            }
            Ok(())
        })
    }

    /// Load trust delegations from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the delegations cannot be read.
    pub fn load_trust_delegations(&self) -> Result<Vec<wot::TrustDelegation>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare(
                    "SELECT delegator, delegate, scope, max_depth, sequence, issued_at, expires_at, revoked_at, signature
                 FROM trust_delegations ORDER BY issued_at",
                )
                .map_err(|error| database_error("failed to prepare delegations query", error))?;
            let delegations = stmt
                .query_map([], |row| {
                    let scope: Option<Vec<u8>> = row.get(2)?;
                    let hash = scope.and_then(|b| {
                        let arr: [u8; 32] = b.try_into().ok()?;
                        Some(Hash::from(arr))
                    });
                    let max_depth: Option<i64> = row.get(3)?;
                    let revoked_at: Option<i64> = row.get(7)?;
                    Ok(wot::TrustDelegation {
                        delegator: row.get::<_, String>(0)?,
                        delegate: row.get::<_, String>(1)?,
                        scope: hash,
                        max_depth: max_depth.map(|v| u32::try_from(v).unwrap_or(u32::MAX)),
                        sequence: u64::try_from(row.get::<_, i64>(4)?).unwrap_or_default(),
                        issued_at: u64::try_from(row.get::<_, i64>(5)?).unwrap_or_default(),
                        expires_at: u64::try_from(row.get::<_, i64>(6)?).unwrap_or_default(),
                        revoked_at: revoked_at.map(|v| u64::try_from(v).unwrap_or(u64::MAX)),
                        signature: row.get(8)?,
                    })
                })
                .map_err(|error| database_error("failed to query delegations", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read delegation rows", error))?;
            Ok(delegations)
        })
    }

    /// Save moderation records to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the records cannot be persisted.
    pub fn save_moderation_records(&self, records: &[wot::ModerationRecord]) -> Result<()> {
        self.with_connection(|connection| {
            connection.execute("DELETE FROM moderation_records_v2", [])
                .map_err(|error| database_error("failed to clear moderation records", error))?;
            for r in records {
                let scope_json = serde_json::to_string(&r.scope)
                    .map_err(|error| database_error("failed to serialize scope", error))?;
                connection.execute(
                    "INSERT INTO moderation_records_v2(content_hash, moderator, action, scope_json, sequence, created_at, reason, signature)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        r.content.as_bytes().to_vec(), r.moderator,
                        format!("{:?}", r.action), scope_json,
                        i64::try_from(r.sequence).unwrap_or(i64::MAX),
                        i64::try_from(r.created_at).unwrap_or(i64::MAX),
                        r.reason,
                        r.signature,
                    ],
                ).map_err(|error| database_error("failed to save moderation record", error))?;
            }
            Ok(())
        })
    }

    /// Load moderation records from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the records cannot be read.
    pub fn load_moderation_records(&self) -> Result<Vec<wot::ModerationRecord>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare(
                    "SELECT content_hash, moderator, action, scope_json, sequence, created_at, reason, signature
                 FROM moderation_records_v2 ORDER BY created_at",
                )
                .map_err(|error| database_error("failed to prepare moderation query", error))?;
            let records = stmt
                .query_map([], |row| {
                    let hash_bytes: Vec<u8> = row.get(0)?;
                    let arr: [u8; 32] = hash_bytes.try_into().map_err(|error: Vec<u8>| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid hash length",
                            format!("expected 32 bytes, got {}", error.len()),
                        )))
                    })?;
                    let scope_json: String = row.get(3)?;
                    let scope: wot::ModerationScope = serde_json::from_str(&scope_json)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                    let action_str: String = row.get(2)?;
                    let action = match action_str.as_str() {
                        "Show" => wot::ModerationAction::Show,
                        "Hide" => wot::ModerationAction::Hide,
                        "Warn" => wot::ModerationAction::Warn,
                        "Quarantine" => wot::ModerationAction::Quarantine,
                        "Restore" => wot::ModerationAction::Restore,
                        _ => {
                            return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                                SyncwebError::operation("unknown moderation action", action_str),
                            )));
                        }
                    };
                    Ok(wot::ModerationRecord {
                        content: Hash::from(arr),
                        moderator: row.get::<_, String>(1)?,
                        action,
                        scope,
                        sequence: u64::try_from(row.get::<_, i64>(4)?).unwrap_or_default(),
                        created_at: u64::try_from(row.get::<_, i64>(5)?).unwrap_or_default(),
                        reason: row.get::<_, String>(6)?,
                        signature: row.get(7)?,
                    })
                })
                .map_err(|error| database_error("failed to query moderation records", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read moderation rows", error))?;
            Ok(records)
        })
    }

    /// Save attestations to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the attestations cannot be persisted.
    pub fn save_attestations(&self, attestations: &[wot::Attestation]) -> Result<()> {
        self.with_connection(|connection| {
            connection.execute("DELETE FROM attestations_v2", [])
                .map_err(|error| database_error("failed to clear attestations", error))?;
            for a in attestations {
                let (kind, kind_other) = match &a.kind {
                    wot::AttestationKind::License => ("license", None),
                    wot::AttestationKind::Provenance => ("provenance", None),
                    wot::AttestationKind::Derivative => ("derivative", None),
                    wot::AttestationKind::Other(v) => ("other", Some(v.as_str())),
                };
                connection.execute(
                    "INSERT INTO attestations_v2(content_hash, issuer, kind, kind_other, value, sequence, issued_at, signature)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        a.content.as_bytes().to_vec(), a.issuer,
                        kind, kind_other, a.value,
                        i64::try_from(a.sequence).unwrap_or(i64::MAX),
                        i64::try_from(a.issued_at).unwrap_or(i64::MAX),
                        a.signature,
                    ],
                ).map_err(|error| database_error("failed to save attestation", error))?;
            }
            Ok(())
        })
    }

    /// Load attestations from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the attestations cannot be read.
    pub fn load_attestations(&self) -> Result<Vec<wot::Attestation>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare(
                    "SELECT content_hash, issuer, kind, kind_other, value, sequence, issued_at, signature
                 FROM attestations_v2 ORDER BY issued_at",
                )
                .map_err(|error| database_error("failed to prepare attestations query", error))?;
            let attestations = stmt
                .query_map([], |row| {
                    let hash_bytes: Vec<u8> = row.get(0)?;
                    let arr: [u8; 32] = hash_bytes.try_into().map_err(|error: Vec<u8>| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid hash length",
                            format!("expected 32 bytes, got {}", error.len()),
                        )))
                    })?;
                    let kind_str: String = row.get(2)?;
                    let kind_other: Option<String> = row.get(3)?;
                    let kind = match kind_str.as_str() {
                        "license" => wot::AttestationKind::License,
                        "provenance" => wot::AttestationKind::Provenance,
                        "derivative" => wot::AttestationKind::Derivative,
                        "other" => wot::AttestationKind::Other(kind_other.unwrap_or_default()),
                        _ => {
                            return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                                SyncwebError::operation("unknown attestation kind", kind_str),
                            )));
                        }
                    };
                    Ok(wot::Attestation {
                        content: Hash::from(arr),
                        issuer: row.get::<_, String>(1)?,
                        kind,
                        value: row.get::<_, String>(4)?,
                        sequence: u64::try_from(row.get::<_, i64>(5)?).unwrap_or_default(),
                        issued_at: u64::try_from(row.get::<_, i64>(6)?).unwrap_or_default(),
                        signature: row.get(7)?,
                    })
                })
                .map_err(|error| database_error("failed to query attestations", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read attestation rows", error))?;
            Ok(attestations)
        })
    }

    /// Save content reports to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the reports cannot be persisted.
    pub fn save_content_reports(&self, reports: &[ReportRecord]) -> Result<()> {
        self.with_connection(|connection| {
            connection
                .execute("DELETE FROM content_reports_v2", [])
                .map_err(|error| database_error("failed to clear content reports", error))?;
            for r in reports {
                let reporter = r.reporter.as_deref().unwrap_or("cli");
                connection
                    .execute(
                        "INSERT INTO content_reports_v2(content_hash, reporter, reason, scope, created_at, signature)
                     VALUES (?1, ?2, ?3, 'global', ?4, ?5)",
                        params![
                            r.content.as_bytes().to_vec(),
                            reporter,
                            r.reason,
                            i64::try_from(r.created_at).unwrap_or(i64::MAX),
                            r.signature,
                        ],
                    )
                    .map_err(|error| database_error("failed to save content report", error))?;
            }
            Ok(())
        })
    }

    /// Load content reports from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the reports cannot be read.
    pub fn load_content_reports(&self) -> Result<Vec<ReportRecord>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare(
                    "SELECT content_hash, reason, created_at, reporter, signature
                     FROM content_reports_v2 ORDER BY created_at",
                )
                .map_err(|error| database_error("failed to prepare reports query", error))?;
            let reports = stmt
                .query_map([], |row| {
                    let hash_bytes: Vec<u8> = row.get(0)?;
                    let arr: [u8; 32] = hash_bytes.try_into().map_err(|error: Vec<u8>| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid hash length",
                            format!("expected 32 bytes, got {}", error.len()),
                        )))
                    })?;
                    Ok(ReportRecord {
                        content: Hash::from(arr),
                        reason: row.get::<_, String>(1)?,
                        created_at: u64::try_from(row.get::<_, i64>(2)?).unwrap_or_default(),
                        reporter: row.get::<_, Option<String>>(3)?,
                        signature: row.get::<_, Option<String>>(4)?,
                    })
                })
                .map_err(|error| database_error("failed to query content reports", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read report rows", error))?;
            Ok(reports)
        })
    }

    /// Save provider bans to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the bans cannot be persisted.
    pub fn save_provider_bans(&self, bans: &[resilience::BanRecord]) -> Result<()> {
        self.with_connection(|connection| {
            connection
                .execute("DELETE FROM provider_bans_v2", [])
                .map_err(|error| database_error("failed to clear provider bans", error))?;
            for b in bans {
                connection
                    .execute(
                        "INSERT INTO provider_bans_v2(provider, content_hash, banned_at, expires_at, reason, source)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            b.provider.to_string(),
                            b.hash.map(|h| h.as_bytes().to_vec()),
                            i64::try_from(b.banned_at).unwrap_or(i64::MAX),
                            b.expires_at.map(|v| i64::try_from(v).unwrap_or(i64::MAX)),
                            b.reason,
                            format!("{:?}", b.source),
                        ],
                    )
                    .map_err(|error| database_error("failed to save provider ban", error))?;
            }
            Ok(())
        })
    }

    /// Load provider bans from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the bans cannot be read.
    pub fn load_provider_bans(&self) -> Result<Vec<resilience::BanRecord>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare(
                    "SELECT provider, content_hash, banned_at, expires_at, reason, source
                 FROM provider_bans_v2 ORDER BY banned_at",
                )
                .map_err(|error| database_error("failed to prepare bans query", error))?;
            let bans = stmt
                .query_map([], |row| {
                    let provider_str: String = row.get(0)?;
                    let content_hash: Option<Vec<u8>> = row.get(1)?;
                    let source_str: String = row.get(5)?;
                    let provider = provider_str.parse::<PublicKey>().map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid provider",
                            error,
                        )))
                    })?;
                    let hash = content_hash.and_then(|b| {
                        let arr: [u8; 32] = b.try_into().ok()?;
                        Some(Hash::from(arr))
                    });
                    let source = match source_str.as_str() {
                        "Automated" => resilience::BanSource::Automated,
                        "WoT" => resilience::BanSource::WoT,
                        _ => resilience::BanSource::Manual,
                    };
                    Ok(resilience::BanRecord {
                        provider,
                        hash,
                        banned_at: u64::try_from(row.get::<_, i64>(2)?).unwrap_or_default(),
                        expires_at: row
                            .get::<_, Option<i64>>(3)?
                            .map(|v| u64::try_from(v).unwrap_or_default()),
                        reason: row.get::<_, String>(4)?,
                        source,
                    })
                })
                .map_err(|error| database_error("failed to query provider bans", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read ban rows", error))?;
            Ok(bans)
        })
    }

    /// Insert or replace a single provider lease.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease cannot be persisted.
    pub fn insert_provider_lease(&self, lease: &resilience::ProviderLease) -> Result<()> {
        self.with_connection(|connection| {
            connection.execute(
                "INSERT OR REPLACE INTO provider_leases(provider, content_hash, ticket, sequence, issued_at, expires_at, signature)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    lease.provider.to_string(),
                    lease.hash.as_bytes().to_vec(),
                    lease.ticket,
                    i64::try_from(lease.sequence).unwrap_or(i64::MAX),
                    i64::try_from(lease.issued_at).unwrap_or(i64::MAX),
                    i64::try_from(lease.expires_at).unwrap_or(i64::MAX),
                    lease.signature,
                ],
            )
            .map_err(|error| database_error("failed to insert provider lease", error))?;
            Ok(())
        })
    }

    /// Load active (non-expired) provider leases from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the leases cannot be read.
    pub fn load_active_leases(&self, now: u64) -> Result<Vec<resilience::ProviderLease>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare(
                    "SELECT provider, content_hash, ticket, sequence, issued_at, expires_at, signature
                 FROM provider_leases WHERE expires_at > ?1 ORDER BY issued_at",
                )
                .map_err(|error| database_error("failed to prepare active leases query", error))?;
            let leases = stmt
                .query_map(params![i64::try_from(now).unwrap_or(i64::MAX)], |row| {
                    let provider_str: String = row.get(0)?;
                    let hash_bytes: Vec<u8> = row.get(1)?;
                    let arr: [u8; 32] = hash_bytes.try_into().map_err(|error: Vec<u8>| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid hash length",
                            format!("expected 32 bytes, got {}", error.len()),
                        )))
                    })?;
                    let provider = provider_str.parse::<PublicKey>().map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid provider",
                            error,
                        )))
                    })?;
                    Ok(resilience::ProviderLease {
                        hash: Hash::from(arr),
                        provider,
                        ticket: row.get(2)?,
                        sequence: u64::try_from(row.get::<_, i64>(3)?).unwrap_or_default(),
                        issued_at: u64::try_from(row.get::<_, i64>(4)?).unwrap_or_default(),
                        expires_at: u64::try_from(row.get::<_, i64>(5)?).unwrap_or_default(),
                        signature: row.get(6)?,
                    })
                })
                .map_err(|error| database_error("failed to query active leases", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read lease rows", error))?;
            Ok(leases)
        })
    }

    /// Insert or replace a single provider ban record.
    ///
    /// # Errors
    ///
    /// Returns an error if the ban cannot be persisted.
    pub fn insert_provider_ban(&self, ban: &resilience::BanRecord) -> Result<()> {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT OR REPLACE INTO provider_bans_v2(provider, content_hash, banned_at, expires_at, reason, source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        ban.provider.to_string(),
                        ban.hash.map(|h| h.as_bytes().to_vec()),
                        i64::try_from(ban.banned_at).unwrap_or(i64::MAX),
                        ban.expires_at.map(|v| i64::try_from(v).unwrap_or(i64::MAX)),
                        ban.reason,
                        format!("{:?}", ban.source),
                    ],
                )
                .map_err(|error| database_error("failed to insert provider ban", error))?;
            Ok(())
        })
    }

    /// Load active (non-expired) provider bans from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the bans cannot be read.
    pub fn load_active_bans(&self, now: u64) -> Result<Vec<resilience::BanRecord>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare(
                    "SELECT provider, content_hash, banned_at, expires_at, reason, source
                 FROM provider_bans_v2 WHERE expires_at IS NULL OR expires_at > ?1 ORDER BY banned_at",
                )
                .map_err(|error| database_error("failed to prepare active bans query", error))?;
            let bans = stmt
                .query_map(params![i64::try_from(now).unwrap_or(i64::MAX)], |row| {
                    let provider_str: String = row.get(0)?;
                    let content_hash: Option<Vec<u8>> = row.get(1)?;
                    let source_str: String = row.get(5)?;
                    let provider = provider_str.parse::<PublicKey>().map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid provider",
                            error,
                        )))
                    })?;
                    let hash = content_hash.and_then(|b| {
                        let arr: [u8; 32] = b.try_into().ok()?;
                        Some(Hash::from(arr))
                    });
                    let source = match source_str.as_str() {
                        "Automated" => resilience::BanSource::Automated,
                        "WoT" => resilience::BanSource::WoT,
                        _ => resilience::BanSource::Manual,
                    };
                    Ok(resilience::BanRecord {
                        provider,
                        hash,
                        banned_at: u64::try_from(row.get::<_, i64>(2)?).unwrap_or_default(),
                        expires_at: row
                            .get::<_, Option<i64>>(3)?
                            .map(|v| u64::try_from(v).unwrap_or_default()),
                        reason: row.get::<_, String>(4)?,
                        source,
                    })
                })
                .map_err(|error| database_error("failed to query active provider bans", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read ban rows", error))?;
            Ok(bans)
        })
    }

    /// Load all provider reputations from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the reputations cannot be read.
    pub fn load_all_reputations(&self) -> Result<HashMap<PublicKey, reputation::ProviderReputation>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare(
                    "SELECT provider, total_fetches, successful_fetches, failed_fetches,
                     consecutive_failures, last_success_at, last_failure_at
                 FROM provider_reputation",
                )
                .map_err(|error| database_error("failed to prepare reputations query", error))?;
            let rows = stmt
                .query_map([], |row| {
                    let provider_str: String = row.get(0)?;
                    let provider = provider_str.parse::<PublicKey>().map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid provider",
                            error,
                        )))
                    })?;
                    let rep = reputation::ProviderReputation {
                        provider,
                        total_fetches: u64::try_from(row.get::<_, i64>(1)?).unwrap_or_default(),
                        successful_fetches: u64::try_from(row.get::<_, i64>(2)?).unwrap_or_default(),
                        failed_fetches: u64::try_from(row.get::<_, i64>(3)?).unwrap_or_default(),
                        consecutive_failures: u32::try_from(row.get::<_, i64>(4)?).unwrap_or_default(),
                        last_success_at: row
                            .get::<_, Option<i64>>(5)?
                            .map(|v| u64::try_from(v).unwrap_or_default()),
                        last_failure_at: row
                            .get::<_, Option<i64>>(6)?
                            .map(|v| u64::try_from(v).unwrap_or_default()),
                    };
                    Ok((provider, rep))
                })
                .map_err(|error| database_error("failed to query reputations", error))?
                .collect::<std::result::Result<HashMap<_, _>, _>>()
                .map_err(|error| database_error("failed to read reputation rows", error))?;
            Ok(rows)
        })
    }

    /// Load all signal sequence numbers from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the sequences cannot be read.
    pub fn load_signal_sequences(&self) -> Result<HashMap<(PublicKey, PublicKey), u64>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare("SELECT reporter, provider, last_sequence FROM provider_signal_sequences")
                .map_err(|error| database_error("failed to prepare signal sequences query", error))?;
            let rows = stmt
                .query_map([], |row| {
                    let reporter_str: String = row.get(0)?;
                    let provider_str: String = row.get(1)?;
                    let reporter = reporter_str.parse::<PublicKey>().map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid reporter",
                            error,
                        )))
                    })?;
                    let provider = provider_str.parse::<PublicKey>().map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid provider",
                            error,
                        )))
                    })?;
                    Ok((
                        (reporter, provider),
                        u64::try_from(row.get::<_, i64>(2)?).unwrap_or_default(),
                    ))
                })
                .map_err(|error| database_error("failed to query signal sequences", error))?
                .collect::<std::result::Result<HashMap<_, _>, _>>()
                .map_err(|error| database_error("failed to read signal sequence rows", error))?;
            Ok(rows)
        })
    }

    /// Upsert a provider reputation record.
    ///
    /// # Errors
    ///
    /// Returns an error if the record cannot be persisted.
    pub fn upsert_reputation(
        &self,
        provider: &PublicKey,
        rep: &reputation::ProviderReputation,
        auto_ban_until: Option<u64>,
        auto_ban_count: u32,
    ) -> Result<()> {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT OR REPLACE INTO provider_reputation
                 (provider, total_fetches, successful_fetches, failed_fetches,
                  consecutive_failures, last_success_at, last_failure_at,
                  auto_ban_until, auto_ban_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        provider.to_string(),
                        i64::try_from(rep.total_fetches).unwrap_or(i64::MAX),
                        i64::try_from(rep.successful_fetches).unwrap_or(i64::MAX),
                        i64::try_from(rep.failed_fetches).unwrap_or(i64::MAX),
                        i64::from(rep.consecutive_failures),
                        rep.last_success_at.map(|v| i64::try_from(v).unwrap_or(i64::MAX)),
                        rep.last_failure_at.map(|v| i64::try_from(v).unwrap_or(i64::MAX)),
                        auto_ban_until.map(|v| i64::try_from(v).unwrap_or(i64::MAX)),
                        i64::from(auto_ban_count),
                    ],
                )
                .map_err(|error| database_error("failed to upsert reputation", error))?;
            Ok(())
        })
    }

    /// Delete stale reputation records last active before the given TTL.
    ///
    /// # Errors
    ///
    /// Returns an error if the deletion fails.
    pub fn delete_stale_reputations(&self, now: u64, ttl_secs: u64) -> Result<()> {
        self.with_connection(|connection| {
            let cutoff = i64::try_from(now.saturating_sub(ttl_secs)).unwrap_or(i64::MIN);
            connection
                .execute(
                    "DELETE FROM provider_reputation
                 WHERE (last_success_at IS NULL OR last_success_at < ?1)
                   AND (last_failure_at IS NULL OR last_failure_at < ?1)",
                    params![cutoff],
                )
                .map_err(|error| database_error("failed to delete stale reputations", error))?;
            Ok(())
        })
    }

    /// Load auto-ban information for all providers.
    ///
    /// Returns a map of provider to (`auto_ban_until`, `auto_ban_count`).
    ///
    /// # Errors
    ///
    /// Returns an error if the data cannot be read.
    pub fn load_auto_bans(&self) -> Result<HashMap<PublicKey, (Option<u64>, u32)>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare("SELECT provider, auto_ban_until, auto_ban_count FROM provider_reputation")
                .map_err(|error| database_error("failed to prepare auto bans query", error))?;
            let rows = stmt
                .query_map([], |row| {
                    let provider_str: String = row.get(0)?;
                    let provider = provider_str.parse::<PublicKey>().map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid provider",
                            error,
                        )))
                    })?;
                    Ok((
                        provider,
                        (
                            row.get::<_, Option<i64>>(1)?
                                .map(|v| u64::try_from(v).unwrap_or_default()),
                            u32::try_from(row.get::<_, i64>(2)?).unwrap_or_default(),
                        ),
                    ))
                })
                .map_err(|error| database_error("failed to query auto bans", error))?
                .collect::<std::result::Result<HashMap<_, _>, _>>()
                .map_err(|error| database_error("failed to read auto ban rows", error))?;
            Ok(rows)
        })
    }

    /// Upsert a provider signal sequence tracker.
    ///
    /// # Errors
    ///
    /// Returns an error if the sequence cannot be persisted.
    pub fn upsert_signal_sequence(&self, reporter: &PublicKey, provider: &PublicKey, sequence: u64) -> Result<()> {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT OR REPLACE INTO provider_signal_sequences(reporter, provider, last_sequence)
                 VALUES (?1, ?2, ?3)",
                    params![
                        reporter.to_string(),
                        provider.to_string(),
                        i64::try_from(sequence).unwrap_or(i64::MAX),
                    ],
                )
                .map_err(|error| database_error("failed to upsert signal sequence", error))?;
            Ok(())
        })
    }

    /// Save provider trust records to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the records cannot be persisted.
    pub fn save_provider_trust_records(&self, records: &[wot::ProviderTrustRecord]) -> Result<()> {
        self.with_connection(|connection| {
            connection.execute("DELETE FROM provider_trust_records_v2", [])
                .map_err(|error| database_error("failed to clear provider trust records", error))?;
            for r in records {
                let action_str = format!("{:?}", r.action);
                connection.execute(
                    "INSERT INTO provider_trust_records_v2(provider, action, scope, issuer, sequence, issued_at, expires_at, reason, signature)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        r.provider.to_string(), action_str,
                        r.scope.map(|h| h.as_bytes().to_vec()),
                        r.issuer,
                        i64::try_from(r.sequence).unwrap_or(i64::MAX),
                        i64::try_from(r.issued_at).unwrap_or(i64::MAX),
                        r.expires_at.map(|v| i64::try_from(v).unwrap_or(i64::MAX)),
                        r.reason,
                        r.signature,
                    ],
                ).map_err(|error| database_error("failed to save provider trust record", error))?;
            }
            Ok(())
        })
    }

    /// Load provider trust records from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the records cannot be read.
    pub fn load_provider_trust_records(&self) -> Result<Vec<wot::ProviderTrustRecord>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare(
                    "SELECT provider, action, scope, issuer, sequence, issued_at, expires_at, reason, signature
                 FROM provider_trust_records_v2 ORDER BY issued_at",
                )
                .map_err(|error| database_error("failed to prepare trust records query", error))?;
            let records = stmt
                .query_map([], |row| {
                    let provider_str: String = row.get(0)?;
                    let action_str: String = row.get(1)?;
                    let scope: Option<Vec<u8>> = row.get(2)?;
                    let provider = provider_str.parse::<PublicKey>().map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid provider",
                            error,
                        )))
                    })?;
                    let action = match action_str.as_str() {
                        "Trust" => wot::ProviderTrustAction::Trust,
                        "Distrust" => wot::ProviderTrustAction::Distrust,
                        "Vouch" => wot::ProviderTrustAction::Vouch,
                        "Warn" => wot::ProviderTrustAction::Warn,
                        _ => {
                            return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                                SyncwebError::operation("unknown trust action", action_str),
                            )));
                        }
                    };
                    let hash = scope.and_then(|b| {
                        let arr: [u8; 32] = b.try_into().ok()?;
                        Some(Hash::from(arr))
                    });
                    Ok(wot::ProviderTrustRecord {
                        provider,
                        action,
                        scope: hash,
                        issuer: row.get::<_, String>(3)?,
                        sequence: u64::try_from(row.get::<_, i64>(4)?).unwrap_or_default(),
                        issued_at: u64::try_from(row.get::<_, i64>(5)?).unwrap_or_default(),
                        expires_at: row
                            .get::<_, Option<i64>>(6)?
                            .map(|v| u64::try_from(v).unwrap_or_default()),
                        reason: row.get::<_, String>(7)?,
                        signature: row.get(8)?,
                    })
                })
                .map_err(|error| database_error("failed to query provider trust records", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read trust record rows", error))?;
            Ok(records)
        })
    }

    /// Save provider trust signals to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the signals cannot be persisted.
    pub fn save_provider_trust_signals(&self, signals: &[reputation::ProviderTrustSignal]) -> Result<()> {
        self.with_connection(|connection| {
            connection.execute("DELETE FROM provider_trust_signals_v2", [])
                .map_err(|error| database_error("failed to clear provider trust signals", error))?;
            for s in signals {
                let kind_str = format!("{:?}", s.signal);
                connection.execute(
                    "INSERT INTO provider_trust_signals_v2(reporter, provider, signal_kind, content_hash, sequence, timestamp, signature)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        s.reporter.to_string(), s.provider.to_string(),
                        kind_str,
                        s.hash.map(|h| h.as_bytes().to_vec()),
                        i64::try_from(s.sequence).unwrap_or(i64::MAX),
                        i64::try_from(s.timestamp).unwrap_or(i64::MAX),
                        s.signature,
                    ],
                ).map_err(|error| database_error("failed to save trust signal", error))?;
            }
            Ok(())
        })
    }

    /// Load provider trust signals from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the signals cannot be read.
    pub fn load_provider_trust_signals(&self) -> Result<Vec<reputation::ProviderTrustSignal>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare(
                    "SELECT reporter, provider, signal_kind, content_hash, sequence, timestamp, signature
                 FROM provider_trust_signals_v2 ORDER BY timestamp",
                )
                .map_err(|error| database_error("failed to prepare trust signals query", error))?;
            let signals = stmt
                .query_map([], |row| {
                    let reporter_str: String = row.get(0)?;
                    let provider_str: String = row.get(1)?;
                    let kind_str: String = row.get(2)?;
                    let content_hash: Option<Vec<u8>> = row.get(3)?;
                    let reporter = reporter_str.parse::<PublicKey>().map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid reporter",
                            error,
                        )))
                    })?;
                    let provider = provider_str.parse::<PublicKey>().map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(SyncwebError::operation(
                            "invalid provider",
                            error,
                        )))
                    })?;
                    let kind = match kind_str.as_str() {
                        "ObservedSuccess" => reputation::TrustSignalKind::ObservedSuccess,
                        "ObservedFailure" => reputation::TrustSignalKind::ObservedFailure,
                        "ObservedCorruption" => reputation::TrustSignalKind::ObservedCorruption,
                        _ => {
                            return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                                SyncwebError::operation("unknown signal kind", kind_str),
                            )));
                        }
                    };
                    let hash = content_hash.and_then(|b| {
                        let arr: [u8; 32] = b.try_into().ok()?;
                        Some(Hash::from(arr))
                    });
                    Ok(reputation::ProviderTrustSignal {
                        provider,
                        signal: kind,
                        hash,
                        reporter,
                        timestamp: u64::try_from(row.get::<_, i64>(5)?).unwrap_or_default(),
                        sequence: u64::try_from(row.get::<_, i64>(4)?).unwrap_or_default(),
                        signature: row.get(6)?,
                    })
                })
                .map_err(|error| database_error("failed to query trust signals", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read trust signal rows", error))?;
            Ok(signals)
        })
    }

    /// Save trust streams to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the streams cannot be persisted.
    pub fn save_trust_streams(&self, streams: &[String]) -> Result<()> {
        self.with_connection(|connection| {
            connection
                .execute("DELETE FROM trust_streams", [])
                .map_err(|error| database_error("failed to clear trust streams", error))?;
            let now = now_seconds();
            for ns in streams {
                connection
                    .execute(
                        "INSERT INTO trust_streams(namespace, subscribed_at) VALUES (?1, ?2)",
                        params![ns, now],
                    )
                    .map_err(|error| database_error("failed to save trust stream", error))?;
            }
            Ok(())
        })
    }

    /// Load trust streams from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the streams cannot be read.
    pub fn load_trust_streams(&self) -> Result<Vec<String>> {
        self.with_connection(|connection| {
            let mut stmt = connection
                .prepare("SELECT namespace FROM trust_streams ORDER BY subscribed_at")
                .map_err(|error| database_error("failed to prepare trust streams query", error))?;
            let streams = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|error| database_error("failed to query trust streams", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| database_error("failed to read trust stream rows", error))?;
            Ok(streams)
        })
    }
}

/// The opt-in indexing service for synchronized folders.
#[derive(Clone)]
pub struct IndexingService {
    database: IndexingDatabase,
    events: broadcast::Sender<IndexingEvent>,
    tasks: Arc<Mutex<HashMap<NamespaceId, JoinHandle<()>>>>,
    denylist: denylist::DenylistService,
}

impl std::fmt::Debug for IndexingService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IndexingService")
            .field("database", &self.database)
            .finish_non_exhaustive()
    }
}

impl IndexingService {
    /// Start an indexing service backed by `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        Self::with_database(IndexingDatabase::open(path)?)
    }

    /// Start an in-memory indexing service.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot initialize the schema.
    pub fn in_memory() -> Result<Self> {
        Self::with_database(IndexingDatabase::in_memory()?)
    }

    /// Start a service from an already opened database.
    ///
    /// # Errors
    ///
    /// Returns an error if the denylist rules cannot be loaded from the database.
    pub fn with_database(database: IndexingDatabase) -> Result<Self> {
        let (events, _) = broadcast::channel(EVENT_CAPACITY);
        let denylist = denylist::DenylistService::with_database(database.clone())?;
        Ok(Self {
            database,
            events,
            tasks: Arc::new(Mutex::new(HashMap::new())),
            denylist,
        })
    }

    /// Create a catalog service backed by this indexing service.
    #[must_use]
    pub fn catalog_service(
        &self,
        docs: &crate::node::docs_engine::DocsEngine,
        blobs: &crate::node::blob_store::BlobStore,
        author: iroh_docs::AuthorId,
    ) -> CatalogService {
        CatalogService::new(self, docs, blobs, author)
    }

    /// Create a lease-based resilience service for this indexer.
    #[must_use]
    pub fn resilience_service(&self, config: resilience::ResilienceConfig) -> resilience::ResilienceService {
        resilience::ResilienceService::with_database(self.database.clone(), config)
    }

    /// Create a resilience service using this indexer's local `WoT` policy.
    #[must_use]
    pub fn resilience_service_with_wot(
        &self,
        config: resilience::ResilienceConfig,
        wot: wot::WotService,
    ) -> resilience::ResilienceService {
        resilience::ResilienceService::with_database(self.database.clone(), config).with_trust_policy(wot)
    }

    /// Create a local Web-of-Trust metadata service for this indexer.
    #[must_use]
    pub fn wot_service(&self, policy: wot::TrustPolicy) -> wot::WotService {
        wot::WotService::new(self, policy)
    }

    /// Return the thread-safe local denylist used by indexing hooks.
    #[must_use]
    pub fn denylist_service(&self) -> denylist::DenylistService {
        self.denylist.clone()
    }

    /// Alias for [`Self::denylist_service`].
    #[must_use]
    pub fn denylist(&self) -> denylist::DenylistService {
        self.denylist_service()
    }

    #[must_use]
    pub const fn database(&self) -> &IndexingDatabase {
        &self.database
    }

    #[must_use]
    pub const fn db(&self) -> &IndexingDatabase {
        &self.database
    }

    /// Search entries in folders enabled for local indexing.
    ///
    /// # Errors
    ///
    /// Returns an error if the query is invalid or `SQLite` cannot read results.
    pub fn search_local(&self, query: &str, limit: usize) -> Result<Vec<IndexedEntry>> {
        self.database.search(query, limit)
    }

    /// Search records imported from subscribed catalogs.
    ///
    /// # Errors
    ///
    /// Returns an error if the query is invalid or `SQLite` cannot read results.
    pub fn search_global(&self, query: &str, limit: usize) -> Result<Vec<CatalogRecord>> {
        self.database.search_catalogs(query, limit)
    }

    /// Search records imported from subscribed catalogs.
    ///
    /// This is the service-level search used by `indexing search`.
    ///
    /// # Errors
    ///
    /// Returns an error if the query is invalid or `SQLite` cannot read results.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<CatalogRecord>> {
        self.search_global(query, limit)
    }

    /// Subscribe to indexing and core-engine events.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<IndexingEvent> {
        self.events.subscribe()
    }

    /// Subscribe to indexing and core-engine events.
    #[must_use]
    pub fn subscribe_events(&self) -> broadcast::Receiver<IndexingEvent> {
        self.subscribe()
    }

    /// Opt a folder into indexing and begin consuming its document events.
    ///
    /// Existing document entries are indexed before the method returns.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder cannot be registered, read, or watched.
    pub async fn enable_folder(&self, folder: &SyncwebFolder) -> Result<IndexingHandle> {
        let namespace_id = folder.namespace_id();
        if self
            .tasks
            .lock()
            .map_err(|error| SyncwebError::operation("indexing task lock poisoned", error))?
            .get(&namespace_id)
            .is_some_and(|task| !task.is_finished())
        {
            return Ok(IndexingHandle { namespace_id });
        }

        let live_events = folder.docs_engine().watch(folder.doc()).await?;
        self.database.enable_folder(namespace_id, &namespace_id.to_string())?;
        send_event(&self.events, IndexingEvent::FolderEnabled { namespace_id });

        for entry in folder.docs_engine().list_latest(folder.doc()).await? {
            let indexed = self.index_entry(namespace_id, &entry)?;
            send_event(&self.events, IndexingEvent::EntryIndexed(indexed));
        }

        let database = self.database.clone();
        let events = self.events.clone();
        let task = tokio::spawn(async move {
            consume_folder_events(namespace_id, live_events, database, events).await;
        });
        self.tasks
            .lock()
            .map_err(|error| SyncwebError::operation("indexing task lock poisoned", error))?
            .insert(namespace_id, task);
        Ok(IndexingHandle { namespace_id })
    }

    /// Stop indexing a folder and remove its local index entries.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder index cannot be removed.
    pub async fn disable_folder(&self, namespace_id: NamespaceId) -> Result<()> {
        let task_handle = self
            .tasks
            .lock()
            .map_err(|error| SyncwebError::operation("indexing task lock poisoned", error))?
            .remove(&namespace_id);
        if let Some(task) = task_handle {
            task.abort();
        }
        let database = self.database.clone();
        tokio::task::spawn_blocking(move || database.disable_folder(namespace_id))
            .await
            .map_err(|error| SyncwebError::operation("indexing folder removal task failed", error))??;
        send_event(&self.events, IndexingEvent::FolderDisabled { namespace_id });
        Ok(())
    }

    /// Return whether an indexing task is active for a folder.
    ///
    /// # Errors
    ///
    /// Returns an error if the task registry lock is poisoned.
    pub fn is_folder_enabled(&self, namespace_id: NamespaceId) -> Result<bool> {
        let active_task = self
            .tasks
            .lock()
            .map_err(|error| SyncwebError::operation("indexing task lock poisoned", error))?
            .get(&namespace_id)
            .is_some_and(|task| !task.is_finished());
        Ok(active_task || self.database.is_folder_enabled(namespace_id)?)
    }

    /// Index an entry supplied by another catalog or an application.
    ///
    /// This method does not require a folder task, but the namespace must
    /// already be enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if the namespace is not enabled or the entry cannot be
    /// stored.
    pub fn index_entry(&self, namespace_id: NamespaceId, entry: &Entry) -> Result<IndexedEntry> {
        if !self.database.is_folder_enabled(namespace_id)? {
            return Err(SyncwebError::FolderNotFound(namespace_id.to_string()));
        }
        self.database
            .upsert_entry(namespace_id, entry.key(), entry.content_hash(), entry.content_len())
    }
}

async fn consume_folder_events(
    namespace_id: NamespaceId,
    mut live_events: impl n0_future::Stream<Item = Result<LiveEvent>> + Send + Unpin + 'static,
    database: IndexingDatabase,
    events: broadcast::Sender<IndexingEvent>,
) {
    while let Some(event_result) = live_events.next().await {
        let event = match event_result {
            Ok(event) => event,
            Err(error) => {
                send_event(
                    &events,
                    IndexingEvent::Error {
                        namespace_id,
                        message: error.to_string(),
                    },
                );
                break;
            }
        };
        match event {
            LiveEvent::InsertLocal { entry } | LiveEvent::InsertRemote { entry, .. } => {
                match database.upsert_entry(namespace_id, entry.key(), entry.content_hash(), entry.content_len()) {
                    Ok(indexed) => send_event(&events, IndexingEvent::EntryIndexed(indexed)),
                    Err(error) => {
                        send_event(
                            &events,
                            IndexingEvent::Error {
                                namespace_id,
                                message: error.to_string(),
                            },
                        );
                    }
                }
            }
            LiveEvent::ContentReady { hash } => send_event(
                &events,
                IndexingEvent::Core {
                    namespace_id,
                    event: CoreIndexingEvent::ContentReady { hash },
                },
            ),
            LiveEvent::PendingContentReady => send_event(
                &events,
                IndexingEvent::Core {
                    namespace_id,
                    event: CoreIndexingEvent::PendingContentReady,
                },
            ),
            LiveEvent::NeighborUp(_) => send_event(
                &events,
                IndexingEvent::Core {
                    namespace_id,
                    event: CoreIndexingEvent::NeighborUp,
                },
            ),
            LiveEvent::NeighborDown(_) => send_event(
                &events,
                IndexingEvent::Core {
                    namespace_id,
                    event: CoreIndexingEvent::NeighborDown,
                },
            ),
            LiveEvent::SyncFinished(_) => send_event(
                &events,
                IndexingEvent::Core {
                    namespace_id,
                    event: CoreIndexingEvent::SyncFinished,
                },
            ),
        }
    }
}

fn indexed_entry_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IndexedEntry> {
    let namespace = row.get::<_, String>(0)?;
    let namespace_id = NamespaceId::from_str(&namespace).map_err(|error| {
        let conversion_error = std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string());
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(conversion_error))
    })?;
    let key = row.get(1)?;
    let hash_bytes = row.get::<_, Vec<u8>>(2)?;
    let hash_array = <[u8; 32]>::try_from(hash_bytes.as_slice())
        .map_err(|error| rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Blob, Box::new(error)))?;
    let size_value = row.get::<_, i64>(3)?;
    let size = u64::try_from(size_value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Integer, Box::new(error))
    })?;
    Ok(IndexedEntry {
        namespace_id,
        key,
        hash: Hash::from_bytes(hash_array),
        size,
    })
}

fn catalog_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CatalogRecord> {
    let catalog_namespace = row.get::<_, String>(0)?;
    let catalog_namespace_id = NamespaceId::from_str(&catalog_namespace).map_err(|error| {
        let conversion_error = std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string());
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(conversion_error))
    })?;
    let folder_namespace = row.get::<_, String>(1)?;
    let folder_namespace_id = NamespaceId::from_str(&folder_namespace).map_err(|error| {
        let conversion_error = std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string());
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(conversion_error))
    })?;
    let hash_bytes = row.get::<_, Vec<u8>>(3)?;
    let hash_array = <[u8; 32]>::try_from(hash_bytes.as_slice())
        .map_err(|error| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Blob, Box::new(error)))?;
    let size_value = row.get::<_, i64>(4)?;
    let size = u64::try_from(size_value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Integer, Box::new(error))
    })?;
    let tags_json = row.get::<_, String>(7)?;
    let tags = serde_json::from_str(&tags_json)
        .map_err(|error| rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(error)))?;
    Ok(CatalogRecord {
        catalog_namespace_id,
        folder_namespace_id,
        key: row.get(2)?,
        hash: Hash::from_bytes(hash_array),
        size,
        folder_name: row.get(5)?,
        title: row.get(6)?,
        tags,
        publisher: row.get(8)?,
    })
}

fn wot_metadata_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<wot::MetadataEntry> {
    let hash_bytes = row.get::<_, Vec<u8>>(0)?;
    let hash_array = <[u8; 32]>::try_from(hash_bytes.as_slice())
        .map_err(|error| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(error)))?;
    let sequence_value = row.get::<_, i64>(4)?;
    let sequence = u64::try_from(sequence_value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Integer, Box::new(error))
    })?;
    let created_at_value = row.get::<_, i64>(5)?;
    let created_at = u64::try_from(created_at_value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Integer, Box::new(error))
    })?;
    Ok(wot::MetadataEntry {
        content: Hash::from_bytes(hash_array),
        key: row.get(1)?,
        value: row.get(2)?,
        author: row.get(3)?,
        sequence,
        created_at,
        signature: row.get(6)?,
    })
}

const SCHEMA_PART1: &str = "PRAGMA journal_mode = WAL;
     CREATE TABLE IF NOT EXISTS index_metadata (
         key TEXT PRIMARY KEY NOT NULL,
         value TEXT NOT NULL
     );
     CREATE TABLE IF NOT EXISTS indexed_folders (
         namespace_id TEXT PRIMARY KEY NOT NULL,
         label TEXT NOT NULL,
         enabled_at INTEGER NOT NULL
     );
     CREATE TABLE IF NOT EXISTS indexed_entries (
         id INTEGER PRIMARY KEY,
         namespace_id TEXT NOT NULL REFERENCES indexed_folders(namespace_id) ON DELETE CASCADE,
         entry_key BLOB NOT NULL,
         content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
         content_len INTEGER NOT NULL CHECK(content_len >= 0),
         updated_at INTEGER NOT NULL,
         UNIQUE(namespace_id, entry_key)
     );
     CREATE INDEX IF NOT EXISTS indexed_entries_namespace
         ON indexed_entries(namespace_id);
     CREATE VIRTUAL TABLE IF NOT EXISTS indexed_entries_fts USING fts5(
         namespace_id UNINDEXED,
         entry_key,
         title,
         tags,
         tokenize = 'unicode61'
     );
     CREATE TABLE IF NOT EXISTS indexed_catalogs (
         namespace_id TEXT PRIMARY KEY NOT NULL,
         label TEXT NOT NULL,
         subscribed_at INTEGER NOT NULL
     );
     CREATE TABLE IF NOT EXISTS indexed_catalog_entries (
         id INTEGER PRIMARY KEY,
         catalog_namespace_id TEXT NOT NULL REFERENCES indexed_catalogs(namespace_id) ON DELETE CASCADE,
         folder_namespace_id TEXT NOT NULL,
         entry_key BLOB NOT NULL,
         content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
         content_len INTEGER NOT NULL CHECK(content_len >= 0),
         folder_name TEXT NOT NULL,
         title TEXT NOT NULL,
         tags TEXT NOT NULL,
         publisher TEXT NOT NULL,
         updated_at INTEGER NOT NULL,
         UNIQUE(catalog_namespace_id, folder_namespace_id, entry_key)
     );
     CREATE INDEX IF NOT EXISTS indexed_catalog_entries_catalog
         ON indexed_catalog_entries(catalog_namespace_id);
     CREATE VIRTUAL TABLE IF NOT EXISTS indexed_catalog_entries_fts USING fts5(
         catalog_namespace_id UNINDEXED,
         folder_namespace_id UNINDEXED,
         entry_key,
         folder_name,
         title,
         tags,
         publisher,
         content_hash,
         tokenize = 'unicode61'
     );
     CREATE TABLE IF NOT EXISTS wot_metadata (
         id INTEGER PRIMARY KEY,
         content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
         metadata_key TEXT NOT NULL,
         metadata_value TEXT NOT NULL,
         author TEXT NOT NULL,
         sequence INTEGER NOT NULL CHECK(sequence > 0),
         created_at INTEGER NOT NULL CHECK(created_at >= 0),
         signature TEXT NOT NULL,
         UNIQUE(content_hash, metadata_key, author, sequence)
     );
     CREATE INDEX IF NOT EXISTS wot_metadata_content
         ON wot_metadata(content_hash);
     CREATE VIRTUAL TABLE IF NOT EXISTS wot_metadata_fts USING fts5(
         content_hash UNINDEXED,
         metadata_key,
         metadata_value,
         author,
         tokenize = 'unicode61'
     );";

const SCHEMA_PART2: &str = "CREATE TABLE IF NOT EXISTS stable_links (
     link TEXT PRIMARY KEY NOT NULL,
     kind TEXT NOT NULL,
     publisher TEXT,
     alias TEXT,
     content_hash BLOB CHECK(content_hash IS NULL OR length(content_hash) = 32),
     sequence INTEGER NOT NULL CHECK(sequence >= 0),
     version TEXT,
     payload BLOB NOT NULL,
     updated_at INTEGER NOT NULL
 );
CREATE TABLE IF NOT EXISTS link_mirrors (
link TEXT NOT NULL,
provider TEXT NOT NULL,
ticket TEXT NOT NULL,
priority INTEGER NOT NULL DEFAULT 0,
updated_at INTEGER NOT NULL,
PRIMARY KEY(link, provider)
);
CREATE TABLE IF NOT EXISTS revoked_links (
     manifest BLOB NOT NULL CHECK(length(manifest) = 32),
     capability TEXT NOT NULL,
     revoked_at INTEGER NOT NULL,
     PRIMARY KEY(manifest, capability)
);
     CREATE TABLE IF NOT EXISTS denylist_rules (
         rule_type TEXT NOT NULL,
         rule_value BLOB NOT NULL,
         namespace_id TEXT,
         updated_at INTEGER NOT NULL,
         PRIMARY KEY(rule_type, rule_value, namespace_id)
     );
     CREATE TABLE IF NOT EXISTS filter_lists (
         namespace_id TEXT PRIMARY KEY NOT NULL,
         sequence INTEGER NOT NULL CHECK(sequence > 0),
         publisher TEXT NOT NULL,
         payload BLOB NOT NULL,
         updated_at INTEGER NOT NULL
     );
     CREATE TABLE IF NOT EXISTS moderation_records (
         content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
         scope TEXT NOT NULL,
         sequence INTEGER NOT NULL CHECK(sequence > 0),
         payload BLOB NOT NULL,
         updated_at INTEGER NOT NULL,
         PRIMARY KEY(content_hash, scope)
     );
     CREATE TABLE IF NOT EXISTS provider_leases (
         provider TEXT NOT NULL,
         content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
         ticket TEXT NOT NULL,
         sequence INTEGER NOT NULL,
         issued_at INTEGER NOT NULL,
         expires_at INTEGER NOT NULL,
         signature TEXT,
         PRIMARY KEY(provider, content_hash)
     );
     CREATE INDEX IF NOT EXISTS idx_provider_leases_hash ON provider_leases(content_hash);";

const SCHEMA_PART3: &str = "CREATE TABLE IF NOT EXISTS provider_reputation (
     provider TEXT PRIMARY KEY NOT NULL,
     total_fetches INTEGER NOT NULL DEFAULT 0,
     successful_fetches INTEGER NOT NULL DEFAULT 0,
     failed_fetches INTEGER NOT NULL DEFAULT 0,
     consecutive_failures INTEGER NOT NULL DEFAULT 0,
     last_success_at INTEGER,
     last_failure_at INTEGER,
     auto_ban_until INTEGER,
     auto_ban_count INTEGER NOT NULL DEFAULT 0
 );
 CREATE TABLE IF NOT EXISTS provider_signal_sequences (
     reporter TEXT NOT NULL,
     provider TEXT NOT NULL,
     last_sequence INTEGER NOT NULL,
     PRIMARY KEY(reporter, provider)
 );
 CREATE TABLE IF NOT EXISTS trust_delegations (
     delegator TEXT NOT NULL,
     delegate TEXT NOT NULL,
     scope BLOB CHECK(scope IS NULL OR length(scope) = 32),
     max_depth INTEGER,
     sequence INTEGER NOT NULL,
     issued_at INTEGER NOT NULL,
     expires_at INTEGER NOT NULL,
     revoked_at INTEGER,
     signature TEXT,
     PRIMARY KEY(delegator, delegate, sequence)
 );
     CREATE TABLE IF NOT EXISTS moderation_records_v2 (
         id INTEGER PRIMARY KEY AUTOINCREMENT,
         content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
         moderator TEXT NOT NULL,
         action TEXT NOT NULL,
         scope_json TEXT NOT NULL,
         sequence INTEGER NOT NULL,
         created_at INTEGER NOT NULL,
         reason TEXT NOT NULL,
         signature TEXT
     );
     CREATE TABLE IF NOT EXISTS attestations_v2 (
         content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
         issuer TEXT NOT NULL,
         kind TEXT NOT NULL,
         kind_other TEXT,
         value TEXT NOT NULL,
         sequence INTEGER NOT NULL,
         issued_at INTEGER NOT NULL,
         signature TEXT,
         PRIMARY KEY(content_hash, issuer, kind)
     );
      CREATE TABLE IF NOT EXISTS content_reports_v2 (
          content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
          reporter TEXT NOT NULL,
          reason TEXT NOT NULL,
          scope TEXT NOT NULL DEFAULT 'global',
          created_at INTEGER NOT NULL,
          signature TEXT,
          PRIMARY KEY(content_hash, reporter, reason)
      );
     CREATE TABLE IF NOT EXISTS provider_trust_records_v2 (
         id INTEGER PRIMARY KEY AUTOINCREMENT,
         provider TEXT NOT NULL,
         action TEXT NOT NULL,
         scope BLOB CHECK(scope IS NULL OR length(scope) = 32),
         issuer TEXT NOT NULL,
         sequence INTEGER NOT NULL,
         issued_at INTEGER NOT NULL,
         expires_at INTEGER,
         reason TEXT NOT NULL,
         signature TEXT
     );
     CREATE TABLE IF NOT EXISTS provider_bans_v2 (
         provider TEXT NOT NULL,
         content_hash BLOB CHECK(content_hash IS NULL OR length(content_hash) = 32),
         banned_at INTEGER NOT NULL,
         expires_at INTEGER,
         reason TEXT NOT NULL,
         source TEXT NOT NULL,
         PRIMARY KEY(provider, content_hash)
     );
     CREATE TABLE IF NOT EXISTS provider_trust_signals_v2 (
         reporter TEXT NOT NULL,
         provider TEXT NOT NULL,
         signal_kind TEXT NOT NULL,
         content_hash BLOB CHECK(content_hash IS NULL OR length(content_hash) = 32),
         sequence INTEGER NOT NULL,
         timestamp INTEGER NOT NULL,
         signature TEXT,
         PRIMARY KEY(reporter, provider, sequence)
     );
     CREATE INDEX IF NOT EXISTS idx_trust_signals_provider_v2 ON provider_trust_signals_v2(provider);
     CREATE INDEX IF NOT EXISTS idx_trust_signals_ts_v2 ON provider_trust_signals_v2(timestamp);
     CREATE TABLE IF NOT EXISTS trust_streams (
         namespace TEXT PRIMARY KEY,
         subscribed_at INTEGER NOT NULL
     );";

fn initialize_connection(connection: &Connection) -> Result<()> {
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|error| database_error("failed to configure indexing database", error))?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| database_error("failed to enable indexing foreign keys", error))?;
    initialize_schema(connection)?;
    migrate_database(connection)?;
    connection
        .execute(
            "INSERT INTO index_metadata(key, value)
             VALUES ('schema_version', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [SCHEMA_VERSION],
        )
        .map_err(|error| database_error("failed to persist indexing schema version", error))?;
    Ok(())
}

/// Run one-shot migrations from earlier schema versions.
fn migrate_database(connection: &Connection) -> Result<()> {
    let version: String = connection
        .query_row(
            "SELECT value FROM index_metadata WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();
    match version.as_str() {
        "" | "1" => {
            // Migration: add signature column to content_reports_v2
            let has_col: bool = connection
                .query_row(
                    "SELECT COUNT(*) > 0 FROM pragma_table_info('content_reports_v2') WHERE name='signature'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(false);
            if !has_col {
                connection
                    .execute_batch("ALTER TABLE content_reports_v2 ADD COLUMN signature TEXT")
                    .map_err(|error| database_error("failed to add signature column to content_reports_v2", error))?;
            }
        }
        "2" => {
            // Migration: add revoked_links table
            connection
                .execute_batch(
                    "CREATE TABLE IF NOT EXISTS revoked_links (
                        manifest BLOB NOT NULL CHECK(length(manifest) = 32),
                        capability TEXT NOT NULL,
                        revoked_at INTEGER NOT NULL,
                        PRIMARY KEY(manifest, capability)
                    )",
                )
                .map_err(|error| database_error("failed to create revoked_links table", error))?;
        }
        _ => {}
    }
    Ok(())
}

fn initialize_schema(connection: &Connection) -> Result<()> {
    for part in [SCHEMA_PART1, SCHEMA_PART2, SCHEMA_PART3] {
        connection
            .execute_batch(part)
            .map_err(|error| database_error("failed to initialize indexing database schema", error))?;
    }
    Ok(())
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
}

fn database_error(context: &'static str, error: impl std::fmt::Display) -> SyncwebError {
    SyncwebError::operation(context, error)
}

fn send_event(events: &broadcast::Sender<IndexingEvent>, event: IndexingEvent) {
    let _ = events.send(event);
}

/// Deterministic gossip topic for broadcasting signed moderation reports.
pub const REPORT_GOSSIP_TOPIC: &[u8] = b"syncweb/reports/v1";

/// A content report stored in the indexing database.
///
/// Reports may be cryptographically signed by the reporter's node key.
/// Unsigned reports (reporter == `None`, signature == `None`) are legacy
/// records from the pre-signing era or from CLI usage without a node key.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReportRecord {
    pub content: Hash,
    pub reason: String,
    pub created_at: u64,
    pub reporter: Option<String>,
    pub signature: Option<String>,
}

impl ReportRecord {
    #[must_use]
    pub const fn new(content: Hash, reason: String, created_at: u64) -> Self {
        Self {
            content,
            reason,
            created_at,
            reporter: None,
            signature: None,
        }
    }

    /// Sign this report with the node's Ed25519 signing key.
    ///
    /// Sets `reporter` to the hex-encoded verifying key and `signature` to
    /// the hex-encoded Ed25519 signature over the canonical payload.
    ///
    /// # Errors
    ///
    /// Returns an error if the canonical signing payload cannot be serialized.
    pub fn sign_with(mut self, signing_key: &SigningKey) -> Result<Self> {
        let payload = self.signing_payload()?;
        let sig = signing_key.sign(&payload);
        self.reporter = Some(hex::encode(signing_key.verifying_key().to_bytes()));
        self.signature = Some(hex::encode(sig.to_bytes()));
        Ok(self)
    }

    /// Verify the report's signature against the given public key.
    ///
    /// # Errors
    ///
    /// Returns an error if the signature is missing, malformed, or invalid.
    pub fn verify(&self, public_key: &VerifyingKey) -> Result<()> {
        let sig_hex = self
            .signature
            .as_ref()
            .ok_or_else(|| SyncwebError::MissingSignature("report has no signature".into()))?;
        let sig_bytes = hex::decode(sig_hex).map_err(|e| SyncwebError::operation("report signature hex decode", e))?;
        let sig_len = sig_bytes.len();
        let sig: [u8; 64] = sig_bytes.try_into().map_err(|_v| {
            SyncwebError::InvalidSignature(format!(
                "report signature has wrong length (expected 64, got {sig_len})"
            ))
        })?;
        let signature = Signature::from_bytes(&sig);
        public_key
            .verify(&self.signing_payload()?, &signature)
            .map_err(|e| SyncwebError::InvalidSignature(format!("report signature does not match: {e}")))
    }

    /// Canonical payload for signing: `(content_hash_bytes, reason, created_at)`.
    ///
    /// The reporter and signature fields are excluded — the reporter IS the
    /// signer, and the signature cannot sign itself.
    fn signing_payload(&self) -> Result<Vec<u8>> {
        let canonical = (self.content.as_bytes(), &self.reason, self.created_at);
        serde_json::to_vec(&canonical).map_err(|e| SyncwebError::operation("canonical report serialization", e))
    }
}

impl SignedGossipMessage for ReportRecord {
    fn verify_signature(&self) -> Result<()> {
        match &self.reporter {
            Some(reporter_hex) => {
                let bytes = hex::decode(reporter_hex).map_err(|e| SyncwebError::operation("decode reporter key", e))?;
                let key_len = bytes.len();
                let key_bytes: [u8; 32] = bytes.try_into().map_err(|_v| {
                    SyncwebError::InvalidSignature(format!(
                        "reporter key has wrong length (expected 32, got {key_len})"
                    ))
                })?;
                let public_key = VerifyingKey::from_bytes(&key_bytes)
                    .map_err(|e| SyncwebError::InvalidSignature(format!("invalid reporter key: {e}")))?;
                self.verify(&public_key)
            }
            None => {
                // Unsigned legacy report — accept without verification
                Ok(())
            }
        }
    }
}
