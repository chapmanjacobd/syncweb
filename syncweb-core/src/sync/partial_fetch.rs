use std::path::PathBuf;

use iroh_blobs::Hash;

/// A document blob considered for a partial fetch.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct FetchCandidate {
    pub path: PathBuf,
    pub hash: Hash,
    pub size: u64,
    pub peer_count: usize,
    pub local: bool,
}

impl FetchCandidate {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, hash: Hash, size: u64, peer_count: usize, local: bool) -> Self {
        Self {
            path: path.into(),
            hash,
            size,
            peer_count,
            local,
        }
    }
}

/// Constraints for selecting a subset of folder content.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct FetchFilter {
    pub paths: Option<Vec<PathBuf>>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub min_peers: Option<usize>,
    pub max_peers: Option<usize>,
    pub min_count: Option<usize>,
    pub max_count: Option<usize>,
}

impl FetchFilter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            paths: None,
            min_size: None,
            max_size: None,
            min_peers: None,
            max_peers: None,
            min_count: None,
            max_count: None,
        }
    }

    #[must_use]
    pub fn with_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.paths = Some(paths);
        self
    }

    #[must_use]
    pub const fn with_min_size(mut self, size: u64) -> Self {
        self.min_size = Some(size);
        self
    }

    #[must_use]
    pub const fn with_max_size(mut self, size: u64) -> Self {
        self.max_size = Some(size);
        self
    }

    #[must_use]
    pub const fn with_min_peers(mut self, peers: usize) -> Self {
        self.min_peers = Some(peers);
        self
    }

    #[must_use]
    pub const fn with_max_peers(mut self, peers: usize) -> Self {
        self.max_peers = Some(peers);
        self
    }

    #[must_use]
    pub const fn with_min_count(mut self, count: usize) -> Self {
        self.min_count = Some(count);
        self
    }

    #[must_use]
    pub const fn with_max_count(mut self, count: usize) -> Self {
        self.max_count = Some(count);
        self
    }

    /// Return candidates satisfying this filter.
    ///
    /// Candidates with fewer peers are selected first so a capped fetch
    /// improves folder seeding rather than repeatedly selecting common blobs.
    #[must_use]
    pub fn select(&self, candidates: &[FetchCandidate]) -> Vec<FetchCandidate> {
        let mut selected = candidates
            .iter()
            .filter(|candidate| self.matches(candidate))
            .cloned()
            .collect::<Vec<_>>();
        selected.sort_by(|left, right| {
            left.peer_count
                .cmp(&right.peer_count)
                .then(left.path.cmp(&right.path))
                .then(left.hash.cmp(&right.hash))
        });
        if let Some(max_count) = self.max_count {
            selected.truncate(max_count);
        }
        selected
    }

    #[must_use]
    pub fn matches(&self, candidate: &FetchCandidate) -> bool {
        self.paths.as_ref().is_none_or(|paths| {
            paths
                .iter()
                .any(|path| candidate.path == *path || candidate.path.starts_with(path))
        }) && self.min_size.is_none_or(|size| candidate.size >= size)
            && self.max_size.is_none_or(|size| candidate.size <= size)
            && self.min_peers.is_none_or(|peers| candidate.peer_count >= peers)
            && self.max_peers.is_none_or(|peers| candidate.peer_count <= peers)
    }
}

/// Controls whether a fetch reconciles all content or only matching content.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum FetchStrategy {
    #[default]
    All,
    Filter(FetchFilter),
}

impl FetchStrategy {
    #[must_use]
    pub const fn filter(filter: FetchFilter) -> Self {
        Self::Filter(filter)
    }

    #[must_use]
    pub fn select(&self, candidates: &[FetchCandidate]) -> Vec<FetchCandidate> {
        match self {
            Self::All => candidates.to_vec(),
            Self::Filter(filter) => filter.select(candidates),
        }
    }
}

/// Per-blob availability information used by the health command.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BlobHealth {
    pub path: PathBuf,
    pub hash: Hash,
    pub size: u64,
    pub peer_count: usize,
    pub local: bool,
}

/// Aggregate availability counters for a folder.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct HealthReport {
    pub total: usize,
    pub well_seeded: usize,
    pub under_seeded: usize,
    pub unseeded: usize,
    pub least_seeded: Vec<BlobHealth>,
}

impl HealthReport {
    /// Build a report using `well_seeded_threshold` as the minimum healthy peer count.
    #[must_use]
    pub fn from_candidates(candidates: &[FetchCandidate], well_seeded_threshold: usize) -> Self {
        let mut least_seeded = candidates
            .iter()
            .map(|candidate| BlobHealth {
                path: candidate.path.clone(),
                hash: candidate.hash,
                size: candidate.size,
                peer_count: candidate.peer_count,
                local: candidate.local,
            })
            .collect::<Vec<_>>();
        least_seeded.sort_by(|left, right| left.peer_count.cmp(&right.peer_count).then(left.path.cmp(&right.path)));
        let total = candidates.len();
        let well_seeded = candidates
            .iter()
            .filter(|candidate| candidate.peer_count >= well_seeded_threshold)
            .count();
        let unseeded = candidates.iter().filter(|candidate| candidate.peer_count == 0).count();
        Self {
            total,
            well_seeded,
            under_seeded: total.saturating_sub(well_seeded).saturating_sub(unseeded),
            unseeded,
            least_seeded,
        }
    }
}
