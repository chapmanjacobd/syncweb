use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use async_compression::tokio::bufread::ZstdDecoder;
use iroh_blobs::Hash;
use iroh_docs::NamespaceId;
use semver::Version;
use tokio::{
    fs,
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
    sync::Mutex,
};
use uuid::Uuid;

use crate::{
    error::{Result, SyncwebError},
    filter::{FilterAction, FilterEngine, FilterEntry},
    node::iroh_node::IrohNode,
};

use super::{CollectionManifest, CollectionStore, FolderManager, SyncMode};

const CAR_VERSION: u64 = 1;
const CID_VERSION: u64 = 1;
const RAW_CODEC: u64 = 0x55;
const BLAKE3_MULTIHASH: u64 = 0x1e;
const HASH_SIZE: u64 = 32;
const MAX_HEADER_SIZE: u64 = 64 * 1024;
const MAX_MANIFEST_SIZE: u64 = 16 * 1024 * 1024;
const COPY_BUFFER_SIZE: usize = 64 * 1024;

/// Options controlling validation and filtering during drop import.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct DropImportOptions {
    filter: Option<FilterEngine>,
    available_dependencies: BTreeMap<Uuid, Version>,
}

impl DropImportOptions {
    #[must_use]
    pub fn with_filter(mut self, filter: FilterEngine) -> Self {
        self.filter = Some(filter);
        self
    }

    #[must_use]
    pub fn with_available_dependencies(mut self, available: BTreeMap<Uuid, Version>) -> Self {
        self.available_dependencies = available;
        self
    }

    #[must_use]
    pub const fn filter(&self) -> Option<&FilterEngine> {
        self.filter.as_ref()
    }

    #[must_use]
    pub const fn available_dependencies(&self) -> &BTreeMap<Uuid, Version> {
        &self.available_dependencies
    }
}

/// Information about a successfully imported drop.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DropImportResult {
    pub manifest: Hash,
    pub collection_manifest: CollectionManifest,
    pub collection_id: Uuid,
    pub version: String,
    pub entry_count: usize,
    pub imported_entry_count: usize,
    pub skipped_entry_count: usize,
    pub block_count: usize,
    pub imported_block_count: usize,
    pub namespace_id: Option<NamespaceId>,
}

/// Streams, verifies, and imports compressed CAR drop archives.
#[derive(Clone)]
pub struct DropImporter {
    blob_store: crate::node::blob_store::BlobStore,
    import_lock: Arc<Mutex<()>>,
}

