use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    time::SystemTime,
};

use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;

use crate::error::{Result, SyncwebError};

/// The kind of a filesystem entry returned by a scanner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FileType {
    File,
    Directory,
    Symlink,
}

/// Metadata and content digest for a regular file.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FileEntry {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub hash: blake3::Hash,
    pub file_type: FileType,
}

impl FileEntry {
    #[must_use]
    pub fn builder() -> FileEntryBuilder {
        FileEntryBuilder::default()
    }
}

#[derive(Clone, Default)]
pub struct FileEntryBuilder {
    path: Option<PathBuf>,
    relative_path: Option<PathBuf>,
    size: Option<u64>,
    modified: Option<SystemTime>,
    hash: Option<blake3::Hash>,
    file_type: Option<FileType>,
}

impl FileEntryBuilder {
    #[must_use]
    pub fn path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }
    #[must_use]
    pub fn relative_path(mut self, relative_path: PathBuf) -> Self {
        self.relative_path = Some(relative_path);
        self
    }
    #[must_use]
    pub const fn size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }
    #[must_use]
    pub const fn modified(mut self, modified: SystemTime) -> Self {
        self.modified = Some(modified);
        self
    }
    #[must_use]
    pub const fn hash(mut self, hash: blake3::Hash) -> Self {
        self.hash = Some(hash);
        self
    }
    #[must_use]
    pub const fn file_type(mut self, file_type: FileType) -> Self {
        self.file_type = Some(file_type);
        self
    }
    /// Builds the `FileEntry`.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the required fields have not been set.
    pub fn build(self) -> std::result::Result<FileEntry, &'static str> {
        Ok(FileEntry {
            path: self.path.ok_or("path missing")?,
            relative_path: self.relative_path.ok_or("relative_path missing")?,
            size: self.size.ok_or("size missing")?,
            modified: self.modified.ok_or("modified missing")?,
            hash: self.hash.ok_or("hash missing")?,
            file_type: self.file_type.ok_or("file_type missing")?,
        })
    }
}

/// Glob patterns used to exclude files and directories from a scan.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IgnoreFilter {
    patterns: Vec<String>,
}

impl IgnoreFilter {
    #[must_use]
    pub fn new<I, S>(patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            patterns: patterns.into_iter().map(Into::into).collect(),
        }
    }

    #[must_use]
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }
}

impl From<Vec<String>> for IgnoreFilter {
    fn from(patterns: Vec<String>) -> Self {
        Self { patterns }
    }
}

/// Accepted thread-count forms for [`ParallelScanner::new`].
pub trait ThreadCount {
    fn into_thread_count(self) -> Option<usize>;
}

impl ThreadCount for usize {
    fn into_thread_count(self) -> Option<usize> {
        (self > 0).then_some(self)
    }
}

impl ThreadCount for Option<usize> {
    fn into_thread_count(self) -> Option<usize> {
        self.filter(|threads| *threads > 0)
    }
}

impl FileEntry {
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.path.file_name().and_then(|name| name.to_str())
    }
}

#[derive(Clone)]
struct IgnoreSet {
    set: GlobSet,
}

impl IgnoreSet {
    fn new<I, S>(patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            if let Ok(glob) = Glob::new(pattern.as_ref()) {
                builder.add(glob);
            }
        }
        let set = builder
            .build()
            .unwrap_or_else(|_| GlobSetBuilder::new().build().unwrap_or_default());
        Self { set }
    }

    fn matches(&self, relative: &Path) -> bool {
        self.set.is_match(relative)
            || relative
                .components()
                .any(|component| self.set.is_match(Path::new(component.as_os_str())))
    }
}

/// Sequential recursive filesystem scanner.
#[derive(Clone)]
pub struct Scanner {
    root: PathBuf,
    ignore: IgnoreSet,
}

