use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use async_compression::tokio::bufread::ZstdDecoder;
use iroh_blobs::Hash;
use tokio::{
    fs,
    io::{AsyncRead, AsyncReadExt, BufReader},
};
use uuid::Uuid;

use crate::{
    error::{Result, SyncwebError},
    folder::CollectionManifest,
};

const CAR_VERSION: u64 = 1;
const CID_VERSION: u64 = 1;
const RAW_CODEC: u64 = 0x55;
const BLAKE3_MULTIHASH: u64 = 0x1e;
const HASH_SIZE: u64 = 32;
const MAX_HEADER_SIZE: u64 = 64 * 1024;
const MAX_MANIFEST_SIZE: u64 = 16 * 1024 * 1024;
const COPY_BUFFER_SIZE: usize = 64 * 1024;

/// The result of verifying a complete compressed drop archive.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DropVerifyResult {
    pub manifest: Hash,
    pub collection_id: Uuid,
    pub version: String,
    pub entry_count: usize,
    pub block_count: usize,
    pub bytes_verified: u64,
}

/// Compatibility name for [`DropVerifyResult`].
pub type DropVerificationResult = DropVerifyResult;

/// Streams and verifies a Zstandard-compressed CAR drop archive.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct DropVerifier;

impl DropVerifier {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Verify a compressed drop archive at a filesystem path.
    ///
    /// # Errors
    ///
    /// Returns an error if the archive is truncated, malformed, tampered with,
    /// or inconsistent with its manifest.
    pub async fn verify(&self, path: impl AsRef<Path>) -> Result<DropVerifyResult> {
        let file = fs::File::open(path)
            .await
            .map_err(|error| SyncwebError::operation("failed to open drop archive", error))?;
        self.verify_reader(file).await
    }

    /// Verify a compressed drop archive from an asynchronous reader.
    ///
    /// The reader is consumed incrementally; the compressed archive is never
    /// loaded into memory as a whole.
    ///
    /// # Errors
    ///
    /// Returns an error if the archive is truncated, malformed, tampered with,
    /// or inconsistent with its manifest.
    pub async fn verify_reader<R>(&self, reader: R) -> Result<DropVerifyResult>
    where
        R: AsyncRead + Unpin,
    {
        let decoder = ZstdDecoder::new(BufReader::new(reader));
        verify_car(decoder).await
    }
}

/// Verify a compressed drop archive at a filesystem path.
///
/// # Errors
///
/// Returns an error if the archive is truncated, malformed, tampered with, or
/// inconsistent with its manifest.
pub async fn verify_drop(path: impl AsRef<Path>) -> Result<DropVerifyResult> {
    DropVerifier::new().verify(path).await
}

/// Verify a compressed drop archive from an asynchronous reader.
///
/// # Errors
///
/// Returns an error if the archive is truncated, malformed, tampered with, or
/// inconsistent with its manifest.
pub async fn verify_drop_reader<R>(reader: R) -> Result<DropVerifyResult>
where
    R: AsyncRead + Unpin,
{
    DropVerifier::new().verify_reader(reader).await
}

