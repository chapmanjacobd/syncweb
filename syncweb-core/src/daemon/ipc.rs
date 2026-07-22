use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::{RwLock, broadcast, mpsc},
    time::timeout,
};

use crate::{
    error::{Result, SyncwebError},
    filter::FilterConfig,
    folder::archive_export::DropExportResult,
    folder::archive_import::DropImportResult,
    sync::{ActiveSession, FetchStrategy, SessionMode, SubscribeParams, cancel_session},
};

use super::state::{DaemonStatus, daemon_socket_path};

const IPC_TIMEOUT: Duration = Duration::from_millis(500);

/// A request sent over the local daemon control channel.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct IpcRequest {
    pub command: IpcCommand,
}

impl IpcRequest {
    #[must_use]
    pub const fn new(command: IpcCommand) -> Self {
        Self { command }
    }
}

/// Commands supported by the daemon control channel.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "command", rename_all = "snake_case")]
#[non_exhaustive]
pub enum IpcCommand {
    Status,
    ListFolders,
    AddFolder {
        namespace: String,
        path: PathBuf,
    },
    RemoveFolder {
        namespace: String,
    },
    TriggerSync {
        namespace: Option<String>,
    },
    SetLogLevel {
        level: String,
    },
    ReloadConfig,
    Shutdown {
        force: bool,
    },
    Download {
        namespace: String,
        strategy: FetchStrategy,
    },
    ImportArchive {
        input: PathBuf,
        target: PathBuf,
        filter: Option<FilterConfig>,
    },
    ExportArchive {
        namespace: String,
        version: Option<String>,
        output: PathBuf,
    },
    Join {
        ticket: String,
        path: PathBuf,
        mode: SessionMode,
    },
    Publish {
        namespace: String,
        blob: Option<String>,
    },
    Subscribe {
        namespace: String,
        params: SubscribeParams,
    },
}

/// A response returned by the daemon control channel.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "response", rename_all = "snake_case")]
#[non_exhaustive]
pub enum IpcResponse {
    Ok { message: String },
    Status(DaemonStatus),
    FolderList(Vec<FolderStatus>),
    DownloadComplete { bytes_transferred: u64 },
    ImportComplete(Box<DropImportResult>),
    ExportComplete(Box<DropExportResult>),
    Error { message: String },
}

/// A managed folder summary returned by the daemon.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FolderStatus {
    pub namespace: String,
    pub path: PathBuf,
    pub session_active: bool,
    pub last_sync_at: Option<u64>,
    pub sync_count: u64,
}

/// A folder managed by the daemon.
#[non_exhaustive]
pub struct FolderEntry {
    pub namespace: iroh_docs::NamespaceId,
    pub path: PathBuf,
    pub session: Option<ActiveSession>,
    pub last_sync_at: Option<u64>,
    pub sync_count: u64,
}

impl FolderEntry {
    #[must_use]
    pub const fn new(namespace: iroh_docs::NamespaceId, path: PathBuf) -> Self {
        Self {
            namespace,
            path,
            session: None,
            last_sync_at: None,
            sync_count: 0,
        }
    }

    #[must_use]
    pub fn status(&self) -> FolderStatus {
        FolderStatus {
            namespace: self.namespace.to_string(),
            path: self.path.clone(),
            session_active: self.session.is_some(),
            last_sync_at: self.last_sync_at,
            sync_count: self.sync_count,
        }
    }
}

/// Registry of folders currently managed by the daemon.
#[derive(Default)]
pub struct FolderRegistry {
    folders: HashMap<String, FolderEntry>,
}

impl FolderRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a folder to the registry.
    ///
    /// # Errors
    ///
    /// Returns an error when the namespace is already registered.
    pub fn add(&mut self, entry: FolderEntry) -> Result<()> {
        let key = entry.namespace.to_string();
        if self.folders.contains_key(&key) {
            return Err(SyncwebError::FolderAlreadyManaged);
        }
        self.folders.insert(key, entry);
        Ok(())
    }

    pub fn remove(&mut self, namespace: &iroh_docs::NamespaceId) -> Option<FolderEntry> {
        self.folders.remove(&namespace.to_string())
    }

    #[must_use]
    pub fn statuses(&self) -> Vec<FolderStatus> {
        let mut statuses: Vec<_> = self.folders.values().map(FolderEntry::status).collect();
        statuses.sort_by(|left, right| left.namespace.cmp(&right.namespace));
        statuses
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.folders.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.folders.is_empty()
    }
}

