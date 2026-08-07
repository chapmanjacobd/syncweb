use std::path::{Component, Path, PathBuf};

use iroh_blobs::Hash;
use iroh_docs::NamespaceId;
use serde::{Deserialize, Serialize};

use crate::{Result, SyncwebError};

/// A configured filesystem root that can receive materialized content.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct StorageRoot {
    pub id: String,
    pub path: PathBuf,
    pub min_free: u64,
    pub enabled: bool,
}

impl StorageRoot {
    /// Create an enabled storage root.
    #[must_use]
    pub fn new(id: impl Into<String>, path: impl Into<PathBuf>, min_free: u64) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
            min_free,
            enabled: true,
        }
    }

    /// Set whether this root participates in allocation.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Capacity information for a storage root.
///
/// `available` is the free capacity remaining after existing reservations.
/// A root's `min_free` is applied separately by [`allocate`].
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct RootCapacity {
    pub total: u64,
    pub free: u64,
    pub reserved: u64,
    pub available: u64,
}

impl RootCapacity {
    /// Build capacity information from filesystem totals and existing reservations.
    #[must_use]
    pub const fn new(total: u64, free: u64, reserved: u64) -> Self {
        Self {
            total,
            free,
            reserved,
            available: free.saturating_sub(reserved),
        }
    }

    /// Return the capacity that may be allocated while preserving `min_free`.
    #[must_use]
    pub const fn allocatable(self, min_free: u64) -> u64 {
        self.available.saturating_sub(min_free)
    }
}

/// A content item that may be materialized under a storage root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct AllocationCandidate {
    pub namespace: NamespaceId,
    pub path: PathBuf,
    pub hash: Hash,
    pub size: u64,
    pub peer_count: usize,
    pub local: bool,
}

impl AllocationCandidate {
    /// Create an allocation candidate.
    #[must_use]
    pub fn new(
        namespace: NamespaceId,
        path: impl Into<PathBuf>,
        hash: Hash,
        size: u64,
        peer_count: usize,
        local: bool,
    ) -> Self {
        Self {
            namespace,
            path: path.into(),
            hash,
            size,
            peer_count,
            local,
        }
    }
}

/// The root and destination selected for a candidate.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct AllocationDecision {
    pub candidate: AllocationCandidate,
    pub root_id: String,
    pub destination: PathBuf,
}

/// Construct the safe materialization path for a candidate.
///
/// The namespace is always one path component, followed by the candidate's
/// relative path. Absolute paths and parent-directory traversal are rejected.
///
/// # Errors
///
/// Returns an error if the candidate path is empty, absolute, or traverses a
/// parent directory.
pub fn materialization_path(root: &StorageRoot, candidate: &AllocationCandidate) -> Result<PathBuf> {
    validate_relative_path(&candidate.path)?;
    Ok(root.path.join(candidate.namespace.to_string()).join(&candidate.path))
}

/// Allocate candidates to roots using deterministic first-fit packing.
///
/// Candidates are considered by increasing peer count, then path, then hash.
/// Roots are considered in the order supplied. A candidate is allocated at
/// most once, and each allocation reserves its size for subsequent
/// candidates on that root. Disabled roots, roots below their minimum free
/// threshold, and roots without sufficient remaining capacity are skipped.
///
/// Invalid candidate paths are returned as unallocated candidates.
#[must_use]
pub fn allocate(
    roots: &[(StorageRoot, RootCapacity)],
    candidates: &[AllocationCandidate],
) -> (Vec<AllocationDecision>, Vec<AllocationCandidate>) {
    let mut ordered = candidates.to_vec();
    ordered.sort_by(|left, right| {
        left.peer_count
            .cmp(&right.peer_count)
            .then(left.path.cmp(&right.path))
            .then(left.hash.cmp(&right.hash))
            .then(left.namespace.to_string().cmp(&right.namespace.to_string()))
    });

    let mut reserved = vec![0_u64; roots.len()];
    let mut decisions = Vec::new();
    let mut unallocated = Vec::new();

    for candidate in ordered {
        if validate_relative_path(&candidate.path).is_err() {
            unallocated.push(candidate);
            continue;
        }

        let mut decision = None;
        for ((root, capacity), reserved_for_root) in roots.iter().zip(reserved.iter_mut()) {
            if !root.enabled {
                continue;
            }

            let remaining = capacity.available.saturating_sub(*reserved_for_root);
            if candidate.size > remaining.saturating_sub(root.min_free) {
                continue;
            }

            let Ok(destination) = materialization_path(root, &candidate) else {
                continue;
            };
            *reserved_for_root = reserved_for_root.saturating_add(candidate.size);
            decision = Some(AllocationDecision {
                candidate: candidate.clone(),
                root_id: root.id.clone(),
                destination,
            });
            break;
        }

        if let Some(selected) = decision {
            decisions.push(selected);
        } else {
            unallocated.push(candidate);
        }
    }

    (decisions, unallocated)
}

