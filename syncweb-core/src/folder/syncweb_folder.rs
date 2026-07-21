use std::{collections::HashMap, sync::Arc};

use iroh::PublicKey;
use iroh_blobs::{Hash, ticket::BlobTicket};
use iroh_docs::{
    AuthorId, DocTicket, NamespaceId,
    api::{Doc, protocol::ShareMode},
};
use tokio::sync::RwLock;

use crate::error::{Result, SyncwebError};
use crate::node::{blob_store::BlobStore, docs_engine::DocsEngine};
use crate::snapshot::{Snapshot, SnapshotDiff, SnapshotId, SnapshotStore};

use super::SyncMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Capability {
    Admin,
    Write,
    Read,
}

impl Capability {
    #[must_use]
    pub const fn can_write(self) -> bool {
        matches!(self, Self::Admin | Self::Write)
    }
}

#[derive(Clone)]
pub struct SyncwebFolder {
    doc: Doc,
    namespace_id: NamespaceId,
    author: AuthorId,
    blob_store: BlobStore,
    docs_engine: DocsEngine,
    sync_mode: SyncMode,
    capabilities: Arc<RwLock<HashMap<PublicKey, Capability>>>,
}

impl SyncwebFolder {
    #[must_use]
    pub fn new(
        doc: Doc,
        author: AuthorId,
        blob_store: BlobStore,
        docs_engine: DocsEngine,
        sync_mode: SyncMode,
    ) -> Self {
        let namespace_id = doc.id();
        Self {
            doc,
            namespace_id,
            author,
            blob_store,
            docs_engine,
            sync_mode,
            capabilities: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new folder by allocating a namespace in the docs engine.
    ///
    /// # Errors
    ///
    /// Returns an error if the namespace cannot be created or the default author cannot be retrieved.
    pub async fn create(docs_engine: DocsEngine, blob_store: BlobStore, sync_mode: SyncMode) -> Result<Self> {
        let doc = docs_engine
            .create_namespace()
            .await
            .map_err(|error| SyncwebError::operation("failed to create folder namespace", error))?;
        let author = docs_engine
            .author()
            .await
            .map_err(|error| SyncwebError::operation("failed to retrieve folder author", error))?;
        Ok(Self::new(doc, author, blob_store, docs_engine, sync_mode))
    }

    /// Accept a locally available folder by namespace ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the namespace cannot be opened.
    pub async fn accept(docs_engine: DocsEngine, blob_store: BlobStore, namespace_id: NamespaceId) -> Result<Self> {
        let doc = docs_engine
            .open(namespace_id)
            .await
            .map_err(|error| SyncwebError::operation("failed to open folder namespace", error))?
            .ok_or(SyncwebError::NamespaceNotAvailable)?;
        let author = docs_engine
            .author()
            .await
            .map_err(|error| SyncwebError::operation("failed to retrieve folder author", error))?;
        Ok(Self::new(doc, author, blob_store, docs_engine, SyncMode::ReceiveOnly))
    }

    /// Drop this folder's namespace from the docs engine.
    ///
    /// # Errors
    ///
    /// Returns an error if the namespace cannot be dropped.
    pub async fn drop_namespace(&self) -> Result<()> {
        self.docs_engine
            .drop_namespace(self.namespace_id)
            .await
            .map_err(|error| SyncwebError::operation("failed to drop folder namespace", error))
    }

    #[must_use]
    pub const fn namespace_id(&self) -> NamespaceId {
        self.namespace_id
    }

    #[must_use]
    pub const fn author(&self) -> AuthorId {
        self.author
    }

    #[must_use]
    pub const fn mode(&self) -> SyncMode {
        self.sync_mode
    }

    #[must_use]
    pub const fn doc(&self) -> &Doc {
        &self.doc
    }

    pub async fn grant(&self, node_id: PublicKey, capability: Capability) {
        self.capabilities.write().await.insert(node_id, capability);
    }

    pub async fn capability(&self, node_id: PublicKey) -> Option<Capability> {
        self.capabilities.read().await.get(&node_id).copied()
    }

    pub async fn can_write_as(&self, node_id: PublicKey) -> bool {
        self.sync_mode.can_write() && self.capability(node_id).await.is_some_and(Capability::can_write)
    }

    /// Check whether a blob is complete in this folder's local store.
    ///
    /// # Errors
    ///
    /// Returns an error if the blob store cannot be queried.
    pub async fn has_local(&self, hash: Hash) -> Result<bool> {
        self.blob_store.has(hash).await
    }

    /// Create a content-addressed snapshot of this folder.
    ///
    /// # Errors
    ///
    /// Returns an error if document entries cannot be read or referenced blobs
    /// are unavailable.
    pub async fn create_snapshot(&self, description: Option<String>) -> Result<Snapshot> {
        SnapshotStore::with_docs(self.blob_store.clone(), self.docs_engine.clone())
            .create_for_folder(self, description)
            .await
    }

    /// Restore this folder's document entries from a snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot belongs to another folder or content
    /// is unavailable.
    pub async fn restore_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        SnapshotStore::with_docs(self.blob_store.clone(), self.docs_engine.clone())
            .restore_for_folder(self, snapshot)
            .await
    }

    /// List snapshots stored in the local blob store.
    ///
    /// # Errors
    ///
    /// Returns an error if snapshot manifests cannot be read.
    pub async fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        SnapshotStore::with_docs(self.blob_store.clone(), self.docs_engine.clone())
            .list()
            .await
    }

