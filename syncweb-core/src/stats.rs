use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{Result, SyncwebError};

/// Transfer counters for one synchronized folder.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FolderStats {
    pub upload: u64,
    pub download: u64,
    pub files_transferred: u64,
}

/// Transfer counters for one peer.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct PeerStats {
    pub upload: u64,
    pub download: u64,
    pub connection_count: u64,
}

/// Persisted bandwidth accounting for the local node.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct BandwidthStats {
    pub total_upload: u64,
    pub total_download: u64,
    pub per_folder: BTreeMap<String, FolderStats>,
    pub per_peer: BTreeMap<String, PeerStats>,
    pub period_start: u64,
}

impl Default for BandwidthStats {
    fn default() -> Self {
        Self {
            total_upload: 0,
            total_download: 0,
            per_folder: BTreeMap::new(),
            per_peer: BTreeMap::new(),
            period_start: now_seconds(),
        }
    }
}

impl BandwidthStats {
    /// Load counters from JSON, returning empty counters for a missing file.
    ///
    /// # Errors
    ///
    /// Returns an error if the counters cannot be read or decoded.
    pub fn load(stats_path_impl: impl AsRef<Path>) -> Result<Self> {
        let stats_path = stats_path_impl.as_ref();
        if !stats_path.exists() {
            return Ok(Self::default());
        }
        let bytes = std::fs::read(stats_path)
            .map_err(|error| SyncwebError::operation("failed to read bandwidth stats", error))?;
        serde_json::from_slice(&bytes)
            .map_err(|error| SyncwebError::operation("failed to parse bandwidth stats", error))
    }

    /// Persist counters atomically.
    ///
    /// # Errors
    ///
    /// Returns an error if the counters cannot be serialized or persisted.
    pub fn save(&self, stats_path_impl: impl AsRef<Path>) -> Result<()> {
        let stats_path = stats_path_impl.as_ref();
        if let Some(parent) = stats_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .map_err(|error| SyncwebError::operation("failed to create stats directory", error))?;
        }
        let temporary = temporary_path(stats_path);
        let result = (|| -> Result<()> {
            let bytes = serde_json::to_vec_pretty(self)
                .map_err(|error| SyncwebError::operation("failed to serialize bandwidth stats", error))?;
            std::fs::write(&temporary, bytes)
                .map_err(|error| SyncwebError::operation("failed to write temporary bandwidth stats", error))?;
            std::fs::rename(&temporary, stats_path)
                .map_err(|error| SyncwebError::operation("failed to persist bandwidth stats", error))
        })();
        if result.is_err()
            && let Err(error) = std::fs::remove_file(&temporary)
        {
            tracing::warn!(
                path = %temporary.display(),
                ?error,
                "failed to clean up temporary bandwidth stats"
            );
        }
        result
    }

    /// Reset all counters while retaining the same storage object.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Record downloaded bytes, optionally scoped to a folder and peer.
    pub fn record_download(
        &mut self,
        bytes: u64,
        files: u64,
        folder_name_option: Option<&str>,
        peer_id_option: Option<&str>,
    ) {
        self.total_download = self.total_download.saturating_add(bytes);
        if let Some(folder_name) = folder_name_option {
            let stats = self.per_folder.entry(folder_name.to_owned()).or_default();
            stats.download = stats.download.saturating_add(bytes);
            stats.files_transferred = stats.files_transferred.saturating_add(files);
        }
        if let Some(peer_id) = peer_id_option {
            let stats = self.per_peer.entry(peer_id.to_owned()).or_default();
            stats.download = stats.download.saturating_add(bytes);
        }
    }

    /// Record uploaded bytes, optionally scoped to a folder and peer.
    pub fn record_upload(
        &mut self,
        bytes: u64,
        files: u64,
        folder_name_option: Option<&str>,
        peer_id_option: Option<&str>,
    ) {
        self.total_upload = self.total_upload.saturating_add(bytes);
        if let Some(folder_name) = folder_name_option {
            let stats = self.per_folder.entry(folder_name.to_owned()).or_default();
            stats.upload = stats.upload.saturating_add(bytes);
            stats.files_transferred = stats.files_transferred.saturating_add(files);
        }
        if let Some(peer_id) = peer_id_option {
            let stats = self.per_peer.entry(peer_id.to_owned()).or_default();
            stats.upload = stats.upload.saturating_add(bytes);
        }
    }

    /// Record a newly observed peer connection.
    pub fn record_connection(&mut self, peer: &str) {
        let stats = self.per_peer.entry(peer.to_owned()).or_default();
        stats.connection_count = stats.connection_count.saturating_add(1);
    }
}

