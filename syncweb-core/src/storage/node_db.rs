use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Mutex},
};

use iroh::PublicKey;
use iroh_blobs::Hash;
use iroh_docs::NamespaceId;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tracing;
use uuid::Uuid;

use crate::{
    Result, SyncwebError,
    daemon::{
        BandwidthSnapshot, DaemonState, DaemonStatus, DaemonStatusReport, FolderStatusReport, ScheduleStatus,
        current_timestamp,
    },
    filter::{FilterAction, FilterEngine, FilterEntry, FilterRule},
    folder::{CollectionState, InstalledCollection},
    net::network::{Network, NetworkId, network_topic, parse_public_key},
    storage::config::Config as AppConfig,
};

/// Parameters for upserting a sync entry progress record.
#[derive(Debug)]
#[non_exhaustive]
pub struct SyncEntryParams<'a> {
    pub namespace_id: &'a str,
    pub session_id: &'a str,
    pub entry_key: &'a [u8],
    pub hash: &'a [u8],
    pub size: u64,
    pub status: &'a str,
    pub retries: u32,
    pub error_message: Option<&'a str>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct StorageRootRecord {
    pub id: String,
    pub path: PathBuf,
    pub min_free: u64,
    pub enabled: bool,
}

impl StorageRootRecord {
    /// Create a storage root record.
    #[must_use]
    pub fn new(id: impl Into<String>, path: impl Into<PathBuf>, min_free: u64, enabled: bool) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
            min_free,
            enabled,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct TransferJobRecord {
    pub id: String,
    pub namespace_id: String,
    pub entry_key: Vec<u8>,
    pub hash: [u8; 32],
    pub size: u64,
    pub root_id: Option<String>,
    pub destination: Option<PathBuf>,
    pub state: String,
    pub bytes_transferred: u64,
    pub peer_count: u64,
    pub eta_seconds: Option<u64>,
    pub retries: u32,
    pub error_message: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct NewTransferJob<'a> {
    pub namespace_id: &'a str,
    pub entry_key: &'a [u8],
    pub hash: &'a [u8; 32],
    pub size: u64,
    pub root_id: Option<&'a str>,
    pub destination: Option<&'a Path>,
}

impl<'a> NewTransferJob<'a> {
    /// Create a transfer job insertion record.
    #[must_use]
    pub const fn new(
        namespace_id: &'a str,
        entry_key: &'a [u8],
        hash: &'a [u8; 32],
        size: u64,
        root_id: Option<&'a str>,
        destination: Option<&'a Path>,
    ) -> Self {
        Self {
            namespace_id,
            entry_key,
            hash,
            size,
            root_id,
            destination,
        }
    }
}

fn transfer_job_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TransferJobRecord> {
    let hash_bytes: Vec<u8> = row.get(3)?;
    let hash = hash_bytes.try_into().map_err(|bytes: Vec<u8>| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Blob,
            format!("expected 32-byte hash, got {} bytes", bytes.len()).into(),
        )
    })?;
    let size: i64 = row.get(4)?;
    let bytes_transferred: i64 = row.get(8)?;
    let peer_count: i64 = row.get(9)?;
    let eta_seconds: Option<i64> = row.get(10)?;
    let retries: i64 = row.get(11)?;
    let created_at: i64 = row.get(13)?;
    let updated_at: i64 = row.get(14)?;
    let retry_count = u32::try_from(retries).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(11, rusqlite::types::Type::Integer, Box::new(error))
    })?;
    Ok(TransferJobRecord {
        id: row.get(0)?,
        namespace_id: row.get(1)?,
        entry_key: row.get(2)?,
        hash,
        size: size.cast_unsigned(),
        root_id: row.get(5)?,
        destination: row.get::<_, Option<String>>(6)?.map(PathBuf::from),
        state: row.get(7)?,
        bytes_transferred: bytes_transferred.cast_unsigned(),
        peer_count: peer_count.cast_unsigned(),
        eta_seconds: eta_seconds.map(i64::cast_unsigned),
        retries: retry_count,
        error_message: row.get(12)?,
        created_at: created_at.cast_unsigned(),
        updated_at: updated_at.cast_unsigned(),
    })
}

#[derive(Clone, Debug)]
pub struct NodeDatabase {
    connection: Arc<Mutex<Connection>>,
}

