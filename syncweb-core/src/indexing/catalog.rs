//! Public metadata catalogs for the opt-in indexing service.
//!
//! Catalogs are ordinary iroh-docs namespaces. The catalog contains metadata
//! only; the synchronized folder remains the source of the actual content.

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use iroh_blobs::Hash;
use iroh_docs::{
    AuthorId, DocTicket, Entry, NamespaceId,
    api::{Doc, protocol::ShareMode},
    engine::LiveEvent,
};
use n0_future::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::{
    error::{Result, SyncwebError},
    folder::SyncwebFolder,
    indexing::{IndexingDatabase, IndexingService},
    node::{blob_store::BlobStore, docs_engine::DocsEngine},
};

const CATALOG_METADATA_KEY: &[u8] = b"sys/syncweb/catalog/metadata";
const CATALOG_RECORD_PREFIX: &[u8] = b"record/";
const RETRY_COUNT: usize = 20;

/// Metadata describing a catalog namespace.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CatalogMetadata {
    pub name: String,
    pub description: Option<String>,
    pub publisher: String,
}

impl CatalogMetadata {
    /// Create catalog metadata with no description.
    #[must_use]
    pub fn new(name: impl Into<String>, publisher: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            publisher: publisher.into(),
        }
    }

    /// # Errors
    ///
    /// Returns an error if the catalog name is empty.
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(SyncwebError::InvalidConfig("catalog name cannot be empty".to_owned()));
        }
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if the metadata cannot be encoded.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|error| SyncwebError::operation("failed to serialize catalog metadata", error))
    }

    /// # Errors
    ///
    /// Returns an error if the metadata cannot be decoded or is invalid.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let metadata: Self = serde_json::from_slice(bytes.as_ref())
            .map_err(|error| SyncwebError::operation("failed to deserialize catalog metadata", error))?;
        metadata.validate()?;
        Ok(metadata)
    }
}

/// A searchable metadata record published by a folder.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CatalogRecord {
    /// The catalog containing this record. This is set by the catalog service
    /// when a record is written or imported.
    #[serde(default = "empty_namespace")]
    pub catalog_namespace_id: NamespaceId,
    pub folder_namespace_id: NamespaceId,
    pub key: Vec<u8>,
    pub hash: Hash,
    pub size: u64,
    pub folder_name: String,
    pub title: String,
    pub tags: Vec<String>,
    pub publisher: String,
}

impl CatalogRecord {
    /// Create a record using the key's file name as its title.
    #[must_use]
    pub fn new(folder_namespace_id: NamespaceId, key: impl AsRef<[u8]>, hash: Hash, size: u64) -> Self {
        let key_bytes = key.as_ref().to_vec();
        let key_text = String::from_utf8_lossy(&key_bytes);
        let title = Path::new(key_text.as_ref())
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| key_text.as_ref())
            .to_owned();
        Self {
            catalog_namespace_id: empty_namespace(),
            folder_namespace_id,
            key: key_bytes,
            hash,
            size,
            folder_name: folder_namespace_id.to_string(),
            title,
            tags: Vec::new(),
            publisher: String::new(),
        }
    }

    /// Set the namespace containing this record.
    #[must_use]
    pub const fn in_catalog(mut self, namespace_id: NamespaceId) -> Self {
        self.catalog_namespace_id = namespace_id;
        self
    }

    /// # Errors
    ///
    /// Returns an error if required searchable metadata is empty.
    pub fn validate(&self) -> Result<()> {
        if self.folder_name.trim().is_empty() {
            return Err(SyncwebError::InvalidConfig(
                "catalog record folder name cannot be empty".to_owned(),
            ));
        }
        if self.title.trim().is_empty() {
            return Err(SyncwebError::InvalidConfig(
                "catalog record title cannot be empty".to_owned(),
            ));
        }
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if the record is invalid or cannot be encoded.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|error| SyncwebError::operation("failed to serialize catalog record", error))
    }

    /// # Errors
    ///
    /// Returns an error if the record cannot be decoded or is invalid.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let record: Self = serde_json::from_slice(bytes.as_ref())
            .map_err(|error| SyncwebError::operation("failed to deserialize catalog record", error))?;
        record.validate()?;
        Ok(record)
    }
}

/// A local handle to an iroh-docs catalog namespace.
#[derive(Clone)]
pub struct Catalog {
    doc: Doc,
    namespace_id: NamespaceId,
    name: String,
}