impl Scanner {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, ignore_patterns_arg: impl Into<IgnoreFilter>) -> Self {
        let ignore_patterns = ignore_patterns_arg.into();
        Self {
            root: root.into(),
            ignore: IgnoreSet::new(ignore_patterns.patterns()),
        }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Scan all regular files below the configured root.
    /// # Errors
    ///
    /// Returns an error if the filesystem cannot be accessed or directories cannot be read.
    pub fn scan(&self) -> Result<Vec<FileEntry>> {
        if self.root.is_file() {
            let relative = self.root.file_name().map_or_else(PathBuf::new, PathBuf::from);
            return Ok(vec![hash_file(&self.root, &relative)?]);
        }
        let mut entries = Vec::new();
        self.scan_dir(&self.root, Path::new(""), &mut entries)?;
        entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        Ok(entries)
    }

    /// Scan files and filesystem entries below the configured root.
    ///
    /// [`Scanner::scan`] intentionally returns only regular files because it
    /// is used by import and hashing pipelines. This variant is useful for
    /// metadata queries such as finding directories or symlinks.
    /// # Errors
    ///
    /// Returns an error if the filesystem cannot be accessed or directories cannot be read.
    pub fn scan_all(&self) -> Result<Vec<FileEntry>> {
        if self.root.is_file() {
            let relative = self.root.file_name().map_or_else(PathBuf::new, PathBuf::from);
            return Ok(vec![metadata_entry(&self.root, &relative)?]);
        }
        let mut entries = Vec::new();
        self.scan_dir_all(&self.root, Path::new(""), &mut entries)?;
        entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        Ok(entries)
    }

    fn scan_dir(&self, directory: &Path, relative: &Path, output: &mut Vec<FileEntry>) -> Result<()> {
        let mut children = fs::read_dir(directory)?.collect::<std::result::Result<Vec<_>, io::Error>>()?;
        children.sort_by_key(std::fs::DirEntry::file_name);
        for child in children {
            let path = child.path();
            let child_relative = relative.join(child.file_name());
            if self.ignore.matches(&child_relative) {
                continue;
            }
            let file_type = child.file_type()?;
            if file_type.is_dir() {
                self.scan_dir(&path, &child_relative, output)?;
            } else if file_type.is_file() {
                output.push(hash_file(&path, &child_relative)?);
            } else {
                // do nothing
            }
        }
        Ok(())
    }

    fn scan_dir_all(&self, directory: &Path, relative: &Path, output: &mut Vec<FileEntry>) -> Result<()> {
        let mut children = fs::read_dir(directory)?.collect::<std::result::Result<Vec<_>, io::Error>>()?;
        children.sort_by_key(std::fs::DirEntry::file_name);
        for child in children {
            let path = child.path();
            let child_relative = relative.join(child.file_name());
            if self.ignore.matches(&child_relative) {
                continue;
            }
            let file_type = child.file_type()?;
            output.push(metadata_entry(&path, &child_relative)?);
            if file_type.is_dir() {
                self.scan_dir_all(&path, &child_relative, output)?;
            }
        }
        Ok(())
    }
}

/// Rayon-backed recursive filesystem scanner.
#[derive(Clone)]
pub struct ParallelScanner {
    scanner: Scanner,
    threads: Option<usize>,
}

impl ParallelScanner {
    #[must_use]
    pub fn new<T: ThreadCount>(root: impl Into<PathBuf>, ignore_patterns: impl Into<IgnoreFilter>, threads: T) -> Self {
        Self {
            scanner: Scanner::new(root, ignore_patterns),
            threads: threads.into_thread_count(),
        }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        self.scanner.root()
    }

    /// Scan and hash files concurrently.
    /// # Errors
    ///
    /// Returns an error if the filesystem cannot be accessed or directories cannot be read.
    pub fn scan(&self) -> Result<Vec<FileEntry>> {
        if self.scanner.root.is_file() {
            let relative = self.scanner.root.file_name().map_or_else(PathBuf::new, PathBuf::from);
            return Ok(vec![hash_file(&self.scanner.root, &relative)?]);
        }
        let mut paths = Vec::new();
        collect_paths(self.scanner.root(), Path::new(""), &self.scanner.ignore, &mut paths)?;
        let run = || {
            paths
                .par_iter()
                .map(|(path, relative)| hash_file(path, relative))
                .collect::<Result<Vec<_>>>()
        };
        let mut entries = if let Some(threads) = self.threads {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .map_err(|error| SyncwebError::operation("failed to create scanner thread pool", error))?
                .install(run)?
        } else {
            run()?
        };
        entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        Ok(entries)
    }