impl DropImporter {
    #[must_use]
    pub fn new(blob_store: crate::node::blob_store::BlobStore) -> Self {
        Self {
            blob_store,
            import_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Import a drop into the local blob store.
    ///
    /// The archive is fully validated before any blob is added. Content is
    /// staged in bounded streaming writes, so importing does not load the
    /// archive or individual content blocks into memory.
    ///
    /// # Errors
    ///
    /// Returns an error if decompression, CAR parsing, signature validation,
    /// dependency validation, or blob ingestion fails.
    pub async fn import_archive(
        &self,
        input: impl AsRef<Path>,
        options: DropImportOptions,
    ) -> Result<DropImportResult> {
        let _import_guard = self.import_lock.lock().await;
        let input_path = input.as_ref();
        let file = fs::File::open(input_path)
            .await
            .map_err(|error| SyncwebError::operation("failed to open drop archive", error))?;
        let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
        let staging = parent.join(format!(".syncweb-drop-import-{}", Uuid::new_v4()));
        fs::create_dir(&staging)
            .await
            .map_err(|error| SyncwebError::operation("failed to create drop import staging directory", error))?;

        let parse_result = read_archive(ZstdDecoder::new(BufReader::new(file)), &staging, &options).await;
        let parsed = match parse_result {
            Ok(parsed) => parsed,
            Err(error) => {
                remove_staging(&staging).await?;
                return Err(error);
            }
        };

        let import_result = self.ingest(parsed).await;
        let cleanup_result = remove_staging(&staging).await;
        match (import_result, cleanup_result) {
            (Ok(result), Ok(())) => Ok(result),
            (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(cleanup_error)) => Err(SyncwebError::operation(
                "failed to clean up drop import staging directory",
                format!("{error}; cleanup failed: {cleanup_error}"),
            )),
        }
    }

    /// Materialize an imported manifest into a new directory.
    ///
    /// The destination must not already exist. Files are written to a sibling
    /// staging directory and renamed into place only after every blob has been
    /// exported successfully.
    ///
    /// # Errors
    ///
    /// Returns an error if the destination exists, a blob is unavailable, or
    /// materialization cannot be finalized.
    pub async fn materialize(&self, result: &DropImportResult, target: impl AsRef<Path>) -> Result<()> {
        let target_path = target.as_ref();
        if fs::try_exists(target_path).await? {
            return Err(SyncwebError::InvalidConfig(format!(
                "drop materialization target already exists: {}",
                target_path.display()
            )));
        }
        let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).await?;
        let staging = parent.join(format!(".syncweb-materialize-{}", Uuid::new_v4()));
        fs::create_dir(&staging)
            .await
            .map_err(|error| SyncwebError::operation("failed to create materialization staging directory", error))?;

        let write_result = async {
            for entry in &result.collection_manifest.entries {
                let destination = staging.join(&entry.logical_path);
                let entry_parent = destination
                    .parent()
                    .ok_or_else(|| SyncwebError::InvalidConfig("drop entry has no parent directory".to_owned()))?;
                fs::create_dir_all(entry_parent).await?;
                let size = self.blob_store.export_to_path(entry.content_id, &destination).await?;
                if size != entry.size {
                    return Err(SyncwebError::InvalidConfig(format!(
                        "materialized drop entry size does not match manifest: {}",
                        entry.logical_path.display()
                    )));
                }
            }
            fs::rename(&staging, target_path)
                .await
                .map_err(|error| SyncwebError::operation("failed to finalize drop materialization", error))?;
            Ok(())
        }
        .await;
        if let Err(error) = write_result {
            remove_staging(&staging).await?;
            return Err(error);
        }
        Ok(())
    }

    async fn ingest(&self, parsed: ParsedDrop) -> Result<DropImportResult> {
        let manifest_hash = Hash::new(&parsed.manifest_bytes);
        let stored_manifest = self.blob_store.add_bytes(&parsed.manifest_bytes).await?;
        if stored_manifest != manifest_hash {
            return Err(SyncwebError::InvalidConfig(
                "imported manifest hash does not match its content".to_owned(),
            ));
        }

        for block in &parsed.blocks {
            let stored = self.blob_store.add_file(&block.path).await?;
            if stored != block.hash {
                return Err(SyncwebError::InvalidConfig(format!(
                    "imported content hash does not match its CID: expected {}, got {}",
                    block.hash, stored
                )));
            }
        }

        let collection_id = parsed.manifest.collection_id;
        let version = parsed.manifest.version.clone();
        let entry_count = parsed.entry_count;
        let imported_entry_count = parsed.manifest.entries.len();
        let skipped_entry_count = entry_count.saturating_sub(imported_entry_count);
        let block_count = parsed.block_count;
        let imported_block_count = parsed.blocks.len().saturating_add(1);
        let collection_manifest = parsed.manifest;
        Ok(DropImportResult {
            manifest: manifest_hash,
            collection_manifest,
            collection_id,
            version,
            entry_count,
            imported_entry_count,
            skipped_entry_count,
            block_count,
            imported_block_count,
            namespace_id: None,
        })
    }
}

/// Import, register, and materialize a drop in one operation.
///
/// A new writable folder namespace is created for the imported collection and
/// its manifest is published there. The destination must not already exist.
///
/// # Errors
///
/// Returns an error if import, materialization, namespace creation, or
/// manifest publication fails.
pub async fn import_archive(
    node: &IrohNode,
    input: impl AsRef<Path>,
    target_dir: impl AsRef<Path>,
    filter: Option<&FilterEngine>,
) -> Result<DropImportResult> {
    let options = filter.map_or_else(DropImportOptions::default, |value| {
        DropImportOptions::default().with_filter(value.clone())
    });
    let importer = DropImporter::new(node.blob_store().clone());
    let mut result = importer.import_archive(input, options).await?;
    importer.materialize(&result, target_dir).await?;

    let folder = FolderManager::new(node).create(SyncMode::SendReceive).await?;
    let store = CollectionStore::new(
        folder.doc().clone(),
        folder.author(),
        node.blob_store().clone(),
        node.docs_engine().clone(),
    );
    store.publish(&result.collection_manifest, 1).await?;
    result.namespace_id = Some(folder.namespace_id());
    Ok(result)
}

struct ParsedDrop {
    manifest: CollectionManifest,
    manifest_bytes: Vec<u8>,
    blocks: Vec<StagedBlock>,
    entry_count: usize,
    block_count: usize,
}

struct StagedBlock {
    hash: Hash,
    path: PathBuf,
}

struct SectionHeader {
    hash: Hash,
    payload_size: u64,
}

async fn read_archive<R>(mut reader: R, staging: &Path, options: &DropImportOptions) -> Result<ParsedDrop>
where
    R: AsyncRead + Unpin,
{
    let header_size = read_required_varint(&mut reader, "CAR header length").await?;
    if header_size > MAX_HEADER_SIZE {
        return Err(SyncwebError::InvalidConfig(format!(
            "CAR header exceeds {MAX_HEADER_SIZE} bytes"
        )));
    }
    let header_len = usize::try_from(header_size)
        .map_err(|error| SyncwebError::operation("CAR header length is not representable", error))?;
    let mut header = vec![0_u8; header_len];
    read_exact(&mut reader, &mut header, "failed to read CAR header").await?;
    let root = parse_car_header(&header)?;

    let manifest_section = read_section_header(&mut reader)
        .await?
        .ok_or_else(|| SyncwebError::InvalidConfig("drop archive has no manifest block".to_owned()))?;
    if manifest_section.hash != root {
        return Err(SyncwebError::InvalidConfig(
            "drop manifest CID does not match the CAR root".to_owned(),
        ));
    }
    if manifest_section.payload_size > MAX_MANIFEST_SIZE {
        return Err(SyncwebError::InvalidConfig(format!(
            "drop manifest exceeds {MAX_MANIFEST_SIZE} bytes"
        )));
    }
    let manifest_bytes = read_payload_to_vec(&mut reader, manifest_section.payload_size, "drop manifest").await?;
    let manifest = CollectionManifest::from_bytes(&manifest_bytes)?;
    manifest.verify_signature()?;
    if Hash::new(&manifest_bytes) != root || manifest.blob_id()? != root {
        return Err(SyncwebError::InvalidConfig(
            "drop manifest hash does not match the CAR root".to_owned(),
        ));
    }
    if !manifest.dependencies_satisfied(options.available_dependencies())? {
        return Err(SyncwebError::InvalidConfig(
            "drop package has missing or incompatible dependencies".to_owned(),
        ));
    }

    let filtered_manifest = filter_manifest(manifest.clone(), options.filter())?;
    let mut expected = BTreeMap::<Hash, (u64, bool)>::new();
    let accepted_hashes = filtered_manifest
        .entries
        .iter()
        .map(|entry| entry.content_id)
        .collect::<BTreeSet<_>>();
    for entry in &manifest.entries {
        match expected.entry(entry.content_id) {
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert((entry.size, accepted_hashes.contains(&entry.content_id)));
            }
            std::collections::btree_map::Entry::Occupied(slot) => {
                let (size, accepted) = slot.into_mut();
                if *size != entry.size {
                    return Err(SyncwebError::InvalidConfig(format!(
                        "manifest uses content hash {} with inconsistent sizes",
                        entry.content_id
                    )));
                }
                *accepted |= accepted_hashes.contains(&entry.content_id);
            }
        }
    }