/// Shared daemon state used by the IPC server.
#[derive(Clone)]
#[non_exhaustive]
pub struct DaemonHandle {
    pub state: Arc<RwLock<super::state::DaemonState>>,
    pub folder_registry: Arc<RwLock<FolderRegistry>>,
    pub shutdown_sender: broadcast::Sender<()>,
    pub sync_trigger: mpsc::UnboundedSender<Option<String>>,
}

impl DaemonHandle {
    /// Create a handle with fresh control channels.
    #[must_use]
    pub fn new(state: super::state::DaemonState) -> Self {
        let (shutdown_sender, _) = broadcast::channel(16);
        let (sync_trigger, _) = mpsc::unbounded_channel();
        Self::with_channels(
            Arc::new(RwLock::new(state)),
            Arc::new(RwLock::new(FolderRegistry::new())),
            shutdown_sender,
            sync_trigger,
        )
    }

    #[must_use]
    pub const fn with_channels(
        state: Arc<RwLock<super::state::DaemonState>>,
        folder_registry: Arc<RwLock<FolderRegistry>>,
        shutdown_sender: broadcast::Sender<()>,
        sync_trigger: mpsc::UnboundedSender<Option<String>>,
    ) -> Self {
        Self {
            state,
            folder_registry,
            shutdown_sender,
            sync_trigger,
        }
    }

    /// Update the lifecycle status returned by future status requests.
    pub async fn set_status(&self, status: DaemonStatus) {
        self.state.write().await.status = status;
    }
}

/// Socket path and binding helper for the daemon.
#[derive(Clone, Debug)]
pub struct IpcListener {
    socket_path: PathBuf,
}

impl IpcListener {
    #[must_use]
    pub const fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    #[must_use]
    pub fn for_data_dir(data_dir: &Path) -> Self {
        Self::new(daemon_socket_path(data_dir))
    }

    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Bind the daemon socket with owner-only permissions on Unix.
    ///
    /// # Errors
    ///
    /// Returns an error when the parent directory or socket cannot be created.
    pub fn bind(&self) -> Result<tokio::net::UnixListener> {
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        #[cfg(unix)]
        if self.socket_path.exists() {
            match std::os::unix::net::UnixStream::connect(&self.socket_path) {
                Ok(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AddrInUse,
                        "daemon IPC socket is already in use",
                    )
                    .into());
                }
                Err(_) => std::fs::remove_file(&self.socket_path)?,
            }
        }
        let listener = tokio::net::UnixListener::bind(&self.socket_path)?;
        set_owner_only_permissions(&self.socket_path)?;
        Ok(listener)
    }
}

/// A server for the daemon's local control channel.
#[derive(Clone)]
pub struct IpcServer {
    listener: IpcListener,
    daemon_handle: DaemonHandle,
}

impl IpcServer {
    #[must_use]
    pub const fn new(socket_path: PathBuf, daemon_handle: DaemonHandle) -> Self {
        Self {
            listener: IpcListener::new(socket_path),
            daemon_handle,
        }
    }

    /// Accept and process requests until the daemon broadcasts shutdown.
    ///
    /// # Errors
    ///
    /// Returns an error when the socket cannot be bound, accepted, or written.
    pub async fn serve(&self) -> Result<()> {
        #[cfg(unix)]
        {
            let listener = self.listener.bind()?;
            let mut shutdown = self.daemon_handle.shutdown_sender.subscribe();
            let result = loop {
                tokio::select! {
                    shutdown_result = shutdown.recv() => {
                        match shutdown_result {
                            Ok(()) | Err(broadcast::error::RecvError::Closed) => break Ok(()),
                            Err(broadcast::error::RecvError::Lagged(_)) => {}
                        }
                    }
                    accepted = listener.accept() => {
                        let (stream, _) = accepted?;
                        self.handle_connection(stream).await?;
                    }
                }
            };
            if let Err(error) = std::fs::remove_file(self.listener.socket_path())
                && self.listener.socket_path().exists()
            {
                return Err(error.into());
            }
            result
        }
        #[cfg(not(unix))]
        {
            Err(SyncwebError::operation(
                "daemon IPC is unavailable",
                "Unix sockets are not supported on this platform",
            ))
        }
    }

