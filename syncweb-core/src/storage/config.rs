use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::error::{Result, SyncwebError};
use crate::net::RelayConfig;
use crate::schedule::{ScheduleConfig, TimeWindow, parse_rate};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Config {
    #[serde(default)]
    pub bep: BepConfig,
    #[serde(default)]
    pub schedule: ScheduleConfig,
    #[serde(default)]
    pub bandwidth: BandwidthConfig,
    #[serde(default)]
    pub parallel: ParallelConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub advanced: AdvancedConfig,
    #[serde(default)]
    pub discovery: DiscoveryConfig,
    #[serde(default)]
    pub default_path: Option<PathBuf>,
    #[serde(default = "default_sync_mode")]
    pub default_sync_mode: String,
}

impl Config {
    /// # Errors
    ///
    /// Returns an error if the config file cannot be read or parsed.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(path_ref)
            .map_err(|error| SyncwebError::operation("failed to read config", error))?;
        toml::from_str(&contents).map_err(|error| SyncwebError::operation("failed to parse config", error))
    }

    /// # Errors
    ///
    /// Returns an error if the config cannot be serialized or written to disk.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path_ref = path.as_ref();
        let contents = toml::to_string_pretty(self)
            .map_err(|error| SyncwebError::operation("failed to serialize config", error))?;
        crate::fs::atomic::atomic_write(path_ref, contents.as_bytes())
    }

    #[must_use]
    pub fn relay_config(&self) -> RelayConfig {
        self.bep.relay_config()
    }

    /// # Errors
    ///
    /// Returns an error if the key is unknown or the value cannot be parsed.
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "bep.enabled" => self.bep.enabled = parse_bool(key, value)?,
            "bep.relay_urls" => self.bep.relay_urls = parse_string_list(key, value)?,
            "bep.relay_timeout" => {
                self.bep.relay_timeout = value.parse().map_err(|error| {
                    SyncwebError::InvalidConfig(format!("{key} must be a non-negative integer: {error}"))
                })?;
            }
            "bep.auto_fallback" => self.bep.auto_fallback = parse_bool(key, value)?,
            "schedule.active_hours" => {
                TimeWindow::parse(value)?;
                value.clone_into(&mut self.schedule.active_hours);
            }
            "bandwidth.max_upload" => {
                parse_rate(value)?;
                value.clone_into(&mut self.bandwidth.max_upload);
            }
            "bandwidth.max_download" => {
                parse_rate(value)?;
                value.clone_into(&mut self.bandwidth.max_download);
            }
            "parallel.threads" => {
                self.parallel.threads = value.parse().map_err(|error| {
                    SyncwebError::InvalidConfig(format!("{key} must be a non-negative integer: {error}"))
                })?;
            }
            "cache.max_cache_size" => {
                self.cache.max_cache_size = value.parse().map_err(|error| {
                    SyncwebError::InvalidConfig(format!("{key} must be a non-negative integer: {error}"))
                })?;
            }
            "advanced.blob_cache_size_gb" => {
                self.advanced.blob_cache_size_gb = value.parse().map_err(|error| {
                    SyncwebError::InvalidConfig(format!("{key} must be a non-negative integer: {error}"))
                })?;
            }
            "discovery.mdns" => self.discovery.mdns = parse_bool(key, value)?,
            "discovery.beacon" => self.discovery.beacon = parse_bool(key, value)?,
            "discovery.beacon_base_port" => {
                self.discovery.beacon_base_port = value.parse().map_err(|error| {
                    SyncwebError::InvalidConfig(format!("{key} must be a valid port number: {error}"))
                })?;
            }
            "discovery.beacon_interval_ms" => {
                self.discovery.beacon_interval_ms = value.parse().map_err(|error| {
                    SyncwebError::InvalidConfig(format!("{key} must be a non-negative integer: {error}"))
                })?;
            }
            "discovery.interface" => self.discovery.interface = Some(value.to_owned()),
            "default_path" => self.default_path = Some(PathBuf::from(value)),
            "default_sync_mode" => {
                value.parse::<crate::folder::SyncMode>()?;
                value.clone_into(&mut self.default_sync_mode);
            }
            _ => {
                return Err(SyncwebError::InvalidConfig(format!(
                    "unsupported config key {key:?}; supported keys: \
                     bep.enabled, bep.relay_urls, bep.relay_timeout, bep.auto_fallback, schedule.active_hours, \
                     bandwidth.max_upload, bandwidth.max_download, parallel.threads, cache.max_cache_size, \
                     advanced.blob_cache_size_gb, discovery.mdns, discovery.beacon, discovery.beacon_base_port, \
                     discovery.beacon_interval_ms, discovery.interface, default_path, default_sync_mode"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct BandwidthConfig {
    #[serde(default)]
    pub max_upload: String,
    #[serde(default)]
    pub max_download: String,
}

impl Default for BandwidthConfig {
    fn default() -> Self {
        Self {
            max_upload: "0".to_owned(),
            max_download: "0".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[derive(Default)]
pub struct ParallelConfig {
    /// Thread count for parallel operations. `0` (the default) means
    /// auto-detect the number of available CPU cores.
    #[serde(default)]
    pub threads: usize,
}

/// Controls the in-memory peer availability cache used during blob resolution.
/// This is **not** a blob data cache — it limits how many peer entries are tracked
/// for which peers serve which blobs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct CacheConfig {
    /// Maximum number of entries in the peer availability tracker.
    #[serde(default = "default_cache_size")]
    pub max_cache_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_cache_size: default_cache_size(),
        }
    }
}

/// Advanced / experimental configuration options.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct AdvancedConfig {
    /// Maximum Iroh blob cache size in GiB (default: 2). Controls how much
    /// local disk space is used for caching synced blob data.
    #[serde(default = "default_blob_cache_size")]
    pub blob_cache_size_gb: u64,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            blob_cache_size_gb: default_blob_cache_size(),
        }
    }
}

/// Local discovery settings. `interface` only applies to the UDP beacon; the
/// mDNS lookup always uses the system's default multicast interface.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct DiscoveryConfig {
    /// Whether the mDNS address lookup is registered (default: true).
    #[serde(default = "default_discovery_enabled")]
    pub mdns: bool,
    /// Whether the UDP beacon address lookup is registered (default: true).
    #[serde(default = "default_discovery_enabled")]
    pub beacon: bool,
    /// Base UDP port the beacon spreads network scopes over (default: 15200).
    #[serde(default = "default_beacon_base_port")]
    pub beacon_base_port: u16,
    /// How often the beacon re-broadcasts, in milliseconds (default: 1000).
    #[serde(default = "default_beacon_interval_ms")]
    pub beacon_interval_ms: u64,
    /// Restrict the beacon to a single network interface by name, if any.
    #[serde(default)]
    pub interface: Option<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            mdns: default_discovery_enabled(),
            beacon: default_discovery_enabled(),
            beacon_base_port: default_beacon_base_port(),
            beacon_interval_ms: default_beacon_interval_ms(),
            interface: None,
        }
    }
}

