use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use iroh_blobs::Hash;
use n0_future::{BufferedStreamExt, TryStreamExt, stream};
use rayon::prelude::*;

use crate::{
    error::{Result, SyncwebError},
    node::blob_store::BlobStore,
};

/// A content-addressed file to write to disk.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ExportEntry {
    pub relative_path: PathBuf,
    pub hash: Hash,
    pub size: u64,
}

impl ExportEntry {
    #[must_use]
    pub fn new(relative_path: impl Into<PathBuf>, hash: Hash, size: u64) -> Self {
        Self {
            relative_path: relative_path.into(),
            hash,
            size,
        }
    }
}

/// Sequential blob exporter.
#[derive(Clone)]
pub struct Exporter {
    blob_store: BlobStore,
    destination: PathBuf,
}

impl Exporter {
    #[must_use]
    pub fn new(blob_store: BlobStore, destination: impl Into<PathBuf>) -> Self {
        Self {
            blob_store,
            destination: destination.into(),
        }
    }

    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    /// Export one entry, creating parent directories as needed.
    /// # Errors
    ///
    /// Returns an error if the filesystem or database cannot be accessed.
    pub async fn export_entry(&self, entry: &ExportEntry) -> Result<PathBuf> {
        let target = safe_join(&self.destination, &entry.relative_path)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = self.blob_store.get(entry.hash).await?;
        fs::write(&target, &bytes)?;
        Ok(target)
    }

    /// Export one blob and verify that the written bytes have the expected hash.
    /// # Errors
    ///
    /// Returns an error if the filesystem or database cannot be accessed.
    pub async fn export_verified(&self, entry: &ExportEntry) -> Result<PathBuf> {
        let target = self.export_entry(entry).await?;
        let actual = blake3::hash(&fs::read(&target)?);
        if actual.as_bytes() != entry.hash.as_bytes() {
            return Err(SyncwebError::operation(
                "exported blob hash does not match entry",
                target.display(),
            ));
        }
        Ok(target)
    }

    /// Export all entries in order.
    /// # Errors
    ///
    /// Returns an error if the filesystem or database cannot be accessed.
    pub async fn export(&self, entries: &[ExportEntry]) -> Result<Vec<PathBuf>> {
        let mut output = Vec::with_capacity(entries.len());
        for entry in entries {
            output.push(self.export_entry(entry).await?);
        }
        Ok(output)
    }

    /// Export all entries and verify each written blob.
    /// # Errors
    ///
    /// Returns an error if the filesystem or database cannot be accessed.
    pub async fn export_all_verified(&self, entries: &[ExportEntry]) -> Result<Vec<PathBuf>> {
        let mut output = Vec::with_capacity(entries.len());
        for entry in entries {
            output.push(self.export_verified(entry).await?);
        }
        Ok(output)
    }
}

/// Rayon-backed exporter. Blob retrieval remains asynchronous, while writes
/// are performed by a bounded rayon pool.
#[derive(Clone)]
pub struct ParallelExporter {
    exporter: Arc<Exporter>,
    threads: Option<usize>,
}

impl ParallelExporter {
    #[must_use]
    pub fn new(blob_store: BlobStore, destination: impl Into<PathBuf>) -> Self {
        Self {
            exporter: Arc::new(Exporter::new(blob_store, destination)),
            threads: None,
        }
    }

    #[must_use]
    pub const fn with_threads(mut self, threads: usize) -> Self {
        self.threads = if threads > 0 { Some(threads) } else { None };
        self
    }

    /// Export all entries concurrently.
    /// # Errors
    ///
    /// Returns an error if the filesystem or database cannot be accessed.
    pub async fn export(&self, entries: &[ExportEntry]) -> Result<Vec<PathBuf>> {
        let concurrency = self
            .threads
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get));
        let store = self.exporter.blob_store.clone();
        let mut contents = stream::iter(entries.iter().enumerate().map(|(index, item)| {
            let store_cloned = store.clone();
            let entry_cloned = item.clone();
            async move {
                let bytes = store_cloned.get(entry_cloned.hash).await?;
                Ok::<_, SyncwebError>((index, entry_cloned, bytes))
            }
        }))
        .buffered_unordered(concurrency)
        .try_collect::<Vec<_>>()
        .await?;
        contents.sort_by_key(|(index, _, _)| *index);
        let destination = self.exporter.destination.clone();
        let write = move || {
            contents
                .par_iter()
                .map(|(_, entry, bytes)| {
                    let target = safe_join(&destination, &entry.relative_path)?;
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&target, bytes)?;
                    Ok(target)
                })
                .collect::<Result<Vec<_>>>()
        };
        let paths = if let Some(threads) = self.threads {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .map_err(|error| SyncwebError::operation("failed to create exporter thread pool", error))?
                .install(write)?
        } else {
            write()?
        };
        Ok(paths)
    }

    /// Export all entries concurrently and verify each written blob.
    /// # Errors
    ///
    /// Returns an error if the filesystem or database cannot be accessed.
    pub async fn export_parallel(&self, entries: &[ExportEntry]) -> Result<Vec<PathBuf>> {
        let paths = self.export(entries).await?;
        let verified = entries
            .par_iter()
            .zip(paths.par_iter())
            .map(|(entry, path)| {
                let actual = blake3::hash(&fs::read(path)?);
                if actual.as_bytes() != entry.hash.as_bytes() {
                    return Err(SyncwebError::operation(
                        "exported blob hash does not match entry",
                        path.display(),
                    ));
                }
                Ok(path.clone())
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(verified)
    }
}

fn safe_join(root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| component == std::path::Component::ParentDir)
    {
        return Err(SyncwebError::InvalidConfig(format!(
            "export path escapes destination: {}",
            relative.display()
        )));
    }
    Ok(root.join(relative))
}
