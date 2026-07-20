use std::{collections::HashMap, sync::Arc};

use iroh::PublicKey;
use iroh_blobs::Hash;
use iroh_docs::{
    AuthorId, DocTicket, NamespaceId,
    api::{Doc, protocol::ShareMode},
};
use tokio::sync::RwLock;

use crate::error::{Result, SyncwebError};
use crate::node::{blob_store::BlobStore, docs_engine::DocsEngine};

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
}
