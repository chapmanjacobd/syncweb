//! Search and ranking APIs for local and synchronized entries.

use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use regex::Regex;

use crate::{
    error::{Result, SyncwebError},
    fs::{FileEntry, FileType, ParallelScanner},
    parsing::{parse_relative_time, parse_size_constraint},
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
    pub min_depth: Option<usize>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub modified_after: Option<SystemTime>,
    pub modified_before: Option<SystemTime>,
    pub extension: Option<String>,
    pub extensions: Vec<String>,
    pub file_type: Option<FileType>,
    pub case_sensitive: Option<bool>,
    pub fixed_strings: bool,
    pub full_path: bool,
    pub hidden: bool,
    pub follow_links: bool,
    pub absolute_path: bool,
    pub downloadable: bool,
    pub sync_mode: Option<String>,
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
    pub fn extensions(mut self, exts: Vec<String>) -> Self {
        self.extensions = exts
            .into_iter()
            .map(|e| e.trim_start_matches('.').to_lowercase())
            .collect();
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

    /// Parse size constraints from fd-find style strings.
    ///
    /// # Errors
    ///
    /// Returns an error if the string cannot be parsed.
    pub fn parse_size_constraints(size_strs: &[String]) -> Result<(Option<u64>, Option<u64>)> {
        let mut min_size: Option<u64> = None;
        let mut max_size: Option<u64> = None;

        for s in size_strs {
            let (min, max) = parse_size_constraint(s)?;
            if let Some(new_min) = min {
                min_size = Some(min_size.map_or(new_min, |current| current.max(new_min)));
            }
            if let Some(new_max) = max {
                max_size = Some(max_size.map_or(new_max, |current| current.min(new_max)));
            }
        }

        Ok((min_size, max_size))
    }

    /// Parse time constraints from human-readable strings.
    ///
    /// # Errors
    ///
    /// Returns an error if the string cannot be parsed.
    pub fn parse_time_constraints(
        within_strs: &[String],
        before_strs: &[String],
        modified_strs: &[String],
    ) -> Result<(Option<SystemTime>, Option<SystemTime>)> {
        let mut after: Option<SystemTime> = None;
        let mut before: Option<SystemTime> = None;

        // "3 days" means after = now - 3 days
        for s in within_strs {
            let time = parse_relative_time(&format!("-{s}"))?;
            after = Some(after.map_or(time, |current| if time > current { time } else { current }));
        }

        // "3 years" means before = now - 3 years (older than)
        for s in before_strs {
            let time = parse_relative_time(&format!("-{s}"))?;
            before = Some(before.map_or(time, |current| if time < current { time } else { current }));
        }

        // "-3 days" means after = now - 3 days (newer than)
        // "+3 days" means before = now - 3 days (older than)
        for s in modified_strs {
            let time = parse_relative_time(s)?;
            let now = SystemTime::now();
            if time < now {
                // Past time
                after = Some(after.map_or(time, |current| if time > current { time } else { current }));
            } else {
                // Future time (shouldn't happen, but handle gracefully)
                before = Some(before.map_or(time, |current| if time < current { time } else { current }));
            }
        }

        Ok((after, before))
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
            // Depth filtering
            if let Some(max_depth) = query.max_depth
                && entry.relative_path.components().count() > max_depth
            {
                return false;
            }
            if let Some(min_depth) = query.min_depth
                && entry.relative_path.components().count() < min_depth
            {
                return false;
            }

            // Size filtering
            if query.min_size.is_some_and(|size| entry.size < size)
                || query.max_size.is_some_and(|size| entry.size > size)
            {
                return false;
            }

            // Time filtering
            if query.modified_after.is_some_and(|time| entry.modified <= time)
                || query.modified_before.is_some_and(|time| entry.modified >= time)
            {
                return false;
            }

            // Single extension filtering
            if query.extension.as_ref().is_some_and(|extension| {
                entry.path.extension().and_then(|value| value.to_str()) != Some(extension.as_str())
            }) {
                return false;
            }

            // Multiple extensions filtering (OR logic)
            if !query.extensions.is_empty() {
                let has_valid_extension = entry
                    .path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|ext| query.extensions.iter().any(|e| e == ext));
                if !has_valid_extension {
                    return false;
                }
            }

            // File type filtering
            if query.file_type.is_some_and(|file_type| entry.file_type != file_type) {
                return false;
            }

            // Hidden files filtering
            if !query.hidden
                && let Some(name) = entry.name()
                && name.starts_with('.')
            {
                return false;
            }

            // Downloadable filtering (exclude PublicReadOnly or SendOnly)
            if query.downloadable
                && let Some(ref mode) = query.sync_mode
                && (mode == "publicreadonly" || mode == "sendonly")
            {
                return false;
            }

            // Pattern matching
            if query.pattern.is_empty() {
                return true;
            }

            let relative = entry.relative_path.to_string_lossy();
            let name = entry.name().unwrap_or_default();
            let search_target = if query.full_path { relative.as_ref() } else { name };

            match query.kind {
                MatchKind::Exact => {
                    if query.fixed_strings {
                        search_target.contains(&query.pattern)
                    } else {
                        // Case-insensitive exact match
                        let pattern = if query.case_sensitive.unwrap_or(false) {
                            query.pattern.clone()
                        } else {
                            query.pattern.to_lowercase()
                        };
                        let target = if query.case_sensitive.unwrap_or(false) {
                            search_target.to_string()
                        } else {
                            search_target.to_lowercase()
                        };
                        target.contains(&pattern)
                    }
                }
                MatchKind::Glob => {
                    let ignore_case = query
                        .case_sensitive
                        .map_or_else(|| query.pattern.to_lowercase() == query.pattern, |cs| !cs);
                    glob_match_case(&query.pattern, search_target, ignore_case)
                }
                MatchKind::Regex => {
                    let ignore_case = query
                        .case_sensitive
                        .map_or_else(|| query.pattern.to_lowercase() == query.pattern, |cs| !cs);
                    regex.as_ref().is_some_and(|re| {
                        if ignore_case {
                            re.is_match(search_target)
                        } else {
                            re.captures(search_target).is_some()
                        }
                    })
                }
            }
        })
        .cloned()
        .collect()
}

