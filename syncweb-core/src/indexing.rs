//! Opt-in local indexing for synchronized folders.
//!
//! The indexer deliberately owns its database and event consumers. The core
//! synchronization path does not depend on the index being enabled or healthy.

pub mod catalog;
pub mod denylist;
pub mod links;
pub use catalog::{Catalog, CatalogMetadata, CatalogRecord, CatalogService};
pub use denylist::{Denied, DenyReason, Denylist, DenylistRule, DenylistService, FilterContext, FilterList};
pub use links::{
    CapabilityLink, ContentLink, ImmutableLink, Link, LinkResolution, LinkResolver, LinkStore, Mirror, MutableLink,
    MutablePointer, NameLink, PrivateLink, ProviderFetch, ResolveOptions, ResolvedLink, SignedMutablePointer,
    collect_files, current_epoch_seconds, fetch_from_mirrors, hash_source,
};

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use iroh_blobs::Hash;
use iroh_docs::{Entry, NamespaceId, engine::LiveEvent};
use n0_future::StreamExt;
use rusqlite::{Connection, OptionalExtension, params};
use tokio::{sync::broadcast, task::JoinHandle};

use crate::{
    error::{Result, SyncwebError},
    folder::SyncwebFolder,
};

/// Current indexing database schema version.
pub const SCHEMA_VERSION: &str = "1";
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

    /// Persist the mapping from a channel name to its catalog namespace.
    ///
    /// # Errors
    ///
    /// Returns an error if the mapping cannot be stored.
    pub fn set_channel_namespace(&self, channel_name: &str, namespace_id: NamespaceId) -> Result<()> {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO channel_namespaces(channel_name, catalog_namespace_id, created_at)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(channel_name) DO UPDATE SET catalog_namespace_id = excluded.catalog_namespace_id",
                    params![channel_name, namespace_id.to_string(), now_seconds()],
                )
                .map_err(|error| database_error("failed to store channel namespace", error))?;
            Ok(())
        })
    }

    /// Look up the catalog namespace for a channel name.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn get_channel_namespace(&self, channel_name: &str) -> Result<Option<NamespaceId>> {
        use std::str::FromStr;
        self.with_connection(|connection| {
            let result = connection
                .query_row(
                    "SELECT catalog_namespace_id FROM channel_namespaces WHERE channel_name = ?1",
                    [channel_name],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|error| database_error("failed to look up channel namespace", error))?;
            match result {
                Some(ns) => {
                    let namespace_id = NamespaceId::from_str(&ns)
                        .map_err(|error| database_error("channel namespace is invalid", error))?;
                    Ok(Some(namespace_id))
                }
                None => Ok(None),
            }
        })
    }

    /// Return all channel-to-namespace mappings.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn load_channel_namespaces(&self) -> Result<Vec<(String, NamespaceId)>> {
        use std::str::FromStr;
        self.with_connection(|connection| {
            let mut statement = connection
                .prepare("SELECT channel_name, catalog_namespace_id FROM channel_namespaces")
                .map_err(|error| database_error("failed to prepare channel listing", error))?;
            let rows = statement
                .query_map([], |row| {
                    let name: String = row.get(0)?;
                    let namespace: String = row.get(1)?;
                    Ok((name, namespace))
                })
                .map_err(|error| database_error("failed to read channel namespaces", error))?;
            let mut channels = Vec::new();
            for row in rows {
                let (name, namespace) =
                    row.map_err(|error| database_error("failed to read channel namespace row", error))?;
                let namespace_id = NamespaceId::from_str(&namespace)
                    .map_err(|error| database_error("channel namespace is invalid", error))?;
                channels.push((name, namespace_id));
            }
            Ok(channels)
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
                    let capability_blob: Vec<u8> = row.get(1)?;
                    if manifest_bytes.len() != 32 {
                        return Err(rusqlite::Error::InvalidColumnType(
                            0,
                            "manifest must be 32 bytes".to_owned(),
                            rusqlite::types::Type::Blob,
                        ));
                    }
                    if capability_blob.len() != 32 {
                        return Err(rusqlite::Error::InvalidColumnType(
                            1,
                            "capability must be 32 bytes".to_owned(),
                            rusqlite::types::Type::Blob,
                        ));
                    }
                    let Ok(manifests) = manifest_bytes.try_into() else {
                        return Err(rusqlite::Error::InvalidColumnType(
                            0,
                            "manifest conversion failed despite length check".to_owned(),
                            rusqlite::types::Type::Blob,
                        ));
                    };
                    let hash = Hash::from_bytes(manifests);
                    let Ok(cap) = capability_blob.try_into() else {
                        return Err(rusqlite::Error::InvalidColumnType(
                            1,
                            "capability conversion failed despite length check".to_owned(),
                            rusqlite::types::Type::Blob,
                        ));
                    };
                    Ok((hash, cap))
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
                    params![link.manifest.as_bytes(), link.capability.as_slice(), now],
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
    /// This is the service-level search used by `indexing search`. Records
    /// blocked by the local denylist are excluded from discovery.
    ///
    /// # Errors
    ///
    /// Returns an error if the query is invalid or `SQLite` cannot read results.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<CatalogRecord>> {
        Ok(self
            .search_global(query, limit)?
            .into_iter()
            .filter(|record| self.denylist.check_discovery(record).is_ok())
            .collect())
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
      );";

const SCHEMA_PART3: &str = "CREATE TABLE IF NOT EXISTS channel_namespaces (
     channel_name TEXT PRIMARY KEY NOT NULL,
     catalog_namespace_id TEXT NOT NULL,
     created_at INTEGER NOT NULL
 );";

fn initialize_connection(connection: &Connection) -> Result<()> {
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|error| database_error("failed to configure indexing database", error))?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| database_error("failed to enable indexing foreign keys", error))?;
    initialize_schema(connection)?;
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