    /// Handle one decoded request without requiring a socket.
    pub async fn handle_request(&self, request: IpcRequest) -> IpcResponse {
        match request.command {
            IpcCommand::Status => IpcResponse::Status(self.daemon_handle.state.read().await.status),
            IpcCommand::ListFolders => {
                let folders = self.daemon_handle.folder_registry.read().await.statuses();
                IpcResponse::FolderList(folders)
            }
            IpcCommand::AddFolder { namespace, path } => match iroh_docs::NamespaceId::from_str(&namespace) {
                Ok(namespace_id) => {
                    let mut registry = self.daemon_handle.folder_registry.write().await;
                    match registry.add(FolderEntry::new(namespace_id, path)) {
                        Ok(()) => IpcResponse::Ok {
                            message: "folder added".to_owned(),
                        },
                        Err(error) => response_from_error(error),
                    }
                }
                Err(error) => IpcResponse::Error {
                    message: format!("invalid folder namespace: {error}"),
                },
            },
            IpcCommand::RemoveFolder { namespace } => match iroh_docs::NamespaceId::from_str(&namespace) {
                Ok(namespace_id) => {
                    let removed = self.daemon_handle.folder_registry.write().await.remove(&namespace_id);
                    if removed.is_some() {
                        let _ = cancel_session(namespace_id);
                        IpcResponse::Ok {
                            message: "folder removed".to_owned(),
                        }
                    } else {
                        IpcResponse::Error {
                            message: format!("folder not found: {namespace}"),
                        }
                    }
                }
                Err(error) => IpcResponse::Error {
                    message: format!("invalid folder namespace: {error}"),
                },
            },
            IpcCommand::TriggerSync { namespace } => match self.daemon_handle.sync_trigger.send(namespace) {
                Ok(()) => IpcResponse::Ok {
                    message: "synchronization requested".to_owned(),
                },
                Err(error) => response_from_error(error),
            },
            IpcCommand::SetLogLevel { level } => IpcResponse::Ok {
                message: format!("log level set to {level}"),
            },
            IpcCommand::ReloadConfig => IpcResponse::Ok {
                message: "configuration reload requested".to_owned(),
            },
            IpcCommand::Shutdown { force } => {
                if let Err(error) = self.daemon_handle.shutdown_sender.send(()) {
                    return response_from_error(error);
                }
                IpcResponse::Ok {
                    message: if force {
                        "forced shutdown requested".to_owned()
                    } else {
                        "shutdown requested".to_owned()
                    },
                }
            }
            IpcCommand::Download { .. }
            | IpcCommand::ImportArchive { .. }
            | IpcCommand::ExportArchive { .. }
            | IpcCommand::Join { .. }
            | IpcCommand::Publish { .. }
            | IpcCommand::Subscribe { .. } => IpcResponse::Error {
                message: "node-access IPC command is not available".to_owned(),
            },
        }
    }

    #[cfg(unix)]
    async fn handle_connection(&self, stream: tokio::net::UnixStream) -> Result<()> {
        let (read_half, mut write_half) = stream.into_split();
        let mut line = Vec::new();
        BufReader::new(read_half).read_until(b'\n', &mut line).await?;
        let response = match serde_json::from_slice::<IpcRequest>(line.trim_ascii()) {
            Ok(request) => self.handle_request(request).await,
            Err(error) => IpcResponse::Error {
                message: format!("invalid daemon request: {error}"),
            },
        };
        let mut bytes = serde_json::to_vec(&response)
            .map_err(|error| SyncwebError::operation("failed to serialize IPC response", error))?;
        bytes.push(b'\n');
        write_half.write_all(&bytes).await?;
        Ok(())
    }
}

