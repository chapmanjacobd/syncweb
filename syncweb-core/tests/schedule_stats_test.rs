use std::collections::BTreeMap;

use syncweb_core::{
    schedule::{BandwidthWindowConfig, ScheduleConfig, ScheduleFolderConfig, ScheduleManager},
    stats::BandwidthStats,
};

#[test]
fn schedule_evaluates_cross_midnight_windows_and_overrides() -> anyhow::Result<()> {
    let mut folders = BTreeMap::new();
    let mut media = ScheduleFolderConfig::default();
    media.active_hours = Some("01:00-05:00".to_owned());
    media.max_download = Some("50MB/s".to_owned());
    folders.insert("media".to_owned(), media);
    let mut config = ScheduleConfig::default();
    config.active_hours = "22:00-06:00".to_owned();
    config.bandwidth = vec![BandwidthWindowConfig::new("22:00-06:00", "1MB/s", "5MB/s")];
    config.folders = folders;

    let manager = ScheduleManager::from_config(&config)?;
    anyhow::ensure!(manager.is_active_at(None, 23 * 60));
    anyhow::ensure!(!manager.is_active_at(None, 12 * 60));
    anyhow::ensure!(manager.is_active_at(Some("media"), 2 * 60));
    anyhow::ensure!(!manager.is_active_at(Some("media"), 12 * 60));
    anyhow::ensure!(manager.current_limits_at(None, 23 * 60).max_upload == Some(1_000_000));
    anyhow::ensure!(manager.current_limits_at(Some("media"), 23 * 60).max_download == Some(50_000_000));
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
