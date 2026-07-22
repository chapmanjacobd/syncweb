use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{Result, SyncwebError};

/// TOML representation of the global and per-folder synchronization schedule.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ScheduleConfig {
    #[serde(default)]
    pub active_hours: String,
    #[serde(default)]
    pub bandwidth: Vec<BandwidthWindowConfig>,
    #[serde(default)]
    pub folders: BTreeMap<String, ScheduleFolderConfig>,
}

/// A bandwidth window as represented in the configuration file.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct BandwidthWindowConfig {
    pub hours: String,
    #[serde(default = "unlimited_rate")]
    pub max_upload: String,
    #[serde(default = "unlimited_rate")]
    pub max_download: String,
}

impl Default for BandwidthWindowConfig {
    fn default() -> Self {
        Self {
            hours: "00:00-24:00".to_owned(),
            max_upload: unlimited_rate(),
            max_download: unlimited_rate(),
        }
    }
}

impl BandwidthWindowConfig {
    #[must_use]
    pub fn new(hours: impl Into<String>, max_upload: impl Into<String>, max_download: impl Into<String>) -> Self {
        Self {
            hours: hours.into(),
            max_upload: max_upload.into(),
            max_download: max_download.into(),
        }
    }
}

/// Per-folder schedule settings. Missing fields inherit the global schedule.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ScheduleFolderConfig {
    #[serde(default)]
    pub active_hours: Option<String>,
    #[serde(default)]
    pub max_upload: Option<String>,
    #[serde(default)]
    pub max_download: Option<String>,
}

fn unlimited_rate() -> String {
    "0".to_owned()
}

/// A half-open time interval in minutes after midnight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct TimeWindow {
    start: u16,
    end: u16,
}

impl TimeWindow {
    /// Parse a `HH:MM-HH:MM` interval.
    ///
    /// An empty interval means that the schedule is active all day.
    ///
    /// # Errors
    ///
    /// Returns an error if the interval is not in `HH:MM-HH:MM` form.
    pub fn parse(value: &str) -> Result<Option<Self>> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let (start, end) = trimmed.split_once('-').ok_or_else(|| {
            SyncwebError::InvalidConfig(format!("invalid schedule hours {trimmed:?}; expected HH:MM-HH:MM"))
        })?;
        Ok(Some(Self {
            start: parse_clock(start)?,
            end: parse_clock(end)?,
        }))
    }

    #[must_use]
    pub const fn contains(self, minute: u16) -> bool {
        if self.start == self.end {
            return true;
        }
        if self.start < self.end {
            minute >= self.start && minute < self.end
        } else {
            minute >= self.start || minute < self.end
        }
    }

    #[must_use]
    pub const fn start(self) -> u16 {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> u16 {
        self.end
    }
}

impl Display for TimeWindow {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:02}:{:02}-{:02}:{:02}",
            self.start.div_euclid(60),
            self.start.rem_euclid(60),
            self.end.div_euclid(60),
            self.end.rem_euclid(60)
        )
    }
}

fn parse_clock(value: &str) -> Result<u16> {
    let (hour_text, minute_text) = value
        .trim()
        .split_once(':')
        .ok_or_else(|| SyncwebError::InvalidConfig(format!("invalid schedule time {value:?}; expected HH:MM")))?;
    let hour_value = hour_text
        .parse::<u16>()
        .map_err(|error| SyncwebError::InvalidConfig(format!("invalid schedule hour {value:?}: {error}")))?;
    let minute_value = minute_text
        .parse::<u16>()
        .map_err(|error| SyncwebError::InvalidConfig(format!("invalid schedule minute {value:?}: {error}")))?;
    if hour_value > 24 || minute_value > 59 || (hour_value == 24 && minute_value != 0) {
        return Err(SyncwebError::InvalidConfig(format!(
            "schedule time {value:?} is outside the 24-hour clock"
        )));
    }
    Ok(hour_value.saturating_mul(60).saturating_add(minute_value))
}

/// Parse a byte-per-second rate such as `5MB/s`, `500KB/s`, or `0`.
///
/// # Errors
///
/// Returns an error if the rate is not a whole number with a supported suffix.
pub fn parse_rate(value: &str) -> Result<Option<u64>> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() || normalized == "0" || normalized == "unlimited" {
        return Ok(None);
    }
    let rate_number = normalized.strip_suffix("/s").unwrap_or(&normalized).trim();
    let (numeric_part, multiplier) = split_rate_suffix(rate_number);
    let amount = numeric_part.trim().parse::<u64>().map_err(|error| {
        SyncwebError::InvalidConfig(format!(
            "invalid bandwidth rate {value:?}; expected a whole number with an optional B/s suffix: {error}"
        ))
    })?;
    amount
        .checked_mul(multiplier)
        .map(Some)
        .ok_or_else(|| SyncwebError::InvalidConfig(format!("bandwidth rate {value:?} is too large")))
}

#[must_use]
fn split_rate_suffix(value: &str) -> (&str, u64) {
    if let Some(stripped) = value.strip_suffix("kib") {
        return (stripped, 1024_u64);
    }
    if let Some(stripped) = value.strip_suffix("mib") {
        return (stripped, 1024_u64.saturating_mul(1024));
    }
    if let Some(stripped) = value.strip_suffix("gib") {
        return (stripped, 1024_u64.saturating_mul(1024).saturating_mul(1024));
    }
    if let Some(stripped) = value.strip_suffix("kb") {
        return (stripped, 1000_u64);
    }
    if let Some(stripped) = value.strip_suffix("mb") {
        return (stripped, 1000_u64.saturating_mul(1000));
    }
    if let Some(stripped) = value.strip_suffix("gb") {
        return (stripped, 1000_u64.saturating_mul(1000).saturating_mul(1000));
    }
    if let Some(stripped) = value.strip_suffix('b') {
        return (stripped, 1);
    }
    (value, 1)
}