    let mut seen = BTreeSet::new();
    let mut blocks = Vec::new();
    let mut block_count = 1_usize;
    while let Some(section) = read_section_header(&mut reader).await? {
        let (expected_size, accepted) = expected.get(&section.hash).copied().ok_or_else(|| {
            SyncwebError::InvalidConfig(format!(
                "drop contains a block not referenced by the manifest: {}",
                section.hash
            ))
        })?;
        if !seen.insert(section.hash) {
            return Err(SyncwebError::InvalidConfig(format!(
                "drop contains duplicate content block: {}",
                section.hash
            )));
        }
        if section.payload_size != expected_size {
            return Err(SyncwebError::InvalidConfig(format!(
                "drop content size does not match the manifest for {}",
                section.hash
            )));
        }
        let staged_path = accepted.then(|| staging.join(format!("block-{}.blob", blocks.len())));
        let (actual_hash, actual_size) =
            read_payload_to_path(&mut reader, section.payload_size, staged_path.as_deref()).await?;
        if actual_hash != section.hash {
            return Err(SyncwebError::InvalidConfig(format!(
                "drop content hash does not match its CID: expected {}, got {}",
                section.hash, actual_hash
            )));
        }
        if actual_size != expected_size {
            return Err(SyncwebError::InvalidConfig(format!(
                "drop content size does not match the manifest for {}",
                section.hash
            )));
        }
        if let Some(block_path) = staged_path {
            blocks.push(StagedBlock {
                hash: section.hash,
                path: block_path,
            });
        }
        block_count = block_count
            .checked_add(1)
            .ok_or_else(|| SyncwebError::operation("drop block count overflow", "usize limit exceeded"))?;
    }

