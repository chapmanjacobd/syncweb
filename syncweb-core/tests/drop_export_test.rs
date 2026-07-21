use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use async_compression::tokio::bufread::ZstdDecoder;
use syncweb_core::{
    filter::{FilterAction, FilterConfig, FilterEngine, FilterRule, MatchCriteria},
    folder::{CollectionEntry, CollectionManifest, DropExportOptions, DropExporter},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
};
use tokio::io::{AsyncReadExt, BufReader};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Result<Self> {
        let path = std::env::temp_dir().join(format!("syncweb-drop-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("failed to remove test directory {}: {error}", self.0.display());
        }
    }
}

async fn test_node(directory: &TestDirectory) -> Result<IrohNode> {
    let root = directory.path().join("node");
    let identity = IdentityManager::new(root.join("identity.key"))?;
    Ok(IrohNode::new(identity, root.join("data"), RelayMode::Default).await?)
}

fn manifest(collection_id: uuid::Uuid, version: &str, entries: &[(&str, &[u8])]) -> Result<CollectionManifest> {
    let mut manifest = CollectionManifest::new(collection_id, version);
    for (path, data) in entries {
        manifest.entries.push(CollectionEntry::new(
            iroh_blobs::Hash::new(data),
            *path,
            u64::try_from(data.len())?,
        )?);
    }
    Ok(manifest)
}

async fn read_archive(path: &Path) -> Result<Vec<(iroh_blobs::Hash, Vec<u8>)>> {
    let file = tokio::fs::File::open(path).await?;
    let mut decoder = ZstdDecoder::new(BufReader::new(file));
    let mut bytes = Vec::new();
    decoder.read_to_end(&mut bytes).await?;
    let mut offset = 0;
    let header_size = read_varint(&bytes, &mut offset)?;
    offset = offset
        .checked_add(usize::try_from(header_size)?)
        .context("CAR header exceeds archive")?;
    let mut blocks = Vec::new();
    while offset < bytes.len() {
        let section_size = usize::try_from(read_varint(&bytes, &mut offset)?)?;
        let section_end = offset.checked_add(section_size).context("CAR section overflows")?;
        anyhow::ensure!(section_end <= bytes.len(), "CAR section exceeds archive");
        let section = bytes
            .get(offset..section_end)
            .context("CAR section is outside archive")?;
        let mut cid_offset = 0;
        let hash = read_cid_hash(section, &mut cid_offset)?;
        blocks.push((
            hash,
            section.get(cid_offset..).context("CAR CID exceeds section")?.to_vec(),
        ));
        offset = section_end;
    }
    Ok(blocks)
}

fn read_cid_hash(section: &[u8], offset: &mut usize) -> Result<iroh_blobs::Hash> {
    for _ in 0..4 {
        let _ = read_varint(section, offset)?;
    }
    let end = offset.checked_add(32).context("invalid CAR hash")?;
    anyhow::ensure!(end <= section.len(), "CAR CID exceeds section");
    let bytes: [u8; 32] = section
        .get(*offset..end)
        .context("CAR hash exceeds section")?
        .try_into()?;
    *offset = end;
    Ok(iroh_blobs::Hash::from_bytes(bytes))
}

fn read_varint(bytes: &[u8], offset: &mut usize) -> Result<u64> {
    let mut value = 0_u64;
    let mut shift = 0_u32;
    loop {
        let byte = *bytes.get(*offset).context("truncated CAR varint")?;
        *offset = (*offset).checked_add(1).context("CAR offset overflows")?;
        value |= u64::from(byte & 0x7f)
            .checked_shl(shift)
            .context("CAR varint is too large")?;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        shift = shift.checked_add(7).context("CAR varint is too large")?;
        anyhow::ensure!(shift < 64, "CAR varint is too large");
    }
}