/// A group of files sharing the same extension.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ExtensionGroup {
    pub count: u64,
    pub total_size: u64,
}

/// Report produced by [`FileStatsCollector`].
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FileStatsReport {
    pub total_files: u64,
    pub total_size: u64,
    pub by_extension: BTreeMap<String, ExtensionGroup>,
    pub size_buckets: BTreeMap<String, u64>,
    /// Age distribution of entry insertion timestamps.
    /// Labels: <24h, 1d-7d, 7d-30d, 1m-1y, >1y, unknown.
    pub time_buckets: BTreeMap<String, u64>,
}

/// Collects file-level statistics from existing metadata (doc entries).
///
/// Does **not** scan the filesystem — entries are fed via [`add_entry`].
#[derive(Clone, Debug, Default)]
pub struct FileStatsCollector {
    extensions: BTreeMap<String, ExtensionGroup>,
    sizes: Vec<u64>,
    /// Entry insertion timestamps in microseconds since Unix epoch.
    /// `None` when the timestamp was not provided.
    timestamps: Vec<Option<u64>>,
}

impl FileStatsCollector {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one file entry.
    pub fn add_entry(&mut self, path: &str, size: u64) {
        let raw = path
            .rsplit('.')
            .next()
            .filter(|s| !s.contains(std::path::MAIN_SEPARATOR))
            .unwrap_or("");
        let normalized = if raw.is_empty() || raw == path { "" } else { raw };
        let group = self.extensions.entry(normalized.to_lowercase()).or_default();
        group.count = group.count.saturating_add(1);
        group.total_size = group.total_size.saturating_add(size);
        self.sizes.push(size);
    }

    /// Attempt to record an entry key stored as raw bytes (UTF-8).  Non-UTF-8 keys are silently skipped.
    pub fn add_entry_bytes(&mut self, key: &[u8], size: u64) {
        if let Ok(path) = std::str::from_utf8(key) {
            self.add_entry(path, size);
        }
    }

    /// Record one file entry with its insertion timestamp (microseconds since Unix epoch).
    pub fn add_entry_with_time(&mut self, path: &str, size: u64, timestamp_micros: Option<u64>) {
        self.add_entry(path, size);
        self.timestamps.push(timestamp_micros);
    }

    /// UTF-8 bytes variant of [`add_entry_with_time`].
    /// Non-UTF-8 keys are silently skipped and no timestamp is recorded.
    pub fn add_entry_bytes_with_time(&mut self, key: &[u8], size: u64, timestamp_micros: Option<u64>) {
        if let Ok(path) = std::str::from_utf8(key) {
            self.add_entry_with_time(path, size, timestamp_micros);
        }
    }

