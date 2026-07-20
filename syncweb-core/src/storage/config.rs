use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::net::RelayConfig;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Config {
    #[serde(default)]
    pub bep: BepConfig,
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
            .with_context(|| format!("failed to read config {}", path_ref.display()))?;
        toml::from_str(&contents).with_context(|| format!("failed to parse config {}", path_ref.display()))
    }

    /// # Errors
    ///
    /// Returns an error if the config cannot be serialized or written to disk.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path_ref = path.as_ref();
        let contents = toml::to_string_pretty(self).context("failed to serialize config")?;
        atomic_write(path_ref, contents.as_bytes())
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
                self.bep.relay_timeout = value
                    .parse()
                    .with_context(|| format!("{key} must be a non-negative integer"))?;
            }
            "bep.auto_fallback" => self.bep.auto_fallback = parse_bool(key, value)?,
            _ => bail!(
                "unsupported config key {key:?}; supported keys: \
                 bep.enabled, bep.relay_urls, bep.relay_timeout, bep.auto_fallback"
            ),
        }
        Ok(())
    }
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
    value.parse().with_context(|| format!("{key} must be true or false"))
}

fn parse_string_list(key: &str, value: &str) -> Result<Vec<String>> {
    let parsed: toml::Value = toml::from_str(&format!("value = {value}"))
        .with_context(|| format!("{key} must be a TOML string array, for example [\"tcp://relay:22270\"]"))?;
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
        .with_context(|| format!("{key} must be a TOML string array, for example [\"tcp://relay:22270\"]"))
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }

    let temporary_path = temporary_path(path);
    let result = (|| -> Result<()> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary_path)
            .with_context(|| format!("failed to create temporary config {}", temporary_path.display()))?;
        file.write_all(contents)?;
        file.sync_all()?;
        std::fs::rename(&temporary_path, path).with_context(|| format!("failed to persist config {}", path.display()))
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&temporary_path);
    }
    result
}

fn temporary_path(path: &Path) -> PathBuf {
    path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()))
}