#[tokio::test]
async fn test_export_drop_basic_and_empty_package() -> Result<()> {
    let directory = TestDirectory::new()?;
    let node = test_node(&directory).await?;
    let collection_id = uuid::Uuid::new_v4();
    let files = [("README.md", b"hello".as_slice()), ("empty", b"".as_slice())];
    let collection_manifest = manifest(collection_id, "1.0.0", &files)?;
    for (_, data) in files {
        node.blob_store().add_bytes(data).await?;
    }
    let output = directory.path().join("package.car.zst");
    let result = DropExporter::new(node.blob_store().clone())
        .export_drop(&collection_manifest, &output)
        .await?;
    anyhow::ensure!(result.entry_count == 2);
    anyhow::ensure!(result.block_count == 3);
    let blocks = read_archive(&output).await?;
    anyhow::ensure!(blocks.len() == 3);
    let first_block = blocks.first().context("CAR has no manifest block")?;
    anyhow::ensure!(first_block.0 == result.manifest);
    anyhow::ensure!(CollectionManifest::from_bytes(&first_block.1)? == collection_manifest);

    let empty_manifest = CollectionManifest::new(uuid::Uuid::new_v4(), "1.0.0");
    let empty_output = directory.path().join("empty.car.zst");
    let empty_result = DropExporter::new(node.blob_store().clone())
        .export_drop(&empty_manifest, &empty_output)
        .await?;
    anyhow::ensure!(empty_result.entry_count == 0);
    anyhow::ensure!(read_archive(&empty_output).await?.len() == 1);
    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_export_drop_filter_and_version_selection() -> Result<()> {
    let directory = TestDirectory::new()?;
    let node = test_node(&directory).await?;
    let collection_id = uuid::Uuid::new_v4();
    let v1_files = [("old.txt", b"old".as_slice())];
    let v2_files = [("keep.txt", b"keep".as_slice()), ("video.mp4", b"video".as_slice())];
    let v1 = manifest(collection_id, "1.0.0", &v1_files)?;
    let v2 = manifest(collection_id, "2.0.0", &v2_files)?;
    for (_, data) in v1_files.into_iter().chain(v2_files) {
        node.blob_store().add_bytes(data).await?;
    }
    let mut config = FilterConfig::default();
    let mut criteria = MatchCriteria::default();
    criteria.extensions = Some(vec!["mp4".to_owned()]);
    config.rules = vec![FilterRule::new(FilterAction::Reject, criteria)];
    let filter = FilterEngine::new(config)?;
    let output = directory.path().join("filtered.car.zst");
    DropExporter::new(node.blob_store().clone())
        .export_manifests(
            &[v1, v2],
            &output,
            DropExportOptions::default().with_version("2.0.0").with_filter(filter),
        )
        .await?;
    let blocks = read_archive(&output).await?;
    let first_block = blocks.first().context("CAR has no manifest block")?;
    let filtered = CollectionManifest::from_bytes(&first_block.1)?;
    anyhow::ensure!(filtered.version == "2.0.0");
    anyhow::ensure!(filtered.entries.len() == 1);
    anyhow::ensure!(
        filtered
            .entries
            .first()
            .context("filtered manifest has no entries")?
            .logical_path
            == Path::new("keep.txt")
    );
    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_export_drop_concurrent_replacement_is_valid() -> Result<()> {
    let directory = TestDirectory::new()?;
    let node = test_node(&directory).await?;
    let files = [("file.txt", b"content".as_slice())];
    let manifest = manifest(uuid::Uuid::new_v4(), "1.0.0", &files)?;
    node.blob_store().add_bytes(b"content").await?;
    let exporter = DropExporter::new(node.blob_store().clone());
    let output = directory.path().join("concurrent.car.zst");
    let (first, second) = tokio::join!(
        exporter.export_drop(&manifest, &output),
        exporter.export_drop(&manifest, &output)
    );
    first?;
    second?;
    let blocks = read_archive(&output).await?;
    anyhow::ensure!(blocks.len() == 2);
    anyhow::ensure!(
        CollectionManifest::from_bytes(&blocks.first().context("CAR has no manifest block")?.1)?.version == "1.0.0"
    );
    node.stop().await?;
    Ok(())
}
