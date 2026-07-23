use std::{collections::BTreeMap, fs};

use anyhow::{Result, ensure};
use async_compression::tokio::bufread::ZstdDecoder;
use ed25519_dalek::SigningKey;
use syncweb_core::{
    daemon::ManagedPool,
    filter::{FilterAction, FilterConfig, FilterEngine, FilterRule, MatchCriteria},
    folder::{CollectionEntry, CollectionManifest, DropExporter, DropImportOptions, DropImporter},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use crate::test_utils::TestDirectory;

async fn node(directory: &TestDirectory, name: &str) -> Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    Ok(IrohNode::new(identity, root.join("data"), RelayMode::Default).await?)
}

#[tokio::test]
async fn import_drop_roundtrip_and_materialization() -> Result<()> {
    let directory = TestDirectory::new("syncweb-drop-import-test")?;
    let source = node(&directory, "source").await?;
    let destination = node(&directory, "destination").await?;
    let content = b"import me";
    let hash = source.blob_store().add_bytes(content).await?;
    let mut manifest = CollectionManifest::new(uuid::Uuid::new_v4(), "1.0.0");
    manifest.entries.push(CollectionEntry::new(
        hash,
        "nested/file.txt",
        u64::try_from(content.len())?,
    )?);
    let archive = directory.path().join("package.car.zst");

    DropExporter::new(source.blob_store().clone())
        .export_archive(&manifest, &archive)
        .await?;
    let importer = DropImporter::new(destination.blob_store().clone());
    let result = importer
        .import_archive(&archive, DropImportOptions::default(), None)
        .await?;
    ensure!(result.collection_manifest == manifest);
    ensure!(destination.blob_store().has(hash).await?);

    let target = directory.path().join("materialized");
    importer.materialize(&result, &target).await?;
    ensure!(fs::read(target.join("nested/file.txt"))? == content);

    source.stop().await?;
    destination.stop().await?;
    Ok(())
}

#[tokio::test]
async fn import_drop_filter_skips_rejected_content() -> Result<()> {
    let directory = TestDirectory::new("syncweb-drop-import-test")?;
    let source = node(&directory, "source").await?;
    let destination = node(&directory, "destination").await?;
    let keep = b"keep";
    let reject = b"reject";
    let keep_hash = source.blob_store().add_bytes(keep).await?;
    let reject_hash = source.blob_store().add_bytes(reject).await?;
    let mut manifest = CollectionManifest::new(uuid::Uuid::new_v4(), "1.0.0");
    manifest
        .entries
        .push(CollectionEntry::new(keep_hash, "keep.txt", u64::try_from(keep.len())?)?);
    manifest.entries.push(CollectionEntry::new(
        reject_hash,
        "skip.mp4",
        u64::try_from(reject.len())?,
    )?);
    let archive = directory.path().join("package.car.zst");
    DropExporter::new(source.blob_store().clone())
        .export_archive(&manifest, &archive)
        .await?;

    let mut criteria = MatchCriteria::default();
    criteria.extensions = Some(vec!["mp4".to_owned()]);
    let mut filter_config = FilterConfig::default();
    filter_config.rules = vec![FilterRule::new(FilterAction::Reject, criteria)];
    let filter = FilterEngine::new(filter_config)?;
    let result = DropImporter::new(destination.blob_store().clone())
        .import_archive(&archive, DropImportOptions::default().with_filter(filter), None)
        .await?;
    ensure!(result.imported_entry_count == 1);
    ensure!(result.skipped_entry_count == 1);
    ensure!(destination.blob_store().has(keep_hash).await?);
    ensure!(!destination.blob_store().has(reject_hash).await?);

    source.stop().await?;
    destination.stop().await?;
    Ok(())
}

#[tokio::test]
async fn import_drop_corrupted_archive_fails() -> Result<()> {
    let directory = TestDirectory::new("syncweb-drop-import-test")?;
    let destination = node(&directory, "destination").await?;
    let garbage = directory.path().join("corrupted.car.zst");
    fs::write(&garbage, b"this is not a valid CAR archive")?;

    let result = DropImporter::new(destination.blob_store().clone())
        .import_archive(&garbage, DropImportOptions::default(), None)
        .await;
    ensure!(result.is_err(), "corrupted archive must be rejected");

    destination.stop().await?;
    Ok(())
}

#[tokio::test]
async fn import_drop_tampered_content_fails() -> Result<()> {
    let directory = TestDirectory::new("syncweb-drop-import-test")?;
    let source = node(&directory, "source").await?;
    let destination = node(&directory, "destination").await?;
    let content = b"original content";
    let hash = source.blob_store().add_bytes(content).await?;
    let mut manifest = CollectionManifest::new(Uuid::new_v4(), "1.0.0");
    manifest
        .entries
        .push(CollectionEntry::new(hash, "file.txt", u64::try_from(content.len())?)?);
    let archive = directory.path().join("package.car.zst");
    DropExporter::new(source.blob_store().clone())
        .export_archive(&manifest, &archive)
        .await?;

    let bytes = fs::read(&archive)?;
    let mut tampered = bytes;
    if let Some(last) = tampered.last_mut() {
        *last = last.wrapping_add(1);
    }
    fs::write(&archive, &tampered)?;

    let result = DropImporter::new(destination.blob_store().clone())
        .import_archive(&archive, DropImportOptions::default(), None)
        .await;
    ensure!(result.is_err(), "tampered content must be rejected");

    source.stop().await?;
    destination.stop().await?;
    Ok(())
}

