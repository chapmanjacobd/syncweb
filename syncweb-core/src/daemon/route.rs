use std::path::Path;

use crate::error::Result;

use super::{DaemonStatus, IpcClient, PidLock};

/// Return an IPC client only when a lock owner answers a status probe.
///
/// # Errors
///
/// Returns an error when the lock cannot be inspected.
pub fn daemon_client(data_dir: &Path) -> Result<Option<IpcClient>> {
    let lock = PidLock::new(data_dir);
    if lock.try_acquire()? {
        lock.release()?;
        return Ok(None);
    }
    let client = IpcClient::new(data_dir);
    match client.status_sync() {
        Ok(DaemonStatus::Starting | DaemonStatus::Running) => Ok(Some(client)),
        Ok(_) | Err(_) => Ok(None),
    }
}

/// Alias for the one-shot command routing check.
///
/// # Errors
///
/// Returns an error when the lock cannot be inspected.
pub fn try_daemon(data_dir: &Path) -> Result<Option<IpcClient>> {
    daemon_client(data_dir)
}

/// Run a node operation with an already-running daemon client.
///
/// The operation is deliberately synchronous so callers can construct and
/// pass a client without opening the node in the CLI process.
///
/// # Errors
///
/// Returns an error when no responsive daemon owns the data directory.
pub async fn with_node<F, R>(data_dir: &Path, operation: F) -> Result<R>
where
    F: FnOnce(IpcClient) -> R,
{
    let client = daemon_client(data_dir)?
        .ok_or_else(|| crate::error::SyncwebError::operation("daemon not running", "start with `syncweb daemon`"))?;
    let result = operation(client);
    Ok(result)
}
