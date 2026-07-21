use anyhow::ensure;
use std::path::PathBuf;

use iroh_blobs::Hash;
use syncweb_core::folder::{CollectionEntry, CollectionManifest, PackageAnnouncement, PackageManager};
use uuid::Uuid;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> anyhow::Result<Self> {
        let path = std::env::temp_dir().join(format!("syncweb-collection-{}", Uuid::new_v4()));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            eprintln!("failed to remove test directory {}: {error}", self.0.display());
        }
    }
}

fn manifest(collection_id: Uuid, version: &str, bytes: &[u8]) -> anyhow::Result<CollectionManifest> {
    let mut manifest = CollectionManifest::new(collection_id, version);
    manifest.entries.push(CollectionEntry::new(
        Hash::new(bytes),
        PathBuf::from("bin/tool"),
        u64::try_from(bytes.len())?,
    )?);
    Ok(manifest)
}

#[test]
fn manifest_round_trip_and_content_id_are_stable() -> anyhow::Result<()> {
    let collection_id = Uuid::new_v4();
    let manifest = manifest(collection_id, "1.0.0", b"v1")?;
    let decoded = CollectionManifest::from_bytes(manifest.to_bytes()?)?;

    anyhow::ensure!(decoded == manifest);
    anyhow::ensure!(decoded.content_id()? == manifest.content_id()?);
    Ok(())
}

#[test]
fn package_install_switch_and_verify_are_atomic() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let source_v1 = directory.0.join("source-v1");
    let source_v2 = directory.0.join("source-v2");
    std::fs::create_dir_all(source_v1.join("bin"))?;
    std::fs::create_dir_all(source_v2.join("bin"))?;
    std::fs::write(source_v1.join("bin/tool"), b"v1")?;
    std::fs::write(source_v2.join("bin/tool"), b"v2")?;

    let collection_id = Uuid::new_v4();
    let v1 = manifest(collection_id, "1.0.0", b"v1")?;
    let v2 = manifest(collection_id, "2.0.0", b"v2")?;
    let packages = PackageManager::new(directory.0.join("packages"));
    packages.install(&v1, &source_v1)?;
    packages.install(&v2, &source_v2)?;
    packages.switch(collection_id, "1.0.0")?;
    packages.verify(&v1)?;

    let current = packages.root().join(collection_id.to_string()).join("current/bin/tool");
    anyhow::ensure!(std::fs::read(current)? == b"v1");
    anyhow::ensure!(
        packages
            .state()?
            .current(collection_id)
            .is_some_and(|installed| installed.current == "1.0.0")
    );
    Ok(())
}

#[test]
fn manifests_reject_paths_that_escape_the_package_root() -> anyhow::Result<()> {
    let result = CollectionEntry::new(Hash::EMPTY, "../outside", 0);
    ensure!(result.is_err());
    Ok(())
}

#[test]
fn announcement_round_trip_validates_manifest_ticket() -> anyhow::Result<()> {
    let endpoint = iroh::SecretKey::generate().public();
    let hash = Hash::new(b"manifest");
    let ticket =
        iroh_blobs::ticket::BlobTicket::new(iroh::EndpointAddr::new(endpoint), hash, iroh_blobs::BlobFormat::Raw);
    let announcement = PackageAnnouncement::new(
        Uuid::new_v4(),
        "example",
        "1.0.0",
        1,
        hash,
        ticket.to_string(),
        endpoint,
    )?;
    anyhow::ensure!(PackageAnnouncement::from_bytes(announcement.to_bytes()?)? == announcement);
    Ok(())
}