    /// Scan and hash files concurrently.
    /// # Errors
    ///
    /// Returns an error if the filesystem cannot be accessed or directories cannot be read.
    pub fn scan_parallel(&self) -> Result<Vec<FileEntry>> {
        self.scan()
    }

    /// Scan files and filesystem entries below the configured root.
    /// # Errors
    ///
    /// Returns an error if the filesystem cannot be accessed or directories cannot be read.
    pub fn scan_all(&self) -> Result<Vec<FileEntry>> {
        if self.scanner.root.is_file() {
            let relative = self.scanner.root.file_name().map_or_else(PathBuf::new, PathBuf::from);
            return Ok(vec![metadata_entry(&self.scanner.root, &relative)?]);
        }
        let mut paths = Vec::new();
        collect_all_paths(self.scanner.root(), Path::new(""), &self.scanner.ignore, &mut paths)?;
        let run = || {
            paths
                .par_iter()
                .map(|(path, relative)| metadata_entry(path, relative))
                .collect::<Result<Vec<_>>>()
        };
        let mut entries = if let Some(threads) = self.threads {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .map_err(|error| SyncwebError::operation("failed to create scanner thread pool", error))?
                .install(run)?
        } else {
            run()?
        };
        entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        Ok(entries)
    }
}

fn collect_paths(
    directory: &Path,
    relative: &Path,
    ignore: &IgnoreSet,
    paths: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<()> {
    let mut children = fs::read_dir(directory)?.collect::<std::result::Result<Vec<_>, io::Error>>()?;
    children.sort_by_key(std::fs::DirEntry::file_name);
    for child in children {
        let path = child.path();
        let child_relative = relative.join(child.file_name());
        if ignore.matches(&child_relative) {
            continue;
        }
        let file_type = child.file_type()?;
        if file_type.is_dir() {
            collect_paths(&path, &child_relative, ignore, paths)?;
        } else if file_type.is_file() {
            paths.push((path, child_relative));
        } else {
            // do nothing
        }
    }
    Ok(())
}

fn collect_all_paths(
    directory: &Path,
    relative: &Path,
    ignore: &IgnoreSet,
    paths: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<()> {
    let mut children = fs::read_dir(directory)?.collect::<std::result::Result<Vec<_>, io::Error>>()?;
    children.sort_by_key(std::fs::DirEntry::file_name);
    for child in children {
        let path = child.path();
        let child_relative = relative.join(child.file_name());
        if ignore.matches(&child_relative) {
            continue;
        }
        let file_type = child.file_type()?;
        paths.push((path.clone(), child_relative.clone()));
        if file_type.is_dir() {
            collect_all_paths(&path, &child_relative, ignore, paths)?;
        }
    }
    Ok(())
}

fn hash_file(path: &Path, relative: &Path) -> Result<FileEntry> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0_u8; 128 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        if let Some(chunk) = buffer.get(..read) {
            hasher.update(chunk);
        }
    }
    let metadata = fs::metadata(path)?;
    Ok(FileEntry {
        path: path.to_path_buf(),
        relative_path: relative.to_path_buf(),
        size: metadata.len(),
        modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        hash: hasher.finalize(),
        file_type: if path.is_symlink() {
            FileType::Symlink
        } else if path.is_dir() {
            FileType::Directory
        } else {
            FileType::File
        },
    })
}

fn metadata_entry(path: &Path, relative: &Path) -> Result<FileEntry> {
    let metadata = fs::symlink_metadata(path)?;
    let file_type = if metadata.file_type().is_symlink() {
        FileType::Symlink
    } else if metadata.is_dir() {
        FileType::Directory
    } else {
        FileType::File
    };
    let hash = if file_type == FileType::File {
        hash_file(path, relative)?.hash
    } else {
        blake3::hash(&[])
    };
    Ok(FileEntry {
        path: path.to_path_buf(),
        relative_path: relative.to_path_buf(),
        size: metadata.len(),
        modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        hash,
        file_type,
    })
}