fn validate_relative_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(SyncwebError::InvalidConfig(format!(
            "materialization path must be a non-empty relative path: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iroh_blobs::Hash;

    use super::{AllocationCandidate, RootCapacity, StorageRoot, allocate, materialization_path};

    fn namespace(value: u8) -> iroh_docs::NamespaceId {
        iroh_docs::NamespaceId::from([value; 32])
    }

    fn candidate(name: &str, size: u64, peers: usize, hash_byte: u8) -> AllocationCandidate {
        AllocationCandidate::new(
            namespace(hash_byte),
            PathBuf::from(name),
            Hash::from([hash_byte; 32]),
            size,
            peers,
            false,
        )
    }

    fn root(id: &str, capacity: u64) -> (StorageRoot, RootCapacity) {
        (
            StorageRoot::new(id, format!("/storage/{id}"), 0),
            RootCapacity::new(capacity, capacity, 0),
        )
    }

    #[test]
    fn packs_candidates_across_roots() {
        let roots = vec![root("one", 10), root("two", 5)];
        let candidates = vec![
            candidate("large", 6, 0, 1),
            candidate("small", 4, 1, 2),
            candidate("other", 5, 2, 3),
        ];

        let (decisions, unallocated) = allocate(&roots, &candidates);

        assert_eq!(decisions.len(), 3);
        assert!(unallocated.is_empty());
        let assigned_roots = decisions
            .iter()
            .map(|decision| decision.root_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(assigned_roots, vec!["one", "one", "two"]);
    }

    #[test]
    fn honors_min_free_and_available_capacity() {
        let roots = vec![
            (
                StorageRoot::new("protected", "/protected", 4),
                RootCapacity::new(20, 10, 2),
            ),
            root("available", 5),
        ];
        let candidates = vec![candidate("five", 5, 0, 1), candidate("six", 6, 1, 2)];

        let (decisions, unallocated) = allocate(&roots, &candidates);

        assert_eq!(decisions.len(), 1);
        assert_eq!(
            decisions.first().map(|decision| decision.root_id.as_str()),
            Some("available")
        );
        assert_eq!(unallocated.len(), 1);
        assert_eq!(
            unallocated.first().map(|candidate| candidate.path.clone()),
            Some(PathBuf::from("six"))
        );
    }

    #[test]
    fn orders_by_peers_path_and_hash() {
        let roots = vec![root("one", 30)];
        let candidates = vec![
            candidate("z", 1, 2, 1),
            candidate("b", 1, 1, 3),
            candidate("a", 1, 1, 2),
            candidate("same", 1, 1, 5),
            candidate("same", 1, 1, 4),
        ];

        let (decisions, _) = allocate(&roots, &candidates);

        let paths = decisions
            .iter()
            .map(|decision| decision.candidate.path.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec![
                PathBuf::from("a"),
                PathBuf::from("b"),
                PathBuf::from("same"),
                PathBuf::from("same"),
                PathBuf::from("z")
            ]
        );
        assert_eq!(
            decisions.get(2).map(|decision| decision.candidate.hash),
            Some(Hash::from([4; 32]))
        );
    }

    #[test]
    fn skips_disabled_roots() {
        let roots = vec![
            (
                StorageRoot::new("disabled", "/disabled", 0).with_enabled(false),
                RootCapacity::new(100, 100, 0),
            ),
            root("enabled", 1),
        ];

        let (decisions, unallocated) = allocate(&roots, &[candidate("file", 1, 0, 1)]);

        assert!(unallocated.is_empty());
        assert_eq!(
            decisions.first().map(|decision| decision.root_id.as_str()),
            Some("enabled")
        );
    }

    #[test]
    fn rejects_unsafe_materialization_paths() {
        let root = StorageRoot::new("one", "/storage", 0);
        let absolute = AllocationCandidate::new(
            namespace(1),
            PathBuf::from("/outside"),
            Hash::from([1; 32]),
            1,
            0,
            false,
        );
        let traversal = AllocationCandidate::new(
            namespace(1),
            PathBuf::from("../outside"),
            Hash::from([2; 32]),
            1,
            0,
            false,
        );

        assert!(materialization_path(&root, &absolute).is_err());
        assert!(materialization_path(&root, &traversal).is_err());
        let (_, unallocated) = allocate(&[(root, RootCapacity::new(10, 10, 0))], &[absolute, traversal]);
        assert_eq!(unallocated.len(), 2);
    }
}
