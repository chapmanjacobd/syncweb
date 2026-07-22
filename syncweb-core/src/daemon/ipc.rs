use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    time::timeout,
};

use crate::{
    error::{Result, SyncwebError},
    filter::FilterConfig,
    folder::archive_export::DropExportResult,
    folder::archive_import::DropImportResult,
    sync::{FetchStrategy, SessionMode, SubscribeParams},
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
                .map_err(|error| SyncwebError::operation("daemon IPC connection timed out", error))??;
            let mut message = serde_json::to_vec(&request)
                .map_err(|error| SyncwebError::operation("failed to serialize IPC request", error))?;
            message.push(b'\n');
            timeout(IPC_TIMEOUT, stream.write_all(&message))
                .await
                .map_err(|error| SyncwebError::operation("daemon IPC write timed out", error))??;
            let mut response = Vec::new();
            let mut reader = BufReader::new(stream);
            timeout(IPC_TIMEOUT, reader.read_until(b'\n', &mut response))
                .await
                .map_err(|error| SyncwebError::operation("daemon IPC read timed out", error))??;
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
