mod test_utils;

use std::{
    io,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};

use anyhow::{Context as _, Result};
use async_compression::tokio::write::ZstdEncoder;
use iroh_blobs::Hash;
use syncweb_core::{
    DropVerifier,
    folder::{CollectionEntry, CollectionManifest},
    verify_archive,
};
use tokio::{
    fs,
    io::{AsyncRead, AsyncWriteExt, ReadBuf},
};

use crate::test_utils::TestDirectory;

fn manifest(data: &[u8]) -> Result<CollectionManifest> {
    let mut manifest = CollectionManifest::new(uuid::Uuid::new_v4(), "1.0.0");
    manifest.entries.push(CollectionEntry::new(
        Hash::new(data),
        "file.txt",
        u64::try_from(data.len())?,
    )?);
    Ok(manifest)
}

fn encode_varint(mut value: u64, output: &mut Vec<u8>) {
    while value >= 0x80 {
        output.push(value.to_le_bytes()[0] | 0x80);
        value >>= 7;
    }
    output.push(value.to_le_bytes()[0]);
}

fn cid(hash: Hash) -> Vec<u8> {
    let mut value = Vec::new();
    encode_varint(1, &mut value);
    encode_varint(0x55, &mut value);
    encode_varint(0x1e, &mut value);
    encode_varint(32, &mut value);
    value.extend_from_slice(hash.as_bytes());
    value
}

fn car_header(root: Hash) -> Vec<u8> {
    let mut header = Vec::new();
    header.push(0x0a);
    let root_cid = cid(root);
    encode_varint(u64::try_from(root_cid.len()).unwrap_or_default(), &mut header);
    header.extend_from_slice(&root_cid);
    header.push(0x10);
    encode_varint(1, &mut header);

    let mut car = Vec::new();
    encode_varint(u64::try_from(header.len()).unwrap_or_default(), &mut car);
    car.extend_from_slice(&header);
    car
}

fn section(hash: Hash, payload: &[u8]) -> Vec<u8> {
    let content_id = cid(hash);
    let section_len = content_id.len().checked_add(payload.len()).unwrap_or_default();
    let section_size = u64::try_from(section_len).unwrap_or_default();
    let mut section_bytes = Vec::new();
    encode_varint(section_size, &mut section_bytes);
    section_bytes.extend_from_slice(&content_id);
    section_bytes.extend_from_slice(payload);
    section_bytes
}

fn car(manifest: &CollectionManifest, payload: &[u8], root: Option<Hash>) -> Result<Vec<u8>> {
    let manifest_bytes = manifest.to_bytes()?;
    let manifest_hash = Hash::new(&manifest_bytes);
    let mut archive = car_header(root.unwrap_or(manifest_hash));
    archive.extend(section(manifest_hash, &manifest_bytes));
    let content_hash = manifest
        .entries
        .first()
        .map_or_else(|| Hash::new(payload), |entry| entry.content_id);
    archive.extend(section(content_hash, payload));
    Ok(archive)
}

