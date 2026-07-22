use std::collections::HashMap;

use rand::seq::SliceRandom;

use crate::parsing::{parse_depth_constraints, parse_size_constraint};

/// Ranking strategy for synchronized entries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SortCriterion {
    Niche,
    Frecency,
    Peers,
    Random,
    FolderAggregate,
    // NEW:
    Time,
    Date,
    Week,
    Month,
    Year,
    Size,
    FolderSize,
    FolderAvgSize,
    FolderDate,
    FolderTime,
    Count,
}

impl SortCriterion {
    /// Parse a criterion from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid criterion.
    pub fn parse_str(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "niche" => Ok(Self::Niche),
            "frecency" => Ok(Self::Frecency),
            "peers" | "seeds" | "copies" => Ok(Self::Peers),
            "random" => Ok(Self::Random),
            "folder" | "folderaggregate" => Ok(Self::FolderAggregate),
            "time" => Ok(Self::Time),
            "date" | "day" => Ok(Self::Date),
            "week" => Ok(Self::Week),
            "month" => Ok(Self::Month),
            "year" => Ok(Self::Year),
            "size" => Ok(Self::Size),
            "folder-size" | "foldersize" => Ok(Self::FolderSize),
            "folder-avg-size" | "folder-size-avg" | "foldersize-avg" => Ok(Self::FolderAvgSize),
            "folder-date" | "folderdate" => Ok(Self::FolderDate),
            "folder-time" | "foldertime" => Ok(Self::FolderTime),
            "count" | "file-count" => Ok(Self::Count),
            _ => Err(format!("unsupported sort criterion: {s:?}")),
        }
    }
}

/// Configuration for the sorter.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SortConfig {
    pub criteria: Vec<(SortCriterion, bool)>, // (criterion, descending)
    pub niche: usize,
    pub frecency_weight: u64,
    pub min_seeders: Option<usize>,
    pub max_seeders: Option<usize>,
    pub limit_size: Option<u64>,
    pub min_depth: Option<usize>,
    pub max_depth: Option<usize>,
}

impl Default for SortConfig {
    fn default() -> Self {
        Self {
            criteria: vec![(SortCriterion::Niche, true), (SortCriterion::Frecency, true)],
            niche: 3,
            frecency_weight: 3,
            min_seeders: None,
            max_seeders: None,
            limit_size: None,
            min_depth: None,
            max_depth: None,
        }
    }
}

impl SortConfig {
    /// Parse sort criteria from strings like "-niche", "-frecency", "time".
    #[must_use]
    pub fn parse_criteria(criteria: &[String]) -> Vec<(SortCriterion, bool)> {
        criteria
            .iter()
            .filter_map(|s| {
                let (descending, name) = s.strip_prefix('-').map_or((false, s.as_str()), |rest| (true, rest));
                SortCriterion::parse_str(name)
                    .ok()
                    .map(|criterion| (criterion, descending))
            })
            .collect()
    }

    /// Parse limit size from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the string cannot be parsed.
    pub fn parse_limit_size(s: &str) -> Result<u64, crate::error::SyncwebError> {
        let (min, _max) = parse_size_constraint(s)?;
        min.ok_or_else(|| crate::error::SyncwebError::InvalidConfig(format!("invalid limit size: {s:?}")))
    }
}

/// Metadata consumed by [`Sorter`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SortEntry {
    pub path: std::path::PathBuf,
    pub folder: String,
    pub niche: f64,
    pub frequency: u64,
    pub peers: usize,
    pub modified: std::time::SystemTime,
    pub size: u64,
}

impl SortEntry {
    #[must_use]
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            path: path.into(),
            folder: String::new(),
            niche: 0.0,
            frequency: 0,
            peers: 0,
            modified: std::time::SystemTime::UNIX_EPOCH,
            size: 0,
        }
    }

    #[must_use]
    pub fn with_folder(mut self, folder: impl Into<String>) -> Self {
        self.folder = folder.into();
        self
    }

    #[must_use]
    pub const fn with_niche(mut self, niche: f64) -> Self {
        self.niche = niche;
        self
    }

    #[must_use]
    pub const fn with_frequency(mut self, frequency: u64) -> Self {
        self.frequency = frequency;
        self
    }

    #[must_use]
    pub const fn with_peers(mut self, peers: usize) -> Self {
        self.peers = peers;
        self
    }

    #[must_use]
    pub const fn with_modified(mut self, modified: std::time::SystemTime) -> Self {
        self.modified = modified;
        self
    }

    #[must_use]
    pub const fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }
}

/// Folder aggregates computed from entries.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct FolderAggregate {
    pub modified_median: std::time::SystemTime,
    pub size_median: u64,
    pub size_sum: u64,
    pub file_count: usize,
}

