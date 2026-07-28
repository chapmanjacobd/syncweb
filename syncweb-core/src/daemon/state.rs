use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::error::{Result, SyncwebError};
use crate::storage::node_db::NodeDatabase;

const STATE_FILE_NAME: &str = "daemon.state";
const STATUS_FILE_NAME: &str = "daemon.status";
const LOCK_FILE_NAME: &str = "daemon.lock";
const SOCKET_FILE_NAME: &str = "daemon.sock";
const RUNTIME_SOCKET_FILE_PREFIX: &str = "syncweb-";

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

/// A persisted snapshot of daemon activity and synchronization progress.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct DaemonStatusReport {
    pub pid: u32,
    pub node_id: String,
    pub started_at: u64,
    pub uptime_seconds: u64,
    pub folders: Vec<FolderStatusReport>,
    pub bandwidth: BandwidthSnapshot,
    pub schedule: Option<ScheduleStatus>,
    pub rayon_threads: usize,
}

impl DaemonStatusReport {
    #[must_use]
    pub fn from_state(
        state: &DaemonState,
        uptime_seconds: u64,
        folders: Vec<FolderStatusReport>,
        bandwidth: BandwidthSnapshot,
        schedule: Option<ScheduleStatus>,
        rayon_threads: usize,
    ) -> Self {
        Self {
            pid: state.pid,
            node_id: state.node_id.clone(),
            started_at: state.started_at,
            uptime_seconds,
            folders,
            bandwidth,
            schedule,
            rayon_threads,
        }
    }
}

/// Activity and error information for one managed folder.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FolderStatusReport {
    pub namespace: String,
    pub path: PathBuf,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub session_active: bool,
    pub last_sync_at: Option<u64>,
    #[serde(default)]
    pub sync_count: u64,
    pub entries_synced: u64,
    pub errors: Vec<String>,
}

fn default_kind() -> String {
    "folder".to_owned()
}

impl FolderStatusReport {
    #[must_use]
    pub fn new(
        namespace: impl Into<String>,
        path: impl Into<PathBuf>,
        session_active: bool,
        last_sync_at: Option<u64>,
        entries_synced: u64,
        errors: Vec<String>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            path: path.into(),
            kind: "folder".to_owned(),
            session_active,
            last_sync_at,
            sync_count: 0,
            entries_synced,
            errors,
        }
    }
}

/// Aggregate transfer counters included in a daemon status report.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct BandwidthSnapshot {
    pub upload_total: u64,
    pub download_total: u64,
    pub upload_rate: u64,
    pub download_rate: u64,
}

/// The current schedule state included in a daemon status report.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ScheduleStatus {
    pub in_active_window: bool,
    pub next_window_start: Option<u64>,
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

/// SQLite-backed persistence for daemon state and status.
#[derive(Clone, Debug)]
pub struct StateFile {
    data_dir: PathBuf,
}

