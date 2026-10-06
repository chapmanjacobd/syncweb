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
    pub folders: BTreeMap<String, ScheduleFolderConfig>,
}

/// Per-folder schedule settings. Missing fields inherit the global schedule.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ScheduleFolderConfig {
    #[serde(default)]
    pub active_hours: Option<String>,
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
            return false;
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

/// Parse a byte size such as `5MB`, `500KB`, `2M`, `500K`, `4096`, or `0`.
///
/// The scale letter (`k`/`m`/`g`/`t`) is case-insensitive; the unit letter
/// distinguishes bytes (`B`) from bits (`b`). So `5KB` is 5000 bytes while
/// `5Kb` is 5000 bits (625 bytes), and bare scales such as `2M` mean bytes.
/// A trailing `/s` is accepted for backward compatibility but ignored.
///
/// # Errors
///
/// Returns an error if the value is not a whole number of bytes with a
/// supported suffix.
pub fn parse_byte_size(value: &str) -> Result<Option<u64>> {
    let normalized = value.trim();
    if normalized.is_empty() || normalized == "0" || normalized.eq_ignore_ascii_case("unlimited") {
        return Ok(None);
    }
    let size_number = normalized
        .strip_suffix("/s")
        .or_else(|| normalized.strip_suffix("/S"))
        .unwrap_or(normalized)
        .trim();
    let (numeric_part, multiplier, is_bytes) = split_size_suffix(size_number);
    let amount = numeric_part.trim().parse::<u64>().map_err(|error| {
        SyncwebError::InvalidConfig(format!(
            "invalid byte size {value:?}; expected a whole number with an optional B, KB, or Kb suffix: {error}"
        ))
    })?;
    let scaled = amount.checked_mul(multiplier);
    if is_bytes {
        scaled
            .map(Some)
            .ok_or_else(|| SyncwebError::InvalidConfig(format!("byte size {value:?} is too large")))
    } else {
        let bits = scaled.ok_or_else(|| SyncwebError::InvalidConfig(format!("byte size {value:?} is too large")))?;
        if bits.rem_euclid(8) != 0 {
            return Err(SyncwebError::InvalidConfig(format!(
                "byte size {value:?} is not a whole number of bytes"
            )));
        }
        Ok(Some(bits.checked_div(8).unwrap_or_default()))
    }
}

/// Split a size into its numeric prefix, scale multiplier, and whether the
/// unit is bytes (true) or bits (false).
#[must_use]
fn split_size_suffix(value: &str) -> (&str, u64, bool) {
    let lower = value.to_ascii_lowercase();
    for (suffix, multiplier) in [
        ("tibit", 1_u64 << 40),
        ("gibit", 1_u64 << 30),
        ("mibit", 1_u64 << 20),
        ("kibit", 1_u64 << 10),
        ("tib", 1_u64 << 40),
        ("gib", 1_u64 << 30),
        ("mib", 1_u64 << 20),
        ("kib", 1_u64 << 10),
        ("tbit", 1_000_000_000_000_u64),
        ("gbit", 1_000_000_000_u64),
        ("mbit", 1_000_000_u64),
        ("kbit", 1_000_u64),
        ("tb", 1_000_000_000_000_u64),
        ("gb", 1_000_000_000_u64),
        ("mb", 1_000_000_u64),
        ("kb", 1_000_u64),
        ("bit", 1_u64),
    ] {
        if let Some(rest) = lower.strip_suffix(suffix) {
            let is_bytes = if suffix.ends_with('t') {
                false
            } else {
                matches!(value.as_bytes().last(), Some(b'B'))
            };
            return (&value[..rest.len()], multiplier, is_bytes);
        }
    }
    let is_bytes = !matches!(value.as_bytes().last(), Some(b'b'));
    let (cut, multiplier) = if lower.ends_with('t') {
        (value.len().saturating_sub(1), 1_000_000_000_000_u64)
    } else if lower.ends_with('g') {
        (value.len().saturating_sub(1), 1_000_000_000_u64)
    } else if lower.ends_with('m') {
        (value.len().saturating_sub(1), 1_000_000_u64)
    } else if lower.ends_with('k') {
        (value.len().saturating_sub(1), 1_000_u64)
    } else if lower.ends_with('b') {
        (value.len().saturating_sub(1), 1_u64)
    } else {
        (value.len(), 1_u64)
    };
    (&value[..cut], multiplier, is_bytes)
}