    /// Finalise and return the collected report.
    #[must_use]
    pub fn report(&self) -> FileStatsReport {
        let total_files = self.extensions.values().map(|g| g.count).sum();
        let total_size = self.extensions.values().map(|g| g.total_size).sum();

        let mut size_buckets = BTreeMap::new();
        for &size in &self.sizes {
            let label = match size {
                0..=1023 => "<1KB",
                1024..=1_048_575 => "1KB-1MB",
                1_048_576..=104_857_599 => "1MB-100MB",
                _ => ">100MB",
            };
            *size_buckets.entry(label.to_owned()).or_insert(0_u64) =
                size_buckets.get(label).copied().unwrap_or(0).saturating_add(1);
        }

        let now_dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let now_micros = now_dur
            .as_secs()
            .saturating_mul(1_000_000)
            .saturating_add(u64::from(now_dur.subsec_micros()));
        let mut time_buckets = BTreeMap::new();
        for &ts_opt in &self.timestamps {
            let label = ts_opt.map_or("unknown", |ts| {
                let age = now_micros.saturating_sub(ts);
                if age < 86_400_000_000 {
                    "<24h"
                } else if age < 604_800_000_000 {
                    "1d-7d"
                } else if age < 2_592_000_000_000 {
                    "7d-30d"
                } else if age < 31_536_000_000_000 {
                    "1m-1y"
                } else {
                    ">1y"
                }
            });
            *time_buckets.entry(label.to_owned()).or_insert(0_u64) =
                time_buckets.get(label).copied().unwrap_or(0).saturating_add(1);
        }

        FileStatsReport {
            total_files,
            total_size,
            by_extension: self.extensions.clone(),
            size_buckets,
            time_buckets,
        }
    }
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn temporary_path(path: &Path) -> PathBuf {
    path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::ensure;

    #[test]
    fn filestats_empty() {
        let collector = FileStatsCollector::new();
        let report = collector.report();
        assert_eq!(report.total_files, 0);
        assert_eq!(report.total_size, 0);
        assert!(report.by_extension.is_empty());
        assert!(report.size_buckets.is_empty());
        assert!(report.time_buckets.is_empty());
    }

    #[test]
    fn filestats_counts_by_extension() -> anyhow::Result<()> {
        let mut collector = FileStatsCollector::new();
        collector.add_entry("a.txt", 5);
        collector.add_entry("b.txt", 10);
        collector.add_entry("img.png", 100);
        let report = collector.report();
        ensure!(report.total_files == 3, "total_files should be 3");
        ensure!(report.total_size == 115, "total_size should be 115");
        let txt = report.by_extension.get("txt");
        ensure!(txt.is_some(), "expected txt extension");
        ensure!(txt.unwrap().count == 2, "txt count should be 2");
        let png = report.by_extension.get("png");
        ensure!(png.is_some(), "expected png extension");
        ensure!(png.unwrap().count == 1, "png count should be 1");
        ensure!(txt.unwrap().total_size == 15, "txt total_size should be 15");
        Ok(())
    }

    #[test]
    fn filestats_size_distribution() {
        let mut collector = FileStatsCollector::new();
        collector.add_entry("tiny.txt", 1);
        collector.add_entry("medium.txt", 50_000);
        collector.add_entry("large.txt", 5_000_000);
        let report = collector.report();
        assert_eq!(report.size_buckets.get("<1KB").copied().unwrap_or(0), 1);
        assert!(report.size_buckets.get("1KB-1MB").copied().unwrap_or(0) >= 1);
        assert!(report.size_buckets.get("1MB-100MB").copied().unwrap_or(0) >= 1);
    }

    #[test]
    fn filestats_no_extension() -> anyhow::Result<()> {
        let mut collector = FileStatsCollector::new();
        collector.add_entry("Makefile", 200);
        collector.add_entry("README", 50);
        let report = collector.report();
        ensure!(report.total_files == 2, "total_files should be 2");
        let no_ext = report.by_extension.get("");
        ensure!(no_ext.is_some(), "expected empty extension");
        ensure!(no_ext.unwrap().count == 2, "no-ext count should be 2");
        Ok(())
    }

    #[test]
    fn filestats_bytes_key() -> anyhow::Result<()> {
        let mut collector = FileStatsCollector::new();
        collector.add_entry_bytes(b"notes.txt", 42);
        collector.add_entry_bytes(b"sub/doc.pdf", 300);
        let report = collector.report();
        ensure!(report.total_files == 2, "total_files should be 2");
        let txt = report.by_extension.get("txt");
        ensure!(txt.is_some(), "expected txt extension");
        ensure!(txt.unwrap().count == 1, "txt count should be 1");
        let pdf = report.by_extension.get("pdf");
        ensure!(pdf.is_some(), "expected pdf extension");
        ensure!(pdf.unwrap().count == 1, "pdf count should be 1");
        Ok(())
    }

    #[test]
    fn filestats_extension_case_insensitive() -> anyhow::Result<()> {
        let mut collector = FileStatsCollector::new();
        collector.add_entry("a.TXT", 5);
        collector.add_entry("b.txt", 10);
        let report = collector.report();
        ensure!(report.by_extension.len() == 1, "should be 1 extension");
        let txt = report.by_extension.get("txt");
        ensure!(txt.is_some(), "expected txt extension");
        ensure!(txt.unwrap().count == 2, "txt count should be 2");
        Ok(())
    }

    #[test]
    fn filestats_empty_path() {
        let mut collector = FileStatsCollector::new();
        collector.add_entry("", 0);
        let report = collector.report();
        assert_eq!(report.total_files, 1);
    }

    #[test]
    fn filestats_time_distribution() {
        let mut collector = FileStatsCollector::new();
        let now = SystemTime::now();
        let ts = |offset_secs: i64| -> u64 {
            let t = if offset_secs >= 0 {
                now + std::time::Duration::from_secs(offset_secs.unsigned_abs())
            } else {
                now - std::time::Duration::from_secs(offset_secs.unsigned_abs())
            };
            let d = t.duration_since(UNIX_EPOCH).unwrap();
            d.as_secs()
                .saturating_mul(1_000_000)
                .saturating_add(u64::from(d.subsec_micros()))
        };
        collector.add_entry_with_time("recent.txt", 100, Some(ts(0)));
        collector.add_entry_with_time("old.txt", 200, Some(ts(-366 * 24 * 3600)));
        collector.add_entry_with_time("no_time.txt", 50, None);
        let report = collector.report();
        assert_eq!(report.time_buckets.get("<24h").copied().unwrap_or(0), 1);
        assert_eq!(report.time_buckets.get(">1y").copied().unwrap_or(0), 1);
        assert_eq!(report.time_buckets.get("unknown").copied().unwrap_or(0), 1);
    }

    #[test]
    fn filestats_time_all_buckets() {
        let mut collector = FileStatsCollector::new();
        let now = SystemTime::now();
        let micros = |secs_ago: u64| -> u64 {
            let t = now - std::time::Duration::from_secs(secs_ago);
            let d = t.duration_since(UNIX_EPOCH).unwrap();
            d.as_secs()
                .saturating_mul(1_000_000)
                .saturating_add(u64::from(d.subsec_micros()))
        };
        // <24h: 1 hour ago
        collector.add_entry_with_time("hour_ago.txt", 10, Some(micros(3600)));
        // 1d-7d: 2 days ago
        collector.add_entry_with_time("two_days.txt", 10, Some(micros(2 * 86400)));
        // 7d-30d: 14 days ago
        collector.add_entry_with_time("two_weeks.txt", 10, Some(micros(14 * 86400)));
        // 1m-1y: 60 days ago
        collector.add_entry_with_time("two_months.txt", 10, Some(micros(60 * 86400)));
        // >1y: 400 days ago
        collector.add_entry_with_time("year_plus.txt", 10, Some(micros(400 * 86400)));
        let report = collector.report();
        assert_eq!(report.time_buckets.len(), 5, "all five time buckets should be present");
        assert_eq!(report.time_buckets.get("<24h").copied().unwrap_or(0), 1);
        assert_eq!(report.time_buckets.get("1d-7d").copied().unwrap_or(0), 1);
        assert_eq!(report.time_buckets.get("7d-30d").copied().unwrap_or(0), 1);
        assert_eq!(report.time_buckets.get("1m-1y").copied().unwrap_or(0), 1);
        assert_eq!(report.time_buckets.get(">1y").copied().unwrap_or(0), 1);
    }

    #[test]
    fn filestats_time_via_bytes() {
        let mut collector = FileStatsCollector::new();
        collector.add_entry_bytes_with_time(b"a.txt", 5, Some(1_700_000_000_000_000));
        collector.add_entry_bytes_with_time(b"b.txt", 10, Some(1_700_000_000_000_001));
        let report = collector.report();
        assert_eq!(report.total_files, 2);
        assert!(
            report.time_buckets.contains_key("<24h")
                || report.time_buckets.contains_key("1d-7d")
                || report.time_buckets.contains_key(">1y")
        );
    }
}
