use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context as _;
use syncweb_core::storage::Config;

#[test]
fn bep_config_round_trips_as_toml() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-config-{}", uuid::Uuid::new_v4()));
    let path = directory.join("config.toml");

    let mut config = Config::default();
    config.bep.enabled = true;
    config.bep.relay_urls = vec!["tcp://relay.example:22270".to_owned()];
    config.bep.relay_timeout = 17;
    config.bep.auto_fallback = false;

    config.save(&path)?;
    anyhow::ensure!(Config::load(&path)? == config);

    let relay = config.relay_config();
    anyhow::ensure!(relay.relay_urls == vec!["tcp://relay.example:22270".to_owned()]);
    anyhow::ensure!(relay.timeout == Duration::from_secs(17));
    anyhow::ensure!(!relay.auto_fallback);

    std::fs::remove_dir_all(directory)?;
    Ok(())
}

#[test]
fn missing_config_uses_safe_defaults_and_supports_updates() -> anyhow::Result<()> {
    let mut config = Config::default();
    anyhow::ensure!(!config.bep.enabled);
    anyhow::ensure!(config.bep.relay_timeout == 10);
    config.set(
        "bep.relay_urls",
        r#"["tcp://relay.example:22270", "tcp://relay2.example:22270"]"#,
    )?;
    config.set("bep.enabled", "true")?;
    anyhow::ensure!(config.bep.relay_urls.len() == 2);
    anyhow::ensure!(config.relay_config().auto_fallback);
    Ok(())
}

#[test]
fn test_config_load_save() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-config-full-{}", uuid::Uuid::new_v4()));
    let path = directory.join("config.toml");

    let mut config = Config::default();
    config.bep.enabled = true;
    config.bep.relay_urls = vec!["tcp://relay.example:22270".to_owned()];
    config.bep.relay_timeout = 20;
    config.bep.auto_fallback = false;
    config.schedule.active_hours = "08:00-22:00".to_owned();
    config.parallel.threads = 8;
    config.cache.max_cache_size = 20000;
    config.advanced.blob_cache_size_gb = 50;
    config.default_path = Some(PathBuf::from("/tmp/syncweb"));
    config.default_sync_mode = "sendonly".to_owned();

    config.save(&path)?;
    let loaded = Config::load(&path)?;
    anyhow::ensure!(loaded == config, "loaded config must equal saved config");
    anyhow::ensure!(loaded.bep.relay_timeout == 20);
    anyhow::ensure!(loaded.schedule.active_hours == "08:00-22:00");
    anyhow::ensure!(loaded.parallel.threads == 8);
    anyhow::ensure!(loaded.cache.max_cache_size == 20000);
    anyhow::ensure!(loaded.advanced.blob_cache_size_gb == 50);
    anyhow::ensure!(loaded.default_path.as_deref() == Some(PathBuf::from("/tmp/syncweb").as_path()));
    anyhow::ensure!(loaded.default_sync_mode == "sendonly");

    std::fs::remove_dir_all(directory)?;
    Ok(())
}

#[test]
fn test_per_folder_overrides() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-config-perfolder-{}", uuid::Uuid::new_v4()));
    let path = directory.join("config.toml");

    let toml_content = r#"
[schedule]
active_hours = "08:00-22:00"

[schedule.folders.media]
active_hours = "01:00-05:00"

[schedule.folders.backup]
active_hours = "02:00-06:00"
"#;
    std::fs::create_dir_all(&directory)?;
    std::fs::write(&path, toml_content)?;

    let config = Config::load(&path)?;
    anyhow::ensure!(
        config.schedule.folders.contains_key("media"),
        "should contain media folder override"
    );
    anyhow::ensure!(
        config.schedule.folders.contains_key("backup"),
        "should contain backup folder override"
    );
    let media = config.schedule.folders.get("media").context("media folder")?;
    anyhow::ensure!(media.active_hours.as_deref() == Some("01:00-05:00"));

    let backup = config.schedule.folders.get("backup").context("backup folder")?;
    anyhow::ensure!(backup.active_hours.as_deref() == Some("02:00-06:00"));

    config.save(&path)?;
    let reloaded = Config::load(&path)?;
    anyhow::ensure!(reloaded == config, "reloaded config must equal original");

    std::fs::remove_dir_all(directory)?;
    Ok(())
}

#[test]
fn transfer_limits_round_trip_and_parse() -> anyhow::Result<()> {
    use syncweb_core::transfer_limits::{TransferLimits, TransferLimitsConfig};

    let mut config = Config::default();
    config.set("transfer_limits.max_download_per_hour", "1G")?;
    config.set("transfer_limits.max_download_per_day", "10G")?;
    config.set("transfer_limits.max_upload_per_month", "500M")?;
    anyhow::ensure!(!config.transfer_limits.is_empty());

    let directory = std::env::temp_dir().join(format!("syncweb-config-limits-{}", uuid::Uuid::new_v4()));
    let path = directory.join("config.toml");
    config.save(&path)?;
    let loaded = Config::load(&path)?;
    anyhow::ensure!(loaded == config, "transfer limits must round-trip");

    let limits = TransferLimits::from_config(&loaded.transfer_limits)?;
    anyhow::ensure!(!limits.download.is_unlimited());
    anyhow::ensure!(limits.download.per_hour == Some(1_000_000_000));
    anyhow::ensure!(limits.download.per_day == Some(10_000_000_000));
    anyhow::ensure!(limits.upload.per_month == Some(500_000_000));

    // Unknown keys are rejected, and an empty config is unlimited.
    let mut empty = TransferLimitsConfig::default();
    anyhow::ensure!(empty.set("transfer_limits.nope", "1G").is_ok_and(|applied| !applied));
    anyhow::ensure!(TransferLimits::from_config(&empty)?.is_unlimited());

    std::fs::remove_dir_all(directory)?;
    Ok(())
}