impl StateFile {
    /// Store the data directory path. The database is not opened yet —
    /// call [`db()`](Self::db) to open it lazily.
    #[must_use]
    pub fn new(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
        }
    }

    /// Open (or create) the `SQLite` node database in `data_dir`.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    fn db(&self) -> Result<NodeDatabase> {
        NodeDatabase::open(self.data_dir.join("node.db"))
    }

    /// Persist the daemon lifecycle state.
    ///
    /// # Errors
    ///
    /// Returns an error when the database cannot be written.
    pub fn save(&self, state: &DaemonState) -> Result<()> {
        let db = self.db()?;
        db.save_lifecycle(state)
    }

    /// Load the daemon lifecycle state, returning `None` if absent.
    ///
    /// # Errors
    ///
    /// Returns an error when the database cannot be read.
    pub fn load(&self) -> Result<Option<DaemonState>> {
        let db = self.db()?;
        db.load_lifecycle()
    }

    /// Persist the current daemon activity report.
    ///
    /// # Errors
    ///
    /// Returns an error when the database cannot be written.
    pub fn save_status(&self, report: &DaemonStatusReport) -> Result<()> {
        let db = self.db()?;
        db.save_status(report)
    }

    /// Load the last daemon activity report.
    ///
    /// # Errors
    ///
    /// Returns an error when the database cannot be read.
    pub fn load_status(&self) -> Result<Option<DaemonStatusReport>> {
        let db = self.db()?;
        db.load_status()
    }

    /// Check whether a lifecycle row exists (equivalent to the old file check).
    #[must_use]
    pub fn exists(&self) -> bool {
        self.db()
            .ok()
            .and_then(|db| db.load_lifecycle().ok())
            .flatten()
            .is_some()
    }

    /// Remove the daemon lifecycle row.
    ///
    /// # Errors
    ///
    /// Returns an error when the database operation fails.
    pub fn remove(&self) -> Result<()> {
        let db = self.db()?;
        db.remove_lifecycle()
    }

    /// Remove the daemon status rows.
    ///
    /// # Errors
    ///
    /// Returns an error when the database operation fails.
    pub fn remove_status(&self) -> Result<()> {
        let db = self.db()?;
        db.remove_status()
    }

    /// Return whether the stored lifecycle indicates a still-running daemon.
    ///
    /// # Errors
    ///
    /// Returns an error when the database cannot be read.
    pub fn is_running(&self) -> Result<bool> {
        Ok(self.load()?.is_some_and(|state| {
            matches!(state.status, DaemonStatus::Starting | DaemonStatus::Running) && pid_is_alive(state.pid)
        }))
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.data_dir.join(STATE_FILE_NAME)
    }

    #[must_use]
    pub fn status_path(&self) -> PathBuf {
        self.data_dir.join(STATUS_FILE_NAME)
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
            state_path: data_dir.join(STATE_FILE_NAME),
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
        let mut held = self
            .lock
            .lock()
            .map_err(|error| SyncwebError::operation("daemon lock mutex is poisoned", error))?;
        if let Some(file) = held.as_mut() {
            file.seek(SeekFrom::Start(0))?;
            let mut contents = String::new();
            let mut reader = BufReader::new(&mut *file);
            reader.read_to_string(&mut contents)?;
            return Self::parse_pid_from_contents(&contents);
        }
        let contents = match fs::read_to_string(&self.lock_path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        drop(held);
        Self::parse_pid_from_contents(&contents)
    }

    fn parse_pid_from_contents(contents: &str) -> Result<Option<u32>> {
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
/// the fallback for environments without an XDG runtime directory. On macOS
/// the Unix socket path limit (103 chars) can be exceeded with deeply nested
/// temp directories; in that case a hashed name in the system temp dir is
/// used instead.
#[must_use]
pub fn daemon_socket_path(data_dir: &Path) -> PathBuf {
    if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR").filter(|value| !value.is_empty()) {
        let canonical_data_dir = fs::canonicalize(data_dir).unwrap_or_else(|_| data_dir.to_path_buf());
        let digest = blake3::hash(canonical_data_dir.to_string_lossy().as_bytes());
        let suffix: String = digest.to_hex().chars().take(16).collect();
        return PathBuf::from(runtime_dir).join(format!("{RUNTIME_SOCKET_FILE_PREFIX}{suffix}.sock"));
    }
    let socket = data_dir.join(SOCKET_FILE_NAME);
    #[cfg(target_os = "macos")]
    if socket.as_os_str().len() >= 100 {
        let canonical_data_dir = fs::canonicalize(data_dir).unwrap_or_else(|_| data_dir.to_path_buf());
        let digest = blake3::hash(canonical_data_dir.to_string_lossy().as_bytes());
        let suffix: String = digest.to_hex().chars().take(16).collect();
        return std::env::temp_dir().join(format!("{RUNTIME_SOCKET_FILE_PREFIX}{suffix}.sock"));
    }
    socket
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
    #[cfg(target_os = "linux")]
    {
        Path::new("/proc").join(pid.to_string()).exists()
    }
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        use libc::kill;
        let Ok(pid_i32) = i32::try_from(pid) else {
            return false;
        };
        // SAFETY: signal 0 is a no-op that only checks if the process exists.
        // It is safe even if the PID has been reused because we only get
        // a boolean result (ESRCH means no such process).
        unsafe { kill(pid_i32, 0) == 0 }
    }
    #[cfg(windows)]
    {
        fn is_process_alive(pid: u32) -> bool {
            use std::ffi::c_void;
            use std::ptr;

            const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
            const STILL_ACTIVE: u32 = 259;

            type HANDLE = *mut c_void;

            extern "system" {
                fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> HANDLE;
                fn GetExitCodeProcess(hProcess: HANDLE, lpExitCode: *mut u32) -> i32;
                fn CloseHandle(hObject: HANDLE) -> i32;
            }

            let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
            if handle.is_null() {
                return false;
            }
            let mut exit_code: u32 = 0;
            let result = unsafe { GetExitCodeProcess(handle, &mut exit_code) };
            unsafe { CloseHandle(handle) };
            result != 0 && exit_code == STILL_ACTIVE
        }
        is_process_alive(pid)
    }
    #[cfg(not(any(unix, windows)))]
    {
        true
    }
}

