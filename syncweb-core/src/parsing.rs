use std::time::{Duration, SystemTime};

use crate::error::{Result, SyncwebError};

/// Parse a human-readable byte size string like "5GB", "10MB", "500K", or "6%10".
///
/// Supports:
/// - Binary: KiB, MiB, GiB, TiB, PiB
/// - Decimal: KB, MB, GB, TB, PB
/// - Plain bytes: B
/// - Percentage: "6%10" means 6MB +/-10%
///
/// Returns (`min_bytes`, `max_bytes`). For exact values, min == max.
///
/// # Errors
///
/// Returns an error if the string cannot be parsed.
pub fn parse_size_constraint(s: &str) -> Result<(Option<u64>, Option<u64>)> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok((None, None));
    }

    if let Some((size_part, percent_part)) = trimmed.split_once('%') {
        let base_size = parse_bytes(size_part)?;
        let percent: u64 = percent_part
            .parse::<u64>()
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid percentage {percent_part:?}: {error}")))?;

        let lower = base_size
            .saturating_mul(100_u64.saturating_sub(percent))
            .checked_div(100)
            .unwrap_or(0);
        let upper = base_size
            .saturating_mul(100_u64.saturating_add(percent))
            .checked_div(100)
            .unwrap_or(0);
        return Ok((Some(lower), Some(upper)));
    }

    let (prefix, value_str) = trimmed
        .strip_prefix('+')
        .map(|rest| ("+", rest))
        .or_else(|| trimmed.strip_prefix('-').map(|rest| ("-", rest)))
        .or_else(|| trimmed.strip_prefix('>').map(|rest| (">", rest)))
        .or_else(|| trimmed.strip_prefix('<').map(|rest| ("<", rest)))
        .or_else(|| trimmed.strip_prefix('=').map(|rest| ("=", rest)))
        .unwrap_or(("=", trimmed));

    let size = parse_bytes(value_str)?;
    match prefix {
        "+" | ">" => Ok((Some(size.saturating_add(1)), None)),
        "-" | "<" => Ok((None, Some(size.saturating_sub(1)))),
        _ => Ok((Some(size), Some(size))),
    }
}

/// Parse a byte size string to bytes.
///
/// # Errors
///
/// Returns an error if the string cannot be parsed.
pub fn parse_bytes(s: &str) -> Result<u64> {
    let trimmed = s.trim().to_ascii_lowercase();
    let (number_str, multiplier) = split_size_suffix(&trimmed);

    let parts: Vec<&str> = number_str.split('.').collect();
    if parts.len() > 2 {
        return Err(SyncwebError::InvalidConfig(format!("invalid size {s:?}")));
    }

    let integer_part: u64 = parts
        .first()
        .copied()
        .unwrap_or("0")
        .parse::<u64>()
        .map_err(|error| SyncwebError::InvalidConfig(format!("invalid size {s:?}: {error}")))?;

    let mut total = integer_part.saturating_mul(multiplier);

    if parts.len() == 2 {
        let fraction_str = parts.get(1).copied().unwrap_or("0");
        let fraction_part: u64 = fraction_str
            .parse::<u64>()
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid size {s:?}: {error}")))?;

        let fraction_divisor = 10_u64.pow(u32::try_from(fraction_str.len()).unwrap_or(0));
        if fraction_divisor > 0 {
            let fraction_bytes = multiplier
                .saturating_mul(fraction_part)
                .checked_div(fraction_divisor)
                .unwrap_or(0);
            total = total.saturating_add(fraction_bytes);
        }
    }

    Ok(total)
}

