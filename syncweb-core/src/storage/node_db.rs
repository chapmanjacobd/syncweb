use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Mutex},
};

use iroh::PublicKey;
use iroh_docs::NamespaceId;
use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

use crate::{
    Result, SyncwebError,
    daemon::{
        BandwidthSnapshot, DaemonState, DaemonStatus, DaemonStatusReport, FolderStatusReport, ScheduleStatus,
        current_timestamp,
    },
    filter::{FilterEngine, FilterEntry, FilterRule},
    folder::{CollectionState, InstalledCollection},
    net::network::{Network, NetworkId, network_topic, parse_public_key},
    storage::config::Config as AppConfig,
};

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
        db.migrate()?;
        Ok(db)
    }

    /// Run database migrations.
    ///
    /// # Errors
    ///
    /// Returns an error if the database schema cannot be created or migrated.
    pub fn migrate(&self) -> Result<()> {
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
            .execute_batch(
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
            );",
            )
            .map_err(|error| SyncwebError::operation("failed to initialize node database schema", error))?;
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
            "INSERT INTO networks(id, name, label, owner, shared_secret, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                network.id.to_string(),
                network.name,
                network.label,
                network.owner.to_string(),
                network.shared_secret.as_ref().map(|s| s.to_vec()),
                now,
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
            .prepare("SELECT id, name, label, owner, shared_secret FROM networks ORDER BY name")
            .map_err(|error| SyncwebError::operation("failed to prepare network query", error))?;
        let networks = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<Vec<u8>>>(4)?,
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
        for (id_str, name, label, owner_str, shared_secret_bytes) in networks {
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
            .execute("DELETE FROM public_subscriptions WHERE hash = ?1", params![hash.to_string()])
            .map_err(|error| SyncwebError::operation("failed to remove subscription", error))?;
        drop(connection);
        Ok(())
    }
}
