use std::{fs, path::Path, sync::Arc};

use iroh_blobs::Hash;
use iroh_docs::{AuthorId, api::Doc};
use n0_future::{BufferedStreamExt, TryStreamExt, stream};

use crate::{
    error::{Result, SyncwebError},
    node::{blob_store::BlobStore, docs_engine::DocsEngine},
};

use super::{FileEntry, ParallelScanner, Scanner};

/// A local file after it has been added to the blob store and document.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ImportEntry {
    pub path: std::path::PathBuf,
    pub relative_path: std::path::PathBuf,
    pub hash: Hash,
    pub size: u64,
}

/// Sequential scanner/blob/document import pipeline.
#[derive(Clone)]
pub struct Importer {
    blob_store: BlobStore,
    docs_engine: DocsEngine,
    doc: Doc,
    author: AuthorId,
    root: Option<std::path::PathBuf>,
    ignore_patterns: Vec<String>,
}

impl Importer {
    #[must_use]
    pub const fn new(blob_store: BlobStore, docs_engine: DocsEngine, doc: Doc, author: AuthorId) -> Self {
        Self {
            blob_store,
            docs_engine,
            doc,
            author,
            root: None,
            ignore_patterns: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_root(mut self, root: impl Into<std::path::PathBuf>) -> Self {
        self.root = Some(root.into());
        self
    }

    #[must_use]
    pub fn with_ignore_patterns(mut self, patterns: Vec<String>) -> Self {
        self.ignore_patterns = patterns;
        self
    }

    /// Import one file or every file below a directory.
    /// # Errors
    ///
    /// Returns an error if the filesystem or database cannot be accessed.
    pub async fn import_path(&self, input: impl AsRef<Path>) -> Result<Vec<ImportEntry>> {
        let files = self.scan_input(input.as_ref(), None)?;
        self.import_entries(files).await
    }

    /// Import one file or every file below a directory using this pipeline.
    /// # Errors
    ///
    /// Returns an error if the filesystem or database cannot be accessed.
    pub async fn import(&self, input: impl AsRef<Path>) -> Result<Vec<ImportEntry>> {
        self.import_path(input).await
    }

    /// Import pre-scanned entries.
    /// # Errors
    ///
    /// Returns an error if the filesystem or database cannot be accessed.
    pub async fn import_entries(&self, source_entries: Vec<FileEntry>) -> Result<Vec<ImportEntry>> {
        let mut imported = Vec::with_capacity(source_entries.len());
        for entry in source_entries {
            imported.push(self.import_one(entry).await?);
        }
        Ok(imported)
    }

    async fn import_one(&self, entry: FileEntry) -> Result<ImportEntry> {
        let hash = self.blob_store.add_file(&entry.path).await?;
        self.docs_engine
            .set_blob(
                &self.doc,
                self.author,
                entry.relative_path.as_os_str().as_encoded_bytes(),
                hash,
                entry.size,
            )
            .await?;
        Ok(ImportEntry {
            path: entry.path,
            relative_path: entry.relative_path,
            hash,
            size: entry.size,
        })
    }

    fn scan_input(&self, input: &Path, threads: Option<usize>) -> Result<Vec<FileEntry>> {
        if !input.exists() {
            return Err(SyncwebError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("input path does not exist: {}", input.display()),
            )));
        }
        let root = self.root.clone().unwrap_or_else(|| {
            if input.is_dir() {
                input.to_path_buf()
            } else {
                input.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
            }
        });
        let mut entries = match threads {
            Some(threads_count) => ParallelScanner::new(&root, self.ignore_patterns.clone(), threads_count).scan()?,
            None => Scanner::new(&root, self.ignore_patterns.clone()).scan()?,
        };
        let target = fs::canonicalize(input)?;
        if input.is_dir() {
            entries.retain(|entry| fs::canonicalize(&entry.path).is_ok_and(|path| path.starts_with(&target)));
        } else {
            entries.retain(|entry| fs::canonicalize(&entry.path).is_ok_and(|path| path == target));
        }
        Ok(entries)
    }
}

/// Parallel import pipeline. Hashing and filesystem reads are parallelized by
/// the scanner; blob/document writes retain their ordering and async safety.
#[derive(Clone)]
pub struct ParallelImporter {
    importer: Arc<Importer>,
    threads: usize,
}

impl ParallelImporter {
    #[must_use]
    pub fn new(blob_store: BlobStore, docs_engine: DocsEngine, doc: Doc, author: AuthorId) -> Self {
        Self {
            importer: Arc::new(Importer::new(blob_store, docs_engine, doc, author)),
            threads: 0,
        }
    }

    #[must_use]
    pub const fn with_threads(mut self, threads: usize) -> Self {
        self.threads = threads;
        self
    }

    #[must_use]
    pub fn with_root(mut self, root: impl Into<std::path::PathBuf>) -> Self {
        self.importer = Arc::new((*self.importer).clone().with_root(root));
        self
    }

    #[must_use]
    pub fn with_ignore_patterns(mut self, patterns: Vec<String>) -> Self {
        self.importer = Arc::new((*self.importer).clone().with_ignore_patterns(patterns));
        self
    }

    /// Import all files under a directory.
    /// # Errors
    ///
    /// Returns an error if the filesystem or database cannot be accessed.
    pub async fn import_path(&self, input: impl AsRef<Path>) -> Result<Vec<ImportEntry>> {
        let entries = self.importer.scan_input(input.as_ref(), Some(self.threads))?;
        self.import_entries(entries).await
    }

    /// Import one file or every file below a directory using the parallel pipeline.
    /// # Errors
    ///
    /// Returns an error if the filesystem or database cannot be accessed.
    pub async fn import_parallel(&self, input: impl AsRef<Path>) -> Result<Vec<ImportEntry>> {
        self.import_path(input).await
    }

    /// Import pre-scanned entries. This method can be used with a custom
    /// scanner while preserving the document update semantics.
    /// # Errors
    ///
    /// Returns an error if the filesystem or database cannot be accessed.
    pub async fn import_entries(&self, source_entries: Vec<FileEntry>) -> Result<Vec<ImportEntry>> {
        let concurrency = if self.threads == 0 {
            std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
        } else {
            self.threads
        };
        let shared_importer = self.importer.clone();
        let mut imported = stream::iter(source_entries.into_iter().enumerate().map(|(index, entry)| {
            let task_importer = shared_importer.clone();
            async move {
                task_importer
                    .import_one(entry)
                    .await
                    .map(|imported_entry| (index, imported_entry))
            }
        }))
        .buffered_unordered(concurrency)
        .try_collect::<Vec<_>>()
        .await?;
        imported.sort_by_key(|(index, _)| *index);
        Ok(imported.into_iter().map(|(_, entry)| entry).collect())
    }
}