async fn verify_car<R>(mut reader: R) -> Result<DropVerifyResult>
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

    let first_section = read_section_header(&mut reader)
        .await?
        .ok_or_else(|| SyncwebError::InvalidConfig("drop archive does not contain a manifest block".to_owned()))?;
    if first_section.hash != root {
        return Err(SyncwebError::InvalidConfig(
            "drop manifest CID does not match the CAR root".to_owned(),
        ));
    }
    if first_section.payload_size > MAX_MANIFEST_SIZE {
        return Err(SyncwebError::InvalidConfig(format!(
            "drop manifest exceeds {MAX_MANIFEST_SIZE} bytes"
        )));
    }
    let (manifest_hash, manifest_data, manifest_size) =
        read_payload(&mut reader, first_section.payload_size, true).await?;
    if manifest_hash != root {
        return Err(SyncwebError::InvalidConfig(
            "drop manifest hash does not match the CAR root".to_owned(),
        ));
    }
    let manifest_bytes = manifest_data
        .ok_or_else(|| SyncwebError::InvalidConfig("drop manifest was not buffered for verification".to_owned()))?;
    let manifest = CollectionManifest::from_bytes(&manifest_bytes)?;
    manifest.verify_signature()?;
    if manifest.blob_id()? != root {
        return Err(SyncwebError::InvalidConfig(
            "drop manifest hash does not match its serialized content".to_owned(),
        ));
    }

    let mut expected = BTreeMap::new();
    for entry in &manifest.entries {
        if let Some(previous_size) = expected.insert(entry.content_id, entry.size)
            && previous_size != entry.size
        {
            return Err(SyncwebError::InvalidConfig(format!(
                "manifest uses content hash {} with inconsistent sizes",
                entry.content_id
            )));
        }
    }

    let mut seen = BTreeSet::new();
    let mut block_count = 1_usize;
    let mut bytes_verified = manifest_size;
    while let Some(section) = read_section_header(&mut reader).await? {
        let expected_size = expected.get(&section.hash).ok_or_else(|| {
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
        if section.payload_size != *expected_size {
            return Err(SyncwebError::InvalidConfig(format!(
                "drop content size does not match the manifest for {}",
                section.hash
            )));
        }
        let (actual_hash, _, payload_size) = read_payload(&mut reader, section.payload_size, false).await?;
        if actual_hash != section.hash {
            return Err(SyncwebError::InvalidConfig(format!(
                "drop content hash does not match its CID: expected {}, got {}",
                section.hash, actual_hash
            )));
        }
        bytes_verified = bytes_verified
            .checked_add(payload_size)
            .ok_or_else(|| SyncwebError::operation("verified drop size overflow", "u64 limit exceeded"))?;
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

    Ok(DropVerifyResult {
        manifest: root,
        collection_id: manifest.collection_id,
        version: manifest.version,
        entry_count: manifest.entries.len(),
        block_count,
        bytes_verified,
    })
}

struct SectionHeader {
    hash: Hash,
    payload_size: u64,
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

async fn read_payload<R>(reader: &mut R, payload_size: u64, collect: bool) -> Result<(Hash, Option<Vec<u8>>, u64)>
where
    R: AsyncRead + Unpin,
{
    let mut remaining = payload_size;
    let mut bytes_read = 0_u64;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0_u8; COPY_BUFFER_SIZE];
    let mut collected = if collect {
        let capacity = usize::try_from(payload_size)
            .map_err(|error| SyncwebError::operation("drop manifest is too large", error))?;
        Some(Vec::with_capacity(capacity))
    } else {
        None
    };
    while remaining > 0 {
        let requested_u64 = remaining.min(
            u64::try_from(buffer.len())
                .map_err(|error| SyncwebError::operation("drop read buffer length is not representable", error))?,
        );
        let requested = usize::try_from(requested_u64)
            .map_err(|error| SyncwebError::operation("drop section length is not representable", error))?;
        let chunk = buffer
            .get_mut(..requested)
            .ok_or_else(|| SyncwebError::operation("drop read buffer range is invalid", requested))?;
        read_exact(reader, chunk, "drop section").await?;
        hasher.update(chunk);
        if let Some(bytes) = &mut collected {
            bytes.extend_from_slice(chunk);
        }
        let requested_bytes = u64::try_from(requested)
            .map_err(|error| SyncwebError::operation("drop section length is not representable", error))?;
        remaining = remaining
            .checked_sub(requested_bytes)
            .ok_or_else(|| SyncwebError::operation("drop section length underflow", "invalid section length"))?;
        bytes_read = bytes_read
            .checked_add(requested_bytes)
            .ok_or_else(|| SyncwebError::operation("verified drop size overflow", "u64 limit exceeded"))?;
    }
    Ok((Hash::from_bytes(*hasher.finalize().as_bytes()), collected, bytes_read))
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
    byte.first()
        .copied()
        .ok_or_else(|| SyncwebError::operation("CAR byte is missing", name))
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
        let current = if index == 0 {
            byte.first()
                .copied()
                .ok_or_else(|| SyncwebError::operation("drop archive varint byte is missing", name))?
        } else {
            read_exact(reader, &mut byte, "failed to read drop archive").await?;
            byte.first()
                .copied()
                .ok_or_else(|| SyncwebError::operation("drop archive varint byte is missing", name))?
        };
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
    let root_marker = read_slice_byte(header, &mut offset, "CAR root field")?;
    if root_marker != 0x0a {
        return Err(SyncwebError::InvalidConfig(
            "CAR header does not contain a root CID".to_owned(),
        ));
    }
    let cid_length_raw = read_slice_varint(header, &mut offset, "CAR root CID length")?;
    let cid_length = usize::try_from(cid_length_raw)
        .map_err(|error| SyncwebError::operation("CAR root CID length is not representable", error))?;
    let cid_end = offset
        .checked_add(cid_length)
        .ok_or_else(|| SyncwebError::operation("CAR root CID range overflow", "usize limit exceeded"))?;
    let cid = header
        .get(offset..cid_end)
        .ok_or_else(|| SyncwebError::InvalidConfig("CAR root CID is truncated".to_owned()))?;
    offset = cid_end;
    let version_marker = read_slice_byte(header, &mut offset, "CAR version field")?;
    if version_marker != 0x10 {
        return Err(SyncwebError::InvalidConfig(
            "CAR header does not contain a version".to_owned(),
        ));
    }
    let version = read_slice_varint(header, &mut offset, "CAR version")?;
    if version != CAR_VERSION || offset != header.len() {
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
    let hash: [u8; 32] = digest
        .try_into()
        .map_err(|error| SyncwebError::InvalidConfig(format!("CID digest must be 32 bytes: {error}")))?;
    Ok(Hash::from_bytes(hash))
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
