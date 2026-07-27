use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use n0_future::split;
use tokio::sync::broadcast;

use crate::{
    error::{Result, SyncwebError},
    node::iroh_node::IrohNode,
};

use super::{service::BridgeService, session::WsSession};

/// WebSocket bridge server that exposes Iroh P2P primitives over WebSocket.
pub struct WsBridgeServer {
    node: Arc<IrohNode>,
    addr: SocketAddr,
    data_dir: PathBuf,
}

impl WsBridgeServer {
    #[must_use]
    pub const fn new(node: Arc<IrohNode>, addr: SocketAddr, data_dir: PathBuf) -> Self {
        Self { node, addr, data_dir }
    }

    /// Run the bridge server, accepting connections until shutdown is
    /// signalled.
    ///
    /// # Errors
    ///
    /// Returns an error if the TCP listener cannot be bound or if an
    /// accepted connection produces a fatal error.
    pub async fn run(self, shutdown: broadcast::Sender<()>) -> Result<()> {
        let listener = Self::bind_listener(self.addr).await?;

        tracing::info!("WebSocket bridge listening on {}", self.addr);

        let service = Arc::new(BridgeService::new(self.node, self.data_dir, shutdown.clone()).await?);

        let mut server_shutdown = shutdown.subscribe();

        loop {
            tokio::select! {
                _ = server_shutdown.recv() => {
                    tracing::info!("bridge server shutting down");
                    break Ok(());
                }
                accepted = listener.accept() => {
                    Self::handle_accept(accepted, &service, &shutdown).await;
                }
            }
        }
    }

    async fn bind_listener(addr: SocketAddr) -> Result<tokio::net::TcpListener> {
        tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|error| SyncwebError::operation(format!("failed to bind bridge server on {addr}"), error))
    }

    #[expect(clippy::unused_async)]
    async fn handle_accept(
        accepted: std::io::Result<(tokio::net::TcpStream, SocketAddr)>,
        service: &Arc<BridgeService>,
        shutdown: &broadcast::Sender<()>,
    ) {
        let (stream, peer_addr) = match accepted {
            Ok(conn) => conn,
            Err(error) => {
                tracing::warn!(%error, "bridge accept failed");
                return;
            }
        };
        let svc = Arc::clone(service);
        let shutdown_rx = shutdown.subscribe();
        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, svc, shutdown_rx).await {
                tracing::warn!(%peer_addr, %error, "bridge session error");
            }
        });
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    service: Arc<BridgeService>,
    shutdown: broadcast::Receiver<()>,
) -> Result<()> {
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .map_err(|error| SyncwebError::operation("websocket handshake failed", error))?;

    let (sender, mut receiver) = split::split(ws_stream);

    let session = WsSession::new(service, sender, shutdown);
    session.run(&mut receiver).await
}