/// Parsed schedule settings for one scope.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct Schedule {
    pub active_hours: Option<TimeWindow>,
}

#[derive(Clone, Debug)]
struct FolderSchedule {
    active_hours: Option<TimeWindow>,
    active_hours_override: bool,
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
    /// Returns an error if a schedule interval is invalid.
    pub fn from_config(config: &ScheduleConfig) -> Result<Self> {
        let global = parse_schedule(&config.active_hours)?;
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
                Ok((
                    name.clone(),
                    FolderSchedule {
                        active_hours,
                        active_hours_override,
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

fn parse_schedule(active_hours: &str) -> Result<Schedule> {
    Ok(Schedule {
        active_hours: TimeWindow::parse(active_hours)?,
    })
}

pub(crate) fn current_minute() -> u16 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let day_seconds = seconds % 86_400;
    u16::try_from(day_seconds.div_euclid(60)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::ensure;

    #[test]
    fn parse_byte_size_accepts_bare_scale_suffixes() -> anyhow::Result<()> {
        ensure!(parse_byte_size("500K")? == Some(500_000), "500K should be 500_000");
        ensure!(parse_byte_size("2M")? == Some(2_000_000), "2M should be 2_000_000");
        ensure!(
            parse_byte_size("1G")? == Some(1_000_000_000),
            "1G should be 1_000_000_000"
        );
        ensure!(
            parse_byte_size("2T")? == Some(2_000_000_000_000),
            "2T should be 2_000_000_000_000"
        );
        ensure!(parse_byte_size("0")?.is_none(), "0 should be unlimited");
        ensure!(parse_byte_size("unlimited")?.is_none(), "unlimited should be unlimited");
        Ok(())
    }

    #[test]
    fn parse_byte_size_preserves_existing_suffixes() -> anyhow::Result<()> {
        ensure!(parse_byte_size("5MB")? == Some(5_000_000), "5MB should be 5_000_000");
        ensure!(parse_byte_size("2MiB")? == Some(2_097_152), "2MiB should be 2_097_152");
        ensure!(parse_byte_size("3KiB")? == Some(3_072), "3KiB should be 3_072");
        ensure!(parse_byte_size("250B")? == Some(250), "250B should be 250");
        ensure!(parse_byte_size("4096")? == Some(4096), "plain number should be bytes");
        ensure!(
            parse_byte_size("5MB/s")? == Some(5_000_000),
            "a legacy /s suffix should be ignored"
        );
        Ok(())
    }

    #[test]
    fn parse_byte_size_distinguishes_bytes_from_bits() -> anyhow::Result<()> {
        ensure!(parse_byte_size("5KB")? == Some(5000), "5KB should be 5000 bytes");
        ensure!(
            parse_byte_size("5Kb")? == Some(625),
            "5Kb should be 5000 bits = 625 bytes"
        );
        ensure!(
            parse_byte_size("1MB")? == Some(1_000_000),
            "1MB should be 1_000_000 bytes"
        );
        ensure!(
            parse_byte_size("1Mb")? == Some(125_000),
            "1Mb should be 1_000_000 bits = 125_000 bytes"
        );
        ensure!(
            parse_byte_size("2Mbit")? == Some(250_000),
            "2Mbit should be 250_000 bytes"
        );
        ensure!(parse_byte_size("8b")? == Some(1), "8b should be 1 byte");
        ensure!(parse_byte_size("1b").is_err(), "1 bit should not be a whole byte");
        ensure!(
            parse_byte_size("2TB")? == Some(2_000_000_000_000),
            "2TB should be 2 TB bytes"
        );
        ensure!(
            parse_byte_size("2Tb")? == Some(250_000_000_000),
            "2Tb should be 2 TB bits = 250 GB bytes"
        );
        ensure!(
            parse_byte_size("1TiB")? == Some(1_099_511_627_776),
            "1TiB should be 2^40 bytes"
        );
        Ok(())
    }

    #[test]
    fn parse_byte_size_rejects_invalid_values() -> anyhow::Result<()> {
        ensure!(parse_byte_size("banana").is_err(), "non-numeric size should fail");
        ensure!(parse_byte_size("-2M").is_err(), "negative size should fail");
        ensure!(parse_byte_size("M").is_err(), "bare suffix should fail");
        Ok(())
    }
}