    /// Delete a snapshot and release its pins.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot cannot be found or pins cannot be released.
    pub async fn delete_snapshot(&self, snapshot_id: SnapshotId) -> Result<()> {
        SnapshotStore::with_docs(self.blob_store.clone(), self.docs_engine.clone())
            .delete(snapshot_id)
            .await
    }

    /// Compare two snapshots.
    ///
    /// # Errors
    ///
    /// Returns an error if either snapshot is invalid.
    pub fn diff_snapshots(&self, first: &Snapshot, second: &Snapshot) -> Result<SnapshotDiff> {
        first.diff(second)
    }

    /// # Errors
    ///
    /// Returns an error if the blob fails to be stored or set.
    pub async fn set_blob(&self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Result<Hash> {
        if !self.sync_mode.can_write() {
            return Err(SyncwebError::WriteDenied {
                mode: self.sync_mode.to_string(),
            });
        }
        let value_bytes = value.as_ref();
        let hash = self.blob_store.add_bytes(value_bytes).await?;
        let len = u64::try_from(value_bytes.len())
            .map_err(|error| SyncwebError::operation("blob size exceeds u64::MAX", error))?;
        self.docs_engine
            .set_blob(&self.doc, self.author, key, hash, len)
            .await?;
        Ok(hash)
    }

    /// Store an existing blob reference in this folder.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder is read-only, the blob is unavailable, or
    /// the document entry cannot be written.
    pub async fn set_blob_ref(&self, key: impl AsRef<[u8]>, hash: Hash, size: u64) -> Result<()> {
        if !self.sync_mode.can_write() {
            return Err(SyncwebError::WriteDenied {
                mode: self.sync_mode.to_string(),
            });
        }
        if !self.blob_store.has(hash).await? {
            return Err(SyncwebError::InvalidConfig(format!("blob is missing: {hash}")));
        }
        self.docs_engine.set_blob(&self.doc, self.author, key, hash, size).await
    }

    /// Delete a folder entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder is read-only or the document entry cannot be deleted.
    pub async fn delete_entry(&self, key: impl AsRef<[u8]>) -> Result<()> {
        if !self.sync_mode.can_write() {
            return Err(SyncwebError::WriteDenied {
                mode: self.sync_mode.to_string(),
            });
        }
        self.docs_engine.delete(&self.doc, self.author, key).await
    }

    /// # Errors
    ///
    /// Returns an error if the folder ticket cannot be created.
    pub async fn ticket(&self, endpoint: iroh::EndpointAddr, writable: bool) -> Result<DocTicket> {
        let mode = if writable && self.sync_mode.can_write() {
            ShareMode::Write
        } else {
            ShareMode::Read
        };
        self.docs_engine.share(&self.doc, mode, endpoint).await
    }

    /// Create an unauthenticated ticket for a blob in this folder and pin it
    /// while it is publicly shared.
    ///
    /// # Errors
    ///
    /// Returns an error if the blob is unavailable or cannot be pinned.
    pub async fn publish_blob(&self, endpoint: iroh::EndpointAddr, hash: Hash) -> Result<BlobTicket> {
        if !self.blob_store.has(hash).await? {
            return Err(SyncwebError::InvalidConfig(format!(
                "cannot publish missing blob {hash}"
            )));
        }
        self.blob_store
            .pin(public_pin_name(self.namespace_id, hash), hash)
            .await?;
        Ok(self.blob_store.ticket_for_addr(endpoint, hash))
    }

    /// Remove the public-sharing pin from a folder blob.
    ///
    /// Existing blob tickets are capabilities and remain usable while another
    /// tag or active transfer retains the blob.
    ///
    /// # Errors
    ///
    /// Returns an error if the public-sharing pin cannot be removed.
    pub async fn unpublish_blob(&self, hash: Hash) -> Result<()> {
        self.blob_store.unpin(public_pin_name(self.namespace_id, hash)).await
    }
}

fn public_pin_name(namespace_id: NamespaceId, hash: Hash) -> String {
    format!("syncweb/public/{namespace_id}/{hash}")
}
