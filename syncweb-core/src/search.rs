//! Search and ranking APIs for local and synchronized entries.

use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use regex::Regex;

use crate::{
    error::{Result, SyncwebError},
    fs::{FileEntry, FileType, ParallelScanner},
};

/// How a find expression is interpreted.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
#[non_exhaustive]
pub enum MatchKind {
    Exact,
    #[default]
    Glob,
    Regex,
}

/// Filters supported by [`FindEngine`].
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct FindQuery {
    pub pattern: String,
    pub kind: MatchKind,
    pub max_depth: Option<usize>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub modified_after: Option<SystemTime>,
    pub modified_before: Option<SystemTime>,
    pub extension: Option<String>,
    pub file_type: Option<FileType>,
}

impl FindQuery {
    #[must_use]
    pub fn exact(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            kind: MatchKind::Exact,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn glob(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            kind: MatchKind::Glob,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn regex(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            kind: MatchKind::Regex,
            ..Self::default()
        }
    }

    #[must_use]
    pub const fn depth(mut self, max_depth: usize) -> Self {
        self.max_depth = Some(max_depth);
        self
    }

    #[must_use]
    pub const fn size(mut self, min: Option<u64>, max: Option<u64>) -> Self {
        self.min_size = min;
        self.max_size = max;
        self
    }

    #[must_use]
    pub fn extension(mut self, extension: impl Into<String>) -> Self {
        self.extension = Some(extension.into().trim_start_matches('.').to_owned());
        self
    }

    #[must_use]
    pub const fn file_type(mut self, file_type: FileType) -> Self {
        self.file_type = Some(file_type);
        self
    }

    #[must_use]
    pub const fn modified_after(mut self, time: SystemTime) -> Self {
        self.modified_after = Some(time);
        self
    }

    #[must_use]
    pub const fn modified_before(mut self, time: SystemTime) -> Self {
        self.modified_before = Some(time);
        self
    }
}

/// Recursive filesystem search engine.
#[derive(Clone)]
pub struct FindEngine {
    root: PathBuf,
    ignore_patterns: Vec<String>,
    threads: usize,
}

impl FindEngine {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            ignore_patterns: Vec::new(),
            threads: 0,
        }
    }

    #[must_use]
    pub fn with_ignore_patterns(mut self, patterns: Vec<String>) -> Self {
        self.ignore_patterns = patterns;
        self
    }

    #[must_use]
    pub const fn with_threads(mut self, threads: usize) -> Self {
        self.threads = threads;
        self
    }

    /// Scan the root and apply a query.
    /// # Errors
    ///
    /// Returns an error if the filesystem cannot be accessed.
    pub fn find(&self, query: &FindQuery) -> Result<Vec<FileEntry>> {
        let scanner = ParallelScanner::new(&self.root, self.ignore_patterns.clone(), self.threads);
        let entries = if query.file_type.is_some_and(|kind| kind != FileType::File) {
            scanner.scan_all()?
        } else {
            scanner.scan()?
        };
        if query.kind == MatchKind::Regex {
            Regex::new(&query.pattern)
                .map_err(|error| SyncwebError::operation("invalid find regular expression", error))?;
        }
        Ok(filter_entries(&entries, query))
    }

    /// Apply a query to already indexed metadata without reading file content.
    #[must_use]
    pub fn filter(&self, entries: &[FileEntry], query: &FindQuery) -> Vec<FileEntry> {
        filter_entries(entries, query)
    }
}

/// Filter an existing metadata index.
#[must_use]
pub fn filter_entries(entries: &[FileEntry], query: &FindQuery) -> Vec<FileEntry> {
    let regex = (query.kind == MatchKind::Regex)
        .then(|| Regex::new(&query.pattern).ok())
        .flatten();
    entries
        .iter()
        .filter(|entry| {
            if query
                .max_depth
                .is_some_and(|depth| entry.relative_path.components().count() > depth)
            {
                return false;
            }
            if query.min_size.is_some_and(|size| entry.size < size)
                || query.max_size.is_some_and(|size| entry.size > size)
            {
                return false;
            }
            if query.modified_after.is_some_and(|time| entry.modified <= time)
                || query.modified_before.is_some_and(|time| entry.modified >= time)
            {
                return false;
            }
            if query.extension.as_ref().is_some_and(|extension| {
                entry.path.extension().and_then(|value| value.to_str()) != Some(extension.as_str())
            }) {
                return false;
            }
            if query.file_type.is_some_and(|file_type| entry.file_type != file_type) {
                return false;
            }
            let relative = entry.relative_path.to_string_lossy();
            let name = entry.name().unwrap_or_default();
            match query.kind {
                MatchKind::Exact => name.contains(&query.pattern) || relative.contains(&query.pattern),
                MatchKind::Glob => glob_match(&query.pattern, &relative) || glob_match(&query.pattern, name),
                MatchKind::Regex => regex
                    .as_ref()
                    .is_some_and(|expression| expression.is_match(&relative) || expression.is_match(name)),
            }
        })
        .cloned()
        .collect()
}

fn glob_match(pattern: &str, value: &str) -> bool {
    globset::Glob::new(pattern).is_ok_and(|glob| glob.compile_matcher().is_match(value))
}

/// Kept as a small helper for callers that need to calculate depth.
#[must_use]
pub fn path_depth(path: &Path) -> usize {
    path.components().count()
}
