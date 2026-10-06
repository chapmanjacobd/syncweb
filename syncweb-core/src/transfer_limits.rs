//! Rolling-window transfer quotas and a queue of content waiting for budget.
//!
//! iroh-blobs has no per-request rate limiter and iroh-docs downloads eagerly,
//! so instead of trying to throttle a transfer in flight this module implements
//! a *download policy* in the spirit of qBittorrent: transfers are queued and
//! only start when their full size fits inside every configured per-hour,
//! per-day, and per-month byte quota. Sharing (upload) is bounded by pausing
//! the folder's live session when an upload quota would be exceeded.

use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Mutex, RwLock},
    time::{Duration, Instant},
};

use iroh_blobs::Hash;
use iroh_docs::NamespaceId;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::schedule::parse_byte_size;

/// Rolling window for the per-hour quota.
pub const WINDOW_HOUR: Duration = Duration::from_hours(1);
/// Rolling window for the per-day quota.
pub const WINDOW_DAY: Duration = Duration::from_hours(24);
/// Rolling window for the per-month quota (30 days).
pub const WINDOW_MONTH: Duration = Duration::from_hours(720);

/// Which side of a transfer a quota applies to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Direction {
    Download,
    Upload,
}

/// Parsed byte quotas for one direction. `None` means unlimited.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct TransferQuota {
    pub per_hour: Option<u64>,
    pub per_day: Option<u64>,
    pub per_month: Option<u64>,
}

impl TransferQuota {
    #[must_use]
    pub const fn is_unlimited(&self) -> bool {
        self.per_hour.is_none() && self.per_day.is_none() && self.per_month.is_none()
    }

    /// The most restrictive configured cap, if any.
    #[must_use]
    pub fn tightest_cap(&self) -> Option<u64> {
        [self.per_hour, self.per_day, self.per_month]
            .into_iter()
            .flatten()
            .min()
    }
}

/// Serializable `[transfer_limits]` configuration.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct TransferLimitsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_download_per_hour: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_download_per_day: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_download_per_month: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_upload_per_hour: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_upload_per_day: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_upload_per_month: Option<String>,
}

impl TransferLimitsConfig {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.max_download_per_hour.is_none()
            && self.max_download_per_day.is_none()
            && self.max_download_per_month.is_none()
            && self.max_upload_per_hour.is_none()
            && self.max_upload_per_day.is_none()
            && self.max_upload_per_month.is_none()
    }

    /// Validate and store one `transfer_limits.*` key.
    ///
    /// # Errors
    ///
    /// Returns an error if the key is not a recognised transfer-limit key or
    /// the value is not a valid byte count.
    pub fn set(&mut self, key: &str, value: &str) -> Result<bool> {
        let slot = match key {
            "transfer_limits.max_download_per_hour" => &mut self.max_download_per_hour,
            "transfer_limits.max_download_per_day" => &mut self.max_download_per_day,
            "transfer_limits.max_download_per_month" => &mut self.max_download_per_month,
            "transfer_limits.max_upload_per_hour" => &mut self.max_upload_per_hour,
            "transfer_limits.max_upload_per_day" => &mut self.max_upload_per_day,
            "transfer_limits.max_upload_per_month" => &mut self.max_upload_per_month,
            _ => return Ok(false),
        };
        parse_byte_size(value)?;
        *slot = Some(value.to_owned());
        Ok(true)
    }
}

/// Parsed per-direction quotas.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct TransferLimits {
    pub download: TransferQuota,
    pub upload: TransferQuota,
}

impl TransferLimits {
    /// Parse a configuration into quotas.
    ///
    /// # Errors
    ///
    /// Returns an error if any configured value is not a valid byte count.
    pub fn from_config(config: &TransferLimitsConfig) -> Result<Self> {
        let quota = |hour: &Option<String>, day: &Option<String>, month: &Option<String>| -> Result<TransferQuota> {
            Ok(TransferQuota {
                per_hour: hour.as_deref().map(parse_byte_size).transpose()?.flatten(),
                per_day: day.as_deref().map(parse_byte_size).transpose()?.flatten(),
                per_month: month.as_deref().map(parse_byte_size).transpose()?.flatten(),
            })
        };
        Ok(Self {
            download: quota(
                &config.max_download_per_hour,
                &config.max_download_per_day,
                &config.max_download_per_month,
            )?,
            upload: quota(
                &config.max_upload_per_hour,
                &config.max_upload_per_day,
                &config.max_upload_per_month,
            )?,
        })
    }

    #[must_use]
    pub const fn is_unlimited(&self) -> bool {
        self.download.is_unlimited() && self.upload.is_unlimited()
    }

