use std::{
    collections::{HashMap, hash_map::Entry},
    str::FromStr,
    sync::Arc,
};

use anyhow::{Context, Result};
use iroh::PublicKey;
use iroh_docs::{DocTicket, NamespaceId, api::Doc};
use n0_future::StreamExt;
use tokio::sync::RwLock;

use crate::node::iroh_node::IrohNode;

use super::{Capability, SyncMode, SyncwebFolder};

#[derive(Clone)]
pub struct FolderManager {
    endpoint_addr: iroh::EndpointAddr,
    node_id: PublicKey,
    blob_store: crate::node::blob_store::BlobStore,
    docs_engine: crate::node::docs_engine::DocsEngine,
    folders: Arc<RwLock<HashMap<NamespaceId, SyncwebFolder>>>,
}

impl FolderManager {
    pub fn new(node: &IrohNode) -> Self {
        Self {
            endpoint_addr: node.endpoint().addr(),
            node_id: node.endpoint().id(),
            blob_store: node.blob_store().clone(),
            docs_engine: node.docs_engine().clone(),
            folders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create(&self, mode: SyncMode) -> Result<SyncwebFolder> {
        let doc = self.docs_engine.create_namespace().await?;
        let folder = self.folder_from_doc(doc, mode).await?;
        folder.grant(self.node_id, Capability::Admin).await;
        self.folders
            .write()
            .await
            .insert(folder.namespace_id(), folder.clone());
        Ok(folder)
    }

    pub async fn join(&self, ticket: impl AsRef<str>, mode: SyncMode) -> Result<SyncwebFolder> {
        let ticket = DocTicket::from_str(ticket.as_ref()).context("invalid folder ticket")?;
        let doc = self.docs_engine.import_ticket(ticket).await?;
        let folder = self.folder_from_doc(doc, mode).await?;
        self.folders
            .write()
            .await
            .insert(folder.namespace_id(), folder.clone());
        Ok(folder)
    }

    pub async fn accept(&self, namespace_id: NamespaceId) -> Result<SyncwebFolder> {
        if let Some(folder) = self.folders.read().await.get(&namespace_id).cloned() {
            return Ok(folder);
        }
        let doc = self
            .docs_engine
            .open(namespace_id)
            .await?
            .context("folder is not available locally")?;
        let folder = self.folder_from_doc(doc, SyncMode::ReceiveOnly).await?;
        self.folders
            .write()
            .await
            .insert(namespace_id, folder.clone());
        Ok(folder)
    }

    pub async fn drop(&self, namespace_id: NamespaceId) -> Result<()> {
        self.docs_engine.drop_namespace(namespace_id).await?;
        self.folders.write().await.remove(&namespace_id);
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<SyncwebFolder>> {
        let mut documents = self.docs_engine.inner().list().await?;
        let mut listed = Vec::new();
        while let Some(document) = documents.next().await {
            listed.push(document?);
        }
        for (namespace_id, capability) in listed {
            if self.folders.read().await.contains_key(&namespace_id) {
                continue;
            }
            let doc = self
                .docs_engine
                .open(namespace_id)
                .await?
                .context("listed folder could not be opened")?;
            let mode = match capability {
                iroh_docs::CapabilityKind::Write => SyncMode::SendReceive,
                iroh_docs::CapabilityKind::Read => SyncMode::ReceiveOnly,
            };
            let folder = self.folder_from_doc(doc, mode).await?;
            if let Entry::Vacant(entry) = self.folders.write().await.entry(namespace_id) {
                entry.insert(folder);
            }
        }
        Ok(self.folders.read().await.values().cloned().collect())
    }

    pub async fn ticket(&self, namespace_id: NamespaceId, writable: bool) -> Result<DocTicket> {
        let folder = self
            .folders
            .read()
            .await
            .get(&namespace_id)
            .cloned()
            .context("folder is not managed")?;
        folder.ticket(self.endpoint_addr.clone(), writable).await
    }

    async fn folder_from_doc(&self, doc: Doc, mode: SyncMode) -> Result<SyncwebFolder> {
        Ok(SyncwebFolder::new(
            doc,
            self.docs_engine.author().await?,
            self.blob_store.clone(),
            self.docs_engine.clone(),
            mode,
        ))
    }
}