/// Split a size string into numeric part and byte multiplier.
#[must_use]
fn split_size_suffix(value: &str) -> (&str, u64) {
    if let Some(stripped) = value.strip_suffix("pib") {
        return (stripped, 1024_u64.pow(5));
    }
    if let Some(stripped) = value.strip_suffix("tib") {
        return (stripped, 1024_u64.pow(4));
    }
    if let Some(stripped) = value.strip_suffix("gib") {
        return (stripped, 1024_u64.pow(3));
    }
    if let Some(stripped) = value.strip_suffix("mib") {
        return (stripped, 1024_u64.pow(2));
    }
    if let Some(stripped) = value.strip_suffix("kib") {
        return (stripped, 1024);
    }
    if let Some(stripped) = value.strip_suffix("pb") {
        return (stripped, 1_000_000_000_000_000);
    }
    if let Some(stripped) = value.strip_suffix("tb") {
        return (stripped, 1_000_000_000_000);
    }
    if let Some(stripped) = value.strip_suffix("gb") {
        return (stripped, 1_000_000_000);
    }
    if let Some(stripped) = value.strip_suffix("mb") {
        return (stripped, 1_000_000);
    }
    if let Some(stripped) = value.strip_suffix("kb") {
        return (stripped, 1000);
    }
    if let Some(stripped) = value.strip_suffix('b') {
        return (stripped, 1);
    }
    (value, 1)
}

/// Parse a human-readable time duration string like "3 days", "2 weeks", "1 year".
///
/// # Errors
///
/// Returns an error if the string cannot be parsed.
pub fn parse_duration(s: &str) -> Result<Duration> {
    let trimmed = s.trim().to_ascii_lowercase();
    let (number_str, unit) = split_duration_parts(&trimmed);

    let parts: Vec<&str> = number_str.split('.').collect();
    if parts.len() > 2 {
        return Err(SyncwebError::InvalidConfig(format!("invalid duration {s:?}")));
    }

    let integer_part: u64 = parts
        .first()
        .copied()
        .unwrap_or("0")
        .parse::<u64>()
        .map_err(|error| SyncwebError::InvalidConfig(format!("invalid duration {s:?}: {error}")))?;

    let multiplier = duration_multiplier(unit)?;
    let mut total_secs = integer_part.saturating_mul(multiplier);

    if parts.len() == 2 {
        let fraction_str = parts.get(1).copied().unwrap_or("0");
        let fraction_part: u64 = fraction_str
            .parse::<u64>()
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid duration {s:?}: {error}")))?;

        let fraction_divisor = 10_u64.pow(u32::try_from(fraction_str.len()).unwrap_or(0));
        if fraction_divisor > 0 {
            let fraction_secs = multiplier
                .saturating_mul(fraction_part)
                .checked_div(fraction_divisor)
                .unwrap_or(0);
            total_secs = total_secs.saturating_add(fraction_secs);
        }
    }

    Ok(Duration::from_secs(total_secs))
}

/// Split a duration string into numeric part and unit.
#[must_use]
fn split_duration_parts(value: &str) -> (&str, &str) {
    let unit_start = value
        .char_indices()
        .find_map(|(index, character)| character.is_ascii_alphabetic().then_some(index))
        .unwrap_or(value.len());
    let (number, unit_str) = value.split_at(unit_start);
    let unit = unit_str.strip_suffix('s').unwrap_or(unit_str);
    (number.trim(), unit)
}

/// Get the multiplier for a duration unit.
fn duration_multiplier(unit: &str) -> Result<u64> {
    match unit {
        "s" | "sec" | "second" => Ok(1),
        "m" | "min" | "minute" => Ok(60),
        "h" | "hr" | "hour" => Ok(3600),
        "d" | "day" => Ok(86400),
        "w" | "week" => Ok(604_800),
        "mo" | "mon" | "month" => Ok(2_592_000),
        "y" | "yr" | "year" => Ok(31_536_000),
        _ => Err(SyncwebError::InvalidConfig(format!(
            "unsupported duration unit {unit:?}"
        ))),
    }
}