    if seen.len() != expected.len() {
        let missing = expected
            .keys()
            .find(|hash| !seen.contains(*hash))
            .copied()
            .map_or_else(|| "unknown".to_owned(), |hash| hash.to_string());
        return Err(SyncwebError::InvalidConfig(format!(
            "drop is missing content referenced by the manifest: {missing}"
        )));
    }

    let filtered_manifest_bytes = filtered_manifest.to_bytes()?;
    Ok(ParsedDrop {
        manifest: filtered_manifest,
        manifest_bytes: filtered_manifest_bytes,
        blocks,
        entry_count: manifest.entries.len(),
        block_count,
    })
}

fn filter_manifest(manifest: CollectionManifest, filter_option: Option<&FilterEngine>) -> Result<CollectionManifest> {
    let Some(filter) = filter_option else {
        return Ok(manifest);
    };
    let version = Version::parse(&manifest.version)
        .map_err(|error| SyncwebError::InvalidConfig(format!("invalid collection version: {error}")))?;
    let mut filtered = manifest;
    filtered.entries.retain(|entry| {
        let filter_entry = FilterEntry::new(&entry.logical_path, entry.size).with_version(version.clone());
        filter.evaluate(&filter_entry) == FilterAction::Accept
    });
    filtered.signature = None;
    filtered.public_key = None;
    filtered.validate()?;
    Ok(filtered)
}

async fn read_payload_to_vec<R>(reader: &mut R, size: u64, context: &'static str) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let capacity =
        usize::try_from(size).map_err(|error| SyncwebError::operation("drop payload is too large", error))?;
    let mut bytes = Vec::with_capacity(capacity);
    let mut buffer = vec![0_u8; COPY_BUFFER_SIZE];
    let mut remaining = size;
    while remaining > 0 {
        let requested = usize::try_from(
            remaining.min(
                u64::try_from(buffer.len())
                    .map_err(|error| SyncwebError::operation("drop buffer length is not representable", error))?,
            ),
        )
        .map_err(|error| SyncwebError::operation("drop payload size is not representable", error))?;
        let chunk = buffer
            .get_mut(..requested)
            .ok_or_else(|| SyncwebError::operation("drop payload range is invalid", requested))?;
        read_exact(reader, chunk, context).await?;
        bytes.extend_from_slice(chunk);
        remaining = remaining
            .checked_sub(
                u64::try_from(requested)
                    .map_err(|error| SyncwebError::operation("drop payload is too large", error))?,
            )
            .ok_or_else(|| SyncwebError::operation("drop payload size underflow", context))?;
    }
    Ok(bytes)
}

