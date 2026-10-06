//! Daemon lifecycle, locking, and local IPC support.
mod ipc;
mod pool;
mod route;
#[path = "daemon/daemon.rs"]
mod runtime;
mod state;
mod supervisor;
mod transfers;

pub use transfers::{TransferJobSummary, process_transfer_jobs};

#[cfg(unix)]
pub use ipc::UnixSocketTransport;
pub use ipc::{
    BlobPeerAvailability, DaemonHandle, EntryRow, FolderEntry, FolderRegistry, FolderStatus, IpcClient, IpcCommand,
    IpcListener, IpcRequest, IpcResponse, IpcServer, IpcTransport, JoinOptions, PackageInfoResult,
    PeerAvailabilityReport, PeerInfo, SnapshotDiffReport, SnapshotInfo, WatchConfig,
};
pub use pool::ManagedPool;
pub use route::{daemon_client, try_daemon, with_node};
pub use runtime::{Daemon, DaemonConfig};
pub use state::{
    BandwidthSnapshot, DaemonState, DaemonStatus, DaemonStatusReport, FolderStatusReport, PidLock, ScheduleStatus,
    StateFile, current_timestamp, daemon_socket_path, load_filter_engine, pid_is_alive,
};
pub use supervisor::{DEFAULT_RETRY_WINDOW, IntentSupervisor, SupervisedIntent};
