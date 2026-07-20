use std::path::Path;

use iroh::endpoint::Endpoint;
use iroh_blobs::Hash;
use iroh_blobs::ticket::BlobTicket;

use crate::{
    error::Result,
    node::{blob_store::BlobStore, docs_engine::DocsEngine},
    sync::{IntentHandle, SyncEvent},
};

/// On-demand content fetcher for selective sync.
#[derive(Clone)]
pub struct LazyFetch {
    blob_store: BlobStore,
    _docs_engine: DocsEngine,
}

impl LazyFetch {
    #[must_use]
    pub const fn new(blob_store: BlobStore, docs_engine: DocsEngine) -> Self {
        Self {
            blob_store,
            _docs_engine: docs_engine,
        }
    }

    /// Fetch a blob only when the caller asks for it.
    /// # Errors
    ///
    /// Returns an error if the database cannot be accessed.
    pub async fn fetch(&self, hash: Hash) -> Result<bytes::Bytes> {
        self.blob_store.get(hash).await
    }

    /// Fetch a missing blob from a peer ticket, then read it locally.
    /// # Errors
    ///
    /// Returns an error if the network or database cannot be accessed.
    pub async fn fetch_remote(&self, endpoint: &Endpoint, ticket: &BlobTicket) -> Result<bytes::Bytes> {
        self.blob_store.fetch(endpoint, ticket).await?;
        self.fetch(ticket.hash()).await
    }

    /// Fetch and write a blob to a local path.
    /// # Errors
    ///
    /// Returns an error if the filesystem or database cannot be accessed.
    pub async fn fetch_to(&self, hash: Hash, path: impl AsRef<Path>) -> Result<()> {
        let bytes = self.fetch(hash).await?;
        if let Some(parent) = path.as_ref().parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, bytes).await?;
        Ok(())
    }

    /// Create an intent that reports the local fetch lifecycle.
    #[must_use]
    pub fn fetch_intent(&self, hash: Hash) -> IntentHandle {
        let (events, mut commands, handle) = IntentHandle::channel();
        let store = self.blob_store.clone();
        tokio::spawn(async move {
            let _ = events.send(SyncEvent::Started);
            let mut paused = false;
            let mut commands_open = true;
            loop {
                tokio::select! {
                    result = store.get(hash), if !paused => {
                        match result {
                            Ok(bytes) => {
                                let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
                                let _ = events.send(SyncEvent::Progress { completed: size, total: Some(size) });
                                let _ = events.send(SyncEvent::Finished);
                            }
                            Err(error) => { let _ = events.send(SyncEvent::Failed(error.to_string())); }
                        }
                        break;
                    }
                    command = commands.recv(), if commands_open => {
                        match command {
                            Some(crate::sync::SyncCommand::Pause) if !paused => {
                                paused = true;
                                let _ = events.send(SyncEvent::Paused);
                            }
                            Some(crate::sync::SyncCommand::Resume) if paused => {
                                paused = false;
                                let _ = events.send(SyncEvent::Resumed);
                            }
                            Some(crate::sync::SyncCommand::Cancel) => {
                                let _ = events.send(SyncEvent::Cancelled);
                                break;
                            }
                            Some(_) => {}
                            None => {
                                commands_open = false;
                                paused = false;
                            }
                        }
                    }
                }
            }
        });
        handle
    }
}
