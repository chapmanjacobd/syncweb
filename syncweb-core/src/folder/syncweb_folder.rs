use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, bail};
use iroh::PublicKey;
use iroh_blobs::Hash;
use iroh_docs::{
    AuthorId, DocTicket, NamespaceId,
    api::{Doc, protocol::ShareMode},
};
use tokio::sync::RwLock;

use crate::node::{blob_store::BlobStore, docs_engine::DocsEngine};

use super::SyncMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    Admin,
    Write,
    Read,
}

impl Capability {
    pub fn can_write(self) -> bool {
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
    pub fn new(
        doc: Doc,
        author: AuthorId,
        blob_store: BlobStore,
        docs_engine: DocsEngine,
        sync_mode: SyncMode,
    ) -> Self {
        Self {
            namespace_id: doc.id(),
            doc,
            author,
            blob_store,
            docs_engine,
            sync_mode,
            capabilities: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn namespace_id(&self) -> NamespaceId {
        self.namespace_id
    }

    pub fn author(&self) -> AuthorId {
        self.author
    }

    pub fn mode(&self) -> SyncMode {
        self.sync_mode
    }

    pub fn doc(&self) -> &Doc {
        &self.doc
    }

    pub async fn grant(&self, node_id: PublicKey, capability: Capability) {
        self.capabilities.write().await.insert(node_id, capability);
    }

    pub async fn capability(&self, node_id: PublicKey) -> Option<Capability> {
        self.capabilities.read().await.get(&node_id).copied()
    }

    pub async fn can_write_as(&self, node_id: PublicKey) -> bool {
        self.sync_mode.can_write()
            && self
                .capability(node_id)
                .await
                .is_some_and(Capability::can_write)
    }

    pub async fn set_blob(&self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Result<Hash> {
        if !self.sync_mode.can_write() {
            bail!(
                "folder mode {} does not permit local writes",
                self.sync_mode
            );
        }
        let value = value.as_ref();
        let hash = self.blob_store.add_bytes(value).await?;
        self.docs_engine
            .set_blob(&self.doc, self.author, key, hash, value.len() as u64)
            .await?;
        Ok(hash)
    }

    pub async fn ticket(&self, endpoint: iroh::EndpointAddr, writable: bool) -> Result<DocTicket> {
        let mode = if writable && self.sync_mode.can_write() {
            ShareMode::Write
        } else {
            ShareMode::Read
        };
        self.docs_engine.share(&self.doc, mode, endpoint).await
    }
}