impl Default for FolderAggregate {
    fn default() -> Self {
        Self {
            modified_median: std::time::SystemTime::UNIX_EPOCH,
            size_median: 0,
            size_sum: 0,
            file_count: 0,
        }
    }
}

/// Compute folder aggregates for sorting.
#[must_use]
pub fn aggregate_folders(
    entries: &[SortEntry],
    depth_list: &[String],
    min_depth: usize,
    max_depth: Option<usize>,
) -> HashMap<String, FolderAggregate> {
    let (effective_min, effective_max) = parse_depth_constraints(depth_list, min_depth, max_depth);

    let mut grouped: HashMap<String, Vec<&SortEntry>> = HashMap::new();
    for entry in entries {
        let path = entry.path.to_string_lossy();
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let folder_parts = if parts.len() > 1 {
            parts.get(..parts.len().saturating_sub(1)).unwrap_or(&[])
        } else {
            &[]
        };

        if effective_min == 0 && effective_max.is_none() {
            // Default: immediate parent folder
            let parent = if folder_parts.is_empty() {
                String::new()
            } else {
                folder_parts.join("/")
            };
            grouped.entry(parent).or_default().push(entry);
        } else {
            // Multi-level aggregation based on depth constraints
            let root = if path.starts_with('/') { "/" } else { "" };
            for depth in effective_min..=effective_max.unwrap_or(folder_parts.len()) {
                if depth <= folder_parts.len() {
                    let folder = if depth == 0 {
                        root.to_string()
                    } else {
                        format!("{root}{}", folder_parts.get(..depth).unwrap_or(&[]).join("/"))
                    };
                    grouped.entry(folder).or_default().push(entry);
                }
            }
        }
    }

    let mut aggregates = HashMap::new();
    for (folder, folder_entries) in grouped {
        let mut modified_times: Vec<_> = folder_entries.iter().map(|e| e.modified).collect();
        modified_times.sort();

        let mut sizes: Vec<u64> = folder_entries.iter().map(|e| e.size).collect();
        sizes.sort_unstable();

        let size_sum: u64 = sizes.iter().sum();
        let file_count = folder_entries.len();

        let modified_median = modified_times
            .get(modified_times.len().checked_div(2).unwrap_or(0))
            .copied()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        let size_median = sizes.get(sizes.len().checked_div(2).unwrap_or(0)).copied().unwrap_or(0);

        aggregates.insert(
            folder,
            FolderAggregate {
                modified_median,
                size_median,
                size_sum,
                file_count,
            },
        );
    }

    aggregates
}

/// Sort entries using one of the supported content discovery rankings.
#[derive(Clone, Debug)]
pub struct Sorter {
    config: SortConfig,
}

impl Sorter {
    #[must_use]
    pub const fn new(config: SortConfig) -> Self {
        Self { config }
    }

    /// Filter entries by seeders before sorting.
    #[must_use]
    pub fn filter_seeders(&self, entries: Vec<SortEntry>) -> Vec<SortEntry> {
        entries
            .into_iter()
            .filter(|entry| {
                if let Some(min) = self.config.min_seeders
                    && entry.peers < min
                {
                    return false;
                }
                if let Some(max) = self.config.max_seeders
                    && entry.peers > max
                {
                    return false;
                }
                true
            })
            .collect()
    }