fn glob_match(pattern: &str, value: &str) -> bool {
    globset::Glob::new(pattern).is_ok_and(|glob| glob.compile_matcher().is_match(value))
}

fn glob_match_case(pattern: &str, value: &str, ignore_case: bool) -> bool {
    if ignore_case {
        let pattern_lower = pattern.to_lowercase();
        let value_lower = value.to_lowercase();
        glob_match(&pattern_lower, &value_lower)
    } else {
        glob_match(pattern, value)
    }
}

/// Kept as a small helper for callers that need to calculate depth.
#[must_use]
pub fn path_depth(path: &Path) -> usize {
    path.components().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    #[test]
    fn test_parse_size_constraints() {
        let empty: Vec<String> = vec![];
        let (min1, max1) = FindQuery::parse_size_constraints(&empty).unwrap();
        assert_eq!(min1, None);
        assert_eq!(max1, None);

        let (min2, max2) = FindQuery::parse_size_constraints(&["5GB".to_string()]).unwrap();
        assert_eq!(min2, Some(5_000_000_000));
        assert_eq!(max2, Some(5_000_000_000));

        let (min3, max3) = FindQuery::parse_size_constraints(&["-5GB".to_string()]).unwrap();
        assert_eq!(min3, None);
        assert_eq!(max3, Some(4_999_999_999));
    }

    #[test]
    fn test_filter_entries() {
        let entries = vec![
            FileEntry {
                path: PathBuf::from("/test/file1.txt"),
                relative_path: PathBuf::from("file1.txt"),
                size: 100,
                modified: UNIX_EPOCH,
                hash: blake3::hash(b""),
                file_type: FileType::File,
            },
            FileEntry {
                path: PathBuf::from("/test/.hidden"),
                relative_path: PathBuf::from(".hidden"),
                size: 50,
                modified: UNIX_EPOCH,
                hash: blake3::hash(b""),
                file_type: FileType::File,
            },
            FileEntry {
                path: PathBuf::from("/test/subdir/file2.txt"),
                relative_path: PathBuf::from("subdir/file2.txt"),
                size: 200,
                modified: UNIX_EPOCH,
                hash: blake3::hash(b""),
                file_type: FileType::File,
            },
        ];

        // Test hidden files filtering
        let query1 = FindQuery {
            hidden: false,
            ..FindQuery::default()
        };
        let filtered1 = filter_entries(&entries, &query1);
        assert_eq!(filtered1.len(), 2);

        // Test hidden files shown
        let query2 = FindQuery {
            hidden: true,
            ..FindQuery::default()
        };
        let filtered2 = filter_entries(&entries, &query2);
        assert_eq!(filtered2.len(), 3);

        // Test depth filtering
        let query3 = FindQuery {
            max_depth: Some(1),
            hidden: true,
            ..FindQuery::default()
        };
        let filtered3 = filter_entries(&entries, &query3);
        assert_eq!(filtered3.len(), 2);

        // Test size filtering
        let query4 = FindQuery {
            min_size: Some(150),
            hidden: true,
            ..FindQuery::default()
        };
        let filtered4 = filter_entries(&entries, &query4);
        assert_eq!(filtered4.len(), 1);
        assert_eq!(filtered4.first().unwrap().path, PathBuf::from("/test/subdir/file2.txt"));
    }
}