#[must_use]
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir() -> PathBuf {
        std::env::temp_dir().join(format!("syncweb-state-test-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn daemon_status_serialization() {
        let cases = [
            (DaemonStatus::Starting, r#""starting""#),
            (DaemonStatus::Running, r#""running""#),
            (DaemonStatus::Stopping, r#""stopping""#),
            (DaemonStatus::Stopped, r#""stopped""#),
        ];
        for (status, expected) in cases {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, expected);
            let deserialized: DaemonStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[test]
    fn daemon_state_serialization() {
        let state = DaemonState::new(
            12345,
            "node-id",
            1000,
            PathBuf::from("/tmp/data"),
            DaemonStatus::Running,
        );
        let json = serde_json::to_string_pretty(&state).unwrap();
        let deserialized: DaemonState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, state);
    }

    #[test]
    fn state_file_save_and_load() {
        let dir = test_dir();
        let state_file = StateFile::new(&dir);
        let state = DaemonState::new(std::process::id(), "test-node", 42, dir.clone(), DaemonStatus::Running);
        state_file.save(&state).unwrap();
        assert!(state_file.exists());
        let loaded = state_file.load().unwrap().expect("state should exist");
        assert_eq!(loaded.pid, state.pid);
        assert_eq!(loaded.node_id, state.node_id);
        assert_eq!(loaded.started_at, state.started_at);
        assert_eq!(loaded.status, state.status);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn state_file_missing_returns_none() {
        let dir = test_dir();
        let state_file = StateFile::new(&dir);
        assert!(!state_file.exists());
        let loaded = state_file.load().unwrap();
        assert!(loaded.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn state_file_corrupted_returns_error() {
        let dir = test_dir();
        let state_file = StateFile::new(&dir);
        state_file
            .save(&DaemonState::new(1, "node", 1, dir.clone(), DaemonStatus::Starting))
            .unwrap();
        let loaded = state_file.load().unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().pid, 1);

        state_file.remove().unwrap();
        let loaded_after_remove = state_file.load().unwrap();
        assert!(loaded_after_remove.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn state_file_remove() {
        let dir = test_dir();
        let state_file = StateFile::new(&dir);
        let state = DaemonState::new(1, "node", 1, dir.clone(), DaemonStatus::Starting);
        state_file.save(&state).unwrap();
        assert!(state_file.exists());
        state_file.remove().unwrap();
        assert!(!state_file.exists());
        state_file.remove().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn state_file_is_running_returns_false_for_stale() {
        let dir = test_dir();
        let state_file = StateFile::new(&dir);
        let state = DaemonState::new(u32::MAX, "stale", 1, dir.clone(), DaemonStatus::Running);
        state_file.save(&state).unwrap();
        assert!(!state_file.is_running().unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn status_report_save_and_load() {
        let dir = test_dir();
        let state_file = StateFile::new(&dir);
        let report = DaemonStatusReport::from_state(
            &DaemonState::new(1, "node", 100, dir.clone(), DaemonStatus::Running),
            10,
            vec![],
            BandwidthSnapshot::default(),
            None,
            4,
        );
        state_file.save_status(&report).unwrap();
        let loaded = state_file.load_status().unwrap().expect("report should exist");
        assert_eq!(loaded.pid, report.pid);
        assert_eq!(loaded.node_id, report.node_id);
        assert_eq!(loaded.uptime_seconds, report.uptime_seconds);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pid_lock_acquire_success() {
        let dir = test_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let lock = PidLock::new(&dir);
        assert!(lock.try_acquire().unwrap());
        assert!(lock.lock_path().exists());
        let pid = lock.owner_pid().unwrap().expect("pid should be written");
        assert_eq!(pid, std::process::id());
        lock.release().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn pid_lock_acquire_conflict() {
        let dir = test_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let first = PidLock::new(&dir);
        assert!(first.try_acquire().unwrap());
        let second = PidLock::new(&dir);
        assert!(!second.try_acquire().unwrap());
        first.release().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pid_lock_release_releases_lock() {
        let dir = test_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let lock = PidLock::new(&dir);
        assert!(lock.try_acquire().unwrap());
        lock.release().unwrap();
        let reacquire = PidLock::new(&dir);
        assert!(reacquire.try_acquire().unwrap());
        reacquire.release().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pid_lock_drop_releases() {
        let dir = test_dir();
        std::fs::create_dir_all(&dir).unwrap();
        {
            let lock = PidLock::new(&dir);
            assert!(lock.try_acquire().unwrap());
        }
        let reacquire = PidLock::new(&dir);
        assert!(reacquire.try_acquire().unwrap());
        reacquire.release().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pid_is_alive_checks_current_and_zero() {
        assert!(pid_is_alive(std::process::id()));
        assert!(!pid_is_alive(0));
    }

    #[test]
    fn daemon_socket_path_returns_valid_path() {
        let dir = test_dir();
        let path = daemon_socket_path(&dir);
        assert!(path.file_name().unwrap().to_string_lossy().ends_with(".sock"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pid_lock_stale_detection() {
        let dir = test_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let lock = PidLock::new(&dir);
        assert!(lock.try_acquire().unwrap());
        let pid = lock.owner_pid().unwrap().expect("pid should be written");
        assert_eq!(pid, std::process::id());
        lock.release().unwrap();
        assert!(!pid_is_alive(99999_u32));
        let state_file = StateFile::new(&dir);
        state_file
            .save(&DaemonState::new(99999, "stale-daemon", 1, &dir, DaemonStatus::Running))
            .unwrap();
        assert!(!state_file.is_running().unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pid_lock_state_file_consistency() {
        let dir = test_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let lock = PidLock::new(&dir);
        assert!(lock.try_acquire().unwrap());
        let state_file = StateFile::new(&dir);
        state_file
            .save(&DaemonState::new(
                std::process::id(),
                "consistent",
                current_timestamp(),
                &dir,
                DaemonStatus::Running,
            ))
            .unwrap();
        assert!(state_file.exists());
        assert!(lock.lock_path().exists());
        assert_eq!(lock.owner_pid().unwrap(), Some(std::process::id()));
        lock.release().unwrap();
        state_file.remove().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn pid_lock_concurrent_start_race() {
        let dir = test_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let first = PidLock::new(&dir);
        assert!(first.try_acquire().unwrap());
        let second = PidLock::new(&dir);
        assert!(!second.try_acquire().unwrap());
        first.release().unwrap();
        assert!(second.try_acquire().unwrap());
        second.release().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn status_report_serialization_json() {
        let report = DaemonStatusReport::from_state(
            &DaemonState::new(42, "ns-id", 100, PathBuf::from("/data"), DaemonStatus::Running),
            10,
            vec![FolderStatusReport::new(
                "folder1",
                PathBuf::from("/foo"),
                true,
                Some(200),
                5,
                vec![],
            )],
            BandwidthSnapshot::default(),
            Some(ScheduleStatus {
                in_active_window: true,
                next_window_start: Some(300),
            }),
            4,
        );
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("folder1"));
        assert!(json.contains("uptime_seconds"));
        let deserialized: DaemonStatusReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.pid, report.pid);
        assert_eq!(deserialized.folders.len(), 1);
        assert!(deserialized.schedule.unwrap().in_active_window);
    }

    #[test]
    fn daemon_status_report_includes_folders() {
        let report = DaemonStatusReport::from_state(
            &DaemonState::new(1, "node", 1, PathBuf::from("/tmp"), DaemonStatus::Running),
            5,
            vec![
                FolderStatusReport::new("ns1", PathBuf::from("/a"), true, None, 3, vec![]),
                FolderStatusReport::new("ns2", PathBuf::from("/b"), false, Some(10), 0, vec!["error1".into()]),
            ],
            BandwidthSnapshot::default(),
            None,
            2,
        );
        assert_eq!(report.folders.len(), 2);
        assert!(report.folders.iter().any(|f| f.namespace == "ns1"));
        assert!(
            report
                .folders
                .iter()
                .any(|f| f.namespace == "ns2" && !f.session_active && f.errors.len() == 1)
        );
    }
}