impl NodeDatabase {
    /// Open (or create) the node database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db_path = path.as_ref();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| SyncwebError::operation("failed to create node db directory", error))?;
        }
        let connection = Connection::open(db_path)
            .map_err(|error| SyncwebError::operation("failed to open node database", error))?;
        let db = Self {
            connection: Arc::new(Mutex::new(connection)),
        };
        db.init_schema()?;
        Ok(db)
    }

    const CREATE_SCHEMA_SQL: &'static str =
        "CREATE TABLE IF NOT EXISTS schema_version (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS daemon_lifecycle (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            pid INTEGER NOT NULL,
            node_id TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('starting','running','stopping','stopped')),
            data_dir TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS daemon_status (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            pid INTEGER NOT NULL,
            node_id TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            uptime_seconds INTEGER NOT NULL,
            upload_total INTEGER NOT NULL DEFAULT 0,
            download_total INTEGER NOT NULL DEFAULT 0,
            upload_rate INTEGER NOT NULL DEFAULT 0,
            download_rate INTEGER NOT NULL DEFAULT 0,
            has_schedule INTEGER NOT NULL DEFAULT 0,
            in_active_window INTEGER NOT NULL DEFAULT 0,
            next_window_start INTEGER,
            rayon_threads INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS folder_status_reports (
            namespace_id TEXT NOT NULL,
            path TEXT NOT NULL,
            session_active INTEGER NOT NULL DEFAULT 0,
            last_sync_at INTEGER,
            entries_synced INTEGER NOT NULL DEFAULT 0,
            errors_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL,
            PRIMARY KEY(namespace_id)
        );
        CREATE TABLE IF NOT EXISTS networks (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            label TEXT NOT NULL DEFAULT '',
            owner TEXT NOT NULL,
            shared_secret BLOB,
            created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS network_members (
            network_id TEXT NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
            member TEXT NOT NULL,
            PRIMARY KEY(network_id, member)
        );
        CREATE TABLE IF NOT EXISTS network_folders (
            network_id TEXT NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
            namespace_id TEXT NOT NULL,
            PRIMARY KEY(network_id, namespace_id)
        );
        CREATE TABLE IF NOT EXISTS installed_collections (
            collection_id TEXT PRIMARY KEY,
            manifest_hash TEXT NOT NULL,
            current_version TEXT NOT NULL,
            installed_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS collection_versions (
            collection_id TEXT NOT NULL REFERENCES installed_collections(collection_id) ON DELETE CASCADE,
            version TEXT NOT NULL,
            install_path TEXT NOT NULL,
            PRIMARY KEY(collection_id, version)
        );
        CREATE TABLE IF NOT EXISTS app_config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS filter_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            namespace_id TEXT,
            rule_type TEXT NOT NULL,
            pattern TEXT NOT NULL,
            priority INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS public_subscriptions (
            hash TEXT PRIMARY KEY,
            size INTEGER NOT NULL,
            subscribed_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS shares (
            namespace_id TEXT NOT NULL,
            access TEXT NOT NULL CHECK(access IN ('read','write')),
            ticket TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            PRIMARY KEY(namespace_id, access)
        );
        CREATE TABLE IF NOT EXISTS workspace_manifests (
            source_path TEXT NOT NULL,
            manifest_bytes BLOB NOT NULL,
            collection_id TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY(source_path)
        );
        CREATE TABLE IF NOT EXISTS blob_folders (
            content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
            namespace_id TEXT NOT NULL,
            entry_key BLOB NOT NULL,
            added_at INTEGER NOT NULL,
            PRIMARY KEY(content_hash, namespace_id, entry_key)
        );
        CREATE INDEX IF NOT EXISTS idx_blob_folders_hash ON blob_folders(content_hash);
        CREATE TABLE IF NOT EXISTS sync_checkpoints (
            namespace_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            total_entries INTEGER NOT NULL DEFAULT 0,
            processed_entries INTEGER NOT NULL DEFAULT 0,
            failed_entries INTEGER NOT NULL DEFAULT 0,
            bytes_total INTEGER,
            bytes_transferred INTEGER NOT NULL DEFAULT 0,
            started_at INTEGER NOT NULL,
            last_updated_at INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','running','completed','failed','cancelled')),
            PRIMARY KEY(namespace_id, session_id)
        );
        CREATE TABLE IF NOT EXISTS sync_entry_progress (
            namespace_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            entry_key BLOB NOT NULL,
            hash BLOB NOT NULL,
            size INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','downloading','completed','failed','skipped')),
            retries INTEGER NOT NULL DEFAULT 0,
            error_message TEXT,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY(namespace_id, session_id, entry_key),
            FOREIGN KEY(namespace_id, session_id) REFERENCES sync_checkpoints(namespace_id, session_id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS storage_roots (
            id TEXT PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            min_free INTEGER NOT NULL DEFAULT 0 CHECK(min_free >= 0),
            enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS transfer_jobs (
            id TEXT PRIMARY KEY,
            namespace_id TEXT NOT NULL,
            entry_key BLOB NOT NULL,
            hash BLOB NOT NULL CHECK(length(hash) = 32),
            size INTEGER NOT NULL CHECK(size >= 0),
            root_id TEXT,
            destination TEXT,
            state TEXT NOT NULL DEFAULT 'queued'
                CHECK(state IN ('queued','fetching','materializing','paused','completed','failed','cancelled')),
            bytes_transferred INTEGER NOT NULL DEFAULT 0 CHECK(bytes_transferred >= 0),
            peer_count INTEGER NOT NULL DEFAULT 0 CHECK(peer_count >= 0),
            eta_seconds INTEGER,
            retries INTEGER NOT NULL DEFAULT 0 CHECK(retries >= 0),
            error_message TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE(namespace_id, entry_key, root_id),
            FOREIGN KEY(root_id) REFERENCES storage_roots(id) ON DELETE SET NULL
        );";

    /// Returns the SQL statements to create the database schema.
    const fn create_schema_sql() -> &'static str {
        Self::CREATE_SCHEMA_SQL
    }

    /// Initialize the database schema, creating tables and indexes if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the database mutex is poisoned or if a SQL operation fails.
    pub fn init_schema(&self) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|error| SyncwebError::operation("failed to configure node database", error))?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(|error| SyncwebError::operation("failed to set WAL mode", error))?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(|error| SyncwebError::operation("failed to enable foreign keys", error))?;
        connection
            .execute_batch(Self::create_schema_sql())
            .map_err(|error| SyncwebError::operation("failed to initialize node database schema", error))?;
        if let Err(error) = connection.execute_batch("ALTER TABLE networks ADD COLUMN doc_ticket TEXT") {
            tracing::debug!(%error, "migration: add doc_ticket (may already exist)");
        }
        if let Err(error) = connection.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_transfer_jobs_namespace ON transfer_jobs(namespace_id);
             CREATE INDEX IF NOT EXISTS idx_transfer_jobs_state ON transfer_jobs(state);
             CREATE INDEX IF NOT EXISTS idx_transfer_jobs_root ON transfer_jobs(root_id);",
        ) {
            tracing::debug!(%error, "migration: add transfer job indexes (may already exist)");
        }

        connection
            .execute(
                "INSERT INTO schema_version(key, value) VALUES ('version', '1')
             ON CONFLICT(key) DO NOTHING",
                [],
            )
            .map_err(|error| SyncwebError::operation("failed to seed schema version", error))?;
        drop(connection);
        Ok(())
    }

    /// Persist the daemon lifecycle state.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn save_lifecycle(&self, state: &DaemonState) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let status = match state.status {
            DaemonStatus::Starting => "starting",
            DaemonStatus::Running => "running",
            DaemonStatus::Stopping => "stopping",
            DaemonStatus::Stopped => "stopped",
        };
        let now = current_timestamp();
        connection
            .execute(
                "INSERT INTO daemon_lifecycle(id, pid, node_id, started_at, status, data_dir, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                pid = excluded.pid,
                node_id = excluded.node_id,
                started_at = excluded.started_at,
                status = excluded.status,
                data_dir = excluded.data_dir,
                updated_at = excluded.updated_at",
                params![
                    i64::from(state.pid),
                    state.node_id,
                    state.started_at.cast_signed(),
                    status,
                    state.data_dir.to_string_lossy().as_ref(),
                    now.cast_signed(),
                ],
            )
            .map_err(|error| SyncwebError::operation("failed to save daemon lifecycle", error))?;
        drop(connection);
        Ok(())
    }

    /// Load the daemon lifecycle state, returning `None` if absent.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read.
    pub fn load_lifecycle(&self) -> Result<Option<DaemonState>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let result = connection
            .query_row(
                "SELECT pid, node_id, started_at, status, data_dir FROM daemon_lifecycle WHERE id = 1",
                [],
                |row| {
                    let pid: i64 = row.get(0)?;
                    let node_id: String = row.get(1)?;
                    let started_at: i64 = row.get(2)?;
                    let status_str: String = row.get(3)?;
                    let data_dir: String = row.get(4)?;
                    let status = match status_str.as_str() {
                        "starting" => DaemonStatus::Starting,
                        "running" => DaemonStatus::Running,
                        "stopping" => DaemonStatus::Stopping,
                        "stopped" => DaemonStatus::Stopped,
                        _ => {
                            return Err(rusqlite::Error::InvalidColumnType(
                                3,
                                "status".to_string(),
                                rusqlite::types::Type::Text,
                            ));
                        }
                    };
                    Ok(DaemonState::new(
                        u32::try_from(pid).unwrap_or_default(),
                        node_id,
                        started_at.cast_unsigned(),
                        data_dir,
                        status,
                    ))
                },
            )
            .optional()
            .map_err(|error| SyncwebError::operation("failed to load daemon lifecycle", error))?;
        drop(connection);
        Ok(result)
    }

    /// Remove the daemon lifecycle state.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn remove_lifecycle(&self) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute("DELETE FROM daemon_lifecycle WHERE id = 1", [])
            .map_err(|error| SyncwebError::operation("failed to remove daemon lifecycle", error))?;
        drop(connection);
        Ok(())
    }

    /// Persist the daemon status report.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn save_status(&self, report: &DaemonStatusReport) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        let has_schedule = report.schedule.is_some();
        connection
            .execute(
                "INSERT INTO daemon_status(id, pid, node_id, started_at, uptime_seconds,
                upload_total, download_total, upload_rate, download_rate,
                has_schedule, in_active_window, next_window_start, rayon_threads, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET
                pid = excluded.pid,
                node_id = excluded.node_id,
                started_at = excluded.started_at,
                uptime_seconds = excluded.uptime_seconds,
                upload_total = excluded.upload_total,
                download_total = excluded.download_total,
                upload_rate = excluded.upload_rate,
                download_rate = excluded.download_rate,
                has_schedule = excluded.has_schedule,
                in_active_window = excluded.in_active_window,
                next_window_start = excluded.next_window_start,
                rayon_threads = excluded.rayon_threads,
                updated_at = excluded.updated_at",
                params![
                    i64::from(report.pid),
                    report.node_id,
                    report.started_at.cast_signed(),
                    report.uptime_seconds.cast_signed(),
                    report.bandwidth.upload_total.cast_signed(),
                    report.bandwidth.download_total.cast_signed(),
                    report.bandwidth.upload_rate.cast_signed(),
                    report.bandwidth.download_rate.cast_signed(),
                    i64::from(has_schedule),
                    report.schedule.map_or(0, |s| i64::from(s.in_active_window)),
                    report.schedule.and_then(|s| s.next_window_start.map(u64::cast_signed)),
                    i64::try_from(report.rayon_threads).unwrap_or_default(),
                    now,
                ],
            )
            .map_err(|error| SyncwebError::operation("failed to save daemon status", error))?;

        for folder in &report.folders {
            let errors_json = serde_json::to_string(&folder.errors).unwrap_or_else(|_| "[]".to_string());
            connection
                .execute(
                    "INSERT INTO folder_status_reports(namespace_id, path, session_active, last_sync_at,
                    entries_synced, errors_json, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(namespace_id) DO UPDATE SET
                    path = excluded.path,
                    session_active = excluded.session_active,
                    last_sync_at = excluded.last_sync_at,
                    entries_synced = excluded.entries_synced,
                    errors_json = excluded.errors_json,
                    updated_at = excluded.updated_at",
                    params![
                        folder.namespace,
                        folder.path.to_string_lossy().as_ref(),
                        i64::from(folder.session_active),
                        folder.last_sync_at.map(u64::cast_signed),
                        folder.entries_synced.cast_signed(),
                        errors_json,
                        now,
                    ],
                )
                .map_err(|error| SyncwebError::operation("failed to save folder status", error))?;
        }
        drop(connection);
        Ok(())
    }

    /// Load the daemon status report, returning `None` if absent.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read.
    pub fn load_status(&self) -> Result<Option<DaemonStatusReport>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let result: Option<DaemonStatusReport> = connection
            .query_row(
                "SELECT pid, node_id, started_at, uptime_seconds,
                upload_total, download_total, upload_rate, download_rate,
                has_schedule, in_active_window, next_window_start, rayon_threads
             FROM daemon_status WHERE id = 1",
                [],
                |row| {
                    let pid: i64 = row.get(0)?;
                    let node_id: String = row.get(1)?;
                    let started_at: i64 = row.get(2)?;
                    let uptime_seconds: i64 = row.get(3)?;
                    let upload_total: i64 = row.get(4)?;
                    let download_total: i64 = row.get(5)?;
                    let upload_rate: i64 = row.get(6)?;
                    let download_rate: i64 = row.get(7)?;
                    let has_schedule: i64 = row.get(8)?;
                    let in_active_window: i64 = row.get(9)?;
                    let next_window_start: Option<i64> = row.get(10)?;
                    let rayon_threads: i64 = row.get(11)?;

                    let schedule = (has_schedule != 0).then(|| ScheduleStatus {
                        in_active_window: in_active_window != 0,
                        next_window_start: next_window_start.map(i64::cast_unsigned),
                    });

                    Ok(DaemonStatusReport {
                        pid: u32::try_from(pid).unwrap_or_default(),
                        node_id,
                        started_at: started_at.cast_unsigned(),
                        uptime_seconds: uptime_seconds.cast_unsigned(),
                        folders: Vec::new(),
                        bandwidth: BandwidthSnapshot {
                            upload_total: upload_total.cast_unsigned(),
                            download_total: download_total.cast_unsigned(),
                            upload_rate: upload_rate.cast_unsigned(),
                            download_rate: download_rate.cast_unsigned(),
                        },
                        schedule,
                        rayon_threads: usize::try_from(rayon_threads).unwrap_or_default(),
                    })
                },
            )
            .optional()
            .map_err(|error| SyncwebError::operation("failed to load daemon status", error))?;

        if let Some(mut report) = result {
            let mut folder_stmt = connection
                .prepare(
                    "SELECT namespace_id, path, session_active, last_sync_at, entries_synced, errors_json
                 FROM folder_status_reports",
                )
                .map_err(|error| SyncwebError::operation("failed to prepare folder status query", error))?;
            let folders: Vec<FolderStatusReport> = folder_stmt
                .query_map([], |row| {
                    let namespace: String = row.get(0)?;
                    let path: String = row.get(1)?;
                    let session_active: i64 = row.get(2)?;
                    let last_sync_at: Option<i64> = row.get(3)?;
                    let entries_synced: i64 = row.get(4)?;
                    let errors_json: String = row.get(5)?;
                    let errors: Vec<String> = serde_json::from_str(&errors_json).unwrap_or_default();
                    Ok(FolderStatusReport::new(
                        namespace,
                        path,
                        session_active != 0,
                        last_sync_at.map(i64::cast_unsigned),
                        entries_synced.cast_unsigned(),
                        errors,
                    ))
                })
                .map_err(|error| SyncwebError::operation("failed to query folder statuses", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| SyncwebError::operation("failed to read folder status rows", error))?;
            report.folders = folders;
            drop(folder_stmt);
            drop(connection);
            Ok(Some(report))
        } else {
            drop(connection);
            Ok(None)
        }
    }

    /// Remove all daemon status data.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn remove_status(&self) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute("DELETE FROM daemon_status WHERE id = 1", [])
            .map_err(|error| SyncwebError::operation("failed to remove daemon status", error))?;
        connection
            .execute("DELETE FROM folder_status_reports", [])
            .map_err(|error| SyncwebError::operation("failed to remove folder status reports", error))?;
        drop(connection);
        Ok(())
    }

    /// Create a network entry in the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn create_network(&self, network: &Network) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        connection.execute(
            "INSERT INTO networks(id, name, label, owner, shared_secret, created_at, doc_ticket) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                network.id.to_string(),
                network.name,
                network.label,
                network.owner.to_string(),
                network.shared_secret.as_ref().map(|s| s.to_vec()),
                now,
                network.doc_ticket,
            ],
        ).map_err(|error| SyncwebError::operation("failed to create network", error))?;
        for member in &network.members {
            connection
                .execute(
                    "INSERT INTO network_members(network_id, member) VALUES (?1, ?2)",
                    params![network.id.to_string(), member.to_string()],
                )
                .map_err(|error| SyncwebError::operation("failed to add network member", error))?;
        }
        for folder in &network.folders {
            connection
                .execute(
                    "INSERT INTO network_folders(network_id, namespace_id) VALUES (?1, ?2)",
                    params![network.id.to_string(), folder.to_string()],
                )
                .map_err(|error| SyncwebError::operation("failed to add network folder", error))?;
        }
        drop(connection);
        Ok(())
    }

    /// Delete a network from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the network is not found or the database cannot be written.
    pub fn delete_network(&self, id: NetworkId) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let affected = connection
            .execute("DELETE FROM networks WHERE id = ?1", params![id.to_string()])
            .map_err(|error| SyncwebError::operation("failed to delete network", error))?;
        if affected == 0 {
            drop(connection);
            return Err(SyncwebError::FolderNotFound(format!("network {id}")));
        }
        drop(connection);
        Ok(())
    }

    /// Add a member to a network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn add_member(&self, network_id: NetworkId, member: PublicKey) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute(
                "INSERT OR IGNORE INTO network_members(network_id, member) VALUES (?1, ?2)",
                params![network_id.to_string(), member.to_string()],
            )
            .map_err(|error| SyncwebError::operation("failed to add network member", error))?;
        drop(connection);
        Ok(())
    }

    /// Remove a member from a network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn remove_member(&self, network_id: NetworkId, member: &PublicKey) -> Result<bool> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let affected = connection
            .execute(
                "DELETE FROM network_members WHERE network_id = ?1 AND member = ?2",
                params![network_id.to_string(), member.to_string()],
            )
            .map_err(|error| SyncwebError::operation("failed to remove network member", error))?;
        drop(connection);
        Ok(affected > 0)
    }

    /// Add a folder to a network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn add_folder_to_network(&self, network_id: NetworkId, namespace_id: NamespaceId) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute(
                "INSERT OR IGNORE INTO network_folders(network_id, namespace_id) VALUES (?1, ?2)",
                params![network_id.to_string(), namespace_id.to_string()],
            )
            .map_err(|error| SyncwebError::operation("failed to add network folder", error))?;
        drop(connection);
        Ok(())
    }

    /// Remove a folder from a network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn remove_folder_from_network(&self, network_id: NetworkId, namespace_id: NamespaceId) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute(
                "DELETE FROM network_folders WHERE network_id = ?1 AND namespace_id = ?2",
                params![network_id.to_string(), namespace_id.to_string()],
            )
            .map_err(|error| SyncwebError::operation("failed to remove network folder", error))?;
        drop(connection);
        Ok(())
    }

    /// List all networks from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read.
    pub fn list_networks(&self) -> Result<Vec<Network>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let mut stmt = connection
            .prepare("SELECT id, name, label, owner, shared_secret, doc_ticket FROM networks ORDER BY name")
            .map_err(|error| SyncwebError::operation("failed to prepare network query", error))?;
        let networks = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<Vec<u8>>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(|error| SyncwebError::operation("failed to query networks", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read network rows", error))?;

        let mut member_stmt = connection
            .prepare("SELECT member FROM network_members WHERE network_id = ?1")
            .map_err(|error| SyncwebError::operation("failed to prepare member query", error))?;
        let mut folder_stmt = connection
            .prepare("SELECT namespace_id FROM network_folders WHERE network_id = ?1")
            .map_err(|error| SyncwebError::operation("failed to prepare folder query", error))?;

        let mut result = Vec::new();
        for (id_str, name, label, owner_str, shared_secret_bytes, doc_ticket) in networks {
            let network_id = NetworkId::from_str(&id_str)
                .map_err(|error| SyncwebError::operation("invalid network ID in database", error))?;
            let owner = parse_public_key(&owner_str)?;
            let shared_secret = shared_secret_bytes
                .map(|bytes| -> Result<[u8; 32]> {
                    bytes.try_into().map_err(|_bytes: Vec<u8>| {
                        SyncwebError::operation("invalid shared secret length in database", "expected 32 bytes")
                    })
                })
                .transpose()?;

            let members: HashSet<PublicKey> = member_stmt
                .query_map(params![id_str], |row| row.get::<_, String>(0))
                .map_err(|error| SyncwebError::operation("failed to query members", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| SyncwebError::operation("failed to read member rows", error))?
                .into_iter()
                .map(|s| parse_public_key(&s))
                .collect::<Result<HashSet<_>>>()?;

            let folders: HashSet<NamespaceId> = folder_stmt
                .query_map(params![id_str], |row| row.get::<_, String>(0))
                .map_err(|error| SyncwebError::operation("failed to query folders", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| SyncwebError::operation("failed to read folder rows", error))?
                .into_iter()
                .map(|s| s.parse())
                .collect::<std::result::Result<HashSet<_>, _>>()
                .map_err(|error| SyncwebError::operation("invalid folder namespace in database", error))?;

            result.push(Network {
                id: network_id,
                name,
                label,
                topic: network_topic(network_id),
                owner,
                members,
                folders,
                shared_secret,
                doc_ticket,
            });
        }
        drop(stmt);
        drop(member_stmt);
        drop(folder_stmt);
        drop(connection);
        Ok(result)
    }

    /// Get the installed collection state.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read.
    pub fn get_collection_state(&self) -> Result<CollectionState> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let mut stmt = connection
            .prepare("SELECT collection_id, manifest_hash, current_version FROM installed_collections")
            .map_err(|error| SyncwebError::operation("failed to prepare collection query", error))?;
        let collections: Vec<(String, String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| SyncwebError::operation("failed to query collections", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read collection rows", error))?;

        let mut version_stmt = connection
            .prepare("SELECT version, install_path FROM collection_versions WHERE collection_id = ?1")
            .map_err(|error| SyncwebError::operation("failed to prepare version query", error))?;

        let mut installed = BTreeMap::new();
        for (id_str, manifest_hash, current_version) in collections {
            let collection_id = Uuid::from_str(&id_str)
                .map_err(|error| SyncwebError::operation("invalid collection ID in database", error))?;
            let manifest = iroh_blobs::Hash::from_str(&manifest_hash)
                .map_err(|error| SyncwebError::operation("invalid manifest hash in database", error))?;

            let versions: BTreeMap<String, PathBuf> = version_stmt
                .query_map(params![id_str], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|error| SyncwebError::operation("failed to query versions", error))?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| SyncwebError::operation("failed to read version rows", error))?
                .into_iter()
                .map(|(v, p)| (v, PathBuf::from(p)))
                .collect();

            installed.insert(
                collection_id,
                InstalledCollection {
                    manifest,
                    versions,
                    current: current_version,
                },
            );
        }
        drop(stmt);
        drop(version_stmt);
        drop(connection);
        Ok(CollectionState { installed })
    }

    /// Install a collection version in the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn install_collection(
        &self,
        collection_id: Uuid,
        manifest_hash: &iroh_blobs::Hash,
        version: &str,
        path: &Path,
    ) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        connection
            .execute(
                "INSERT INTO installed_collections(collection_id, manifest_hash, current_version, installed_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(collection_id) DO UPDATE SET
                manifest_hash = excluded.manifest_hash,
                current_version = excluded.current_version,
                installed_at = excluded.installed_at",
                params![collection_id.to_string(), manifest_hash.to_string(), version, now],
            )
            .map_err(|error| SyncwebError::operation("failed to install collection", error))?;
        connection
            .execute(
                "INSERT OR IGNORE INTO collection_versions(collection_id, version, install_path) VALUES (?1, ?2, ?3)",
                params![collection_id.to_string(), version, path.to_string_lossy().as_ref()],
            )
            .map_err(|error| SyncwebError::operation("failed to record collection version", error))?;
        drop(connection);
        Ok(())
    }

    /// Switch the active version of a collection.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn switch_collection_version(&self, collection_id: Uuid, version: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute(
                "UPDATE installed_collections SET current_version = ?1 WHERE collection_id = ?2",
                params![version, collection_id.to_string()],
            )
            .map_err(|error| SyncwebError::operation("failed to switch collection version", error))?;
        drop(connection);
        Ok(())
    }

    /// Remove a collection version from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn remove_collection_version(&self, collection_id: Uuid, version: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute(
                "DELETE FROM collection_versions WHERE collection_id = ?1 AND version = ?2",
                params![collection_id.to_string(), version],
            )
            .map_err(|error| SyncwebError::operation("failed to remove collection version", error))?;

        let remaining: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM collection_versions WHERE collection_id = ?1",
                params![collection_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|error| SyncwebError::operation("failed to check remaining versions", error))?;
        if remaining == 0 {
            connection
                .execute(
                    "DELETE FROM installed_collections WHERE collection_id = ?1",
                    params![collection_id.to_string()],
                )
                .map_err(|error| SyncwebError::operation("failed to remove installed collection", error))?;
        }
        drop(connection);
        Ok(())
    }

    /// Get a configuration value.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read.
    pub fn get_config(&self, key: &str) -> Result<Option<String>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let result = connection
            .query_row("SELECT value FROM app_config WHERE key = ?1", params![key], |row| {
                row.get::<_, String>(0)
            })
            .optional()
            .map_err(|error| SyncwebError::operation("failed to get config", error))?;
        drop(connection);
        Ok(result)
    }

    /// Set a configuration value.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn set_config(&self, key: &str, value: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        connection
            .execute(
                "INSERT INTO app_config(key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                params![key, value, now],
            )
            .map_err(|error| SyncwebError::operation("failed to set config", error))?;
        drop(connection);
        Ok(())
    }

    /// Load the application configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read or the config cannot be parsed.
    pub fn load_app_config(&self) -> Result<AppConfig> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let blob: Option<String> = connection
            .query_row("SELECT value FROM app_config WHERE key = 'config_blob'", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|error| SyncwebError::operation("failed to load app config", error))?;
        drop(connection);
        blob.map_or_else(
            || Ok(AppConfig::default()),
            |toml_str| {
                toml::from_str(&toml_str).map_err(|error| SyncwebError::operation("failed to parse app config", error))
            },
        )
    }

    /// Save the application configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the config cannot be serialized or the database cannot be written.
    pub fn save_app_config(&self, config: &AppConfig) -> Result<()> {
        let now = current_timestamp().cast_signed();
        let blob = toml::to_string_pretty(config)
            .map_err(|error| SyncwebError::operation("failed to serialize app config", error))?;
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute(
                "INSERT INTO app_config(key, value, updated_at) VALUES ('config_blob', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                params![blob, now],
            )
            .map_err(|error| SyncwebError::operation("failed to save app config", error))?;
        drop(connection);
        Ok(())
    }

    /// Load the filter engine state.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read or the filter engine cannot be created.
    pub fn load_filter_engine(&self) -> Result<Option<FilterEngine>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let mut stmt = connection
            .prepare("SELECT namespace_id, rule_type, pattern FROM filter_rules ORDER BY priority, id")
            .map_err(|error| SyncwebError::operation("failed to prepare filter query", error))?;
        let rows: Vec<(Option<String>, String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| SyncwebError::operation("failed to query filters", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read filter rows", error))?;
        drop(stmt);
        drop(connection);

        if rows.is_empty() {
            return Ok(None);
        }

        let mut global_rules = Vec::new();
        let mut folder_rules: HashMap<String, Vec<FilterRule>> = HashMap::new();
        for (namespace_id, rule_type, pattern) in rows {
            let action = match rule_type.as_str() {
                "include" => crate::filter::FilterAction::Accept,
                "exclude" => crate::filter::FilterAction::Reject,
                _ => continue,
            };
            let criteria = crate::filter::MatchCriteria {
                name: Some(pattern),
                ..Default::default()
            };
            let rule = FilterRule::new(action, criteria);
            match namespace_id {
                Some(ns) => folder_rules.entry(ns).or_default().push(rule),
                None => global_rules.push(rule),
            }
        }

        let filter_config = crate::filter::FilterConfig {
            rules: global_rules,
            folders: folder_rules,
            default_action: FilterAction::default(),
        };
        FilterEngine::new(filter_config).map(Some)
    }

    /// Save filter rules to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn save_filter_rules(&self, rules: &[FilterEntry]) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        connection
            .execute("DELETE FROM filter_rules", [])
            .map_err(|error| SyncwebError::operation("failed to clear filter rules", error))?;
        for entry in rules {
            let rule_type = if entry.size > 0 { "exclude" } else { "include" };
            connection
                .execute(
                    "INSERT INTO filter_rules(namespace_id, rule_type, pattern, priority, updated_at)
                 VALUES (NULL, ?1, ?2, 0, ?3)",
                    params![rule_type, entry.path.to_string_lossy().as_ref(), now],
                )
                .map_err(|error| SyncwebError::operation("failed to save filter rule", error))?;
        }
        drop(connection);
        Ok(())
    }

    /// Load all public subscription hashes and their sizes.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read.
    pub fn load_subscriptions(&self) -> Result<HashSet<iroh_blobs::Hash>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let mut stmt = connection
            .prepare("SELECT hash FROM public_subscriptions")
            .map_err(|error| SyncwebError::operation("failed to prepare subscription query", error))?;
        let hashes: HashSet<iroh_blobs::Hash> = stmt
            .query_map([], |row| {
                let text: String = row.get(0)?;
                Ok(text)
            })
            .map_err(|error| SyncwebError::operation("failed to query subscriptions", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read subscription rows", error))?
            .into_iter()
            .filter_map(|h| h.parse::<iroh_blobs::Hash>().ok())
            .collect();
        drop(stmt);
        drop(connection);
        Ok(hashes)
    }

    /// Load subscription hashes with their cached sizes.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read.
    pub fn load_subscriptions_with_sizes(&self) -> Result<Vec<(iroh_blobs::Hash, u64)>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let mut stmt = connection
            .prepare("SELECT hash, size FROM public_subscriptions")
            .map_err(|error| SyncwebError::operation("failed to prepare subscription query", error))?;
        let results: Vec<(iroh_blobs::Hash, u64)> = stmt
            .query_map([], |row| {
                let text: String = row.get(0)?;
                let size: i64 = row.get(1)?;
                Ok((text, size))
            })
            .map_err(|error| SyncwebError::operation("failed to query subscriptions", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read subscription rows", error))?
            .into_iter()
            .filter_map(|(h, size)| {
                let hash = h.parse::<iroh_blobs::Hash>().ok()?;
                Some((hash, size.cast_unsigned()))
            })
            .collect();
        drop(stmt);
        drop(connection);
        Ok(results)
    }

    /// Persist a public blob subscription.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn save_subscription(&self, hash: &iroh_blobs::Hash, size: u64) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        connection
            .execute(
                "INSERT OR IGNORE INTO public_subscriptions(hash, size, subscribed_at) VALUES (?1, ?2, ?3)",
                params![hash.to_string(), size.cast_signed(), now],
            )
            .map_err(|error| SyncwebError::operation("failed to save subscription", error))?;
        drop(connection);
        Ok(())
    }

    /// Remove a public blob subscription.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be written.
    pub fn remove_subscription(&self, hash: &iroh_blobs::Hash) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute(
                "DELETE FROM public_subscriptions WHERE hash = ?1",
                params![hash.to_string()],
            )
            .map_err(|error| SyncwebError::operation("failed to remove subscription", error))?;
        drop(connection);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Share registry methods
    // ------------------------------------------------------------------

    /// Persist a folder share record (upsert by namespace + access).
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn add_share(&self, namespace_id: &str, access: &str, ticket: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute(
                "INSERT INTO shares (namespace_id, access, ticket, created_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(namespace_id, access)
                 DO UPDATE SET ticket = excluded.ticket",
                params![namespace_id, access, ticket, current_timestamp().cast_signed()],
            )
            .map_err(|error| SyncwebError::operation("failed to save share record", error))?;
        drop(connection);
        Ok(())
    }

    /// Remove a persisted folder share record.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn remove_share(&self, namespace_id: &str, access: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute(
                "DELETE FROM shares WHERE namespace_id = ?1 AND access = ?2",
                params![namespace_id, access],
            )
            .map_err(|error| SyncwebError::operation("failed to remove share record", error))?;
        drop(connection);
        Ok(())
    }

    /// Load all persisted folder share records as `(namespace_id, access, ticket)`.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read.
    pub fn list_shares(&self) -> Result<Vec<(String, String, String)>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let mut stmt = connection
            .prepare("SELECT namespace_id, access, ticket FROM shares ORDER BY created_at")
            .map_err(|error| SyncwebError::operation("failed to prepare share query", error))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| SyncwebError::operation("failed to query shares", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read share rows", error))?;
        drop(stmt);
        drop(connection);
        Ok(rows)
    }

    // ------------------------------------------------------------------
    // Workspace manifest methods (replaces .syncweb-collection.json)
    // ------------------------------------------------------------------

    /// Save a workspace manifest (serialized bytes) keyed by source path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn save_workspace_manifest(&self, source_path: &str, manifest_bytes: &[u8], collection_id: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        connection
            .execute(
                "INSERT OR REPLACE INTO workspace_manifests(source_path, manifest_bytes, collection_id, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
                params![source_path, manifest_bytes, collection_id, now],
            )
            .map_err(|error| SyncwebError::operation("failed to save workspace manifest", error))?;
        drop(connection);
        Ok(())
    }

    /// Load a workspace manifest by source path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn load_workspace_manifest(&self, source_path: &str) -> Result<Option<Vec<u8>>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let result: Option<Vec<u8>> = connection
            .query_row(
                "SELECT manifest_bytes FROM workspace_manifests WHERE source_path = ?1",
                params![source_path],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| SyncwebError::operation("failed to load workspace manifest", error))?;
        drop(connection);
        Ok(result)
    }

    /// Delete a workspace manifest.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn delete_workspace_manifest(&self, source_path: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute(
                "DELETE FROM workspace_manifests WHERE source_path = ?1",
                params![source_path],
            )
            .map_err(|error| SyncwebError::operation("failed to delete workspace manifest", error))?;
        drop(connection);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Blob-folder index methods (network access control)
    // ------------------------------------------------------------------

    /// Record a blob→folder association.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_blob_folder(&self, content_hash: &[u8; 32], namespace_id: &str, entry_key: &[u8]) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        connection
            .execute(
                "INSERT OR IGNORE INTO blob_folders(content_hash, namespace_id, entry_key, added_at)
             VALUES (?1, ?2, ?3, ?4)",
                params![content_hash.as_slice(), namespace_id, entry_key, now,],
            )
            .map_err(|error| SyncwebError::operation("failed to record blob folder association", error))?;
        drop(connection);
        Ok(())
    }

    /// Remove a blob→folder association.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn remove_blob_folder(&self, content_hash: &[u8; 32], namespace_id: &str, entry_key: &[u8]) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute(
                "DELETE FROM blob_folders WHERE content_hash = ?1 AND namespace_id = ?2 AND entry_key = ?3",
                params![content_hash.as_slice(), namespace_id, entry_key],
            )
            .map_err(|error| SyncwebError::operation("failed to remove blob folder association", error))?;
        drop(connection);
        Ok(())
    }

    /// Check if a blob hash is accessible in any network the local node belongs to.
    /// Check if a blob is accessible in any network the member belongs to.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn can_access_blob(&self, hash: &Hash, member: &str) -> Result<bool> {
        self.can_access_blob_in_network(hash.as_bytes(), member)
    }

    /// Check if a blob (by raw hash bytes) is accessible in any network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn can_access_blob_in_network(&self, content_hash: &[u8; 32], member: &str) -> Result<bool> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let exists: bool = connection
            .query_row(
                "SELECT 1 FROM blob_folders bf
             JOIN network_folders nf ON bf.namespace_id = nf.namespace_id
             JOIN network_members nm ON nf.network_id = nm.network_id
             WHERE bf.content_hash = ?1 AND nm.member = ?2
             LIMIT 1",
                params![content_hash.as_slice(), member],
                |_| Ok(()),
            )
            .optional()
            .map_err(|error| SyncwebError::operation("failed to check blob access", error))?
            .is_some();
        drop(connection);
        Ok(exists)
    }

    /// Check if the local node can access a folder namespace through any network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn can_access_folder(&self, namespace_id: &str, member: &str) -> Result<bool> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let exists: bool = connection
            .query_row(
                "SELECT 1 FROM network_folders nf
             JOIN network_members nm ON nf.network_id = nm.network_id
             WHERE nf.namespace_id = ?1 AND nm.member = ?2
             LIMIT 1",
                params![namespace_id, member],
                |_| Ok(()),
            )
            .optional()
            .map_err(|error| SyncwebError::operation("failed to check folder access", error))?
            .is_some();
        drop(connection);
        Ok(exists)
    }

    /// List all namespace IDs associated with a network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn folders_for_network(&self, network_id: &str) -> Result<Vec<String>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let mut stmt = connection
            .prepare("SELECT namespace_id FROM network_folders WHERE network_id = ?1")
            .map_err(|error| SyncwebError::operation("failed to prepare folders query", error))?;
        let folders: Vec<String> = stmt
            .query_map(params![network_id], |row| row.get(0))
            .map_err(|error| SyncwebError::operation("failed to query folders", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read folder rows", error))?;
        drop(stmt);
        drop(connection);
        Ok(folders)
    }

    /// List all network IDs that contain a given folder namespace.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn networks_for_folder(&self, namespace_id: &str) -> Result<Vec<String>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let mut stmt = connection
            .prepare("SELECT network_id FROM network_folders WHERE namespace_id = ?1")
            .map_err(|error| SyncwebError::operation("failed to prepare networks query", error))?;
        let networks: Vec<String> = stmt
            .query_map(params![namespace_id], |row| row.get(0))
            .map_err(|error| SyncwebError::operation("failed to query networks", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read network rows", error))?;
        drop(stmt);
        drop(connection);
        Ok(networks)
    }

    /// List all member `PublicKey` strings for a network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn members_of_network(&self, network_id: &str) -> Result<Vec<String>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let mut stmt = connection
            .prepare("SELECT member FROM network_members WHERE network_id = ?1")
            .map_err(|error| SyncwebError::operation("failed to prepare members query", error))?;
        let members: Vec<String> = stmt
            .query_map(params![network_id], |row| row.get(0))
            .map_err(|error| SyncwebError::operation("failed to query members", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read member rows", error))?;
        drop(stmt);
        drop(connection);
        Ok(members)
    }

    /// Returns true if the given peer is a member of any network.
    /// For the per-network daemon model, a daemon only manages ONE network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn is_peer_network_member(&self, peer_public_key: &str) -> Result<bool> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let exists: bool = connection
            .query_row(
                "SELECT 1 FROM network_members WHERE member = ?1 LIMIT 1",
                params![peer_public_key],
                |_| Ok(()),
            )
            .optional()
            .map_err(|error| SyncwebError::operation("failed to check peer membership", error))?
            .is_some();
        drop(connection);
        Ok(exists)
    }

    // ------------------------------------------------------------------
    // Transfer roots and job methods
    // ------------------------------------------------------------------

    /// Add or update a configured materialization root.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn upsert_storage_root(&self, root: &StorageRootRecord) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        connection
            .execute(
                "INSERT INTO storage_roots(id, path, min_free, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(id) DO UPDATE SET
                path = excluded.path,
                min_free = excluded.min_free,
                enabled = excluded.enabled,
                updated_at = excluded.updated_at",
                params![
                    root.id,
                    root.path.to_string_lossy().as_ref(),
                    root.min_free.cast_signed(),
                    i64::from(root.enabled),
                    now,
                ],
            )
            .map_err(|error| SyncwebError::operation("failed to save storage root", error))?;
        drop(connection);
        Ok(())
    }

    /// List configured materialization roots.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn list_storage_roots(&self) -> Result<Vec<StorageRootRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let mut statement = connection
            .prepare("SELECT id, path, min_free, enabled FROM storage_roots ORDER BY id")
            .map_err(|error| SyncwebError::operation("failed to prepare storage root query", error))?;
        let roots = statement
            .query_map([], |row| {
                let min_free: i64 = row.get(2)?;
                let enabled: i64 = row.get(3)?;
                Ok(StorageRootRecord {
                    id: row.get(0)?,
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    min_free: min_free.cast_unsigned(),
                    enabled: enabled != 0,
                })
            })
            .map_err(|error| SyncwebError::operation("failed to query storage roots", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read storage root rows", error))?;
        drop(statement);
        drop(connection);
        Ok(roots)
    }

    /// Remove a storage root.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn remove_storage_root(&self, id: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute("DELETE FROM storage_roots WHERE id = ?1", params![id])
            .map_err(|error| SyncwebError::operation("failed to remove storage root", error))?;
        drop(connection);
        Ok(())
    }

    /// Enqueue a materialization or fetch job and return its durable ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn enqueue_transfer_job(&self, job: &NewTransferJob<'_>) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        connection
            .execute(
                "INSERT INTO transfer_jobs(
                    id, namespace_id, entry_key, hash, size, root_id, destination,
                    state, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'queued', ?8, ?8)",
                params![
                    id,
                    job.namespace_id,
                    job.entry_key,
                    job.hash.as_slice(),
                    job.size.cast_signed(),
                    job.root_id,
                    job.destination.map(|path| path.to_string_lossy().into_owned()),
                    now,
                ],
            )
            .map_err(|error| SyncwebError::operation("failed to enqueue transfer job", error))?;
        drop(connection);
        Ok(id)
    }

    /// List durable transfer jobs, optionally scoped by namespace and state.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn list_transfer_jobs(
        &self,
        namespace_id: Option<&str>,
        state: Option<&str>,
    ) -> Result<Vec<TransferJobRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let jobs = match (namespace_id, state) {
            (Some(namespace_filter), Some(state_filter)) => {
                let mut statement = connection
                    .prepare(
                        "SELECT id, namespace_id, entry_key, hash, size, root_id, destination,
                            state, bytes_transferred, peer_count, eta_seconds, retries,
                            error_message, created_at, updated_at
                         FROM transfer_jobs
                         WHERE namespace_id = ?1 AND state = ?2
                         ORDER BY created_at, id",
                    )
                    .map_err(|error| SyncwebError::operation("failed to prepare transfer job query", error))?;
                statement
                    .query_map(params![namespace_filter, state_filter], transfer_job_from_row)
                    .map_err(|error| SyncwebError::operation("failed to query transfer jobs", error))?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|error| SyncwebError::operation("failed to read transfer job rows", error))?
            }
            (Some(namespace_filter), None) => {
                let mut statement = connection
                    .prepare(
                        "SELECT id, namespace_id, entry_key, hash, size, root_id, destination,
                            state, bytes_transferred, peer_count, eta_seconds, retries,
                            error_message, created_at, updated_at
                         FROM transfer_jobs
                         WHERE namespace_id = ?1
                         ORDER BY created_at, id",
                    )
                    .map_err(|error| SyncwebError::operation("failed to prepare transfer job query", error))?;
                statement
                    .query_map(params![namespace_filter], transfer_job_from_row)
                    .map_err(|error| SyncwebError::operation("failed to query transfer jobs", error))?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|error| SyncwebError::operation("failed to read transfer job rows", error))?
            }
            (None, Some(state_filter)) => {
                let mut statement = connection
                    .prepare(
                        "SELECT id, namespace_id, entry_key, hash, size, root_id, destination,
                            state, bytes_transferred, peer_count, eta_seconds, retries,
                            error_message, created_at, updated_at
                         FROM transfer_jobs
                         WHERE state = ?1
                         ORDER BY created_at, id",
                    )
                    .map_err(|error| SyncwebError::operation("failed to prepare transfer job query", error))?;
                statement
                    .query_map(params![state_filter], transfer_job_from_row)
                    .map_err(|error| SyncwebError::operation("failed to query transfer jobs", error))?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|error| SyncwebError::operation("failed to read transfer job rows", error))?
            }
            (None, None) => {
                let mut statement = connection
                    .prepare(
                        "SELECT id, namespace_id, entry_key, hash, size, root_id, destination,
                            state, bytes_transferred, peer_count, eta_seconds, retries,
                            error_message, created_at, updated_at
                         FROM transfer_jobs
                         ORDER BY created_at, id",
                    )
                    .map_err(|error| SyncwebError::operation("failed to prepare transfer job query", error))?;
                statement
                    .query_map([], transfer_job_from_row)
                    .map_err(|error| SyncwebError::operation("failed to query transfer jobs", error))?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|error| SyncwebError::operation("failed to read transfer job rows", error))?
            }
        };
        drop(connection);
        Ok(jobs)
    }

    /// Update a job lifecycle state and optional error.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn update_transfer_job_state(&self, id: &str, state: &str, error_message: Option<&str>) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        let updated = connection
            .execute(
                "UPDATE transfer_jobs
                 SET state = ?1, error_message = ?2, updated_at = ?3
                 WHERE id = ?4",
                params![state, error_message, now, id],
            )
            .map_err(|error| SyncwebError::operation("failed to update transfer job state", error))?;
        if updated == 0 {
            return Err(SyncwebError::InvalidConfig(format!("transfer job not found: {id}")));
        }
        drop(connection);
        Ok(())
    }

    /// Transition a job only if it is still in the expected state.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn transition_transfer_job_state(
        &self,
        id: &str,
        expected_state: &str,
        state: &str,
        error_message: Option<&str>,
    ) -> Result<bool> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        let updated = connection
            .execute(
                "UPDATE transfer_jobs
                 SET state = ?1, error_message = ?2, updated_at = ?3
                 WHERE id = ?4 AND state = ?5",
                params![state, error_message, now, id, expected_state],
            )
            .map_err(|error| SyncwebError::operation("failed to transition transfer job state", error))?;
        drop(connection);
        Ok(updated != 0)
    }

    /// Assign a queued job to a root and its stable materialization path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn assign_transfer_job(&self, id: &str, root_id: &str, destination: &Path) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        let updated = connection
            .execute(
                "UPDATE transfer_jobs
                 SET root_id = ?1, destination = ?2, updated_at = ?3
                 WHERE id = ?4 AND state = 'queued'",
                params![root_id, destination.to_string_lossy().as_ref(), now, id],
            )
            .map_err(|error| SyncwebError::operation("failed to assign transfer job", error))?;
        if updated == 0 {
            return Err(SyncwebError::InvalidConfig(format!(
                "transfer job is missing or not queued: {id}"
            )));
        }
        drop(connection);
        Ok(())
    }

    /// Update per-job transfer counters and retry metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn update_transfer_job_progress(
        &self,
        id: &str,
        bytes_transferred: u64,
        peer_count: u64,
        eta_seconds: Option<u64>,
        retries: u32,
    ) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        let updated = connection
            .execute(
                "UPDATE transfer_jobs
                 SET bytes_transferred = ?1, peer_count = ?2, eta_seconds = ?3,
                     retries = ?4, updated_at = ?5
                 WHERE id = ?6",
                params![
                    bytes_transferred.cast_signed(),
                    peer_count.cast_signed(),
                    eta_seconds.map(u64::cast_signed),
                    retries.cast_signed(),
                    now,
                    id,
                ],
            )
            .map_err(|error| SyncwebError::operation("failed to update transfer job progress", error))?;
        if updated == 0 {
            return Err(SyncwebError::InvalidConfig(format!("transfer job not found: {id}")));
        }
        drop(connection);
        Ok(())
    }

    /// Requeue a failed or paused job while recording a retry attempt.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn retry_transfer_job(&self, id: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        let updated = connection
            .execute(
                "UPDATE transfer_jobs
                 SET state = 'queued', error_message = NULL,
                     retries = retries + 1, updated_at = ?1
                 WHERE id = ?2 AND state IN ('failed', 'paused', 'cancelled')",
                params![now, id],
            )
            .map_err(|error| SyncwebError::operation("failed to retry transfer job", error))?;
        if updated == 0 {
            return Err(SyncwebError::InvalidConfig(format!(
                "transfer job is missing or not retryable: {id}"
            )));
        }
        drop(connection);
        Ok(())
    }

    /// Requeue jobs interrupted by a daemon restart.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn recover_transfer_jobs(&self) -> Result<usize> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        let recovered = connection
            .execute(
                "UPDATE transfer_jobs
                 SET state = 'queued', updated_at = ?1
                 WHERE state IN ('fetching', 'materializing')",
                params![now],
            )
            .map_err(|error| SyncwebError::operation("failed to recover transfer jobs", error))?;
        drop(connection);
        Ok(recovered)
    }

    /// Return bytes reserved by pending jobs for a root.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn reserved_transfer_bytes(&self, root_id: &str) -> Result<u64> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let reserved: i64 = connection
            .query_row(
                "SELECT COALESCE(SUM(size), 0)
                 FROM transfer_jobs
                 WHERE root_id = ?1
                   AND state IN ('queued', 'fetching', 'materializing', 'paused')",
                params![root_id],
                |row| row.get(0),
            )
            .map_err(|error| SyncwebError::operation("failed to query reserved transfer bytes", error))?;
        drop(connection);
        Ok(reserved.cast_unsigned())
    }

    // ------------------------------------------------------------------
    // Sync checkpoint methods
    // ------------------------------------------------------------------

    /// Create a new sync checkpoint session.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn create_sync_checkpoint(&self, namespace_id: &str, session_id: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        connection
            .execute(
                "INSERT INTO sync_checkpoints(namespace_id, session_id, started_at, last_updated_at, status)
             VALUES (?1, ?2, ?3, ?4, 'running')",
                params![namespace_id, session_id, now, now],
            )
            .map_err(|error| SyncwebError::operation("failed to create sync checkpoint", error))?;
        drop(connection);
        Ok(())
    }

    /// Upsert a sync entry progress record.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn upsert_sync_entry(&self, p: &SyncEntryParams<'_>) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        connection
            .execute(
                "INSERT INTO sync_entry_progress(namespace_id, session_id, entry_key, hash, size, status, retries, error_message, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(namespace_id, session_id, entry_key) DO UPDATE SET
                status = excluded.status,
                retries = excluded.retries,
                error_message = excluded.error_message,
                updated_at = excluded.updated_at",
                params![p.namespace_id, p.session_id, p.entry_key, p.hash, p.size.cast_signed(), p.status, p.retries.cast_signed(), p.error_message, now],
            )
            .map_err(|error| SyncwebError::operation("failed to upsert sync entry", error))?;
        drop(connection);
        Ok(())
    }

    /// List sync entries for a session filtered by status.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn list_sync_entries(
        &self,
        namespace_id: &str,
        session_id: &str,
        status: &str,
    ) -> Result<Vec<crate::sync::checkpoint::EntryProgress>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let mut stmt = connection
            .prepare(
                "SELECT entry_key, hash, size, status, retries, error_message
             FROM sync_entry_progress
             WHERE namespace_id = ?1 AND session_id = ?2 AND status = ?3",
            )
            .map_err(|error| SyncwebError::operation("failed to prepare sync entry query", error))?;
        let entries = stmt
            .query_map(params![namespace_id, session_id, status], |row| {
                let entry_key: Vec<u8> = row.get(0)?;
                let hash_bytes: Vec<u8> = row.get(1)?;
                let hash_str = String::from_utf8_lossy(&hash_bytes);
                let hash = hash_str.parse::<iroh_blobs::Hash>().unwrap_or(iroh_blobs::Hash::EMPTY);
                let size: i64 = row.get(2)?;
                let status_str: String = row.get(3)?;
                let retries: i64 = row.get(4)?;
                let error_message: Option<String> = row.get(5)?;
                Ok(crate::sync::checkpoint::EntryProgress {
                    entry_key,
                    hash,
                    size: size.cast_unsigned(),
                    status: status_str,
                    retries: u32::try_from(retries).unwrap_or(0),
                    error_message,
                })
            })
            .map_err(|error| SyncwebError::operation("failed to query sync entries", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read sync entry rows", error))?;
        drop(stmt);
        drop(connection);
        Ok(entries)
    }

    /// Get checkpoint progress for a session.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn get_checkpoint_progress(
        &self,
        namespace_id: &str,
        session_id: &str,
    ) -> Result<crate::sync::checkpoint::CheckpointProgress> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let (total, completed, failed, skipped, pending, bytes_transferred, bytes_total): (
            i64, i64, i64, i64, i64, i64, Option<i64>,
        ) = connection
            .query_row(
                "SELECT
                (SELECT COUNT(*) FROM sync_entry_progress WHERE namespace_id = ?1 AND session_id = ?2),
                COALESCE((SELECT COUNT(*) FROM sync_entry_progress WHERE namespace_id = ?1 AND session_id = ?2 AND status = 'completed'), 0),
                COALESCE((SELECT COUNT(*) FROM sync_entry_progress WHERE namespace_id = ?1 AND session_id = ?2 AND status = 'failed'), 0),
                COALESCE((SELECT COUNT(*) FROM sync_entry_progress WHERE namespace_id = ?1 AND session_id = ?2 AND status = 'skipped'), 0),
                COALESCE((SELECT COUNT(*) FROM sync_entry_progress WHERE namespace_id = ?1 AND session_id = ?2 AND status = 'pending'), 0),
                COALESCE((SELECT SUM(size) FROM sync_entry_progress WHERE namespace_id = ?1 AND session_id = ?2 AND status = 'completed'), 0),
                (SELECT bytes_total FROM sync_checkpoints WHERE namespace_id = ?1 AND session_id = ?2)",
                params![namespace_id, session_id],
                |row| Ok((
                    row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?
                )),
            )
            .map_err(|error| SyncwebError::operation("failed to query checkpoint progress", error))?;
        drop(connection);
        let percentage = if total > 0 {
            let sum = completed.checked_add(skipped).unwrap_or(0);
            let pct_int = sum.checked_mul(100).and_then(|n| n.checked_div(total)).unwrap_or(0);
            f64::from(i32::try_from(pct_int).unwrap_or(0))
        } else {
            0.0
        };
        Ok(crate::sync::checkpoint::CheckpointProgress {
            total: usize::try_from(total).unwrap_or(0),
            completed: usize::try_from(completed).unwrap_or(0),
            failed: usize::try_from(failed).unwrap_or(0),
            skipped: usize::try_from(skipped).unwrap_or(0),
            pending: usize::try_from(pending).unwrap_or(0),
            bytes_transferred: bytes_transferred.cast_unsigned(),
            bytes_total: bytes_total.map(i64::cast_unsigned),
            percentage,
        })
    }

    /// Update checkpoint status.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn update_checkpoint_status(&self, namespace_id: &str, session_id: &str, status: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let now = current_timestamp().cast_signed();
        connection
            .execute(
                "UPDATE sync_checkpoints SET status = ?1, last_updated_at = ?2
             WHERE namespace_id = ?3 AND session_id = ?4",
                params![status, now, namespace_id, session_id],
            )
            .map_err(|error| SyncwebError::operation("failed to update checkpoint status", error))?;
        drop(connection);
        Ok(())
    }

    /// Find the most recent unfinished checkpoint for a namespace.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn find_unfinished_checkpoint(&self, namespace_id: &str) -> Result<Option<String>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let result: Option<String> = connection
            .query_row(
                "SELECT session_id FROM sync_checkpoints
             WHERE namespace_id = ?1 AND status IN ('running', 'pending')
             ORDER BY started_at DESC LIMIT 1",
                params![namespace_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| SyncwebError::operation("failed to find unfinished checkpoint", error))?;
        drop(connection);
        Ok(result)
    }

    /// Delete a sync checkpoint and its entries.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn delete_sync_checkpoint(&self, namespace_id: &str, session_id: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute(
                "DELETE FROM sync_entry_progress WHERE namespace_id = ?1 AND session_id = ?2",
                params![namespace_id, session_id],
            )
            .map_err(|error| SyncwebError::operation("failed to delete sync entry progress", error))?;
        connection
            .execute(
                "DELETE FROM sync_checkpoints WHERE namespace_id = ?1 AND session_id = ?2",
                params![namespace_id, session_id],
            )
            .map_err(|error| SyncwebError::operation("failed to delete sync checkpoint", error))?;
        drop(connection);
        Ok(())
    }

    /// Clean up stale (dangling) checkpoints older than the given number of seconds.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn clean_stale_checkpoints(&self, max_age_secs: i64) -> Result<usize> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let cutoff = current_timestamp().cast_signed().saturating_sub(max_age_secs);
        let deleted = connection
            .execute(
                "DELETE FROM sync_checkpoints WHERE last_updated_at < ?1 AND status NOT IN ('completed')",
                params![cutoff],
            )
            .map_err(|error| SyncwebError::operation("failed to clean stale checkpoints", error))?;
        drop(connection);
        Ok(deleted)
    }

    // ------------------------------------------------------------------
    // Database maintenance methods
    // ------------------------------------------------------------------

    /// Run VACUUM to reclaim space.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn vacuum(&self) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        connection
            .execute_batch("VACUUM")
            .map_err(|error| SyncwebError::operation("vacuum failed", error))?;
        drop(connection);
        Ok(())
    }

    /// Return estimated database size in bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the file metadata cannot be read.
    pub fn size_on_disk(&self) -> Result<u64> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let db_path: String = connection
            .query_row("PRAGMA database_list", [], |row| row.get(2))
            .map_err(|error| SyncwebError::operation("failed to query database path", error))?;
        drop(connection);
        std::fs::metadata(&db_path)
            .map(|meta| meta.len())
            .map_err(|error| SyncwebError::operation("failed to get database file size", error))
    }

    /// Return the freelist page count (indicates whether VACUUM would help).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn freelist_count(&self) -> Result<i64> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let count: i64 = connection
            .pragma_query_value(None, "freelist_count", |row| row.get(0))
            .map_err(|error| SyncwebError::operation("failed to query freelist count", error))?;
        drop(connection);
        Ok(count)
    }

    /// Run `integrity_check` PRAGMA. Returns list of errors (empty = healthy).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn check_integrity(&self) -> Result<Vec<String>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("node database mutex is poisoned", error))?;
        let mut stmt = connection
            .prepare("PRAGMA integrity_check")
            .map_err(|error| SyncwebError::operation("failed to prepare integrity check", error))?;
        let errors: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| SyncwebError::operation("failed to run integrity check", error))?
            .filter_map(std::result::Result::ok)
            .filter(|s| s != "ok")
            .collect();
        drop(stmt);
        drop(connection);
        Ok(errors)
    }
}

