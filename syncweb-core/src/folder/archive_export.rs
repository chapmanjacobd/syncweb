use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_compression::tokio::write::ZstdEncoder;
use iroh_blobs::Hash;
use semver::Version;
use serde::{Deserialize, Serialize};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};
use uuid::Uuid;

use crate::{
    daemon::ManagedPool,
    error::{Result, SyncwebError},
    filter::{FilterAction, FilterEngine, FilterEntry},
    node::blob_store::BlobStore,
};

use super::{
    CollectionEntry, CollectionManifest,
    archive_format::{DropIndex, DropProviderHint, car_header, cid_for_hash, encode_varint},
};
const COPY_BUFFER_SIZE: usize = 64 * 1024;

/// Select which content a drop contributes to an archive.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DropExportMode {
    /// Export every entry in the selected collection version.
    Collection,
    /// Export only the listed blocks that are missing at the destination.
    MissingBlocks(BTreeSet<Hash>),
    /// Export only the listed blocks as a repair bundle.
    RepairBundle(BTreeSet<Hash>),
    /// Export entries in manifest order until the byte budget is exhausted.
    Contribution { max_bytes: u64 },
}

impl Default for DropExportMode {
    fn default() -> Self {
        Self::Collection
    }
}

impl DropExportMode {
    fn label(&self) -> &'static str {
        match self {
            Self::Collection => "collection",
            Self::MissingBlocks(_) => "missing-blocks",
            Self::RepairBundle(_) => "repair-bundle",
            Self::Contribution { .. } => "contribution",
        }
    }
}

/// Options controlling which collection version and entries are included.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DropExportOptions {
    version: Option<String>,
    filter: Option<FilterEngine>,
    mode: DropExportMode,
    source_community: Option<String>,
    provider_hints: Vec<DropProviderHint>,
}

impl Default for DropExportOptions {
    fn default() -> Self {
        Self {
            version: None,
            filter: None,
            mode: DropExportMode::Collection,
            source_community: None,
            provider_hints: Vec::new(),
        }
    }
}

impl DropExportOptions {
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    #[must_use]
    pub fn with_filter(mut self, filter: FilterEngine) -> Self {
        self.filter = Some(filter);
        self
    }

    #[must_use]
    pub fn with_mode(mut self, mode: DropExportMode) -> Self {
        self.mode = mode;
        self
    }

    #[must_use]
    pub fn with_source_community(mut self, community: impl Into<String>) -> Self {
        self.source_community = Some(community.into());
        self
    }

    #[must_use]
    pub fn with_provider_hints(mut self, providers: Vec<DropProviderHint>) -> Self {
        self.provider_hints = providers;
        self
    }

    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    #[must_use]
    pub const fn filter(&self) -> Option<&FilterEngine> {
        self.filter.as_ref()
    }

    #[must_use]
    pub const fn mode(&self) -> &DropExportMode {
        &self.mode
    }
}

/// Information about a completed archive export.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct DropExportResult {
    pub output: PathBuf,
    pub collection_id: uuid::Uuid,
    pub version: String,
    pub manifest: Hash,
    pub index: Hash,
    pub entry_count: usize,
    pub block_count: usize,
    pub archive_size: u64,
}

/// Streams collection manifests and content blobs into atomic `.car.zst` files.
#[derive(Clone)]
pub struct DropExporter {
    blob_store: BlobStore,
    export_lock: Arc<Mutex<()>>,
}

