use std::path::{Path, PathBuf};

use iroh_blobs::Hash;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Restricts subscription events to a portion of a folder.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum AreaFilter {
    All,
    Prefix(PathBuf),
    Glob(String),
    HashRange([u8; 32], [u8; 32]),
}

impl AreaFilter {
    #[must_use]
    pub fn matches_path(&self, path: &Path) -> bool {
        match self {
            Self::Prefix(prefix) => path.starts_with(prefix),
            Self::Glob(pattern) => globset::Glob::new(pattern).is_ok_and(|glob| glob.compile_matcher().is_match(path)),
            Self::All | Self::HashRange(_, _) => true,
        }
    }

    #[must_use]
    pub fn matches_hash(&self, hash: &Hash) -> bool {
        match self {
            Self::All | Self::Prefix(_) | Self::Glob(_) => true,
            Self::HashRange(start, end) => {
                let bytes = hash.as_bytes();
                bytes >= start && bytes < end
            }
        }
    }

    #[must_use]
    pub fn matches_entry(&self, path: &Path, hash: &Hash) -> bool {
        self.matches_path(path) && self.matches_hash(hash)
    }
}

/// Controls which changes are delivered to a folder subscriber.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SubscribeParams {
    pub ingest_only: bool,
    pub ignore_session: Option<Uuid>,
    pub area_filter: Option<AreaFilter>,
    pub area_of_interest: Option<AreaOfInterest>,
}

impl SubscribeParams {
    #[must_use]
    pub const fn ingest_only() -> Self {
        Self {
            ingest_only: true,
            ignore_session: None,
            area_filter: None,
            area_of_interest: None,
        }
    }

    #[must_use]
    pub const fn ignore_session(session_id: Uuid) -> Self {
        Self {
            ingest_only: false,
            ignore_session: Some(session_id),
            area_filter: None,
            area_of_interest: None,
        }
    }

    #[must_use]
    pub fn with_area(mut self, area_filter: AreaFilter) -> Self {
        self.area_filter = Some(area_filter);
        self
    }

    #[must_use]
    pub fn with_limits(mut self, area: AreaOfInterest) -> Self {
        self.area_of_interest = Some(area);
        self
    }

    #[must_use]
    pub fn accepts(&self, path: &Path, hash: &Hash) -> bool {
        let area_matches = self
            .area_of_interest
            .as_ref()
            .is_none_or(|area| area.area.matches_entry(path, hash));
        let filter_matches = self
            .area_filter
            .as_ref()
            .is_none_or(|filter| filter.matches_entry(path, hash));
        area_matches && filter_matches
    }
}

/// A subscription area with aggregate entry and byte limits.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct AreaOfInterest {
    pub area: AreaFilter,
    pub max_size: u64,
    pub max_count: u64,
}

impl AreaOfInterest {
    #[must_use]
    pub const fn unlimited(area: AreaFilter) -> Self {
        Self {
            area,
            max_size: 0,
            max_count: 0,
        }
    }

    #[must_use]
    pub const fn with_count_limit(area: AreaFilter, max_count: u64) -> Self {
        Self {
            area,
            max_size: 0,
            max_count,
        }
    }

    #[must_use]
    pub const fn with_size_limit(area: AreaFilter, max_size: u64) -> Self {
        Self {
            area,
            max_size,
            max_count: 0,
        }
    }

    #[must_use]
    pub const fn with_limits(area: AreaFilter, max_size: u64, max_count: u64) -> Self {
        Self {
            area,
            max_size,
            max_count,
        }
    }

    #[must_use]
    pub const fn is_limit_reached(&self, synced_count: u64, synced_size: u64) -> bool {
        (self.max_count > 0 && synced_count >= self.max_count) || (self.max_size > 0 && synced_size >= self.max_size)
    }

    #[must_use]
    pub const fn permits(&self, synced_count: u64, synced_size: u64, next_size: u64) -> bool {
        let next_count = synced_count.saturating_add(1);
        let next_total_size = synced_size.saturating_add(next_size);
        (self.max_count == 0 || next_count <= self.max_count)
            && (self.max_size == 0 || next_total_size <= self.max_size)
    }
}