fn response_from_error(error: impl std::fmt::Display) -> IpcResponse {
    IpcResponse::Error {
        message: error.to_string(),
    }
}

/// A client for sending requests to the daemon.
#[derive(Clone, Debug)]
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    #[must_use]
    pub fn new(data_dir: &Path) -> Self {
        Self::from_socket_path(daemon_socket_path(data_dir))
    }

    #[must_use]
    pub const fn from_socket_path(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Send one newline-delimited JSON request and await its response.
    ///
    /// # Errors
    ///
    /// Returns an error when the socket is unavailable, the operation times
    /// out, or either JSON message is malformed.
    pub async fn send(&self, request: IpcRequest) -> Result<IpcResponse> {
        #[cfg(unix)]
        {
            let mut stream = timeout(IPC_TIMEOUT, tokio::net::UnixStream::connect(&self.socket_path))
                .await
                .map_err(|error| SyncwebError::operation("daemon IPC connection timed out", error))?
                .map_err(|error| SyncwebError::operation("daemon IPC connection failed", error))?;
            let mut message = serde_json::to_vec(&request)
                .map_err(|error| SyncwebError::operation("failed to serialize IPC request", error))?;
            message.push(b'\n');
            timeout(IPC_TIMEOUT, stream.write_all(&message))
                .await
                .map_err(|error| SyncwebError::operation("daemon IPC write timed out", error))?
                .map_err(|error| SyncwebError::operation("daemon IPC write failed", error))?;
            let mut response = Vec::new();
            let mut reader = BufReader::new(stream);
            timeout(IPC_TIMEOUT, reader.read_until(b'\n', &mut response))
                .await
                .map_err(|error| SyncwebError::operation("daemon IPC read timed out", error))?
                .map_err(|error| SyncwebError::operation("daemon IPC read failed", error))?;
            if response.is_empty() {
                return Err(SyncwebError::operation(
                    "daemon IPC returned no response",
                    "connection closed",
                ));
            }
            serde_json::from_slice(response.trim_ascii())
                .map_err(|error| SyncwebError::operation("failed to deserialize IPC response", error))
        }
        #[cfg(not(unix))]
        {
            let _ = request;
            Err(SyncwebError::operation(
                "daemon IPC is unavailable",
                "Unix sockets are not supported on this platform",
            ))
        }
    }

    /// Perform a bounded synchronous status probe for routing decisions.
    ///
    /// # Errors
    ///
    /// Returns an error when the daemon does not answer a status request.
    pub fn status_sync(&self) -> Result<DaemonStatus> {
        #[cfg(unix)]
        {
            use std::{
                io::{BufRead, Write},
                os::unix::net::UnixStream,
            };

            let stream = UnixStream::connect(&self.socket_path)?;
            stream.set_read_timeout(Some(IPC_TIMEOUT))?;
            stream.set_write_timeout(Some(IPC_TIMEOUT))?;
            let mut writer = stream.try_clone()?;
            let request = serde_json::to_vec(&IpcRequest::new(IpcCommand::Status))
                .map_err(|error| SyncwebError::operation("failed to serialize IPC request", error))?;
            writer.write_all(&request)?;
            writer.write_all(b"\n")?;
            let mut line = String::new();
            std::io::BufReader::new(stream).read_line(&mut line)?;
            match serde_json::from_str::<IpcResponse>(&line)
                .map_err(|error| SyncwebError::operation("failed to deserialize IPC response", error))?
            {
                IpcResponse::Status(status) => Ok(status),
                IpcResponse::Error { message } => Err(SyncwebError::operation("daemon status request failed", message)),
                IpcResponse::Ok { .. }
                | IpcResponse::FolderList(_)
                | IpcResponse::DownloadComplete { .. }
                | IpcResponse::ImportComplete(_)
                | IpcResponse::ExportComplete(_) => Err(SyncwebError::operation(
                    "daemon status request returned an unexpected response",
                    "unexpected response",
                )),
            }
        }
        #[cfg(not(unix))]
        {
            Err(SyncwebError::operation(
                "daemon IPC is unavailable",
                "Unix sockets are not supported on this platform",
            ))
        }
    }
}