impl crate::storage::Vacuumable for NodeDatabase {
    fn vacuum(&self) -> Result<()> {
        self.vacuum()
    }

    fn freelist_count(&self) -> Result<i64> {
        self.freelist_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_db() -> NodeDatabase {
        let conn = Connection::open_in_memory().unwrap();
        let db = NodeDatabase {
            connection: Arc::new(Mutex::new(conn)),
        };
        db.init_schema().unwrap();
        db
    }

    fn make_public_key(seed: u8) -> PublicKey {
        iroh::SecretKey::from_bytes(&[seed; 32]).public()
    }

    #[test]
    fn test_is_peer_network_member_true() {
        let db = test_db();
        let member_key = make_public_key(1);
        let owner = make_public_key(2);

        let mut network = Network::new("test", owner, crate::net::network::NetworkOptions::default());
        network.members.insert(member_key);
        db.create_network(&network).unwrap();

        assert!(db.is_peer_network_member(&member_key.to_string()).unwrap());
    }

    #[test]
    fn test_is_peer_network_member_false() {
        let db = test_db();
        assert!(!db.is_peer_network_member("unknown-peer").unwrap());
    }

    #[test]
    fn test_transfer_roots_and_jobs_survive_lifecycle_updates() {
        let db = test_db();
        db.upsert_storage_root(&StorageRootRecord {
            id: "root".to_owned(),
            path: PathBuf::from("/tmp/syncweb-root"),
            min_free: 10,
            enabled: true,
        })
        .unwrap();
        let hash = [7_u8; 32];
        let entry_key = b"dir/file.bin";
        let job_id = db
            .enqueue_transfer_job(&NewTransferJob {
                namespace_id: "namespace",
                entry_key,
                hash: &hash,
                size: 25,
                root_id: Some("root"),
                destination: Some(Path::new("/tmp/syncweb-root/namespace/dir/file.bin")),
            })
            .unwrap();

        assert_eq!(db.reserved_transfer_bytes("root").unwrap(), 25);
        let jobs = db.list_transfer_jobs(Some("namespace"), Some("queued")).unwrap();
        assert_eq!(jobs.len(), 1);
        let job = jobs.first().expect("queued transfer job should be listed");
        assert_eq!(job.id, job_id);
        assert_eq!(job.hash, hash);
        assert_eq!(
            job.destination.as_deref(),
            Some(Path::new("/tmp/syncweb-root/namespace/dir/file.bin"))
        );

        db.update_transfer_job_progress(&job_id, 12, 2, Some(4), 1).unwrap();
        db.update_transfer_job_state(&job_id, "materializing", None).unwrap();
        assert_eq!(db.recover_transfer_jobs().unwrap(), 1);
        let recovered = db.list_transfer_jobs(None, Some("queued")).unwrap();
        let recovered_job = recovered.first().expect("recovered transfer job should be listed");
        assert_eq!(recovered_job.bytes_transferred, 12);
        assert_eq!(recovered_job.retries, 1);
    }

    #[test]
    fn test_assign_transfer_job_requires_queued_job() {
        let db = test_db();
        db.upsert_storage_root(&StorageRootRecord {
            id: "root".to_owned(),
            path: PathBuf::from("/tmp/syncweb-root"),
            min_free: 0,
            enabled: true,
        })
        .unwrap();
        let hash = [3_u8; 32];
        let job_id = db
            .enqueue_transfer_job(&NewTransferJob {
                namespace_id: "namespace",
                entry_key: b"file",
                hash: &hash,
                size: 1,
                root_id: None,
                destination: None,
            })
            .unwrap();
        db.assign_transfer_job(&job_id, "root", Path::new("/tmp/file")).unwrap();
        let jobs = db.list_transfer_jobs(None, None).unwrap();
        let job = jobs.first().expect("assigned transfer job should be listed");
        assert_eq!(job.root_id.as_deref(), Some("root"));
    }
}
