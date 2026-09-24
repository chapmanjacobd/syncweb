use std::{collections::HashMap, path::Path, sync::Arc};

use async_trait::async_trait;
use iroh::PublicKey;
use iroh_blobs::{Hash, ticket::BlobTicket};
use iroh_docs::{AuthorId, DocTicket, NamespaceId, api::Doc};
use tokio::sync::RwLock;

use crate::error::{Result, SyncwebError};
use crate::node::{blob_store::BlobStore, docs_engine::DocsEngine};

use super::SyncMode;
use super::public_subscription::{EntryLike, FolderLike};

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

    #[must_use]
    pub const fn docs_engine(&self) -> &DocsEngine {
        &self.docs_engine
    }

    pub async fn grant(&self, node_id: PublicKey, capability: Capability) {
        self.capabilities.write().await.insert(node_id, capability);
    }

    pub async fn capability(&self, node_id: PublicKey) -> Option<Capability> {
        self.capabilities.read().await.get(&node_id).copied()
    }

    pub async fn can_write_as(&self, node_id: PublicKey) -> bool {
        self.sync_mode.can_write_locally() && self.capability(node_id).await.is_some_and(Capability::can_write)
    }

    /// Check whether a blob is complete in this folder's local store.
    ///
    /// # Errors
    ///
    /// Returns an error if the blob store cannot be queried.
    pub async fn has_local(&self, hash: Hash) -> Result<bool> {
        self.blob_store.has(hash).await
    }

    /// Track this folder's metadata without fetching content into the local
    /// store (user stories 1 and 4). Doc entries still sync, but iroh-docs'
    /// download policy keeps the blobs remote until an explicit `download`.
    ///
    /// # Errors
    ///
    /// Returns an error if the download policy cannot be persisted.
    pub async fn set_metadata_only(&self) -> Result<()> {
        self.set_download_policy(iroh_docs::store::DownloadPolicy::NothingExcept(Vec::new()))
            .await
    }

    /// Restore the default download policy (fetch all referenced content during
    /// doc sync).
    ///
    /// # Errors
    ///
    /// Returns an error if the download policy cannot be persisted.
    pub async fn clear_metadata_only(&self) -> Result<()> {
        self.set_download_policy(iroh_docs::store::DownloadPolicy::EverythingExcept(Vec::new()))
            .await
    }

    /// Whether this folder is currently in metadata-only mode (content stays
    /// out of the local store until an explicit download).
    ///
    /// # Errors
    ///
    /// Returns an error if the download policy cannot be read.
    pub async fn is_metadata_only(&self) -> Result<bool> {
        let policy = self
            .doc
            .get_download_policy()
            .await
            .map_err(|error| SyncwebError::operation("failed to read folder download policy", error))?;
        Ok(matches!(policy, iroh_docs::store::DownloadPolicy::NothingExcept(_)))
    }

    async fn set_download_policy(&self, policy: iroh_docs::store::DownloadPolicy) -> Result<()> {
        self.doc
            .set_download_policy(policy)
            .await
            .map_err(|error| SyncwebError::operation("failed to set folder download policy", error))
    }

    /// List the folder's content entries, excluding `sys/` metadata keys.
    ///
    /// Used by stats, verify, and health-reporting paths so the "iterate the
    /// latest per-key entries, skip system keys" pattern lives in one place.
    ///
    /// # Errors
    ///
    /// Returns an error if the document query fails.
    pub async fn content_entries(&self) -> Result<Vec<iroh_docs::Entry>> {
        let entries = self.docs_engine.list_latest(self.doc()).await?;
        Ok(entries
            .into_iter()
            .filter(|entry| !entry.key().starts_with(b"sys/"))
            .collect())
    }

    /// # Errors
    ///
    /// Returns an error if the blob fails to be stored or set.
    pub async fn set_blob(&self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Result<Hash> {
        if !self.sync_mode.can_write_locally() {
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
        if !self.sync_mode.can_write_locally() {
            return Err(SyncwebError::WriteDenied {
                mode: self.sync_mode.to_string(),
            });
        }
        self.docs_engine.set_blob(&self.doc, self.author, key, hash, size).await
    }

    /// Delete a folder entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder is read-only or the document entry cannot be deleted.
    pub async fn delete_entry(&self, key: impl AsRef<[u8]>) -> Result<()> {
        if !self.sync_mode.can_write_locally() {
            return Err(SyncwebError::WriteDenied {
                mode: self.sync_mode.to_string(),
            });
        }
        self.docs_engine.delete(&self.doc, self.author, key).await
    }

    /// # Errors
    ///
    /// Returns an error if the folder ticket cannot be created.
    pub async fn ticket(&self, writable: bool) -> Result<DocTicket> {
        let can_write = writable && self.sync_mode.can_grant_write();
        self.docs_engine.share_ticket(&self.doc, can_write).await
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
        let ticket = self.blob_store.ticket_for_addr(endpoint, hash);
        self.blob_store
            .pin(crate::pins::public_blob_pin(self.namespace_id, hash), hash)
            .await?;
        Ok(ticket)
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
        self.blob_store
            .unpin(crate::pins::public_blob_pin(self.namespace_id, hash))
            .await
    }

    /// Pin every blob currently referenced by the folder's document so the
    /// shared content is retained (not evicted by garbage collection).
    ///
    /// Returns the number of distinct blobs pinned.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder's entries cannot be read or a pin fails.
    pub async fn pin_all_content(&self) -> Result<usize> {
        let entries = self.docs_engine.list_latest(&self.doc).await?;
        let mut pinned = 0_usize;
        for entry in entries {
            let hash = entry.content_hash();
            self.blob_store
                .pin(crate::pins::public_blob_pin(self.namespace_id, hash), hash)
                .await?;
            pinned = pinned.saturating_add(1);
        }
        Ok(pinned)
    }

    /// Remove the retention pins placed by [`pin_all_content`](Self::pin_all_content)
    /// for every blob currently referenced by the folder's document.
    ///
    /// Returns the number of pins removed.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder's entries cannot be read or an unpin fails.
    pub async fn unpin_all_content(&self) -> Result<usize> {
        let entries = self.docs_engine.list_latest(&self.doc).await?;
        let mut unpinned = 0_usize;
        for entry in entries {
            let hash = entry.content_hash();
            self.blob_store
                .unpin(crate::pins::public_blob_pin(self.namespace_id, hash))
                .await?;
            unpinned = unpinned.saturating_add(1);
        }
        Ok(unpinned)
    }
}

#[async_trait]
impl FolderLike for SyncwebFolder {
    fn namespace_id(&self) -> String {
        self.namespace_id.to_string()
    }

    fn label(&self) -> String {
        self.namespace_id.to_string()
    }

    fn kind(&self) -> &'static str {
        "folder"
    }

    fn path(&self) -> Option<&Path> {
        None
    }

    async fn list_entries(&self) -> Result<Vec<EntryLike>> {
        let entries = self.docs_engine.list_latest(&self.doc).await?;
        let mut result = Vec::with_capacity(entries.len());
        for entry in entries {
            if entry.key().starts_with(b"sys/") {
                continue;
            }
            let path = String::from_utf8_lossy(entry.key()).to_string();
            result.push(EntryLike {
                path,
                hash: entry.content_hash(),
                size: entry.content_len(),
            });
        }
        Ok(result)
    }
}