impl DropExporter {
    #[must_use]
    pub fn new(blob_store: BlobStore) -> Self {
        Self {
            blob_store,
            export_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Export one manifest without filtering.
    ///
    /// # Errors
    ///
    /// Returns an error if the manifest is invalid, content is unavailable, or
    /// the archive cannot be written.
    pub async fn export_archive(
        &self,
        manifest: &CollectionManifest,
        output: impl AsRef<Path>,
    ) -> Result<DropExportResult> {
        self.export_drop_with_options(
            std::slice::from_ref(manifest),
            output,
            DropExportOptions::default(),
            None,
        )
        .await
    }

    /// Export one or more versions, selecting the requested version or latest
    /// semver version when no version is specified.
    ///
    /// # Errors
    ///
    /// Returns an error if manifests disagree about their collection, the
    /// requested version is unavailable, or the archive cannot be written.
    pub async fn export_manifests(
        &self,
        manifests: &[CollectionManifest],
        output: impl AsRef<Path>,
        options: DropExportOptions,
    ) -> Result<DropExportResult> {
        self.export_drop_with_options(manifests, output, options, None).await
    }

    /// Export a manifest using filtering and version-selection options.
    ///
    /// A filtered manifest is deliberately unsigned because changing its
    /// entries changes the signed content. The archive remains content
    /// addressed and each included blob is verified while it is streamed.
    ///
    /// # Errors
    ///
    /// Returns an error if the selected manifest or any referenced blob is
    /// invalid or unavailable, or if the archive cannot be written.
    pub async fn export_drop_with_options(
        &self,
        manifests: &[CollectionManifest],
        output: impl AsRef<Path>,
        options: DropExportOptions,
        pool: Option<&ManagedPool>,
    ) -> Result<DropExportResult> {
        let _export_guard = self.export_lock.lock().await;
        let selected = in_pool(pool, || select_manifest(manifests, options.version()))?;
        let filtered = in_pool(pool, || filter_manifest(selected, options.filter()))?;
        let filtered = in_pool(pool, || select_mode(filtered, &options.mode))?;
        let (manifest_bytes, manifest_hash, entries, index_bytes, index_hash) = in_pool(pool, || -> Result<_> {
            let manifest_bytes = filtered.to_bytes()?;
            let manifest_hash = Hash::new(&manifest_bytes);
            let entries = unique_entries(&filtered.entries);
            let index = DropIndex::from_manifest(
                &filtered,
                manifest_hash,
                &entries,
                options.mode.label(),
                options.source_community.clone(),
                options.provider_hints.clone(),
            )?;
            let index_bytes = index.to_bytes()?;
            let index_hash = Hash::new(&index_bytes);
            Ok((manifest_bytes, manifest_hash, entries, index_bytes, index_hash))
        })?;
        for entry in &entries {
            if !self.blob_store.has(entry.content_id).await? {
                return Err(SyncwebError::InvalidConfig(format!(
                    "drop content is missing from the blob store: {}",
                    entry.logical_path.display()
                )));
            }
        }

        let output_path = output.as_ref().to_path_buf();
        let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).await?;
        let staging = parent.join(format!(
            "{}{}",
            crate::constants::DROP_EXPORT_STAGING_PREFIX,
            Uuid::new_v4()
        ));
        let write_result = self
            .write_archive(
                &staging,
                &manifest_bytes,
                manifest_hash,
                &index_bytes,
                index_hash,
                &entries,
                pool,
            )
            .await;
        if let Err(error) = write_result {
            remove_if_present(&staging).await?;
            return Err(error);
        }
        if let Err(error) = fs::rename(&staging, &output_path).await {
            remove_if_present(&staging).await?;
            return Err(SyncwebError::operation("failed to finalize drop archive", error));
        }
        let archive_size = fs::metadata(&output_path)
            .await
            .map_err(|error| SyncwebError::operation("failed to inspect drop archive", error))?
            .len();
        Ok(DropExportResult {
            output: output_path,
            collection_id: filtered.collection_id,
            version: filtered.version,
            manifest: manifest_hash,
            index: index_hash,
            entry_count: filtered.entries.len(),
            block_count: entries.len().saturating_add(2),
            archive_size,
        })
    }

    async fn write_archive(
        &self,
        staging: &Path,
        manifest_bytes: &[u8],
        manifest_hash: Hash,
        index_bytes: &[u8],
        index_hash: Hash,
        entries: &[CollectionEntry],
        pool: Option<&ManagedPool>,
    ) -> Result<()> {
        let file = fs::File::create(staging)
            .await
            .map_err(|error| SyncwebError::operation("failed to create drop archive", error))?;
        let mut encoder = ZstdEncoder::new(file);
        let header = car_header(&[manifest_hash, index_hash])?;
        encoder.write_all(&header).await?;
        write_section(&mut encoder, manifest_hash, manifest_bytes).await?;
        write_section(&mut encoder, index_hash, index_bytes).await?;
        let temporary_dir = staging.parent().unwrap_or_else(|| Path::new("."));
        for entry in entries {
            write_blob_section(&self.blob_store, &mut encoder, entry, temporary_dir, pool).await?;
        }
        encoder
            .shutdown()
            .await
            .map_err(|error| SyncwebError::operation("failed to finish drop compression", error))?;
        Ok(())
    }
}

/// Export one collection manifest into a compressed CAR archive.
///
/// # Errors
///
/// Returns an error if the manifest or referenced content is invalid or the
/// archive cannot be written.
pub async fn export_archive(
    blob_store: BlobStore,
    manifest: &CollectionManifest,
    output: impl AsRef<Path>,
    filter: Option<&FilterEngine>,
) -> Result<DropExportResult> {
    let options = filter.map_or_else(DropExportOptions::default, |value| {
        DropExportOptions::default().with_filter(value.clone())
    });
    DropExporter::new(blob_store)
        .export_drop_with_options(std::slice::from_ref(manifest), output, options, None)
        .await
}

fn in_pool<F, R>(pool: Option<&ManagedPool>, operation: F) -> R
where
    F: FnOnce() -> R + Send,
    R: Send,
{
    match pool {
        Some(managed_pool) => managed_pool.install(operation),
        None => operation(),
    }
}

fn select_manifest(manifests: &[CollectionManifest], requested: Option<&str>) -> Result<CollectionManifest> {
    let mut candidates = manifests.iter();
    let Some(first) = candidates.next() else {
        return Err(SyncwebError::InvalidConfig(
            "at least one collection manifest is required".to_owned(),
        ));
    };
    first.validate()?;
    let collection_id = first.collection_id;
    let mut selected = first.clone();
    for manifest in candidates {
        manifest.validate()?;
        if manifest.collection_id != collection_id {
            return Err(SyncwebError::InvalidConfig(
                "drop manifests must belong to the same collection".to_owned(),
            ));
        }
        if requested.is_none() && manifest.is_upgrade_from(&selected)? {
            selected = manifest.clone();
        }
    }
    if let Some(version) = requested {
        Version::parse(version)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid requested collection version: {error}")))?;
        selected = manifests
            .iter()
            .find(|manifest| manifest.version == version)
            .cloned()
            .ok_or_else(|| SyncwebError::InvalidConfig(format!("collection version {version} is unavailable")))?;
        if selected.collection_id != collection_id {
            return Err(SyncwebError::InvalidConfig(
                "drop manifests must belong to the same collection".to_owned(),
            ));
        }
    }
    Ok(selected)
}

fn filter_manifest(manifest: CollectionManifest, filter: Option<&FilterEngine>) -> Result<CollectionManifest> {
    let Some(engine) = filter else {
        return Ok(manifest);
    };
    let version = Version::parse(&manifest.version)
        .map_err(|error| SyncwebError::InvalidConfig(format!("invalid collection version: {error}")))?;
    let mut filtered = manifest;
    filtered.entries.retain(|entry| {
        let filter_entry = FilterEntry::new(&entry.logical_path, entry.size).with_version(version.clone());
        engine.evaluate(&filter_entry) == FilterAction::Accept
    });
    filtered.signature = None;
    filtered.public_key = None;
    filtered.validate()?;
    Ok(filtered)
}

fn select_mode(mut manifest: CollectionManifest, mode: &DropExportMode) -> Result<CollectionManifest> {
    match mode {
        DropExportMode::Collection => Ok(manifest),
        DropExportMode::MissingBlocks(hashes) | DropExportMode::RepairBundle(hashes) => {
            if hashes.is_empty() {
                return Err(SyncwebError::InvalidConfig(
                    "a block-selection export requires at least one hash".to_owned(),
                ));
            }
            manifest.entries.retain(|entry| hashes.contains(&entry.content_id));
            manifest.signature = None;
            manifest.public_key = None;
            manifest.validate()?;
            Ok(manifest)
        }
        DropExportMode::Contribution { max_bytes } => {
            let mut total = 0_u64;
            let mut selected = BTreeSet::new();
            manifest.entries.retain(|entry| {
                if selected.contains(&entry.content_id) {
                    return true;
                }
                let fits = total
                    .checked_add(entry.size)
                    .is_some_and(|candidate| candidate <= *max_bytes);
                if fits {
                    total = total.saturating_add(entry.size);
                    selected.insert(entry.content_id);
                }
                fits
            });
            manifest.signature = None;
            manifest.public_key = None;
            manifest.validate()?;
            Ok(manifest)
        }
    }
}

fn unique_entries(entries: &[CollectionEntry]) -> Vec<CollectionEntry> {
    let mut hashes = BTreeSet::new();
    entries
        .iter()
        .filter(|entry| hashes.insert(entry.content_id))
        .cloned()
        .collect()
}

async fn write_blob_section(
    blob_store: &BlobStore,
    encoder: &mut ZstdEncoder<fs::File>,
    entry: &CollectionEntry,
    temporary_dir: &Path,
    pool: Option<&ManagedPool>,
) -> Result<()> {
    let temporary_path = temporary_dir.join(format!(
        "{}{}",
        crate::constants::DROP_EXPORT_BLOB_STAGING_PREFIX,
        Uuid::new_v4()
    ));
    let result = stream_blob(blob_store, encoder, entry, &temporary_path, pool).await;
    let cleanup = remove_if_present(&temporary_path).await;
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(cleanup_error)) => Err(SyncwebError::operation(
            "failed to clean up temporary drop blob",
            format!("{error}; cleanup failed: {cleanup_error}"),
        )),
    }
}