#[tokio::test]
async fn import_drop_invalid_signature_fails() -> Result<()> {
    let directory = TestDirectory::new("syncweb-drop-import-test")?;
    let source = node(&directory, "source").await?;
    let destination = node(&directory, "destination").await?;
    let content = b"signed content";
    let hash = source.blob_store().add_bytes(content).await?;
    let mut manifest = CollectionManifest::new(Uuid::new_v4(), "1.0.0");
    manifest
        .entries
        .push(CollectionEntry::new(hash, "file.txt", u64::try_from(content.len())?)?);

    let signing_key = SigningKey::from_bytes(&[42; 32]);
    manifest.sign(&signing_key)?;

    let archive = directory.path().join("signed.car.zst");
    DropExporter::new(source.blob_store().clone())
        .export_archive(&manifest, &archive)
        .await?;

    let compressed = fs::read(&archive)?;
    let mut decoder = ZstdDecoder::new(compressed.as_slice());
    let mut raw_bytes = Vec::new();
    decoder.read_to_end(&mut raw_bytes).await?;

    let manifest_json = serde_json::to_string(&manifest)?;
    let bad_key = SigningKey::from_bytes(&[99; 32]);
    let mut tampered_manifest = manifest.clone();
    tampered_manifest.sign(&bad_key)?;
    let bad_json = serde_json::to_string(&tampered_manifest)?;

    if let Some(pos) = raw_bytes
        .windows(manifest_json.len())
        .position(|window| window == manifest_json.as_bytes())
    {
        raw_bytes.splice(pos..pos + manifest_json.len(), bad_json.bytes());
    }

    let mut recompressed = Vec::new();
    {
        let mut encoder = async_compression::tokio::write::ZstdEncoder::new(&mut recompressed);
        encoder.write_all(&raw_bytes).await?;
        encoder.shutdown().await?;
    }
    fs::write(&archive, &recompressed)?;

    let result = DropImporter::new(destination.blob_store().clone())
        .import_archive(&archive, DropImportOptions::default(), None)
        .await;
    ensure!(result.is_err(), "invalid signature must be rejected");

    source.stop().await?;
    destination.stop().await?;
    Ok(())
}

#[tokio::test]
async fn import_drop_missing_dependencies_fails() -> Result<()> {
    let directory = TestDirectory::new("syncweb-drop-import-test")?;
    let source = node(&directory, "source").await?;
    let destination = node(&directory, "destination").await?;
    let content = b"needs deps";
    let hash = source.blob_store().add_bytes(content).await?;
    let dep_id = Uuid::new_v4();
    let mut manifest = CollectionManifest::new(Uuid::new_v4(), "1.0.0");
    manifest
        .entries
        .push(CollectionEntry::new(hash, "file.txt", u64::try_from(content.len())?)?);
    manifest.package = Some(
        syncweb_core::folder::PackageProfile::new("app")
            .with_dependency(syncweb_core::folder::PackageDependency::new(dep_id, "^2.0")),
    );

    let archive = directory.path().join("deps.car.zst");
    DropExporter::new(source.blob_store().clone())
        .export_archive(&manifest, &archive)
        .await?;

    let available = BTreeMap::new();
    let result = DropImporter::new(destination.blob_store().clone())
        .import_archive(
            &archive,
            DropImportOptions::default().with_available_dependencies(available),
            None,
        )
        .await;
    ensure!(result.is_err(), "missing dependencies must cause import failure");
    let error_msg = result.unwrap_err().to_string();
    ensure!(
        error_msg.contains("dependencies"),
        "error should mention dependencies: {error_msg}"
    );

    source.stop().await?;
    destination.stop().await?;
    Ok(())
}

#[tokio::test]
async fn import_drop_with_pool_matches_without_pool() -> Result<()> {
    let directory = TestDirectory::new("syncweb-drop-import-test")?;
    let source = node(&directory, "source").await?;
    let plain_destination = node(&directory, "plain-destination").await?;
    let pooled_destination = node(&directory, "pooled-destination").await?;
    let content = b"pooled import";
    let hash = source.blob_store().add_bytes(content).await?;
    let mut manifest = CollectionManifest::new(Uuid::new_v4(), "1.0.0");
    manifest
        .entries
        .push(CollectionEntry::new(hash, "pooled.txt", u64::try_from(content.len())?)?);
    let archive = directory.path().join("pooled.car.zst");
    DropExporter::new(source.blob_store().clone())
        .export_archive(&manifest, &archive)
        .await?;

    let plain = DropImporter::new(plain_destination.blob_store().clone())
        .import_archive(&archive, DropImportOptions::default(), None)
        .await?;
    let pool = ManagedPool::new("archive-test", 1)?;
    let pooled = DropImporter::new(pooled_destination.blob_store().clone())
        .import_archive(&archive, DropImportOptions::default(), Some(&pool))
        .await?;
    ensure!(plain == pooled);
    ensure!(pooled_destination.blob_store().has(hash).await?);

    source.stop().await?;
    plain_destination.stop().await?;
    pooled_destination.stop().await?;
    Ok(())
}