/// A parsed bandwidth window.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BandwidthWindow {
    pub hours: TimeWindow,
    pub max_upload: Option<u64>,
    pub max_download: Option<u64>,
}

/// Parsed schedule settings for one scope.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct Schedule {
    pub active_hours: Option<TimeWindow>,
    pub bandwidth: Vec<BandwidthWindow>,
}

/// Effective limits at a point in time.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct BandwidthLimits {
    pub max_upload: Option<u64>,
    pub max_download: Option<u64>,
}

#[derive(Clone, Debug)]
struct FolderSchedule {
    active_hours: Option<TimeWindow>,
    active_hours_override: bool,
    max_upload: Option<u64>,
    max_upload_override: bool,
    max_download: Option<u64>,
    max_download_override: bool,
}

/// Evaluates global and per-folder schedules.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ScheduleManager {
    global: Schedule,
    folders: BTreeMap<String, FolderSchedule>,
}

impl ScheduleManager {
    /// Parse a schedule configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if a schedule interval or bandwidth rate is invalid.
    pub fn from_config(config: &ScheduleConfig) -> Result<Self> {
        let global = parse_schedule(&config.active_hours, &config.bandwidth)?;
        let folders = config
            .folders
            .iter()
            .map(|(name, folder)| {
                let (active_hours_override, active_hours) = folder
                    .active_hours
                    .as_deref()
                    .map(TimeWindow::parse)
                    .transpose()?
                    .map_or((false, None), |hours| (true, hours));
                let (max_upload_override, max_upload) = folder
                    .max_upload
                    .as_deref()
                    .map(parse_rate)
                    .transpose()?
                    .map_or((false, None), |rate| (true, rate));
                let (max_download_override, max_download) = folder
                    .max_download
                    .as_deref()
                    .map(parse_rate)
                    .transpose()?
                    .map_or((false, None), |rate| (true, rate));
                Ok((
                    name.clone(),
                    FolderSchedule {
                        active_hours,
                        active_hours_override,
                        max_upload,
                        max_upload_override,
                        max_download,
                        max_download_override,
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        Ok(Self { global, folders })
    }

    /// Return whether a folder is active at the current local wall-clock time.
    #[must_use]
    pub fn is_active(&self, folder: Option<&str>) -> bool {
        self.is_active_at(folder, current_minute())
    }

    /// Return whether a folder is active at a supplied minute after midnight.
    #[must_use]
    pub fn is_active_at(&self, folder: Option<&str>, minute: u16) -> bool {
        let active_hours = folder
            .and_then(|name| self.folders.get(name))
            .filter(|schedule| schedule.active_hours_override)
            .and_then(|schedule| schedule.active_hours)
            .or(self.global.active_hours);
        active_hours.is_none_or(|window| window.contains(minute))
    }

    /// Return the limits for a folder at the current local wall-clock time.
    #[must_use]
    pub fn current_limits(&self, folder: Option<&str>) -> BandwidthLimits {
        self.current_limits_at(folder, current_minute())
    }

    /// Return the limits for a folder at a supplied minute after midnight.
    #[must_use]
    pub fn current_limits_at(&self, folder: Option<&str>, minute: u16) -> BandwidthLimits {
        let mut limits = self
            .global
            .bandwidth
            .iter()
            .find(|window| window.hours.contains(minute))
            .map_or_else(BandwidthLimits::default, |window| BandwidthLimits {
                max_upload: window.max_upload,
                max_download: window.max_download,
            });
        if let Some(folder_schedule) = folder.and_then(|name| self.folders.get(name)) {
            if folder_schedule.max_upload_override {
                limits.max_upload = folder_schedule.max_upload;
            }
            if folder_schedule.max_download_override {
                limits.max_download = folder_schedule.max_download;
            }
        }
        limits
    }

    /// Return the next minute at which the selected schedule becomes active.
    #[must_use]
    pub fn next_active_window_start_at(&self, folder: Option<&str>, minute: u16) -> Option<u16> {
        let window = folder
            .and_then(|name| self.folders.get(name))
            .filter(|schedule| schedule.active_hours_override)
            .and_then(|schedule| schedule.active_hours)
            .or(self.global.active_hours)?;
        if window.contains(minute) {
            return Some(minute);
        }
        for offset in 1_u16..=1_440 {
            let candidate = minute.saturating_add(offset) % 1_440;
            if window.contains(candidate) {
                return Some(candidate);
            }
        }
        None
    }

    #[must_use]
    pub const fn global(&self) -> &Schedule {
        &self.global
    }
}

fn parse_schedule(active_hours: &str, bandwidth_configs: &[BandwidthWindowConfig]) -> Result<Schedule> {
    let bandwidth = bandwidth_configs
        .iter()
        .map(|window| {
            let hours = TimeWindow::parse(&window.hours)?.ok_or_else(|| {
                SyncwebError::InvalidConfig("bandwidth windows must specify a non-empty hours interval".to_owned())
            })?;
            Ok(BandwidthWindow {
                hours,
                max_upload: parse_rate(&window.max_upload)?,
                max_download: parse_rate(&window.max_download)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Schedule {
        active_hours: TimeWindow::parse(active_hours)?,
        bandwidth,
    })
}

fn current_minute() -> u16 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let day_seconds = seconds % 86_400;
    u16::try_from(day_seconds.div_euclid(60)).unwrap_or(0)
}
