use std::{collections::HashMap, path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Metadata retained for a deleted entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct DeletedInfo {
    pub path: PathBuf,
    pub size: u64,
    pub deleted_by: Uuid,
    pub deleted_at: SystemTime,
}

/// A batch of entries pruned by one synchronization session.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct PruneEvent {
    pub pruned: Vec<blake3::Hash>,
    pub by: Uuid,
}

impl PruneEvent {
    #[must_use]
    pub const fn new(pruned: Vec<blake3::Hash>, by: Uuid) -> Self {
        Self { pruned, by }
    }
}

/// Tracks files that were observed locally and later deleted.
#[derive(Clone, Debug, Default)]
pub struct DeletedTracker {
    deleted: HashMap<blake3::Hash, DeletedInfo>,
}

impl DeletedTracker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_deletion(&mut self, hash: blake3::Hash, path: impl Into<PathBuf>, size: u64, session: Uuid) {
        self.deleted.insert(
            hash,
            DeletedInfo {
                path: path.into(),
                size,
                deleted_by: session,
                deleted_at: SystemTime::now(),
            },
        );
    }

    #[must_use]
    pub fn is_deleted(&self, hash: &blake3::Hash) -> bool {
        self.deleted.contains_key(hash)
    }

    #[must_use]
    pub fn deletion_info(&self, hash: &blake3::Hash) -> Option<&DeletedInfo> {
        self.deleted.get(hash)
    }

    pub fn restore(&mut self, hash: &blake3::Hash) -> Option<DeletedInfo> {
        self.deleted.remove(hash)
    }

    /// Restore metadata for an entry under the name used by the CLI.
    pub fn undelete(&mut self, hash: &blake3::Hash) -> Option<DeletedInfo> {
        self.restore(hash)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&blake3::Hash, &DeletedInfo)> {
        self.deleted.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.deleted.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.deleted.is_empty()
    }
}
