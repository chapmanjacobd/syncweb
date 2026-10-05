use std::collections::HashMap;
use std::path::{Path, PathBuf};

use iroh_blobs::Hash;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::folder::SyncwebFolder;
use crate::node::iroh_node::IrohNode;

/// Fetch the blobs selected by `strategy` directly from the folder's namespace
/// peers.
///
/// A metadata-only folder syncs entries while deliberately leaving blobs out of
/// the local store; a later doc re-sync does not re-download that content, so
/// `download` on such a folder must fetch each selected blob from its peers
/// directly. Returns the number of bytes fetched.
///
/// # Errors
///
/// Returns an error if the folder's entries cannot be listed.
pub async fn fetch_selected_content(node: &IrohNode, folder: &SyncwebFolder, strategy: &FetchStrategy) -> Result<u64> {
    let mut candidates = Vec::new();
    for entry in folder.content_entries().await? {
        let hash = entry.content_hash();
        let local = folder.has_local(hash).await?;
        let path = PathBuf::from(String::from_utf8_lossy(entry.key()).into_owned());
        candidates.push(FetchCandidate::new(path, hash, entry.content_len(), 0, local));
    }
    let selected = strategy.select(&candidates);
    let peers = node
        .topic_tracker()
        .find_peers(folder.namespace_id())
        .await
        .unwrap_or_default();
    let mut bytes = 0_u64;
    for candidate in selected {
        if candidate.local {
            continue;
        }
        for peer in &peers {
            if node
                .blob_store()
                .force_fetch_from_peer(node.endpoint(), peer, candidate.hash)
                .await
                .is_ok()
            {
                bytes = bytes.saturating_add(candidate.size);
                break;
            }
        }
    }
    Ok(bytes)
}

/// Write the blobs selected by `strategy` to files under `mount_root`, using
/// the folder-relative entry paths. Returns the number of files written.
///
/// This turns a `download` from a store-only fetch into files on disk.
///
/// # Errors
///
/// Returns an error if `mount_root` cannot be resolved against the current
/// directory, an entry directory cannot be created, or a blob cannot be
/// exported to its destination.
pub async fn materialize_selected_content(
    node: &IrohNode,
    folder: &SyncwebFolder,
    strategy: &FetchStrategy,
    mount_root: &Path,
) -> Result<usize> {
    let mut candidates = Vec::new();
    for entry in folder.content_entries().await? {
        let hash = entry.content_hash();
        let local = folder.has_local(hash).await?;
        let path = PathBuf::from(String::from_utf8_lossy(entry.key()).into_owned());
        candidates.push(FetchCandidate::new(path, hash, entry.content_len(), 0, local));
    }
    let selected = strategy.select(&candidates);
    let root = absolute_base(mount_root)?;
    let mut count = 0_usize;
    for candidate in selected {
        let dest = root.join(&candidate.path);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| crate::error::SyncwebError::operation("failed to create download directory", error))?;
        }
        node.blob_store().export_to_path(candidate.hash, &dest).await?;
        count = count.saturating_add(1);
    }
    Ok(count)
}

/// Resolve `base` to an absolute path without touching the filesystem.
///
/// Blob export rejects relative destinations, so a folder registered with a
/// relative mount path (the CLI default is a relative `--data-dir`) still needs
/// an absolute root to materialize into.
fn absolute_base(base: &Path) -> Result<PathBuf> {
    if base.is_absolute() {
        return Ok(base.to_path_buf());
    }
    let cwd = std::env::current_dir()
        .map_err(|error| crate::error::SyncwebError::operation("failed to resolve current directory", error))?;
    Ok(cwd.join(base))
}

/// Whether everything selected by `strategy` is already present locally.
///
/// A download of already-local content does not need a doc re-sync: fetching
/// blobs the local node already holds can fail on the publishing node (there is
/// no remote peer to fetch from), so the caller can skip straight to
/// materialization. Returns `false` when the strategy selects nothing or when
/// any selected blob is not yet local.
///
/// # Errors
///
/// Returns an error if the folder's entries cannot be listed.
pub async fn strategy_selects_only_local(folder: &SyncwebFolder, strategy: &FetchStrategy) -> Result<bool> {
    let mut candidates = Vec::new();
    for entry in folder.content_entries().await? {
        let hash = entry.content_hash();
        let local = folder.has_local(hash).await?;
        let path = PathBuf::from(String::from_utf8_lossy(entry.key()).into_owned());
        candidates.push(FetchCandidate::new(path, hash, entry.content_len(), 0, local));
    }
    let selected = strategy.select(&candidates);
    Ok(!selected.is_empty() && selected.iter().all(|candidate| candidate.local))
}

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
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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

    /// Build a report with verified peer counts from a `hash -> verified_count` map.
    ///
    /// Each candidate's `peer_count` is overridden with the value from
    /// `peers_per_hash` when present. This lets the health display reflect live
    /// provider-lease data instead of always showing zero.
    #[must_use]
    pub fn from_candidates_with_peers_per_hash(
        candidates: &[FetchCandidate],
        peers_per_hash: &HashMap<Hash, usize>,
        well_seeded_threshold: usize,
    ) -> Self {
        let enriched: Vec<FetchCandidate> = candidates
            .iter()
            .map(|c| {
                let peers = peers_per_hash.get(&c.hash).copied().unwrap_or(c.peer_count);
                FetchCandidate::new(&c.path, c.hash, c.size, peers, c.local)
            })
            .collect();
        Self::from_candidates(&enriched, well_seeded_threshold)
    }
}

#[cfg(test)]
mod absolute_base_tests {
    use super::*;

    #[test]
    fn absolute_paths_are_returned_unchanged() {
        assert_eq!(
            absolute_base(Path::new("/srv/folder")).unwrap(),
            PathBuf::from("/srv/folder")
        );
    }

    #[test]
    fn relative_paths_are_anchored_to_the_current_directory() {
        let base = Path::new("relative/folder");
        let resolved = absolute_base(base).unwrap();
        assert!(
            resolved.is_absolute(),
            "blob export rejects relative roots: {resolved:?}"
        );
        assert!(resolved.ends_with(base), "{resolved:?} should end with {base:?}");
    }

    #[test]
    fn dot_resolves_to_the_current_directory() {
        let resolved = absolute_base(Path::new(".")).unwrap();
        assert!(resolved.is_absolute());
    }
}