impl std::fmt::Debug for Catalog {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Catalog")
            .field("namespace_id", &self.namespace_id)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl Catalog {
    #[must_use]
    pub fn new(doc: Doc, name: impl Into<String>) -> Self {
        let namespace_id = doc.id();
        Self {
            doc,
            namespace_id,
            name: name.into(),
        }
    }

    #[must_use]
    pub const fn doc(&self) -> &Doc {
        &self.doc
    }

    #[must_use]
    pub const fn namespace_id(&self) -> NamespaceId {
        self.namespace_id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// The catalog publisher and subscriber for an indexing database.
#[derive(Clone)]
pub struct CatalogService {
    indexing: IndexingService,
    docs: DocsEngine,
    blobs: BlobStore,
    author: AuthorId,
    tasks: Arc<Mutex<HashMap<NamespaceId, JoinHandle<()>>>>,
}

impl std::fmt::Debug for CatalogService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("CatalogService").finish_non_exhaustive()
    }
}

impl CatalogService {
    /// Construct a catalog service using an existing indexing database.
    #[must_use]
    pub fn new(indexing: &IndexingService, docs: &DocsEngine, blobs: &BlobStore, author: AuthorId) -> Self {
        Self {
            indexing: indexing.clone(),
            docs: docs.clone(),
            blobs: blobs.clone(),
            author,
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[must_use]
    pub const fn database(&self) -> &IndexingDatabase {
        self.indexing.database()
    }

    #[must_use]
    pub const fn indexing(&self) -> &IndexingService {
        &self.indexing
    }

    /// Create and register a catalog namespace.
    ///
    /// # Errors
    ///
    /// Returns an error if the namespace or its metadata cannot be created.
    pub async fn create_catalog(&self, name: impl Into<String>) -> Result<Catalog> {
        let metadata = CatalogMetadata::new(name, self.author.to_string());
        metadata.validate()?;
        let doc = self.docs.create_namespace().await?;
        let catalog = Catalog::new(doc, metadata.name.clone());
        self.docs
            .set(&catalog.doc, self.author, CATALOG_METADATA_KEY, metadata.to_bytes()?)
            .await?;
        self.database().enable_catalog(catalog.namespace_id, catalog.name())?;
        Ok(catalog)
    }

    /// Create a catalog namespace. Alias for [`Self::create_catalog`].
    ///
    /// # Errors
    ///
    /// Returns an error if the namespace or its metadata cannot be created.
    pub async fn create(&self, name: impl Into<String>) -> Result<Catalog> {
        self.create_catalog(name).await
    }

    /// Publish all entries in a folder to a catalog.
    ///
    /// The catalog contains searchable metadata and content hashes, not file
    /// contents. The folder's existing document remains authoritative.
    ///
    /// # Errors
    ///
    /// Returns an error if folder entries or catalog records cannot be read or
    /// written.
    pub async fn publish_folder(&self, catalog: &Catalog, folder: &SyncwebFolder) -> Result<usize> {
        self.publish_folder_with_metadata(catalog, folder, folder.namespace_id().to_string(), &[])
            .await
    }

    /// Publish all entries in a folder with a display name and search tags.
    ///
    /// # Errors
    ///
    /// Returns an error if folder entries or catalog records cannot be read or
    /// written.
    pub async fn publish_folder_with_metadata(
        &self,
        catalog: &Catalog,
        folder: &SyncwebFolder,
        folder_name: impl Into<String>,
        tags: &[String],
    ) -> Result<usize> {
        let folder_name_value = folder_name.into();
        if folder_name_value.trim().is_empty() {
            return Err(SyncwebError::InvalidConfig(
                "catalog folder name cannot be empty".to_owned(),
            ));
        }
        self.database().enable_catalog(catalog.namespace_id, catalog.name())?;
        let entries = folder.docs_engine().list_latest(folder.doc()).await?;
        let mut published = 0_usize;
        for entry in entries {
            if is_system_key(entry.key()) {
                continue;
            }
            let mut record = CatalogRecord::new(
                folder.namespace_id(),
                entry.key(),
                entry.content_hash(),
                entry.content_len(),
            )
            .in_catalog(catalog.namespace_id);
            record.folder_name.clone_from(&folder_name_value);
            record.tags = tags.to_vec();
            record.publisher = self.author.to_string();
            let key = record_key(&record);
            self.docs
                .set(&catalog.doc, self.author, key, record.to_bytes()?)
                .await?;
            self.database().upsert_catalog_record(&record)?;
            published = published.saturating_add(1);
        }
        Ok(published)
    }

    /// Publish a folder using its namespace as the display name.
    ///
    /// # Errors
    ///
    /// Returns an error if folder entries or catalog records cannot be read or
    /// written.
    pub async fn publish(&self, catalog: &Catalog, folder: &SyncwebFolder) -> Result<usize> {
        self.publish_folder(catalog, folder).await
    }

    /// Create a read-only ticket for a catalog namespace.
    ///
    /// # Errors
    ///
    /// Returns an error if the catalog ticket cannot be created.
    pub async fn ticket(&self, catalog: &Catalog, endpoint: iroh::EndpointAddr, writable: bool) -> Result<DocTicket> {
        let mode = if writable { ShareMode::Write } else { ShareMode::Read };
        self.docs.share(&catalog.doc, mode, endpoint).await
    }

    /// Subscribe to a catalog using its iroh-docs ticket.
    ///
    /// Subscription starts document synchronization and keeps a background
    /// watcher which imports new records into the local FTS database.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket cannot be imported or synchronized.
    pub async fn subscribe(&self, ticket: DocTicket) -> Result<Catalog> {
        let doc = self.docs.import_ticket(ticket).await?;
        let catalog = Catalog::new(doc, "remote catalog");
        self.activate(&catalog).await?;
        Ok(catalog)
    }

    /// Subscribe to a catalog. Alias for [`Self::subscribe`].
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket cannot be imported or synchronized.
    pub async fn subscribe_catalog(&self, ticket: DocTicket) -> Result<Catalog> {
        self.subscribe(ticket).await
    }

    /// Open and subscribe to a catalog namespace already available locally.
    ///
    /// # Errors
    ///
    /// Returns an error if the namespace is not available or cannot be synced.
    pub async fn subscribe_namespace(&self, namespace_id: NamespaceId) -> Result<Catalog> {
        let doc = self
            .docs
            .open(namespace_id)
            .await?
            .ok_or(SyncwebError::NamespaceNotAvailable)?;
        let catalog = Catalog::new(doc, "remote catalog");
        self.activate(&catalog).await?;
        Ok(catalog)
    }

    /// Synchronize a catalog once and import all records currently available.
    ///
    /// # Errors
    ///
    /// Returns an error if document synchronization or record indexing fails.
    pub async fn sync_catalog(&self, catalog: &Catalog) -> Result<usize> {
        self.docs.start_sync(&catalog.doc, Vec::new()).await?;
        self.index_catalog(catalog).await
    }

    /// Synchronize a catalog once. Alias for [`Self::sync_catalog`].
    ///
    /// # Errors
    ///
    /// Returns an error if document synchronization or record indexing fails.
    pub async fn sync(&self, catalog: &Catalog) -> Result<usize> {
        self.sync_catalog(catalog).await
    }

    /// Import all currently available records from a catalog.
    ///
    /// # Errors
    ///
    /// Returns an error if catalog entries cannot be read or decoded.
    pub async fn index_catalog(&self, catalog: &Catalog) -> Result<usize> {
        self.database().enable_catalog(catalog.namespace_id, catalog.name())?;
        let entries = self.docs.list_latest(&catalog.doc).await?;
        let mut indexed = 0_usize;
        for entry in entries {
            if self.index_catalog_entry(catalog.namespace_id, &entry).await? {
                indexed = indexed.saturating_add(1);
            }
        }
        Ok(indexed)
    }

    /// Search records imported from all known catalogs.
    ///
    /// # Errors
    ///
    /// Returns an error if the FTS query is invalid.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<CatalogRecord>> {
        self.database().search_catalogs(query, limit)
    }

    /// Search records imported from all known catalogs. Alias for [`Self::search`].
    ///
    /// # Errors
    ///
    /// Returns an error if the FTS query is invalid.
    pub fn search_fts(&self, query: &str, limit: usize) -> Result<Vec<CatalogRecord>> {
        self.search(query, limit)
    }

    /// Stop following a catalog and remove its local records.
    ///
    /// # Errors
    ///
    /// Returns an error if the catalog index cannot be removed.
    pub fn unsubscribe(&self, namespace_id: NamespaceId) -> Result<()> {
        let task_handle = self
            .tasks
            .lock()
            .map_err(|error| SyncwebError::operation("catalog task lock poisoned", error))?
            .remove(&namespace_id);
        if let Some(handle) = task_handle {
            handle.abort();
        }
        self.database().disable_catalog(namespace_id)
    }

    async fn activate(&self, catalog: &Catalog) -> Result<()> {
        let live_events = self.docs.watch(&catalog.doc).await?;
        self.database().enable_catalog(catalog.namespace_id, catalog.name())?;
        self.index_catalog(catalog).await?;

        let namespace_id = catalog.namespace_id;
        let database = self.indexing.database().clone();
        let docs = self.docs.clone();
        let blobs = self.blobs.clone();
        let task = tokio::spawn(async move {
            consume_catalog_events(namespace_id, live_events, database, blobs, docs).await;
        });
        self.tasks
            .lock()
            .map_err(|error| SyncwebError::operation("catalog task lock poisoned", error))?
            .insert(namespace_id, task);
        if let Err(error) = self.docs.start_sync(&catalog.doc, Vec::new()).await {
            if let Ok(mut tasks) = self.tasks.lock()
                && let Some(task_handle) = tasks.remove(&namespace_id)
            {
                task_handle.abort();
            }
            return Err(error);
        }
        Ok(())
    }

    async fn index_catalog_entry(&self, namespace_id: NamespaceId, entry: &Entry) -> Result<bool> {
        if !is_record_key(entry.key()) {
            return Ok(false);
        }
        let bytes = self.blobs.get(entry.content_hash()).await?;
        let mut record = CatalogRecord::from_bytes(bytes)?;
        record.catalog_namespace_id = namespace_id;
        self.database().upsert_catalog_record(&record)?;
        Ok(true)
    }
}

async fn consume_catalog_events(
    namespace_id: NamespaceId,
    mut live_events: impl n0_future::Stream<Item = Result<LiveEvent>> + Send + Unpin + 'static,
    database: IndexingDatabase,
    blobs: BlobStore,
    docs: DocsEngine,
) {
    while let Some(event_result) = live_events.next().await {
        let event = match event_result {
            Ok(event) => event,
            Err(error) => {
                tracing::error!(%namespace_id, error = %error, "catalog document event failed");
                break;
            }
        };
        if let Err(error) = process_catalog_event(namespace_id, event, &database, &blobs, &docs).await {
            tracing::error!(%namespace_id, error = %error, "failed to process catalog event");
        }
    }
}

async fn process_catalog_event(
    namespace_id: NamespaceId,
    event: LiveEvent,
    database: &IndexingDatabase,
    blobs: &BlobStore,
    docs: &DocsEngine,
) -> Result<()> {
    match event {
        LiveEvent::InsertLocal { entry } | LiveEvent::InsertRemote { entry, .. } => {
            if !is_record_key(entry.key()) {
                return Ok(());
            }
            let mut record = read_catalog_record(blobs, &entry).await?;
            record.catalog_namespace_id = namespace_id;
            database.upsert_catalog_record(&record)?;
        }
        LiveEvent::ContentReady { .. } => reindex_catalog(docs, blobs, database, namespace_id).await?,
        LiveEvent::PendingContentReady
        | LiveEvent::NeighborUp(_)
        | LiveEvent::NeighborDown(_)
        | LiveEvent::SyncFinished(_) => {}
    }
    Ok(())
}

async fn reindex_catalog(
    docs: &DocsEngine,
    blobs: &BlobStore,
    database: &IndexingDatabase,
    namespace_id: NamespaceId,
) -> Result<()> {
    let doc = docs
        .open(namespace_id)
        .await?
        .ok_or(SyncwebError::NamespaceNotAvailable)?;
    for entry in docs.list_latest(&doc).await? {
        if !is_record_key(entry.key()) {
            continue;
        }
        let bytes = blobs.get(entry.content_hash()).await?;
        let mut record = CatalogRecord::from_bytes(bytes)?;
        record.catalog_namespace_id = namespace_id;
        database.upsert_catalog_record(&record)?;
    }
    Ok(())
}

async fn read_catalog_record(blobs: &BlobStore, entry: &Entry) -> Result<CatalogRecord> {
    let mut last_error = None;
    for attempt in 0..RETRY_COUNT {
        match blobs.get(entry.content_hash()).await {
            Ok(bytes) => return CatalogRecord::from_bytes(bytes),
            Err(error) => {
                last_error = Some(error);
                if attempt.saturating_add(1) < RETRY_COUNT {
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
            }
        }
    }
    Err(last_error.unwrap_or_else(|| SyncwebError::InvalidConfig("catalog record content is unavailable".to_owned())))
}

fn record_key(record: &CatalogRecord) -> Vec<u8> {
    format!(
        "{}{}/{}",
        String::from_utf8_lossy(CATALOG_RECORD_PREFIX),
        record.folder_namespace_id,
        hex::encode(&record.key)
    )
    .into_bytes()
}

fn is_record_key(key: &[u8]) -> bool {
    key.starts_with(CATALOG_RECORD_PREFIX)
}

fn is_system_key(key: &[u8]) -> bool {
    key.starts_with(b"sys/")
}

fn empty_namespace() -> NamespaceId {
    NamespaceId::from([0_u8; 32])
}