const fn default_discovery_enabled() -> bool {
    true
}

const fn default_beacon_base_port() -> u16 {
    crate::node::beacon_lookup::DEFAULT_BEACON_PORT
}

const fn default_beacon_interval_ms() -> u64 {
    1_000
}

fn default_sync_mode() -> String {
    "sendreceive".to_owned()
}

const fn default_cache_size() -> usize {
    5_000
}

const fn default_blob_cache_size() -> u64 {
    2
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct BepConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub relay_urls: Vec<String>,
    #[serde(default = "default_relay_timeout")]
    pub relay_timeout: u64,
    #[serde(default = "default_auto_fallback")]
    pub auto_fallback: bool,
}

impl Default for BepConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            relay_urls: Vec::new(),
            relay_timeout: default_relay_timeout(),
            auto_fallback: default_auto_fallback(),
        }
    }
}

impl BepConfig {
    #[must_use]
    pub fn relay_config(&self) -> RelayConfig {
        RelayConfig {
            relay_urls: self.relay_urls.clone(),
            timeout: Duration::from_secs(self.relay_timeout),
            auto_fallback: self.enabled && self.auto_fallback,
        }
    }
}

const fn default_relay_timeout() -> u64 {
    10
}

const fn default_auto_fallback() -> bool {
    true
}

fn parse_bool(key: &str, value: &str) -> Result<bool> {
    value
        .parse()
        .map_err(|error| SyncwebError::InvalidConfig(format!("{key} must be true or false: {error}")))
}

fn parse_string_list(key: &str, value: &str) -> Result<Vec<String>> {
    let parsed: toml::Value = toml::from_str(&format!("value = {value}")).map_err(|error| {
        SyncwebError::InvalidConfig(format!(
            "{key} must be a TOML string array, for example [\"tcp://relay:22270\"]: {error}"
        ))
    })?;
    parsed
        .get("value")
        .and_then(toml::Value::as_array)
        .filter(|values| values.iter().all(toml::Value::is_str))
        .map(|values| {
            values
                .iter()
                .map(|v| v.as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .ok_or_else(|| {
            SyncwebError::InvalidConfig(format!(
                "{key} must be a TOML string array, for example [\"tcp://relay:22270\"]"
            ))
        })
}