    /// Sort entries by multiple criteria.
    ///
    /// Returns an iterator that yields paths until `limit_size` is reached.
    pub fn sort<'a>(&'a self, entries: &'a mut [SortEntry]) -> SortResult<'a> {
        let folder_aggregates =
            aggregate_folders(entries, &[], self.config.min_depth.unwrap_or(0), self.config.max_depth);

        let frecency_weight = self.config.frecency_weight;

        entries.sort_by(|left, right| {
            for (criterion, descending) in &self.config.criteria {
                let ordering = match criterion {
                    SortCriterion::Niche => left
                        .niche
                        .partial_cmp(&right.niche)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    SortCriterion::Frecency => {
                        let left_frec = frecency(left, frecency_weight);
                        let right_frec = frecency(right, frecency_weight);
                        left_frec.cmp(&right_frec)
                    }
                    SortCriterion::Peers => left.peers.cmp(&right.peers),
                    SortCriterion::Random => std::cmp::Ordering::Equal, // handled separately
                    SortCriterion::FolderAggregate => {
                        let left_count = folder_aggregates.get(&left.folder).map_or(0, |agg| agg.file_count);
                        let right_count = folder_aggregates.get(&right.folder).map_or(0, |agg| agg.file_count);
                        left_count.cmp(&right_count)
                    }
                    SortCriterion::Time => left.modified.cmp(&right.modified),
                    SortCriterion::Date => {
                        let left_day = days_since_epoch(left.modified);
                        let right_day = days_since_epoch(right.modified);
                        left_day.cmp(&right_day)
                    }
                    SortCriterion::Week => {
                        let left_week = weeks_since_epoch(left.modified);
                        let right_week = weeks_since_epoch(right.modified);
                        left_week.cmp(&right_week)
                    }
                    SortCriterion::Month => {
                        let left_month = months_since_epoch(left.modified);
                        let right_month = months_since_epoch(right.modified);
                        left_month.cmp(&right_month)
                    }
                    SortCriterion::Year => {
                        let left_year = years_since_epoch(left.modified);
                        let right_year = years_since_epoch(right.modified);
                        left_year.cmp(&right_year)
                    }
                    SortCriterion::Size => left.size.cmp(&right.size),
                    SortCriterion::FolderSize => {
                        let left_size = folder_aggregates.get(&left.folder).map_or(0, |agg| agg.size_sum);
                        let right_size = folder_aggregates.get(&right.folder).map_or(0, |agg| agg.size_sum);
                        left_size.cmp(&right_size)
                    }
                    SortCriterion::FolderAvgSize => {
                        let left_size = folder_aggregates.get(&left.folder).map_or(0, |agg| agg.size_median);
                        let right_size = folder_aggregates.get(&right.folder).map_or(0, |agg| agg.size_median);
                        left_size.cmp(&right_size)
                    }
                    SortCriterion::FolderDate => {
                        let left_time = folder_aggregates
                            .get(&left.folder)
                            .map_or(std::time::SystemTime::UNIX_EPOCH, |agg| agg.modified_median);
                        let right_time = folder_aggregates
                            .get(&right.folder)
                            .map_or(std::time::SystemTime::UNIX_EPOCH, |agg| agg.modified_median);
                        let left_day = days_since_epoch(left_time);
                        let right_day = days_since_epoch(right_time);
                        left_day.cmp(&right_day)
                    }
                    SortCriterion::FolderTime => {
                        let left_time = folder_aggregates
                            .get(&left.folder)
                            .map_or(std::time::SystemTime::UNIX_EPOCH, |agg| agg.modified_median);
                        let right_time = folder_aggregates
                            .get(&right.folder)
                            .map_or(std::time::SystemTime::UNIX_EPOCH, |agg| agg.modified_median);
                        left_time.cmp(&right_time)
                    }
                    SortCriterion::Count => {
                        let left_count = folder_aggregates.get(&left.folder).map_or(0, |agg| agg.file_count);
                        let right_count = folder_aggregates.get(&right.folder).map_or(0, |agg| agg.file_count);
                        left_count.cmp(&right_count)
                    }
                };

                let final_ordering = if *descending { ordering.reverse() } else { ordering };

                if final_ordering != std::cmp::Ordering::Equal {
                    return final_ordering;
                }
            }
            std::cmp::Ordering::Equal
        });

        // Handle random shuffle if needed
        if self.config.criteria.iter().any(|(c, _)| *c == SortCriterion::Random) {
            let mut rng = rand::rng();
            entries.shuffle(&mut rng);
        }

        SortResult {
            entries,
            limit_size: self.config.limit_size,
            folder_aggregates,
        }
    }
}

/// Result of sorting, with iterator support for `limit_size`.
pub struct SortResult<'a> {
    entries: &'a mut [SortEntry],
    limit_size: Option<u64>,
    folder_aggregates: HashMap<String, FolderAggregate>,
}

impl<'a> SortResult<'a> {
    /// Iterate over sorted entries, stopping when `limit_size` is reached.
    #[must_use]
    pub const fn iter(&'a self) -> SortIterator<'a> {
        SortIterator {
            entries: self.entries,
            index: 0,
            cumulative_size: 0,
            limit_size: self.limit_size,
        }
    }

    /// Get the folder aggregates.
    #[must_use]
    pub const fn folder_aggregates(&self) -> &HashMap<String, FolderAggregate> {
        &self.folder_aggregates
    }
}

impl<'a> IntoIterator for &'a SortResult<'a> {
    type Item = &'a SortEntry;
    type IntoIter = SortIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over sorted entries with size limit.
pub struct SortIterator<'a> {
    entries: &'a [SortEntry],
    index: usize,
    cumulative_size: u64,
    limit_size: Option<u64>,
}

impl<'a> Iterator for SortIterator<'a> {
    type Item = &'a SortEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.entries.len() {
            return None;
        }

        let entry = self.entries.get(self.index)?;
        let new_size = self.cumulative_size.saturating_add(entry.size);

        if let Some(limit) = self.limit_size
            && self.index > 0
            && new_size > limit
        {
            return None;
        }

        self.index = self.index.saturating_add(1);
        self.cumulative_size = new_size;
        Some(entry)
    }
}

