use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use rusqlite::{Connection, params};

use crate::{
    Result, SyncwebError,
    bandwidth_stats::{BandwidthStats, FolderStats, PeerStats},
};

#[derive(Clone, Debug)]
pub struct StatsDatabase {
    connection: Arc<Mutex<Connection>>,
}

impl StatsDatabase {
    /// Open (or create) the stats database at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db_path = path.as_ref();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| SyncwebError::operation("failed to create stats db directory", error))?;
        }
        let connection = Connection::open(db_path)
            .map_err(|error| SyncwebError::operation("failed to open stats database", error))?;
        let db = Self {
            connection: Arc::new(Mutex::new(connection)),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|error| SyncwebError::operation("failed to configure stats database", error))?;
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
            CREATE TABLE IF NOT EXISTS bandwidth_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                direction TEXT NOT NULL CHECK(direction IN ('upload','download')),
                bytes INTEGER NOT NULL CHECK(bytes > 0),
                files INTEGER NOT NULL DEFAULT 1,
                folder_namespace TEXT,
                peer TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_bw_ts ON bandwidth_events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_bw_folder ON bandwidth_events(folder_namespace);
            CREATE INDEX IF NOT EXISTS idx_bw_peer ON bandwidth_events(peer);
            CREATE TABLE IF NOT EXISTS bandwidth_period (
                period_start INTEGER PRIMARY KEY,
                period_end INTEGER,
                total_upload INTEGER NOT NULL DEFAULT 0,
                total_download INTEGER NOT NULL DEFAULT 0,
                closed INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS bandwidth_folder (
                period_start INTEGER NOT NULL REFERENCES bandwidth_period(period_start),
                folder_namespace TEXT NOT NULL,
                upload INTEGER NOT NULL DEFAULT 0,
                download INTEGER NOT NULL DEFAULT 0,
                files_transferred INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY(period_start, folder_namespace)
            );
            CREATE TABLE IF NOT EXISTS bandwidth_peer (
                period_start INTEGER NOT NULL REFERENCES bandwidth_period(period_start),
                peer TEXT NOT NULL,
                upload INTEGER NOT NULL DEFAULT 0,
                download INTEGER NOT NULL DEFAULT 0,
                connection_count INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY(period_start, peer)
            );
            CREATE TABLE IF NOT EXISTS daemon_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                level TEXT NOT NULL CHECK(level IN ('trace','debug','info','warn','error')),
                module TEXT,
                message TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_log_ts ON daemon_log(timestamp);
            CREATE INDEX IF NOT EXISTS idx_log_level ON daemon_log(level);
            CREATE TABLE IF NOT EXISTS network_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                network_id TEXT NOT NULL,
                event_type TEXT NOT NULL CHECK(event_type IN (
                    'peer_joined','peer_left','sync_started','sync_finished',
                    'relay_connected','relay_disconnected','relay_failed',
                    'topic_subscribed','topic_unsubscribed',
                    'member_added','member_removed','folder_added','folder_removed',
                    'ticket_created','ticket_accepted','kicked'
                )),
                peer TEXT,
                details TEXT,
                metadata_json TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_network_events_ts ON network_events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_network_events_network ON network_events(network_id);
            CREATE INDEX IF NOT EXISTS idx_network_events_type ON network_events(event_type);
            CREATE TABLE IF NOT EXISTS network_sync_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                network_id TEXT NOT NULL,
                folder_namespace TEXT NOT NULL,
                started_at INTEGER NOT NULL,
                finished_at INTEGER,
                files_transferred INTEGER NOT NULL DEFAULT 0,
                bytes_transferred INTEGER NOT NULL DEFAULT 0,
                errors INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running','completed','failed','cancelled'))
            );
            CREATE INDEX IF NOT EXISTS idx_network_sessions_network ON network_sync_sessions(network_id);
            CREATE TABLE IF NOT EXISTS relay_health (
                relay_url TEXT NOT NULL,
                checked_at INTEGER NOT NULL,
                connected INTEGER NOT NULL,
                latency_ms INTEGER,
                error_message TEXT,
                PRIMARY KEY(relay_url, checked_at)
            );",
            )
            .map_err(|error| SyncwebError::operation("failed to initialize stats database schema", error))?;
        let _ = connection.execute_batch("ALTER TABLE bandwidth_events ADD COLUMN network_id TEXT");
        let _ = connection.execute_batch(
            "CREATE VIEW IF NOT EXISTS network_bandwidth_summary AS
             SELECT network_id,
                    MIN(timestamp) AS period_start,
                    MAX(timestamp) AS period_end,
                    SUM(CASE WHEN direction='upload' THEN bytes ELSE 0 END) AS total_upload,
                    SUM(CASE WHEN direction='download' THEN bytes ELSE 0 END) AS total_download,
                    COUNT(DISTINCT peer) AS active_peers
             FROM bandwidth_events
             WHERE network_id IS NOT NULL
             GROUP BY network_id",
        );
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

    /// Record an upload bandwidth event.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_upload(
        &self,
        bytes: u64,
        files: u64,
        folder: Option<&str>,
        peer: Option<&str>,
        network_id: Option<&str>,
    ) -> Result<()> {
        self.record_transfer("upload", bytes, files, folder, peer, network_id)
    }

    /// Record a download bandwidth event.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_download(
        &self,
        bytes: u64,
        files: u64,
        folder: Option<&str>,
        peer: Option<&str>,
        network_id: Option<&str>,
    ) -> Result<()> {
        self.record_transfer("download", bytes, files, folder, peer, network_id)
    }

    /// Record a transfer (upload or download) bandwidth event.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_transfer(
        &self,
        direction: &str,
        bytes: u64,
        files: u64,
        folder: Option<&str>,
        peer: Option<&str>,
        network_id: Option<&str>,
    ) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let now = current_timestamp();
        connection
            .execute(
                "INSERT INTO bandwidth_events(timestamp, direction, bytes, files, folder_namespace, peer, network_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    now,
                    direction,
                    bytes.cast_signed(),
                    files.cast_signed(),
                    folder,
                    peer,
                    network_id
                ],
            )
            .map_err(|error| SyncwebError::operation("failed to record transfer event", error))?;
        drop(connection);
        Ok(())
    }

    /// Record a connection event for the given peer.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_connection(&self, peer: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let now = current_timestamp();
        let period = find_or_create_period(&connection, now)
            .map_err(|error| SyncwebError::operation("failed to find or create bandwidth period", error))?;
        connection
            .execute(
                "INSERT INTO bandwidth_peer(period_start, peer, upload, download, connection_count)
             VALUES (?1, ?2, 0, 0, 1)
             ON CONFLICT(period_start, peer) DO UPDATE SET
                connection_count = connection_count + 1",
                params![period, peer],
            )
            .map_err(|error| SyncwebError::operation("failed to record connection", error))?;
        drop(connection);
        Ok(())
    }

    /// Return aggregated bandwidth statistics for the current period.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn current_stats(&self) -> Result<BandwidthStats> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let now = current_timestamp();
        let period_start = find_or_create_period(&connection, now)
            .map_err(|error| SyncwebError::operation("failed to find or create bandwidth period", error))?;

        let (total_upload, total_download): (i64, i64) = connection
            .query_row(
                "SELECT COALESCE(SUM(CASE WHEN direction='upload' THEN bytes ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN direction='download' THEN bytes ELSE 0 END), 0)
             FROM bandwidth_events WHERE timestamp >= ?1",
                params![period_start],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| SyncwebError::operation("failed to query bandwidth totals", error))?;

        let mut per_folder = BTreeMap::new();
        let mut folder_stmt = connection
            .prepare(
                "SELECT folder_namespace, upload, download, files_transferred
             FROM bandwidth_folder WHERE period_start = ?1",
            )
            .map_err(|error| SyncwebError::operation("failed to prepare folder stats query", error))?;
        let folder_rows = folder_stmt
            .query_map(params![period_start], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|error| SyncwebError::operation("failed to query folder stats", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read folder stats", error))?;
        for (ns, upload, download, files) in folder_rows {
            per_folder.insert(
                ns,
                FolderStats {
                    upload: upload.cast_unsigned(),
                    download: download.cast_unsigned(),
                    files_transferred: files.cast_unsigned(),
                },
            );
        }

        let mut per_peer = BTreeMap::new();
        let mut peer_stmt = connection
            .prepare(
                "SELECT peer, upload, download, connection_count
             FROM bandwidth_peer WHERE period_start = ?1",
            )
            .map_err(|error| SyncwebError::operation("failed to prepare peer stats query", error))?;
        let peer_rows = peer_stmt
            .query_map(params![period_start], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|error| SyncwebError::operation("failed to query peer stats", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read peer stats", error))?;
        for (peer, upload, download, conns) in peer_rows {
            per_peer.insert(
                peer,
                PeerStats {
                    upload: upload.cast_unsigned(),
                    download: download.cast_unsigned(),
                    connection_count: conns.cast_unsigned(),
                },
            );
        }

        drop(folder_stmt);
        drop(peer_stmt);
        drop(connection);
        Ok(BandwidthStats {
            total_upload: total_upload.cast_unsigned(),
            total_download: total_download.cast_unsigned(),
            per_folder,
            per_peer,
            period_start: period_start.cast_unsigned(),
        })
    }

    /// Delete all persisted bandwidth counters and transfer events across every
    /// period, zeroing the totals reported by [`Self::current_stats`].
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn reset_bandwidth(&self) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        connection
            .execute("DELETE FROM bandwidth_events", [])
            .map_err(|error| SyncwebError::operation("failed to clear bandwidth events", error))?;
        connection
            .execute("DELETE FROM bandwidth_folder", [])
            .map_err(|error| SyncwebError::operation("failed to clear folder bandwidth", error))?;
        connection
            .execute("DELETE FROM bandwidth_peer", [])
            .map_err(|error| SyncwebError::operation("failed to clear peer bandwidth", error))?;
        connection
            .execute("DELETE FROM bandwidth_period", [])
            .map_err(|error| SyncwebError::operation("failed to clear bandwidth periods", error))?;
        drop(connection);
        Ok(())
    }

    /// Append a daemon log entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn append_log(&self, level: &str, module: Option<&str>, message: &str) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let now = current_timestamp();
        connection
            .execute(
                "INSERT INTO daemon_log(timestamp, level, module, message) VALUES (?1, ?2, ?3, ?4)",
                params![now, level, module, message],
            )
            .map_err(|error| SyncwebError::operation("failed to append log entry", error))?;
        drop(connection);
        Ok(())
    }

    /// Delete bandwidth events older than the given duration.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn purge_old_bandwidth(&self, older_than: Duration) -> Result<usize> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let cutoff = current_timestamp().saturating_sub(older_than.as_secs().cast_signed());
        let deleted = connection
            .execute("DELETE FROM bandwidth_events WHERE timestamp < ?1", params![cutoff])
            .map_err(|error| SyncwebError::operation("failed to purge old bandwidth events", error))?;
        drop(connection);
        Ok(deleted)
    }

    /// Record a network event.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_network_event(
        &self,
        network_id: &str,
        event_type: &str,
        peer: Option<&str>,
        details: Option<&str>,
        metadata_json: Option<&str>,
    ) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let now = current_timestamp();
        connection
            .execute(
                "INSERT INTO network_events(timestamp, network_id, event_type, peer, details, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![now, network_id, event_type, peer, details, metadata_json],
            )
            .map_err(|error| SyncwebError::operation("failed to record network event", error))?;
        drop(connection);
        Ok(())
    }

    /// Record a sync session start and return its session ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_sync_session_start(&self, network_id: &str, folder_namespace: &str) -> Result<i64> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let now = current_timestamp();
        connection
            .execute(
                "INSERT INTO network_sync_sessions(network_id, folder_namespace, started_at)
             VALUES (?1, ?2, ?3)",
                params![network_id, folder_namespace, now],
            )
            .map_err(|error| SyncwebError::operation("failed to record sync session start", error))?;
        let session_id = connection.last_insert_rowid();
        drop(connection);
        Ok(session_id)
    }

    /// Record a sync session finish.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_sync_session_finish(
        &self,
        session_id: i64,
        files: u64,
        bytes: u64,
        errors: u64,
        status: &str,
    ) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let now = current_timestamp();
        connection
            .execute(
                "UPDATE network_sync_sessions SET finished_at = ?1, files_transferred = ?2,
                 bytes_transferred = ?3, errors = ?4, status = ?5 WHERE id = ?6",
                params![
                    now,
                    files.cast_signed(),
                    bytes.cast_signed(),
                    errors.cast_signed(),
                    status,
                    session_id
                ],
            )
            .map_err(|error| SyncwebError::operation("failed to record sync session finish", error))?;
        drop(connection);
        Ok(())
    }

    /// Record a relay health check.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_relay_check(
        &self,
        relay_url: &str,
        connected: bool,
        latency_ms: Option<i64>,
        error_message: Option<&str>,
    ) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let now = current_timestamp();
        connection
            .execute(
                "INSERT INTO relay_health(relay_url, checked_at, connected, latency_ms, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                params![relay_url, now, i64::from(connected), latency_ms, error_message],
            )
            .map_err(|error| SyncwebError::operation("failed to record relay check", error))?;
        drop(connection);
        Ok(())
    }

    /// Query recent network events for a given network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn recent_network_events(&self, network_id: &str, limit: usize) -> Result<Vec<NetworkEventRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let mut stmt = connection
            .prepare(
                "SELECT id, timestamp, network_id, event_type, peer, details, metadata_json
             FROM network_events WHERE network_id = ?1
             ORDER BY timestamp DESC LIMIT ?2",
            )
            .map_err(|error| SyncwebError::operation("failed to prepare network events query", error))?;
        let records = stmt
            .query_map(params![network_id, limit.cast_signed()], |row| {
                Ok(NetworkEventRecord {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    network_id: row.get(2)?,
                    event_type: row.get(3)?,
                    peer: row.get(4)?,
                    details: row.get(5)?,
                    metadata_json: row.get(6)?,
                })
            })
            .map_err(|error| SyncwebError::operation("failed to query network events", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read network event rows", error))?;
        drop(stmt);
        drop(connection);
        Ok(records)
    }

    /// Query recent sync sessions for a given network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn recent_sync_sessions(&self, network_id: &str, limit: usize) -> Result<Vec<SyncSessionRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let mut stmt = connection
            .prepare(
                "SELECT id, network_id, folder_namespace, started_at, finished_at,
                 files_transferred, bytes_transferred, errors, status
             FROM network_sync_sessions WHERE network_id = ?1
             ORDER BY started_at DESC LIMIT ?2",
            )
            .map_err(|error| SyncwebError::operation("failed to prepare sync sessions query", error))?;
        let records = stmt
            .query_map(params![network_id, limit.cast_signed()], |row| {
                Ok(SyncSessionRecord {
                    id: row.get(0)?,
                    network_id: row.get(1)?,
                    folder_namespace: row.get(2)?,
                    started_at: row.get(3)?,
                    finished_at: row.get(4)?,
                    files_transferred: row.get::<_, i64>(5)?.cast_unsigned(),
                    bytes_transferred: row.get::<_, i64>(6)?.cast_unsigned(),
                    errors: row.get::<_, i64>(7)?.cast_unsigned(),
                    status: row.get(8)?,
                })
            })
            .map_err(|error| SyncwebError::operation("failed to query sync sessions", error))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SyncwebError::operation("failed to read sync session rows", error))?;
        drop(stmt);
        drop(connection);
        Ok(records)
    }

    /// Calculate relay uptime as a ratio (0.0-1.0) over the given time window.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn relay_uptime(&self, relay_url: &str, window_secs: i64) -> Result<f64> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let cutoff = current_timestamp().saturating_sub(window_secs);
        let (total, connected): (i64, i64) = connection
            .query_row(
                "SELECT COUNT(*), SUM(connected) FROM relay_health
             WHERE relay_url = ?1 AND checked_at >= ?2",
                params![relay_url, cutoff],
                |row| Ok((row.get(0)?, row.get::<_, Option<i64>>(1)?.unwrap_or(0))),
            )
            .map_err(|error| SyncwebError::operation("failed to query relay uptime", error))?;
        drop(connection);
        Ok(if total > 0 {
            f64::from(i32::try_from(connected).unwrap_or(0))
                .mul_add(f64::from(i32::try_from(total).unwrap_or(1)).recip(), 0.0)
        } else {
            1.0
        })
    }

    /// Run VACUUM to reclaim space.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn vacuum(&self) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
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
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
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
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
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
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
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

    /// Delete daemon log entries older than the given duration.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn purge_old_logs(&self, older_than: Duration) -> Result<usize> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let cutoff = current_timestamp().saturating_sub(older_than.as_secs().cast_signed());
        let deleted = connection
            .execute("DELETE FROM daemon_log WHERE timestamp < ?1", params![cutoff])
            .map_err(|error| SyncwebError::operation("failed to purge old log entries", error))?;
        drop(connection);
        Ok(deleted)
    }
}

/// A record from the `network_events` table.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct NetworkEventRecord {
    pub id: i64,
    pub timestamp: i64,
    pub network_id: String,
    pub event_type: String,
    pub peer: Option<String>,
    pub details: Option<String>,
    pub metadata_json: Option<String>,
}

/// A record from the `network_sync_sessions` table.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SyncSessionRecord {
    pub id: i64,
    pub network_id: String,
    pub folder_namespace: String,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub files_transferred: u64,
    pub bytes_transferred: u64,
    pub errors: u64,
    pub status: String,
}

fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs().cast_signed())
}

impl crate::storage::Vacuumable for StatsDatabase {
    fn vacuum(&self) -> Result<()> {
        self.vacuum()
    }

    fn freelist_count(&self) -> Result<i64> {
        self.freelist_count()
    }
}

fn find_or_create_period(connection: &Connection, now: i64) -> std::result::Result<i64, rusqlite::Error> {
    let period_start = now.div_euclid(3600).saturating_mul(3600);
    connection.execute(
        "INSERT OR IGNORE INTO bandwidth_period(period_start, total_upload, total_download, closed)
         VALUES (?1, 0, 0, 0)",
        params![period_start],
    )?;
    Ok(period_start)
}
