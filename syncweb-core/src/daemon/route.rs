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
        .ok_or_else(|| crate::error::SyncwebError::operation("daemon not running", "start with `syncweb start`"))?;
    let result = operation(client);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_client_returns_none_when_no_lock() {
        let dir = std::env::temp_dir().join(format!("syncweb-route-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let result = daemon_client(&dir);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn try_daemon_returns_none_when_no_lock() {
        let dir = std::env::temp_dir().join(format!("syncweb-route-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let result = try_daemon(&dir);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
