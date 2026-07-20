use std::time::Duration;

use syncweb_core::{
    net::RelayConfig,
    storage::{BepConfig, Config},
};

#[test]
fn bep_config_round_trips_as_toml() {
    let directory = std::env::temp_dir().join(format!("syncweb-config-{}", uuid::Uuid::new_v4()));
    let path = directory.join("config.toml");
    let config = Config {
        bep: BepConfig {
            enabled: true,
            relay_urls: vec!["tcp://relay.example:22270".to_owned()],
            relay_timeout: 17,
            auto_fallback: false,
        },
    };

    config.save(&path).expect("save config");
    assert_eq!(Config::load(&path).expect("load config"), config);
    assert_eq!(
        config.relay_config(),
        RelayConfig {
            relay_urls: vec!["tcp://relay.example:22270".to_owned()],
            timeout: Duration::from_secs(17),
            auto_fallback: false,
        }
    );

    std::fs::remove_dir_all(directory).expect("remove config directory");
}

#[test]
fn missing_config_uses_safe_defaults_and_supports_updates() {
    let mut config = Config::default();
    assert!(!config.bep.enabled);
    assert_eq!(config.bep.relay_timeout, 10);
    config
        .set(
            "bep.relay_urls",
            r#"["tcp://relay.example:22270", "tcp://relay2.example:22270"]"#,
        )
        .expect("set relay URLs");
    config.set("bep.enabled", "true").expect("enable relay");
    assert_eq!(config.bep.relay_urls.len(), 2);
    assert!(config.relay_config().auto_fallback);
}
