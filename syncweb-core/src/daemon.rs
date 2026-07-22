//! Daemon lifecycle, locking, and local IPC support.
mod ipc;
mod route;
mod state;

pub use ipc::{
    DaemonHandle, FolderEntry, FolderRegistry, FolderStatus, IpcClient, IpcCommand, IpcListener, IpcRequest,
    IpcResponse, IpcServer,
};
pub use route::{daemon_client, try_daemon, with_node};
pub use state::{DaemonState, DaemonStatus, PidLock, StateFile, current_timestamp, daemon_socket_path, pid_is_alive};
