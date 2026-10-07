use std::net::SocketAddr;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

use crate::SyncwebError;
use crate::daemon::{IpcRequest, IpcResponse, IpcServer};

/// Default listen address for the local WebSocket bridge.
pub const DEFAULT_BRIDGE_LISTEN: &str = "127.0.0.1:9192";

/// A local WebSocket bridge that proxies JSON [`IpcRequest`] messages to the
/// daemon's control channel and streams the JSON [`IpcResponse`] back.
///
/// This lets scriptable clients (browsers, `websocat`, monitoring tools) drive
/// the daemon over the same surface the CLI uses over its Unix socket.
pub struct BridgeServer {
    addr: SocketAddr,
    ipc: IpcServer,
}

impl BridgeServer {
    /// Construct a bridge from a listen address string.
    ///
    /// # Errors
    ///
    /// Returns an error if the listen address string cannot be parsed.
    pub fn new(listen_addr: &str, ipc: IpcServer) -> Result<Self, SyncwebError> {
        let addr = listen_addr
            .parse::<SocketAddr>()
            .map_err(|error| SyncwebError::operation("invalid websocket bridge listen address", error))?;
        Ok(Self { addr, ipc })
    }

    /// Bind the TCP listener and run until the shutdown signal is received.
    ///
    /// # Errors
    ///
    /// Returns an error if the TCP listener cannot be bound.
    pub async fn run(self, shutdown: broadcast::Sender<()>) -> Result<(), SyncwebError> {
        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|error| SyncwebError::operation("failed to bind websocket bridge", error))?;
        let local_addr = listener
            .local_addr()
            .map_err(|error| SyncwebError::operation("failed to get websocket bridge local address", error))?;
        tracing::info!(addr = %local_addr, "websocket bridge started");

        let mut shutdown_rx = shutdown.subscribe();
        loop {
            tokio::select! {
                shutdown_result = shutdown_rx.recv() => {
                    match shutdown_result {
                        Ok(()) | Err(broadcast::error::RecvError::Closed) => break,
                        Err(broadcast::error::RecvError::Lagged(_)) => {}
                    }
                }
                accepted = listener.accept() => {
                    let (stream, peer) = accepted.map_err(|error| {
                        SyncwebError::operation("websocket bridge accept failed", error)
                    })?;
                    let ipc = self.ipc.clone();
                    tokio::spawn(async move {
                        if let Err(error) = Box::pin(serve_connection(stream, peer, ipc)).await {
                            tracing::warn!(%peer, %error, "websocket bridge connection failed");
                        }
                    });
                }
            }
        }
        tracing::debug!("websocket bridge shutdown signal received");
        Ok(())
    }

    #[must_use]
    pub fn spawn(self, shutdown: broadcast::Sender<()>) -> JoinHandle<Result<(), SyncwebError>> {
        tokio::spawn(async move { self.run(shutdown).await })
    }
}

async fn serve_connection(stream: tokio::net::TcpStream, peer: SocketAddr, ipc: IpcServer) -> Result<(), SyncwebError> {
    let websocket = tokio_tungstenite::accept_async(stream)
        .await
        .map_err(|error| SyncwebError::operation("websocket bridge handshake failed", error))?;
    tracing::debug!(%peer, "websocket bridge connection accepted");
    let (mut sink, mut incoming) = websocket.split();
    while let Some(item) = incoming.next().await {
        let message = item.map_err(|error| SyncwebError::operation("websocket bridge receive failed", error))?;
        match message {
            Message::Text(text) => {
                let response = match serde_json::from_str::<IpcRequest>(text.as_ref()) {
                    Ok(request) => ipc.handle_request(request).await,
                    Err(error) => IpcResponse::Error {
                        message: format!("invalid JSON: {error}"),
                    },
                };
                let payload = serde_json::to_string(&response)
                    .map_err(|error| SyncwebError::operation("failed to serialize bridge response", error))?;
                sink.send(Message::Text(payload.into()))
                    .await
                    .map_err(|error| SyncwebError::operation("websocket bridge send failed", error))?;
            }
            Message::Close(_) => break,
            Message::Binary(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
        }
    }
    Ok(())
}