fn frecency(entry: &SortEntry, frecency_weight: u64) -> i64 {
    let days_since = days_since_modified(entry.modified);

    let freq = i64::try_from(entry.frequency).unwrap_or(i64::MAX);

    let penalty = days_since.checked_div(frecency_weight).unwrap_or(0);

    freq.saturating_sub(i64::try_from(penalty).unwrap_or(i64::MAX))
}

#[must_use]
fn days_since_modified(time: std::time::SystemTime) -> u64 {
    let now = std::time::SystemTime::now();
    now.duration_since(time)
        .map_or(0, |d| d.as_secs().checked_div(86400).unwrap_or(0))
}

#[must_use]
fn days_since_epoch(time: std::time::SystemTime) -> u64 {
    time.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs().checked_div(86400).unwrap_or(0))
}

#[must_use]
fn weeks_since_epoch(time: std::time::SystemTime) -> u64 {
    time.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs().checked_div(604_800).unwrap_or(0))
}

#[must_use]
fn months_since_epoch(time: std::time::SystemTime) -> u64 {
    time.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs().checked_div(2_592_000).unwrap_or(0))
}

#[must_use]
fn years_since_epoch(time: std::time::SystemTime) -> u64 {
    time.duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs().checked_div(31_536_000).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_criteria() {
        let criteria = SortConfig::parse_criteria(&["-niche".to_string(), "-frecency".to_string(), "time".to_string()]);
        assert_eq!(criteria.len(), 3);
        assert_eq!(criteria.first().copied().unwrap(), (SortCriterion::Niche, true));
        assert_eq!(criteria.get(1).copied().unwrap(), (SortCriterion::Frecency, true));
        assert_eq!(criteria.get(2).copied().unwrap(), (SortCriterion::Time, false));
    }

    #[test]
    fn test_sort_by_peers() {
        let config = SortConfig {
            criteria: vec![(SortCriterion::Peers, true)],
            ..SortConfig::default()
        };
        let sorter = Sorter::new(config);
        let mut entries = vec![
            SortEntry::new("a.txt").with_peers(1),
            SortEntry::new("b.txt").with_peers(3),
            SortEntry::new("c.txt").with_peers(2),
        ];
        let result = sorter.sort(&mut entries);
        let paths: Vec<_> = result.iter().map(|e| e.path.clone()).collect();
        assert_eq!(
            paths,
            [
                std::path::PathBuf::from("b.txt"),
                std::path::PathBuf::from("c.txt"),
                std::path::PathBuf::from("a.txt")
            ]
        );
    }

    #[test]
    fn test_sort_by_size() {
        let config = SortConfig {
            criteria: vec![(SortCriterion::Size, false)],
            ..SortConfig::default()
        };
        let sorter = Sorter::new(config);
        let mut entries = vec![
            SortEntry::new("a.txt").with_size(100),
            SortEntry::new("b.txt").with_size(300),
            SortEntry::new("c.txt").with_size(200),
        ];
        let result = sorter.sort(&mut entries);
        let paths: Vec<_> = result.iter().map(|e| e.path.clone()).collect();
        assert_eq!(
            paths,
            [
                std::path::PathBuf::from("a.txt"),
                std::path::PathBuf::from("c.txt"),
                std::path::PathBuf::from("b.txt")
            ]
        );
    }

    #[test]
    fn test_limit_size() {
        let config = SortConfig {
            criteria: vec![(SortCriterion::Size, false)],
            limit_size: Some(300),
            ..SortConfig::default()
        };
        let sorter = Sorter::new(config);
        let mut entries = vec![
            SortEntry::new("a.txt").with_size(100),
            SortEntry::new("b.txt").with_size(300),
            SortEntry::new("c.txt").with_size(200),
        ];
        let result = sorter.sort(&mut entries);
        let paths: Vec<_> = result.iter().map(|e| e.path.clone()).collect();
        assert_eq!(
            paths,
            [std::path::PathBuf::from("a.txt"), std::path::PathBuf::from("c.txt")]
        );
    }

    #[test]
    fn test_filter_seeders() {
        let config = SortConfig {
            criteria: vec![(SortCriterion::Peers, true)],
            min_seeders: Some(2),
            max_seeders: Some(4),
            ..SortConfig::default()
        };
        let sorter = Sorter::new(config);
        let entries = vec![
            SortEntry::new("a.txt").with_peers(1),
            SortEntry::new("b.txt").with_peers(3),
            SortEntry::new("c.txt").with_peers(5),
        ];
        let filtered = sorter.filter_seeders(entries);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.first().unwrap().path, std::path::PathBuf::from("b.txt"));
    }
}
