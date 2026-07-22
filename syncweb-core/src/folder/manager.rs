use std::{
    collections::{HashMap, hash_map::Entry},
    str::FromStr,
    sync::Arc,
};

use iroh::PublicKey;
use iroh_blobs::ticket::BlobTicket;
use iroh_docs::{DocTicket, NamespaceId, api::Doc};
use n0_future::StreamExt;
use tokio::sync::RwLock;

use crate::error::{Result, SyncwebError};
use crate::node::iroh_node::IrohNode;

use super::{Capability, SyncMode, SyncwebFolder};

const MODE_KEY: &[u8] = b"sys/syncweb/mode";

#[derive(Clone)]
pub struct FolderManager {
    endpoint: iroh::Endpoint,
    endpoint_addr: iroh::EndpointAddr,
    node_id: PublicKey,
    blob_store: crate::node::blob_store::BlobStore,
    docs_engine: crate::node::docs_engine::DocsEngine,
    folders: Arc<RwLock<HashMap<NamespaceId, SyncwebFolder>>>,
}

impl FolderManager {
    #[must_use]
    pub fn new(node: &IrohNode) -> Self {
        Self {
            endpoint: node.endpoint().clone(),
            endpoint_addr: node.endpoint().addr(),
            node_id: node.endpoint().id(),
            blob_store: node.blob_store().clone(),
            docs_engine: node.docs_engine().clone(),
            folders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// # Errors
    ///
    /// Returns an error if the folder namespace cannot be created or initialized.
    pub async fn create(&self, mode: SyncMode) -> Result<SyncwebFolder> {
        let doc = self.docs_engine.create_namespace().await?;
        let author = self.docs_engine.author().await?;
        self.docs_engine.set(&doc, author, MODE_KEY, mode.to_string()).await?;
        let folder = SyncwebFolder::new(doc, author, self.blob_store.clone(), self.docs_engine.clone(), mode);
        folder.grant(self.node_id, Capability::Admin).await;
        self.folders.write().await.insert(folder.namespace_id(), folder.clone());
        Ok(folder)
    }

    /// # Errors
    ///
    /// Returns an error if the folder ticket cannot be joined or parsed.
    pub async fn join(&self, ticket_str: impl AsRef<str>, mode: SyncMode) -> Result<SyncwebFolder> {
        let mut ticket_raw = ticket_str.as_ref();
        if let Some(rest) = ticket_raw.strip_prefix("syncweb://") {
            if let Some((_, query)) = rest.split_once('?') {
                for param in query.split('&') {
                    if let Some(val) = param.strip_prefix("ticket=") {
                        ticket_raw = val;
                        break;
                    }
                }
            } else {
                ticket_raw = rest;
            }
        }
        let ticket = DocTicket::from_str(ticket_raw).map_err(|error| SyncwebError::InvalidTicket(error.to_string()))?;
        let doc = self.docs_engine.import_ticket(ticket).await?;
        let folder = self.folder_from_doc(doc, mode).await?;
        self.folders.write().await.insert(folder.namespace_id(), folder.clone());
        Ok(folder)
    }

    /// Subscribe to a public blob ticket by creating a local read-only
    /// namespace containing the fetched content.
    ///
    /// # Errors
    ///
    /// Returns an error if the blob cannot be fetched or the local namespace
    /// cannot be created.
    pub async fn subscribe_public(&self, ticket: &BlobTicket) -> Result<SyncwebFolder> {
        self.blob_store.fetch(&self.endpoint, ticket).await?;
        let bytes = self.blob_store.get(ticket.hash()).await?;
        let doc = self.docs_engine.create_namespace().await?;
        let author = self.docs_engine.author().await?;
        self.docs_engine
            .set(&doc, author, MODE_KEY, SyncMode::PublicReadOnly.to_string())
            .await?;
        self.docs_engine
            .set_blob(
                &doc,
                author,
                b"public/content",
                ticket.hash(),
                u64::try_from(bytes.len())
                    .map_err(|error| SyncwebError::operation("public blob is too large", error))?,
            )
            .await?;
        let folder = SyncwebFolder::new(
            doc,
            author,
            self.blob_store.clone(),
            self.docs_engine.clone(),
            SyncMode::PublicReadOnly,
        );
        self.folders.write().await.insert(folder.namespace_id(), folder.clone());
        Ok(folder)
    }

    /// # Errors
    ///
    /// Returns an error if the folder namespace cannot be accepted.
    pub async fn accept(&self, namespace_id: NamespaceId) -> Result<SyncwebFolder> {
        let existing_folder = self.folders.read().await.get(&namespace_id).cloned();
        if let Some(folder) = existing_folder {
            return Ok(folder);
        }
        let doc = self
            .docs_engine
            .open(namespace_id)
            .await?
            .ok_or(SyncwebError::NamespaceNotAvailable)?;
        let folder = self.folder_from_doc(doc, SyncMode::ReceiveOnly).await?;
        self.folders.write().await.insert(namespace_id, folder.clone());
        Ok(folder)
    }

    /// # Errors
    ///
    /// Returns an error if the folder namespace cannot be dropped.
    pub async fn drop(&self, namespace_id: NamespaceId) -> Result<()> {
        self.docs_engine.drop_namespace(namespace_id).await?;
        self.folders.write().await.remove(&namespace_id);
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if the folders cannot be listed.
    pub async fn list(&self) -> Result<Vec<SyncwebFolder>> {
        let mut documents = self
            .docs_engine
            .inner()
            .list()
            .await
            .map_err(|error| SyncwebError::operation("failed to list documents", error))?;
        let mut listed = Vec::new();
        while let Some(document) = documents.next().await {
            listed.push(document.map_err(|error| SyncwebError::operation("failed to read document list", error))?);
        }
        for (namespace_id, capability) in listed {
            if self.folders.read().await.contains_key(&namespace_id) {
                continue;
            }
            let doc = self
                .docs_engine
                .open(namespace_id)
                .await?
                .ok_or(SyncwebError::NamespaceNotAvailable)?;
            let fallback_mode = match capability {
                iroh_docs::CapabilityKind::Write => SyncMode::SendReceive,
                iroh_docs::CapabilityKind::Read => SyncMode::ReceiveOnly,
            };
            let mode = self.mode_from_doc(&doc, fallback_mode).await?;
            let folder = self.folder_from_doc(doc, mode).await?;
            if let Entry::Vacant(entry) = self.folders.write().await.entry(namespace_id) {
                entry.insert(folder);
            }
        }
        Ok(self.folders.read().await.values().cloned().collect())
    }

    /// Return a managed folder, loading locally available namespaces first.
    ///
    /// # Errors
    ///
    /// Returns an error if the local namespace list cannot be read.
    pub async fn get(&self, namespace_id: NamespaceId) -> Result<SyncwebFolder> {
        let existing = self.folders.read().await.get(&namespace_id).cloned();
        if let Some(folder) = existing {
            return Ok(folder);
        }
        self.list()
            .await?
            .into_iter()
            .find(|folder| folder.namespace_id() == namespace_id)
            .ok_or_else(|| SyncwebError::FolderNotFound(namespace_id.to_string()))
    }

    /// # Errors
    ///
    /// Returns an error if the ticket cannot be generated.
    pub async fn ticket(&self, namespace_id: NamespaceId, writable: bool) -> Result<DocTicket> {
        let folder = self
            .folders
            .read()
            .await
            .get(&namespace_id)
            .cloned()
            .ok_or(SyncwebError::FolderNotFound(namespace_id.to_string()))?;
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

    async fn mode_from_doc(&self, doc: &Doc, fallback: SyncMode) -> Result<SyncMode> {
        let author = self.docs_engine.author().await?;
        let Some(entry) = self.docs_engine.get(doc, author, MODE_KEY).await? else {
            return Ok(fallback);
        };
        let mode_bytes = self.blob_store.get(entry.content_hash()).await?;
        let mode_value = std::str::from_utf8(&mode_bytes)
            .map_err(|error| SyncwebError::operation("folder mode metadata is not UTF-8", error))?;
        mode_value.parse()
    }
}