async fn read_payload_to_path<R>(reader: &mut R, size: u64, destination: Option<&Path>) -> Result<(Hash, u64)>
where
    R: AsyncRead + Unpin,
{
    let mut output = match destination {
        Some(destination_path) => Some(
            fs::File::create(destination_path)
                .await
                .map_err(|error| SyncwebError::operation("failed to stage imported content", error))?,
        ),
        None => None,
    };
    let mut buffer = vec![0_u8; COPY_BUFFER_SIZE];
    let mut remaining = size;
    let mut copied = 0_u64;
    let mut hasher = blake3::Hasher::new();
    while remaining > 0 {
        let requested = usize::try_from(
            remaining.min(
                u64::try_from(buffer.len())
                    .map_err(|error| SyncwebError::operation("drop buffer length is not representable", error))?,
            ),
        )
        .map_err(|error| SyncwebError::operation("drop payload size is not representable", error))?;
        let chunk = buffer
            .get_mut(..requested)
            .ok_or_else(|| SyncwebError::operation("drop payload range is invalid", requested))?;
        read_exact(reader, chunk, "failed to read drop content").await?;
        if let Some(output_file) = &mut output {
            output_file.write_all(chunk).await?;
        }
        hasher.update(chunk);
        let requested_u64 =
            u64::try_from(requested).map_err(|error| SyncwebError::operation("drop payload is too large", error))?;
        remaining = remaining
            .checked_sub(requested_u64)
            .ok_or_else(|| SyncwebError::operation("drop payload size underflow", "invalid section length"))?;
        copied = copied
            .checked_add(requested_u64)
            .ok_or_else(|| SyncwebError::operation("drop payload size overflow", "u64 limit exceeded"))?;
    }
    if let Some(output_file) = &mut output {
        output_file.flush().await?;
    }
    Ok((Hash::from_bytes(*hasher.finalize().as_bytes()), copied))
}

async fn read_section_header<R>(reader: &mut R) -> Result<Option<SectionHeader>>
where
    R: AsyncRead + Unpin,
{
    let Some(section_size) = read_varint(reader, true, "CAR section length").await? else {
        return Ok(None);
    };
    let mut remaining = section_size;
    let version = read_section_varint(reader, &mut remaining, "CID version").await?;
    let codec = read_section_varint(reader, &mut remaining, "CID codec").await?;
    let hash_code = read_section_varint(reader, &mut remaining, "CID multihash code").await?;
    let hash_size = read_section_varint(reader, &mut remaining, "CID multihash length").await?;
    if version != CID_VERSION || codec != RAW_CODEC || hash_code != BLAKE3_MULTIHASH || hash_size != HASH_SIZE {
        return Err(SyncwebError::InvalidConfig(
            "drop contains an unsupported content identifier".to_owned(),
        ));
    }
    let mut hash_bytes = [0_u8; 32];
    read_section_bytes(reader, &mut remaining, &mut hash_bytes, "CID digest").await?;
    Ok(Some(SectionHeader {
        hash: Hash::from_bytes(hash_bytes),
        payload_size: remaining,
    }))
}

