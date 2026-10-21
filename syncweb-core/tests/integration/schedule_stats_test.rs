use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;
use syncweb_core::{
    bandwidth_stats::BandwidthStats,
    net::network_log::NetworkEventType,
    schedule::{ScheduleConfig, ScheduleFolderConfig, ScheduleManager},
    storage::stats_db::StatsDatabase,
};

#[test]
fn schedule_evaluates_cross_midnight_windows_and_overrides() -> anyhow::Result<()> {
    let mut folders = BTreeMap::new();
    let mut media = ScheduleFolderConfig::default();
    media.active_hours = Some("01:00-05:00".to_owned());
    folders.insert("media".to_owned(), media);
    let mut config = ScheduleConfig::default();
    config.active_hours = "22:00-06:00".to_owned();
    config.folders = folders;

    let manager = ScheduleManager::from_config(&config)?;
    anyhow::ensure!(manager.is_active_at(None, 23 * 60));
    anyhow::ensure!(!manager.is_active_at(None, 12 * 60));
    anyhow::ensure!(manager.is_active_at(Some("media"), 2 * 60));
    anyhow::ensure!(!manager.is_active_at(Some("media"), 12 * 60));
    Ok(())
}

#[test]
fn bandwidth_stats_persist_and_reset() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-stats-{}", uuid::Uuid::new_v4()));
    let path = directory.join("stats.json");
    let mut stats = BandwidthStats::default();
    stats.record_download(100, 2, Some("folder"), Some("peer"));
    stats.record_upload(25, 1, Some("folder"), Some("peer"));
    stats.record_connection("peer");
    stats.save(&path)?;

    let loaded = BandwidthStats::load(&path)?;
    anyhow::ensure!(loaded.total_download == 100);
    anyhow::ensure!(loaded.total_upload == 25);
    anyhow::ensure!(
        loaded
            .per_folder
            .get("folder")
            .is_some_and(|folder| folder.files_transferred == 3)
    );
    anyhow::ensure!(
        loaded
            .per_peer
            .get("peer")
            .is_some_and(|peer| peer.connection_count == 1)
    );

    let mut reset = loaded;
    reset.reset();
    anyhow::ensure!(reset.total_download == 0);
    anyhow::ensure!(reset.per_folder.is_empty());
    std::fs::remove_dir_all(directory)?;
    Ok(())
}

#[test]
fn stats_database_reports_event_backed_breakdowns() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-stats-db-{}", uuid::Uuid::new_v4()));
    let database = StatsDatabase::open(directory.join("stats.db"))?;
    database.record_download(100, 2, Some("folder"), Some("peer"), None)?;
    database.record_upload(25, 1, Some("folder"), Some("peer"), None)?;
    database.record_download(0, 1, Some("folder"), Some("peer"), None)?;
    database.record_connection("peer")?;

    let stats = database.current_stats()?;
    anyhow::ensure!(stats.total_download == 100);
    anyhow::ensure!(stats.total_upload == 25);
    anyhow::ensure!(
        stats
            .per_folder
            .get("folder")
            .is_some_and(|folder| folder.download == 100 && folder.upload == 25 && folder.files_transferred == 3)
    );
    anyhow::ensure!(
        stats
            .per_peer
            .get("peer")
            .is_some_and(|peer| peer.download == 100 && peer.upload == 25 && peer.connection_count == 1)
    );

    let since = stats.period_start.cast_signed();
    let windowed = database.stats_since(since)?;
    anyhow::ensure!(
        windowed
            .per_peer
            .get("peer")
            .is_some_and(|peer| peer.connection_count == 1)
    );

    std::fs::remove_dir_all(directory)?;
    Ok(())
}

