use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    str::FromStr,
    sync::Arc,
};

use iroh::PublicKey;
use iroh_blobs::ticket::BlobTicket;
use iroh_docs::{DocTicket, NamespaceId, api::Doc};
use n0_future::StreamExt;
use tokio::sync::RwLock;

use crate::error::{Result, SyncwebError};
use crate::node::discovery::TopicTracker;
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
    subscriptions: Arc<RwLock<HashSet<iroh_blobs::Hash>>>,
    topic_tracker: TopicTracker,
    announced: Arc<RwLock<HashSet<NamespaceId>>>,
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
            subscriptions: Arc::new(RwLock::new(HashSet::new())),
            topic_tracker: node.topic_tracker().clone(),
            announced: Arc::new(RwLock::new(HashSet::new())),
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
        self.announce_namespace(folder.namespace_id()).await;
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
        self.announce_namespace(folder.namespace_id()).await;
        Ok(folder)
    }

    /// Subscribe to a public blob ticket.
    /// Fetches the blob and returns its hash. Does NOT create an Iroh doc
    /// namespace.
    ///
    /// # Errors
    ///
    /// Returns an error if the blob cannot be fetched.
    pub async fn subscribe_public(&self, ticket: &BlobTicket) -> Result<iroh_blobs::Hash> {
        let hash = ticket.hash();
        self.blob_store.fetch(&self.endpoint, ticket).await?;
        self.subscriptions.write().await.insert(hash);
        Ok(hash)
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
        self.announce_namespace(namespace_id).await;
        Ok(folder)
    }

    /// Announce a namespace to the topic tracker if not already announced.
    /// Idempotent — safe to call multiple times for the same namespace.
    pub async fn announce_namespace(&self, namespace_id: NamespaceId) {
        if self.announced.read().await.contains(&namespace_id) {
            return;
        }
        if let Err(error) = self.topic_tracker.announce(namespace_id).await {
            tracing::warn!(%namespace_id, %error, "failed to announce folder namespace");
        } else {
            self.announced.write().await.insert(namespace_id);
        }
    }

    /// Return all managed public subscription hashes.
    #[must_use]
    pub async fn subscriptions(&self) -> Vec<iroh_blobs::Hash> {
        self.subscriptions.read().await.iter().copied().collect()
    }

    /// Seed the subscription set from persisted hashes (e.g. loaded from `SQLite`
    /// on startup).
    pub async fn seed_subscriptions(&self, hashes: impl IntoIterator<Item = iroh_blobs::Hash>) {
        self.subscriptions.write().await.extend(hashes);
    }

    /// Drop (unsubscribe from) a public blob subscription.
    pub async fn drop_subscription(&self, hash: &iroh_blobs::Hash) {
        self.subscriptions.write().await.remove(hash);
    }

    /// Check whether a public subscription for the given hash exists.
    #[must_use]
    pub async fn has_subscription(&self, hash: &iroh_blobs::Hash) -> bool {
        self.subscriptions.read().await.contains(hash)
    }

    /// # Errors
    ///
    /// Returns an error if the folder namespace cannot be dropped.
    pub async fn drop(&self, namespace_id: NamespaceId) -> Result<()> {
        self.docs_engine.drop_namespace(namespace_id).await?;
        self.folders.write().await.remove(&namespace_id);
        Ok(())
    }

    /// Drop a namespace, retrying briefly while its live session replica is
    /// still closing.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder namespace cannot be dropped.
    pub async fn drop_when_ready(&self, namespace_id: NamespaceId) -> Result<()> {
        const MAX_ATTEMPTS: u32 = 50;
        const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(50);
        let mut attempts = 0_u32;
        loop {
            match self.drop(namespace_id).await {
                Err(error) if attempts < MAX_ATTEMPTS && error.to_string().contains("replica is not closed") => {
                    attempts = attempts.saturating_add(1);
                    tokio::time::sleep(RETRY_DELAY).await;
                }
                result => return result,
            }
        }
    }

    /// Delete the local files of a materialized folder at `path`.
    ///
    /// Refuses to delete a filesystem root or the current working directory;
    /// only the folder's registered path may be deleted.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be resolved, is unsafe, or cannot be
    /// deleted.
    pub async fn delete_folder_files(path: &std::path::Path) -> Result<()> {
        let canonical = std::fs::canonicalize(path)
            .map_err(|error| SyncwebError::operation("failed to resolve folder path for deletion", error))?;
        let current = std::env::current_dir()
            .map_err(|error| SyncwebError::operation("failed to resolve current directory", error))?;
        if canonical.as_os_str().is_empty()
            || canonical.parent().is_none_or(|parent| parent.as_os_str().is_empty())
            || canonical == current
        {
            return Err(SyncwebError::operation(
                "refusing to delete files at a filesystem root or the current directory",
                "unsafe folder path",
            ));
        }
        let metadata = tokio::fs::symlink_metadata(&canonical).await?;
        if metadata.is_dir() {
            tokio::fs::remove_dir_all(&canonical).await?;
        } else if metadata.is_file() {
            tokio::fs::remove_file(&canonical).await?;
        } else {
            return Err(SyncwebError::operation(
                "refusing to delete a non-file, non-directory path",
                "unsupported folder path",
            ));
        }
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
