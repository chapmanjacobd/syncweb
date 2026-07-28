use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Router, routing::get};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::node::blob_store::BlobStore;

use super::serve::{MediaState, serve_media};

pub const DEFAULT_MEDIA_PORT: u16 = 9193;

pub struct MediaServer {
    addr: SocketAddr,
    state: Arc<MediaState>,
}

impl MediaServer {
    #[must_use]
    pub fn new(addr: SocketAddr, blob_store: BlobStore) -> Self {
        Self {
            addr,
            state: Arc::new(MediaState { blob_store }),
        }
    }

    /// Bind the TCP listener and run until the shutdown signal is received.
    ///
    /// # Errors
    ///
    /// Returns an error if the TCP listener cannot be bound.
    pub async fn run(self, shutdown: broadcast::Sender<()>) -> Result<(), crate::SyncwebError> {
        let app = Router::new()
            .route("/media/{hash}", get(serve_media))
            .with_state(self.state);

        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|error| crate::SyncwebError::operation("failed to bind media server", error))?;

        let local_addr = listener
            .local_addr()
            .map_err(|error| crate::SyncwebError::operation("failed to get media server local address", error))?;

        tracing::info!(addr = %local_addr, "media server started");

        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let mut rx = shutdown.subscribe();
                let _ = rx.recv().await;
                tracing::debug!("media server shutdown signal received");
            })
            .await
            .map_err(|error| crate::SyncwebError::operation("media server failed", error))?;

        Ok(())
    }

    #[must_use]
    pub fn spawn(self, shutdown: broadcast::Sender<()>) -> JoinHandle<Result<(), crate::SyncwebError>> {
        tokio::spawn(async move { self.run(shutdown).await })
    }
}
