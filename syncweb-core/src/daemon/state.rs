use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::error::{Result, SyncwebError};

const STATE_FILE_NAME: &str = "daemon.state";
const LOCK_FILE_NAME: &str = "daemon.lock";
const SOCKET_FILE_NAME: &str = "daemon.sock";
const RUNTIME_SOCKET_FILE_NAME: &str = "syncweb.sock";

/// The lifecycle state persisted by a running daemon.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum DaemonStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
}

/// Identifying and lifecycle information for a daemon instance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct DaemonState {
    pub pid: u32,
    pub node_id: String,
    pub started_at: u64,
    pub data_dir: PathBuf,
    pub status: DaemonStatus,
}

impl DaemonState {
    #[must_use]
    pub fn new(
        pid: u32,
        node_id: impl Into<String>,
        started_at: u64,
        data_dir: impl Into<PathBuf>,
        status: DaemonStatus,
    ) -> Self {
        Self {
            pid,
            node_id: node_id.into(),
            started_at,
            data_dir: data_dir.into(),
            status,
        }
    }
}

/// Atomic JSON persistence for daemon state.
#[derive(Clone, Debug)]
pub struct StateFile {
    path: PathBuf,
}

impl StateFile {
    #[must_use]
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join(STATE_FILE_NAME),
        }
    }

    /// Persist state using a same-directory temporary file and rename.
    ///
    /// # Errors
    ///
    /// Returns an error when the directory or state file cannot be written.
    pub fn save(&self, state: &DaemonState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(state)
            .map_err(|error| SyncwebError::operation("failed to serialize daemon state", error))?;
        let temporary = self
            .path
            .with_file_name(format!("{}.tmp-{}", STATE_FILE_NAME, std::process::id()));
        let mut file = File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, &self.path)?;
        Ok(())
    }

    /// Load the state, returning `None` when no state file exists.
    ///
    /// # Errors
    ///
    /// Returns an error when the state file cannot be read or contains invalid JSON.
    pub fn load(&self) -> Result<Option<DaemonState>> {
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        serde_json::from_str(&contents)
            .map(Some)
            .map_err(|error| SyncwebError::operation("failed to deserialize daemon state", error))
    }

    #[must_use]
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Remove the state file if it exists.
    ///
    /// # Errors
    ///
    /// Returns an error when the state file cannot be removed.
    pub fn remove(&self) -> Result<()> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    /// Return the initial PID liveness hint stored in the state file.
    ///
    /// A positive result does not prove that the process is the syncweb
    /// daemon; callers should verify it through IPC.
    ///
    /// # Errors
    ///
    /// Returns an error when the state file cannot be read or decoded.
    pub fn is_running(&self) -> Result<bool> {
        Ok(self.load()?.is_some_and(|state| {
            matches!(state.status, DaemonStatus::Starting | DaemonStatus::Running) && pid_is_alive(state.pid)
        }))
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Exclusive process lock protecting the redb-backed node.
#[derive(Debug)]
pub struct PidLock {
    lock_path: PathBuf,
    state_path: PathBuf,
    lock: Mutex<Option<File>>,
}

impl PidLock {
    #[must_use]
    pub fn new(data_dir: &Path) -> Self {
        Self {
            lock_path: data_dir.join(LOCK_FILE_NAME),
            state_path: StateFile::new(data_dir).path,
            lock: Mutex::new(None),
        }
    }

    /// Try to acquire and retain the lock for this process.
    ///
    /// The PID is written only after the OS-level lock succeeds. A `false`
    /// result means another process currently owns the lock.
    ///
    /// # Errors
    ///
    /// Returns an error when the lock file cannot be opened or updated.
    pub fn try_acquire(&self) -> Result<bool> {
        let mut held = self
            .lock
            .lock()
            .map_err(|error| SyncwebError::operation("daemon lock mutex is poisoned", error))?;
        if held.is_some() {
            return Ok(true);
        }
        if let Some(parent) = self.lock_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut candidate = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&self.lock_path)?;
        if let Err(error) = candidate.try_lock_exclusive() {
            if error.kind() == std::io::ErrorKind::WouldBlock {
                return Ok(false);
            }
            return Err(error.into());
        }
        candidate.set_len(0)?;
        writeln!(candidate, "{}", std::process::id())?;
        candidate.sync_all()?;
        *held = Some(candidate);
        drop(held);
        Ok(true)
    }

    /// Release the lock held by this instance.
    ///
    /// # Errors
    ///
    /// Returns an error when the OS refuses to release the lock.
    pub fn release(&self) -> Result<()> {
        let mut held = self
            .lock
            .lock()
            .map_err(|error| SyncwebError::operation("daemon lock mutex is poisoned", error))?;
        if let Some(file) = held.take() {
            file.unlock()?;
        }
        drop(held);
        Ok(())
    }

    /// Read the last PID written to the lock file.
    ///
    /// # Errors
    ///
    /// Returns an error when the lock file cannot be read or its contents are
    /// not a decimal process ID.
    pub fn owner_pid(&self) -> Result<Option<u32>> {
        let contents = match fs::read_to_string(&self.lock_path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        contents
            .split_whitespace()
            .next()
            .map(|value| {
                value
                    .parse()
                    .map_err(|error| SyncwebError::operation("invalid daemon lock PID", error))
            })
            .transpose()
    }

    #[must_use]
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }

    #[must_use]
    pub fn state_path(&self) -> &Path {
        &self.state_path
    }
}

impl Drop for PidLock {
    fn drop(&mut self) {
        if let Ok(mut held) = self.lock.lock()
            && let Some(file) = held.take()
        {
            let _ = file.unlock();
        }
    }
}

/// Return the socket path used for a data directory.
///
/// Runtime sockets belong in `XDG_RUNTIME_DIR`; the data directory remains
/// the fallback for environments without an XDG runtime directory.
#[must_use]
pub fn daemon_socket_path(data_dir: &Path) -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|value| !value.is_empty())
        .map_or_else(
            || data_dir.join(SOCKET_FILE_NAME),
            |runtime_dir| PathBuf::from(runtime_dir).join(RUNTIME_SOCKET_FILE_NAME),
        )
}

/// Check whether a process ID is currently present on the local system.
#[must_use]
pub fn pid_is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    if pid == std::process::id() {
        return true;
    }
    #[cfg(unix)]
    {
        Path::new("/proc").join(pid.to_string()).exists()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[must_use]
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
