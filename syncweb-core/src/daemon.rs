//! Daemon lifecycle, locking, and local IPC support.
mod ipc;
mod pool;
mod route;
#[path = "daemon/daemon.rs"]
mod runtime;
mod state;
mod supervisor;

pub use ipc::{
    DaemonHandle, FolderEntry, FolderRegistry, FolderStatus, IpcClient, IpcCommand, IpcListener, IpcRequest,
    IpcResponse, IpcServer,
};
pub use pool::ManagedPool;
pub use route::{daemon_client, try_daemon, with_node};
pub use runtime::{Daemon, DaemonConfig};
pub use state::{
    BandwidthSnapshot, DaemonState, DaemonStatus, DaemonStatusReport, FolderStatusReport, PidLock, ScheduleStatus,
    StateFile, current_timestamp, daemon_socket_path, pid_is_alive,
};
pub use supervisor::{IntentSupervisor, SupervisedIntent};