async fn compress(path: &Path, bytes: &[u8]) -> Result<()> {
    let file = fs::File::create(path).await?;
    let mut encoder = ZstdEncoder::new(file);
    encoder.write_all(bytes).await?;
    encoder.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn test_drop_verify_valid_archive_streams() -> Result<()> {
    let directory = TestDirectory::new("syncweb-drop-verify-test")?;
    let payload = b"content";
    let manifest = manifest(payload)?;
    let archive = car(&manifest, payload, None)?;
    let path = directory.path().join("valid.car.zst");
    compress(&path, &archive).await?;

    let result = verify_archive(&path).await?;
    anyhow::ensure!(result.manifest == Hash::new(&manifest.to_bytes()?));
    anyhow::ensure!(result.entry_count == 1);
    anyhow::ensure!(result.block_count == 2);
    Ok(())
}

#[tokio::test]
async fn test_drop_tamper_detection() -> Result<()> {
    let directory = TestDirectory::new("syncweb-drop-verify-test")?;
    let payload = b"content";
    let manifest = manifest(payload)?;
    let archive = car(&manifest, b"tamper!", None)?;
    let path = directory.path().join("tampered.car.zst");
    compress(&path, &archive).await?;

    let error = DropVerifier::new()
        .verify(&path)
        .await
        .expect_err("tampered content must fail");
    anyhow::ensure!(error.to_string().contains("hash"));
    Ok(())
}

#[tokio::test]
async fn test_drop_manifest_mismatch() -> Result<()> {
    let directory = TestDirectory::new("syncweb-drop-verify-test")?;
    let payload = b"content";
    let manifest = manifest(payload)?;
    let archive = car(&manifest, payload, Some(Hash::new(b"wrong root")))?;
    let path = directory.path().join("mismatched.car.zst");
    compress(&path, &archive).await?;

    let error = verify_archive(&path).await.expect_err("wrong root must fail");
    anyhow::ensure!(error.to_string().contains("manifest"));
    Ok(())
}

#[tokio::test]
async fn test_drop_dos_protection_rejects_large_varint_without_allocation() -> Result<()> {
    let directory = TestDirectory::new("syncweb-drop-verify-test")?;
    let manifest = CollectionManifest::new(uuid::Uuid::new_v4(), "1.0.0");
    let manifest_bytes = manifest.to_bytes()?;
    let manifest_hash = Hash::new(&manifest_bytes);
    let mut archive = car_header(manifest_hash);
    archive.extend(section(manifest_hash, &manifest_bytes));
    encode_varint(u64::MAX, &mut archive);
    let path = directory.path().join("large-varint.car.zst");
    compress(&path, &archive).await?;

    let error = verify_archive(&path).await.expect_err("large section must fail");
    anyhow::ensure!(error.to_string().contains("section"));
    Ok(())
}

#[tokio::test]
async fn test_drop_streaming_integrity() -> Result<()> {
    let payload = b"content";
    let manifest = manifest(payload)?;
    let archive = car(&manifest, payload, None)?;
    let mut compressed = Vec::new();
    {
        let mut encoder = ZstdEncoder::new(&mut compressed);
        encoder.write_all(&archive).await?;
        encoder.shutdown().await?;
    }

    let result = DropVerifier::new()
        .verify_reader(ChunkedReader::new(compressed))
        .await?;
    anyhow::ensure!(result.manifest == Hash::new(&manifest.to_bytes()?));
    anyhow::ensure!(result.collection_id == manifest.collection_id);
    anyhow::ensure!(result.version == "1.0.0");
    anyhow::ensure!(result.entry_count == 1);
    anyhow::ensure!(result.block_count == 2);
    let verified_len = manifest
        .to_bytes()?
        .len()
        .checked_add(payload.len())
        .context("verified length overflow")?;
    anyhow::ensure!(result.bytes_verified == u64::try_from(verified_len)?);
    Ok(())
}

struct ChunkedReader {
    bytes: Vec<u8>,
    offset: usize,
}

impl ChunkedReader {
    const fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, offset: 0 }
    }
}

impl AsyncRead for ChunkedReader {
    fn poll_read(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buffer: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        if self.offset == self.bytes.len() {
            return Poll::Ready(Ok(()));
        }
        let Some(remaining) = self.bytes.len().checked_sub(self.offset) else {
            return Poll::Ready(Err(io::Error::other("reader offset exceeds input")));
        };
        let count = remaining.min(3).min(buffer.remaining());
        let Some(end) = self.offset.checked_add(count) else {
            return Poll::Ready(Err(io::Error::other("reader offset overflows")));
        };
        let chunk = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| io::Error::other("chunk outside reader"))?
            .to_owned();
        buffer.put_slice(&chunk);
        self.offset = end;
        Poll::Ready(Ok(()))
    }
}
