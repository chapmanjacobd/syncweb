use std::time::Duration;

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