async fn stream_blob(
    blob_store: &BlobStore,
    encoder: &mut ZstdEncoder<fs::File>,
    entry: &CollectionEntry,
    temporary_path: &Path,
    pool: Option<&ManagedPool>,
) -> Result<()> {
    blob_store.export_to_path(entry.content_id, temporary_path).await?;
    let metadata = fs::metadata(temporary_path)
        .await
        .map_err(|error| SyncwebError::operation("failed to inspect exported drop blob", error))?;
    if metadata.len() != entry.size {
        return Err(SyncwebError::InvalidConfig(format!(
            "drop blob size does not match manifest for {}",
            entry.logical_path.display()
        )));
    }
    write_section_prefix(encoder, entry.content_id, entry.size).await?;
    let mut source = fs::File::open(temporary_path)
        .await
        .map_err(|error| SyncwebError::operation("failed to open exported drop blob", error))?;
    let mut buffer = vec![0_u8; COPY_BUFFER_SIZE];
    let mut hasher = blake3::Hasher::new();
    let mut copied = 0_u64;
    loop {
        let read = source.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let chunk = buffer
            .get(..read)
            .ok_or_else(|| SyncwebError::operation("blob reader returned an invalid chunk", read))?;
        encoder.write_all(chunk).await?;
        in_pool(pool, || hasher.update(chunk));
        copied = copied
            .checked_add(u64::try_from(read).map_err(|error| SyncwebError::operation("drop blob is too large", error))?)
            .ok_or_else(|| SyncwebError::operation("drop blob size overflow", "u64 limit exceeded"))?;
    }
    let actual = in_pool(pool, || Hash::from_bytes(*hasher.finalize().as_bytes()));
    if copied != entry.size || actual != entry.content_id {
        return Err(SyncwebError::InvalidConfig(format!(
            "drop blob hash does not match manifest for {}",
            entry.logical_path.display()
        )));
    }
    Ok(())
}

async fn write_section<W: AsyncWrite + Unpin>(writer: &mut W, hash: Hash, bytes: &[u8]) -> Result<()> {
    write_section_prefix(
        writer,
        hash,
        u64::try_from(bytes.len())
            .map_err(|error| SyncwebError::operation("manifest is too large for a drop archive", error))?,
    )
    .await?;
    writer.write_all(bytes).await?;
    Ok(())
}

async fn write_section_prefix<W: AsyncWrite + Unpin>(writer: &mut W, hash: Hash, size: u64) -> Result<()> {
    let cid = cid_for_hash(hash);
    let section_size = u64::try_from(cid.len())
        .map_err(|error| SyncwebError::operation("CAR section is too large", error))?
        .checked_add(size)
        .ok_or_else(|| SyncwebError::operation("CAR section size overflow", "u64 limit exceeded"))?;
    let mut encoded_size = Vec::with_capacity(10);
    encode_varint(section_size, &mut encoded_size);
    writer.write_all(&encoded_size).await?;
    writer.write_all(&cid).await?;
    Ok(())
}

async fn remove_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(SyncwebError::operation(
            "failed to remove temporary archive file",
            error,
        )),
    }
}
