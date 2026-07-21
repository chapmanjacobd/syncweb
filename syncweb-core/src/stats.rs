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

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn temporary_path(path: &Path) -> PathBuf {
    path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()))
}
