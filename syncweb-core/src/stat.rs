use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use iroh_blobs::Hash;

use crate::fs::FileEntry;

/// Detailed metadata for a local or synchronized file.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct StatOutput {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub hash: Option<Hash>,
    pub available: bool,
    pub peers: usize,
    pub version_vector: BTreeMap<String, u64>,
}

/// Output format for [`StatOutput`].
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StatFormat {
    Human,
    Terse,
    Custom(String),
}

impl StatOutput {
    #[must_use]
    pub fn from_entry(entry: &FileEntry) -> Self {
        Self {
            path: entry.path.clone(),
            size: entry.size,
            modified: entry.modified,
            hash: Some(entry.hash.into()),
            available: true,
            peers: 0,
            version_vector: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn from_blob(entry: &FileEntry, hash: Hash, available: bool, peers: usize) -> Self {
        Self {
            hash: Some(hash),
            available,
            peers,
            ..Self::from_entry(entry)
        }
    }

    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.available
    }

    #[must_use]
    pub const fn blocks(&self) -> u64 {
        self.size.saturating_add(511) >> 9
    }

    #[must_use]
    pub fn display(&self, format: StatFormat) -> String {
        match format {
            StatFormat::Human => format!(
                "Path: {}\nSize: {}\nBlocks: {}\nHash: {}\nAvailable: {}\nPeers: {}",
                self.path.display(),
                self.size,
                self.blocks(),
                self.hash.map_or_else(|| "unavailable".to_owned(), |hash| hash.to_hex()),
                self.available,
                self.peers
            ),
            StatFormat::Terse => format!(
                "{}|{}|{}|{}|{}",
                self.path.display(),
                self.size,
                self.blocks(),
                self.hash.map_or_else(|| "-".to_owned(), |hash| hash.to_hex()),
                self.peers
            ),
            StatFormat::Custom(template) => template
                .replace("%n", &self.path.display().to_string())
                .replace("%s", &self.size.to_string())
                .replace("%b", &self.blocks().to_string())
                .replace("%h", &self.hash.map_or_else(|| "-".to_owned(), |hash| hash.to_hex()))
                .replace("%y", &self.modified_seconds())
                .replace("%a", &self.available.to_string())
                .replace("%p", &self.peers.to_string()),
        }
    }

    fn modified_seconds(&self) -> String {
        self.modified
            .duration_since(UNIX_EPOCH)
            .map_or_else(|_| "0".to_owned(), |duration| duration.as_secs().to_string())
    }
}