/// Parse a relative time string like "3 days" or "-3 days" into a `SystemTime`.
///
/// # Errors
///
/// Returns an error if the string cannot be parsed.
pub fn parse_relative_time(s: &str) -> Result<SystemTime> {
    let trimmed = s.trim();
    let (sign, duration_str) = trimmed.strip_prefix('-').map_or_else(
        || trimmed.strip_prefix('+').map_or((1_i64, trimmed), |rest| (1_i64, rest)),
        |rest| (-1_i64, rest),
    );

    let duration = parse_duration(duration_str)?;
    let now = SystemTime::now();
    if sign < 0 {
        now.checked_sub(duration)
            .ok_or_else(|| SyncwebError::InvalidConfig("time overflow".to_owned()))
    } else {
        now.checked_add(duration)
            .ok_or_else(|| SyncwebError::InvalidConfig("time overflow".to_owned()))
    }
}

/// Parse depth constraints from strings like "+2", "-3", "1".
///
/// Returns (`min_depth`, `max_depth`).
#[must_use]
pub fn parse_depth_constraints(
    depth_list: &[String],
    min_depth: usize,
    max_depth: Option<usize>,
) -> (usize, Option<usize>) {
    let mut result_min = min_depth;
    let mut result_max = max_depth;
    for s in depth_list {
        if let Some(rest) = s.strip_prefix('+') {
            if let Ok(val) = rest.parse::<usize>() {
                result_min = result_min.max(val);
            }
            continue;
        }
        if let Some(rest) = s.strip_prefix('-') {
            if let Ok(val) = rest.parse::<usize>() {
                result_max = Some(result_max.map_or(val, |current| current.min(val)));
            }
            continue;
        }
        if let Ok(val) = s.parse::<usize>() {
            result_min = val;
            result_max = Some(val);
        }
    }
    (result_min, result_max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bytes() {
        assert_eq!(parse_bytes("5GB").unwrap(), 5_000_000_000);
        assert_eq!(parse_bytes("5Gib").unwrap(), 5 * 1024 * 1024 * 1024);
        assert_eq!(parse_bytes("500KB").unwrap(), 500_000);
        assert_eq!(parse_bytes("1024").unwrap(), 1024);
        assert_eq!(parse_bytes("1024b").unwrap(), 1024);
    }

    #[test]
    fn test_parse_size_constraint() {
        assert_eq!(
            parse_size_constraint("5GB").unwrap(),
            (Some(5_000_000_000), Some(5_000_000_000))
        );
        assert_eq!(parse_size_constraint("+5GB").unwrap(), (Some(5_000_000_001), None));
        assert_eq!(parse_size_constraint("-5GB").unwrap(), (None, Some(4_999_999_999)));
    }

    #[test]
    fn test_parse_size_constraint_percentage() {
        let (min, max) = parse_size_constraint("6MB%10").unwrap();
        assert!(min.is_some());
        assert!(max.is_some());
        let min_val = min.unwrap();
        let max_val = max.unwrap();
        assert!(min_val < 6_000_000);
        assert!(max_val > 6_000_000);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("3d").unwrap(), Duration::from_hours(72));
        assert_eq!(parse_duration("2 weeks").unwrap(), Duration::from_hours(336));
        assert_eq!(parse_duration("1 year").unwrap(), Duration::from_hours(8760));
        assert_eq!(parse_duration("30 sec").unwrap(), Duration::from_secs(30));
    }

    #[test]
    fn test_parse_depth_constraints() {
        let empty: Vec<String> = vec![];
        assert_eq!(parse_depth_constraints(&empty, 0, None), (0, None));
        assert_eq!(parse_depth_constraints(&["+2".to_owned()], 0, None), (2, None));
        assert_eq!(parse_depth_constraints(&["-3".to_owned()], 0, None), (0, Some(3)));
        assert_eq!(parse_depth_constraints(&["2".to_owned()], 0, None), (2, Some(2)));
        assert_eq!(
            parse_depth_constraints(&["+1".to_owned(), "-3".to_owned()], 0, None),
            (1, Some(3))
        );
    }
}