    #[must_use]
    pub const fn quota(&self, direction: Direction) -> TransferQuota {
        match direction {
            Direction::Download => self.download,
            Direction::Upload => self.upload,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Sample {
    at: Instant,
    direction: Direction,
    bytes: u64,
}

/// Rolling byte accounting against a set of [`TransferLimits`].
#[derive(Debug)]
pub struct TransferBudget {
    limits: RwLock<TransferLimits>,
    samples: Mutex<VecDeque<Sample>>,
}

impl TransferBudget {
    #[must_use]
    pub const fn new(limits: TransferLimits) -> Self {
        Self {
            limits: RwLock::new(limits),
            samples: Mutex::new(VecDeque::new()),
        }
    }

    #[must_use]
    pub fn limits(&self) -> TransferLimits {
        *self.limits.read().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Replace the active quotas without discarding recorded usage.
    pub fn set_limits(&self, limits: TransferLimits) {
        *self.limits.write().unwrap_or_else(std::sync::PoisonError::into_inner) = limits;
    }

    /// Bytes that may still be transferred for `direction` right now.
    #[must_use]
    pub fn available(&self, direction: Direction) -> u64 {
        self.available_at(direction, Instant::now())
    }

    /// Bytes that may still be transferred for `direction` at `now`.
    ///
    /// The result is the smallest remaining allowance across the configured
    /// rolling windows, or [`u64::MAX`] when the direction is unlimited.
    #[must_use]
    pub fn available_at(&self, direction: Direction, now: Instant) -> u64 {
        let quota = self.limits().quota(direction);
        let mut available = u64::MAX;
        for (cap, window) in [
            (quota.per_hour, WINDOW_HOUR),
            (quota.per_day, WINDOW_DAY),
            (quota.per_month, WINDOW_MONTH),
        ] {
            if let Some(limit) = cap {
                let used = self.used_at(direction, window, now);
                available = available.min(limit.saturating_sub(used));
            }
        }
        available
    }

    /// Whether `bytes` can be transferred for `direction` without breaking a quota.
    #[must_use]
    pub fn can_transfer(&self, direction: Direction, bytes: u64) -> bool {
        bytes <= self.available(direction)
    }

    /// Record transferred bytes.
    pub fn record(&self, direction: Direction, bytes: u64) {
        self.record_at(direction, bytes, Instant::now());
    }

    /// Record transferred bytes observed at `now`.
    pub fn record_at(&self, direction: Direction, bytes: u64, now: Instant) {
        let cutoff = now.checked_sub(WINDOW_MONTH).unwrap_or(now);
        let mut samples = self.samples.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        samples.push_back(Sample {
            at: now,
            direction,
            bytes,
        });
        while samples.front().is_some_and(|sample| sample.at < cutoff) {
            samples.pop_front();
        }
        drop(samples);
    }

    /// Bytes transferred for `direction` within `window` ending at `now`.
    #[must_use]
    pub fn used_at(&self, direction: Direction, window: Duration, now: Instant) -> u64 {
        let samples = self.samples.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        samples
            .iter()
            .filter(|sample| sample.direction == direction)
            .filter(|sample| now.saturating_duration_since(sample.at) < window)
            .fold(0_u64, |total, sample| total.saturating_add(sample.bytes))
    }

    /// Total bytes recorded in all windows (diagnostics).
    #[must_use]
    pub fn recorded_total(&self) -> u64 {
        let samples = self.samples.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        samples
            .iter()
            .fold(0_u64, |total, sample| total.saturating_add(sample.bytes))
    }
}

/// A blob waiting for download budget.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PendingTransfer {
    pub namespace: NamespaceId,
    pub hash: Hash,
    pub size: u64,
    pub path: PathBuf,
}

impl PendingTransfer {
    #[must_use]
    pub const fn new(namespace: NamespaceId, hash: Hash, size: u64, path: PathBuf) -> Self {
        Self {
            namespace,
            hash,
            size,
            path,
        }
    }
}

/// FIFO queue of blobs waiting for their download quota.
#[derive(Debug, Default)]
pub struct TransferQueue {
    inner: Mutex<VecDeque<PendingTransfer>>,
}

impl TransferQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a transfer unless the same `(namespace, hash)` is already queued.
    ///
    /// Returns `true` when the item was newly added.
    pub fn enqueue(&self, item: PendingTransfer) -> bool {
        let mut queue = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if queue
            .iter()
            .any(|queued| queued.namespace == item.namespace && queued.hash == item.hash)
        {
            return false;
        }
        queue.push_back(item);
        true
    }

    /// Remove and return the first item that fits the download budget.
    pub fn pop_fitting(&self, budget: &TransferBudget) -> Option<PendingTransfer> {
        let mut queue = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let position = queue
            .iter()
            .position(|item| item.size <= budget.available(Direction::Download))?;
        queue.remove(position)
    }

    /// Drop everything queued for a namespace (e.g. after leaving a folder).
    pub fn remove_namespace(&self, namespace: NamespaceId) {
        let mut queue = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        queue.retain(|item| item.namespace != namespace);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn pending_bytes(&self) -> u64 {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .fold(0_u64, |total, item| total.saturating_add(item.size))
    }

    #[must_use]
    pub fn snapshot(&self) -> Vec<PendingTransfer> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(
        download: (Option<u64>, Option<u64>, Option<u64>),
        upload: (Option<u64>, Option<u64>, Option<u64>),
    ) -> TransferLimits {
        TransferLimits {
            download: TransferQuota {
                per_hour: download.0,
                per_day: download.1,
                per_month: download.2,
            },
            upload: TransferQuota {
                per_hour: upload.0,
                per_day: upload.1,
                per_month: upload.2,
            },
        }
    }

    #[test]
    fn unlimited_quota_has_no_ceiling() {
        let budget = TransferBudget::new(limits((None, None, None), (None, None, None)));
        assert_eq!(budget.available(Direction::Download), u64::MAX);
        assert!(budget.can_transfer(Direction::Download, u64::MAX));
    }

    #[test]
    fn hourly_cap_is_enforced() {
        let budget = TransferBudget::new(limits((Some(1_000), None, None), (None, None, None)));
        let now = Instant::now();
        budget.record_at(Direction::Download, 400, now);
        assert_eq!(budget.available_at(Direction::Download, now), 600);
        budget.record_at(Direction::Download, 600, now);
        assert!(!budget.can_transfer(Direction::Download, 1));
    }

    #[test]
    fn tightest_window_wins() {
        let budget = TransferBudget::new(limits((Some(1_000), Some(1_500), None), (None, None, None)));
        let now = Instant::now();
        budget.record_at(Direction::Download, 900, now);
        // hour remaining 100, day remaining 600 -> 100
        assert_eq!(budget.available_at(Direction::Download, now), 100);
    }

    #[test]
    fn expired_samples_free_budget() {
        let budget = TransferBudget::new(limits((Some(1_000), None, None), (None, None, None)));
        let now = Instant::now();
        let earlier = now.checked_sub(Duration::from_secs(4_000)).expect("underflow");
        budget.record_at(Direction::Download, 1_000, earlier);
        assert_eq!(budget.available_at(Direction::Download, now), 1_000);
    }

    #[test]
    fn directions_are_accounted_separately() {
        let budget = TransferBudget::new(limits((Some(500), None, None), (Some(500), None, None)));
        let now = Instant::now();
        budget.record_at(Direction::Upload, 500, now);
        assert_eq!(budget.available_at(Direction::Upload, now), 0);
        assert_eq!(budget.available_at(Direction::Download, now), 500);
    }

    #[test]
    fn queue_deduplicates_and_respects_budget() {
        let namespace = NamespaceId::from([7_u8; 32]);
        let queue = TransferQueue::new();
        assert!(queue.enqueue(PendingTransfer::new(
            namespace,
            Hash::from_bytes([1; 32]),
            400,
            PathBuf::from("a"),
        )));
        assert!(!queue.enqueue(PendingTransfer::new(
            namespace,
            Hash::from_bytes([1; 32]),
            400,
            PathBuf::from("a"),
        )));
        assert!(queue.enqueue(PendingTransfer::new(
            namespace,
            Hash::from_bytes([2; 32]),
            900,
            PathBuf::from("b"),
        )));
        assert_eq!(queue.len(), 2);

        // A 500-byte budget pops the 400-byte head and leaves the 900-byte item.
        let tight = TransferBudget::new(limits((Some(500), None, None), (None, None, None)));
        let popped = queue.pop_fitting(&tight).expect("400-byte item fits");
        assert_eq!(popped.size, 400);
        assert_eq!(queue.len(), 1);
        assert!(queue.pop_fitting(&tight).is_none(), "900 bytes must not fit");
        assert_eq!(queue.pending_bytes(), 900);
    }

    #[test]
    fn config_parses_byte_suffixes() {
        let mut config = TransferLimitsConfig::default();
        assert!(config.set("transfer_limits.max_download_per_hour", "2G").unwrap());
        assert!(config.set("transfer_limits.max_upload_per_month", "500M").unwrap());
        assert!(!config.set("transfer_limits.unknown", "1").unwrap());
        let limits = TransferLimits::from_config(&config).unwrap();
        assert_eq!(limits.download.per_hour, Some(2_000_000_000));
        assert_eq!(limits.upload.per_month, Some(500_000_000));
        assert!(!limits.is_unlimited());
    }
}
