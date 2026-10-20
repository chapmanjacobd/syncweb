use std::collections::BTreeMap;

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
    assert_eq!(counts.get("discovery_observed"), Some(&1));
    assert_eq!(counts.get("repair_finished"), Some(&2));
    let events = database.recent_network_events("community-a", 10)?;
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].metadata_json.as_deref(), Some(r#"{"source":"lan"}"#));

    std::fs::remove_dir_all(directory)?;
    Ok(())
}
