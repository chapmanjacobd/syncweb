use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use rusqlite::{Connection, params};

use crate::{
    Result, SyncwebError,
    stats::{BandwidthStats, FolderStats, PeerStats},
};

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
            CREATE INDEX IF NOT EXISTS idx_log_level ON daemon_log(level);",
            )
            .map_err(|error| SyncwebError::operation("failed to initialize stats database schema", error))?;
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
    pub fn record_upload(&self, bytes: u64, files: u64, folder: Option<&str>, peer: Option<&str>) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let now = current_timestamp();
        connection
            .execute(
                "INSERT INTO bandwidth_events(timestamp, direction, bytes, files, folder_namespace, peer)
             VALUES (?1, 'upload', ?2, ?3, ?4, ?5)",
                params![now, bytes.cast_signed(), files.cast_signed(), folder, peer],
            )
            .map_err(|error| SyncwebError::operation("failed to record upload event", error))?;
        drop(connection);
        Ok(())
    }

    /// Record a download bandwidth event.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_download(&self, bytes: u64, files: u64, folder: Option<&str>, peer: Option<&str>) -> Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|error| SyncwebError::operation("stats database mutex is poisoned", error))?;
        let now = current_timestamp();
        connection
            .execute(
                "INSERT INTO bandwidth_events(timestamp, direction, bytes, files, folder_namespace, peer)
             VALUES (?1, 'download', ?2, ?3, ?4, ?5)",
                params![now, bytes.cast_signed(), files.cast_signed(), folder, peer],
            )
            .map_err(|error| SyncwebError::operation("failed to record download event", error))?;
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

fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs().cast_signed())
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