#[test]
fn legacy_bandwidth_tables_migrate_without_double_counting() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-legacy-stats-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory)?;
    let path = directory.join("stats.db");
    let now = i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())?;
    let period_start = now.div_euclid(3600) * 3600;
    let connection = Connection::open(&path)?;
    connection.execute_batch(
        "CREATE TABLE bandwidth_period (
            period_start INTEGER PRIMARY KEY,
            period_end INTEGER,
            total_upload INTEGER NOT NULL DEFAULT 0,
            total_download INTEGER NOT NULL DEFAULT 0,
            closed INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE bandwidth_folder (
            period_start INTEGER NOT NULL,
            folder_namespace TEXT NOT NULL,
            upload INTEGER NOT NULL DEFAULT 0,
            download INTEGER NOT NULL DEFAULT 0,
            files_transferred INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY(period_start, folder_namespace)
        );
        CREATE TABLE bandwidth_peer (
            period_start INTEGER NOT NULL,
            peer TEXT NOT NULL,
            upload INTEGER NOT NULL DEFAULT 0,
            download INTEGER NOT NULL DEFAULT 0,
            connection_count INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY(period_start, peer)
        );",
    )?;
    connection.execute(
        "INSERT INTO bandwidth_period(period_start, total_upload, total_download)
         VALUES (?1, 100, 200)",
        [period_start],
    )?;
    connection.execute(
        "INSERT INTO bandwidth_folder(period_start, folder_namespace, upload, download, files_transferred)
         VALUES (?1, 'folder', 100, 200, 3)",
        [period_start],
    )?;
    connection.execute(
        "INSERT INTO bandwidth_peer(period_start, peer, upload, download, connection_count)
         VALUES (?1, 'peer', 100, 200, 2)",
        [period_start],
    )?;
    drop(connection);

    let database = StatsDatabase::open(&path)?;
    let stats = database.current_stats()?;
    anyhow::ensure!(stats.total_upload == 100);
    anyhow::ensure!(stats.total_download == 200);
    anyhow::ensure!(
        stats
            .per_folder
            .get("folder")
            .is_some_and(|folder| folder.upload == 100 && folder.download == 200 && folder.files_transferred == 3)
    );
    anyhow::ensure!(
        stats
            .per_peer
            .get("peer")
            .is_some_and(|peer| peer.upload == 100 && peer.download == 200 && peer.connection_count == 2)
    );

    let migrated = Connection::open(&path)?;
    let legacy_tables: i64 = migrated.query_row(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type = 'table'
           AND name IN ('bandwidth_period', 'bandwidth_folder', 'bandwidth_peer')",
        [],
        |row| row.get(0),
    )?;
    anyhow::ensure!(legacy_tables == 0);
    drop(migrated);

    database.reset_bandwidth()?;
    let reset = database.current_stats()?;
    anyhow::ensure!(reset.total_upload == 0 && reset.total_download == 0);
    anyhow::ensure!(reset.per_folder.is_empty() && reset.per_peer.is_empty());

    std::fs::remove_dir_all(directory)?;
    Ok(())
}

#[test]
fn operational_network_events_are_counted_and_queryable() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-network-events-{}", uuid::Uuid::new_v4()));
    let database = StatsDatabase::open(directory.join("stats.db"))?;
    database.record_network_event(
        "community-a",
        NetworkEventType::DiscoveryObserved.as_str(),
        Some("peer-a"),
        Some("beacon"),
        Some(r#"{"source":"lan"}"#),
    )?;
    database.record_network_event(
        "community-a",
        NetworkEventType::RepairFinished.as_str(),
        Some("peer-b"),
        Some("repaired"),
        None,
    )?;
    database.record_network_event(
        "community-a",
        NetworkEventType::RepairFinished.as_str(),
        Some("peer-c"),
        Some("repaired"),
        None,
    )?;

    let counts = database.network_event_counts("community-a")?;
    anyhow::ensure!(counts.get("discovery_observed") == Some(&1));
    anyhow::ensure!(counts.get("repair_finished") == Some(&2));
    let events = database.recent_network_events("community-a", 10)?;
    anyhow::ensure!(events.len() == 3);
    let first_event = events
        .first()
        .ok_or_else(|| anyhow::anyhow!("recent network events should not be empty"))?;
    anyhow::ensure!(first_event.metadata_json.as_deref() == Some(r#"{"source":"lan"}"#));

    std::fs::remove_dir_all(directory)?;
    Ok(())
}
