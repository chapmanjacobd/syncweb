use std::collections::HashMap;

use rand::seq::SliceRandom;

/// Ranking strategy for synchronized entries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SortCriterion {
    Niche,
    Frecency,
    Peers,
    Random,
    FolderAggregate,
}

/// Metadata consumed by [`Sorter`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SortEntry {
    pub path: std::path::PathBuf,
    pub folder: String,
    pub niche: f64,
    pub frequency: u64,
    pub last_accessed: std::time::SystemTime,
    pub peers: usize,
}

impl SortEntry {
    #[must_use]
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            path: path.into(),
            folder: String::new(),
            niche: 0.0,
            frequency: 0,
            last_accessed: std::time::SystemTime::UNIX_EPOCH,
            peers: 0,
        }
    }

    #[must_use]
    pub const fn with_last_accessed(mut self, last_accessed: std::time::SystemTime) -> Self {
        self.last_accessed = last_accessed;
        self
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
}

/// Sort entries using one of the supported content discovery rankings.
#[derive(Clone, Copy, Debug)]
pub struct Sorter {
    criterion: SortCriterion,
}

impl Sorter {
    #[must_use]
    pub const fn new(criterion: SortCriterion) -> Self {
        Self { criterion }
    }

    #[must_use]
    pub const fn criterion(self) -> SortCriterion {
        self.criterion
    }

    pub fn sort(&self, entries: &mut [SortEntry]) {
        match self.criterion {
            SortCriterion::Niche => entries.sort_by(|left, right| right.niche.total_cmp(&left.niche)),
            SortCriterion::Frecency => entries.sort_by(|left, right| frecency(right).total_cmp(&frecency(left))),
            SortCriterion::Peers => entries.sort_by_key(|entry| std::cmp::Reverse(entry.peers)),
            SortCriterion::Random => {
                let mut rng = rand::rng();
                entries.shuffle(&mut rng);
            }
            SortCriterion::FolderAggregate => {
                let mut counts = HashMap::<String, usize>::new();
                for entry in entries.iter() {
                    let count = counts.entry(entry.folder.clone()).or_default();
                    *count = count.saturating_add(1);
                }
                entries.sort_by_key(|entry| std::cmp::Reverse(counts.get(entry.folder.as_str()).copied().unwrap_or(0)));
            }
        }
    }

    #[must_use]
    pub fn sorted(&self, mut entries: Vec<SortEntry>) -> Vec<SortEntry> {
        self.sort(&mut entries);
        entries
    }
}

fn frecency(entry: &SortEntry) -> f64 {
    use std::ops::{Add, Div};
    let age = entry
        .last_accessed
        .elapsed()
        .map_or(0.0, |duration| duration.as_secs_f64());
    let freq = u32::try_from(entry.frequency).unwrap_or(u32::MAX);
    f64::from(freq).div(1.0.add(age.div(86_400.0)))
}