async fn read_required_varint<R>(reader: &mut R, name: &'static str) -> Result<u64>
where
    R: AsyncRead + Unpin,
{
    read_varint(reader, false, name)
        .await?
        .ok_or_else(|| SyncwebError::InvalidConfig(format!("drop archive ended before {name}")))
}

async fn read_varint<R>(reader: &mut R, allow_eof: bool, name: &'static str) -> Result<Option<u64>>
where
    R: AsyncRead + Unpin,
{
    let mut byte = [0_u8; 1];
    let count = reader
        .read(&mut byte)
        .await
        .map_err(|error| SyncwebError::operation("failed to read drop archive", error))?;
    if count == 0 {
        if allow_eof {
            return Ok(None);
        }
        return Err(SyncwebError::InvalidConfig(format!("drop archive ended before {name}")));
    }

    let mut value = 0_u64;
    let mut shift = 0_u32;
    for index in 0..10 {
        if index > 0 {
            read_exact(reader, &mut byte, "failed to read drop archive").await?;
        }
        let current = byte[0];
        let bits = u64::from(current & 0x7f);
        if shift == 63 && bits > 1 {
            return Err(SyncwebError::InvalidConfig(format!("{name} varint is too large")));
        }
        value |= bits << shift;
        if current & 0x80 == 0 {
            return Ok(Some(value));
        }
        if index == 9 {
            return Err(SyncwebError::InvalidConfig(format!("{name} varint is too large")));
        }
        shift = shift
            .checked_add(7)
            .ok_or_else(|| SyncwebError::operation("CAR varint shift overflow", name))?;
    }
    Err(SyncwebError::InvalidConfig(format!("{name} varint is too large")))
}

async fn read_section_varint<R>(reader: &mut R, remaining: &mut u64, name: &'static str) -> Result<u64>
where
    R: AsyncRead + Unpin,
{
    let mut value = 0_u64;
    let mut shift = 0_u32;
    for index in 0..10 {
        let byte = read_section_byte(reader, remaining, name).await?;
        let bits = u64::from(byte & 0x7f);
        if shift == 63 && bits > 1 {
            return Err(SyncwebError::InvalidConfig(format!("{name} varint is too large")));
        }
        value |= bits << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        if index == 9 {
            return Err(SyncwebError::InvalidConfig(format!("{name} varint is too large")));
        }
        shift = shift
            .checked_add(7)
            .ok_or_else(|| SyncwebError::operation("CAR varint shift overflow", name))?;
    }
    Err(SyncwebError::InvalidConfig(format!("{name} varint is too large")))
}

async fn read_section_byte<R>(reader: &mut R, remaining: &mut u64, name: &'static str) -> Result<u8>
where
    R: AsyncRead + Unpin,
{
    if *remaining == 0 {
        return Err(SyncwebError::InvalidConfig(format!("CAR section ends before {name}")));
    }
    let mut byte = [0_u8; 1];
    read_exact(reader, &mut byte, "failed to read drop section").await?;
    *remaining = remaining
        .checked_sub(1)
        .ok_or_else(|| SyncwebError::operation("CAR section length underflow", name))?;
    Ok(byte[0])
}

async fn read_section_bytes<R>(reader: &mut R, remaining: &mut u64, bytes: &mut [u8], name: &'static str) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    let length = u64::try_from(bytes.len())
        .map_err(|error| SyncwebError::operation("CAR field length is not representable", error))?;
    if length > *remaining {
        return Err(SyncwebError::InvalidConfig(format!("CAR section ends before {name}")));
    }
    read_exact(reader, bytes, "failed to read drop section").await?;
    *remaining = remaining
        .checked_sub(length)
        .ok_or_else(|| SyncwebError::operation("CAR section length underflow", name))?;
    Ok(())
}

async fn read_exact<R>(reader: &mut R, bytes: &mut [u8], context: &'static str) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    reader
        .read_exact(bytes)
        .await
        .map(|_| ())
        .map_err(|error| SyncwebError::operation(context, error))
}

fn parse_car_header(header: &[u8]) -> Result<Hash> {
    let mut offset = 0_usize;
    if read_slice_byte(header, &mut offset, "CAR root field")? != 0x0a {
        return Err(SyncwebError::InvalidConfig(
            "CAR header does not contain a root CID".to_owned(),
        ));
    }
    let cid_length = usize::try_from(read_slice_varint(header, &mut offset, "CAR root CID length")?)
        .map_err(|error| SyncwebError::operation("CAR root CID length is not representable", error))?;
    let cid_end = offset
        .checked_add(cid_length)
        .ok_or_else(|| SyncwebError::operation("CAR root CID range overflow", "usize limit exceeded"))?;
    let cid = header
        .get(offset..cid_end)
        .ok_or_else(|| SyncwebError::InvalidConfig("CAR root CID is truncated".to_owned()))?;
    offset = cid_end;
    if read_slice_byte(header, &mut offset, "CAR version field")? != 0x10 {
        return Err(SyncwebError::InvalidConfig(
            "CAR header does not contain a version".to_owned(),
        ));
    }
    if read_slice_varint(header, &mut offset, "CAR version")? != CAR_VERSION || offset != header.len() {
        return Err(SyncwebError::InvalidConfig(
            "unsupported or malformed CAR header".to_owned(),
        ));
    }
    parse_cid(cid)
}

fn parse_cid(cid: &[u8]) -> Result<Hash> {
    let mut offset = 0_usize;
    let version = read_slice_varint(cid, &mut offset, "CID version")?;
    let codec = read_slice_varint(cid, &mut offset, "CID codec")?;
    let hash_code = read_slice_varint(cid, &mut offset, "CID multihash code")?;
    let hash_size = read_slice_varint(cid, &mut offset, "CID multihash length")?;
    if version != CID_VERSION || codec != RAW_CODEC || hash_code != BLAKE3_MULTIHASH || hash_size != HASH_SIZE {
        return Err(SyncwebError::InvalidConfig(
            "drop contains an unsupported root content identifier".to_owned(),
        ));
    }
    let hash_end = offset
        .checked_add(
            usize::try_from(HASH_SIZE)
                .map_err(|error| SyncwebError::operation("CID hash length is not representable", error))?,
        )
        .ok_or_else(|| SyncwebError::operation("CID hash range overflow", "usize limit exceeded"))?;
    let digest = cid
        .get(offset..hash_end)
        .ok_or_else(|| SyncwebError::InvalidConfig("CID digest is truncated".to_owned()))?;
    if hash_end != cid.len() {
        return Err(SyncwebError::InvalidConfig("CID contains trailing bytes".to_owned()));
    }
    let hash_bytes: [u8; 32] = digest
        .try_into()
        .map_err(|error| SyncwebError::InvalidConfig(format!("CID digest must be 32 bytes: {error}")))?;
    Ok(Hash::from_bytes(hash_bytes))
}

fn read_slice_byte(bytes: &[u8], offset: &mut usize, name: &'static str) -> Result<u8> {
    let byte = bytes
        .get(*offset)
        .copied()
        .ok_or_else(|| SyncwebError::InvalidConfig(format!("{name} is truncated")))?;
    *offset = offset
        .checked_add(1)
        .ok_or_else(|| SyncwebError::operation("CAR offset overflow", name))?;
    Ok(byte)
}

fn read_slice_varint(bytes: &[u8], offset: &mut usize, name: &'static str) -> Result<u64> {
    let mut value = 0_u64;
    let mut shift = 0_u32;
    for index in 0..10 {
        let byte = read_slice_byte(bytes, offset, name)?;
        let bits = u64::from(byte & 0x7f);
        if shift == 63 && bits > 1 {
            return Err(SyncwebError::InvalidConfig(format!("{name} varint is too large")));
        }
        value |= bits << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        if index == 9 {
            return Err(SyncwebError::InvalidConfig(format!("{name} varint is too large")));
        }
        shift = shift
            .checked_add(7)
            .ok_or_else(|| SyncwebError::operation("CAR varint shift overflow", name))?;
    }
    Err(SyncwebError::InvalidConfig(format!("{name} varint is too large")))
}

async fn remove_staging(path: &Path) -> Result<()> {
    match fs::remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(SyncwebError::operation(
            "failed to remove drop staging directory",
            error,
        )),
    }
}