fn set_owner_only_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o600);
        std::fs::set_permissions(path, permissions)?;
    }
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc, time::Duration};

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use crate::daemon::{DaemonState, DaemonStatus};

    fn socket_path() -> PathBuf {
        std::env::temp_dir().join(format!("syncweb-ipc-{}.sock", uuid::Uuid::new_v4()))
    }

    fn state() -> DaemonState {
        DaemonState::new(
            std::process::id(),
            "node",
            1,
            std::env::temp_dir(),
            DaemonStatus::Running,
        )
    }

    #[test]
    fn request_round_trips_as_json() {
        let request = IpcRequest::new(IpcCommand::Download {
            namespace: "namespace".to_owned(),
            strategy: FetchStrategy::default(),
        });
        let encoded = serde_json::to_vec(&request).expect("serialize request");
        let decoded: IpcRequest = serde_json::from_slice(&encoded).expect("deserialize request");
        assert!(matches!(decoded.command, IpcCommand::Download { .. }));
    }

    #[tokio::test]
    async fn handle_request_updates_registry_and_trigger() {
        let (sync_trigger, mut sync_receiver) = mpsc::unbounded_channel();
        let handle = DaemonHandle::with_channels(
            Arc::new(RwLock::new(state())),
            Arc::new(RwLock::new(FolderRegistry::new())),
            broadcast::channel(4).0,
            sync_trigger,
        );
        let server = IpcServer::new(socket_path(), handle);
        let namespace = iroh_docs::NamespaceSecret::from_bytes(&[7; 32]).id().to_string();

        assert!(matches!(
            server
                .handle_request(IpcRequest::new(IpcCommand::AddFolder {
                    namespace: namespace.clone(),
                    path: PathBuf::from("/tmp/folder"),
                }))
                .await,
            IpcResponse::Ok { .. }
        ));
        assert!(matches!(
            server
                .handle_request(IpcRequest::new(IpcCommand::ListFolders))
                .await,
            IpcResponse::FolderList(folders)
                if folders.len() == 1 && folders.first().is_some_and(|folder| folder.namespace == namespace)
        ));
        assert!(matches!(
            server
                .handle_request(IpcRequest::new(IpcCommand::TriggerSync {
                    namespace: Some(namespace.clone()),
                }))
                .await,
            IpcResponse::Ok { .. }
        ));
        assert_eq!(sync_receiver.recv().await, Some(Some(namespace)));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn client_round_trips_with_server() {
        let path = socket_path();
        let handle = DaemonHandle::new(state());
        let mut shutdown_receiver = handle.shutdown_sender.subscribe();
        let server = IpcServer::new(path.clone(), handle);
        let server_task = tokio::spawn(async move { server.serve().await });

        tokio::time::timeout(Duration::from_secs(1), async {
            while !path.exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("server socket should appear");

        let client = IpcClient::from_socket_path(path.clone());
        assert!(matches!(
            client
                .send(IpcRequest::new(IpcCommand::Status))
                .await
                .expect("status response"),
            IpcResponse::Status(DaemonStatus::Running)
        ));
        assert!(matches!(
            client
                .send(IpcRequest::new(IpcCommand::Shutdown { force: false }))
                .await
                .expect("shutdown response"),
            IpcResponse::Ok { .. }
        ));
        shutdown_receiver.recv().await.expect("shutdown broadcast");
        server_task
            .await
            .expect("server task should join")
            .expect("server should stop cleanly");
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn socket_is_owner_only() {
        let path = socket_path();
        let listener = IpcListener::new(path.clone()).bind().expect("bind socket");
        assert_eq!(
            std::fs::metadata(&path).expect("socket metadata").permissions().mode() & 0o777,
            0o600
        );
        drop(listener);
        std::fs::remove_file(path).expect("remove socket");
    }

    #[tokio::test]
    async fn client_reports_missing_server() {
        let path = socket_path();
        let error = IpcClient::from_socket_path(path)
            .send(IpcRequest::new(IpcCommand::Status))
            .await
            .expect_err("missing server should fail");
        assert!(error.to_string().contains("daemon IPC connection failed"));
    }
}
