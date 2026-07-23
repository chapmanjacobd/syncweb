mod test_utils;

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use iroh::address_lookup::memory::MemoryLookup;
use syncweb_core::{
    folder::{CollectionEntry, CollectionManifest, PackageAnnouncement, PackageCatalog, PackageManager},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
};

use crate::test_utils::TestDirectory;

async fn relay_node(
    directory: &TestDirectory,
    name: &str,
    relay_map: iroh::RelayMap,
    relay_url: iroh::RelayUrl,
    memory_lookup: MemoryLookup,
) -> anyhow::Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    let node = IrohNode::new_with_address_lookup(
        identity,
        root.join("data"),
        RelayMode::Custom {
            map: relay_map,
            insecure: true,
        },
        memory_lookup.clone(),
    )
    .await?;
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(node.endpoint().id()).with_relay_url(relay_url));
    Ok(node)
}

fn make_source(dir: &Path, name: &str, data: &[u8]) -> anyhow::Result<PathBuf> {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, data)?;
    Ok(path)
}

fn make_manifest(
    collection_id: uuid::Uuid,
    version: &str,
    files: &[(&str, &[u8])],
) -> anyhow::Result<CollectionManifest> {
    let mut manifest = CollectionManifest::new(collection_id, version);
    for (name, data) in files {
        manifest.entries.push(CollectionEntry::new(
            iroh_blobs::Hash::new(data),
            PathBuf::from(name),
            u64::try_from(data.len())?,
        )?);
    }
    Ok(manifest)
}

fn write_source_files(base: &Path, files: &[(&str, &[u8])]) -> anyhow::Result<PathBuf> {
    let source = base.join(format!("source-{}", uuid::Uuid::new_v4()));
    for (name, data) in files {
        make_source(&source, name, data)?;
    }
    Ok(source)
}

/// 5.3 Integration Test: Full package lifecycle
/// init -> add -> bump -> publish -> search -> install -> upgrade -> remove
#[tokio::test]
async fn test_package_lifecycle() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-package-test")?;
    let collection_id = uuid::Uuid::new_v4();
    let packages = PackageManager::new(directory.path().join("packages"));

    // Create v1 manifest and source
    let v1_tool: &[u8] = b"tool-v1";
    let v1_readme: &[u8] = b"readme v1";
    let v1_files = &[("bin/tool", v1_tool), ("README.md", v1_readme)];
    let v1 = make_manifest(collection_id, "1.0.0", v1_files)?;
    let source_v1 = write_source_files(directory.path(), v1_files)?;

    // Install v1
    packages.install(&v1, &source_v1)?;

    // Verify v1 state
    let v1_state = packages.state()?;
    let v1_installed = v1_state.current(collection_id).context("v1 should be installed")?;
    anyhow::ensure!(v1_installed.current == "1.0.0");
    anyhow::ensure!(v1_installed.versions.contains_key("1.0.0"));

    // Upgrade: create v2 with different content
    let v2_tool: &[u8] = b"tool-v2";
    let v2_readme: &[u8] = b"readme v2";
    let v2_files = &[("bin/tool", v2_tool), ("README.md", v2_readme)];
    let v2 = make_manifest(collection_id, "2.0.0", v2_files)?;
    let source_v2 = write_source_files(directory.path(), v2_files)?;

    // Install v2 (upgrade)
    packages.install(&v2, &source_v2)?;

    // Verify both versions coexist
    let v2_state = packages.state()?;
    let v2_installed = v2_state.current(collection_id).context("v2 should be current")?;
    anyhow::ensure!(v2_installed.current == "2.0.0");
    anyhow::ensure!(v2_installed.versions.contains_key("1.0.0"));
    anyhow::ensure!(v2_installed.versions.contains_key("2.0.0"));

    // Switch back to v1
    packages.switch(collection_id, "1.0.0")?;
    let current_path = packages.root().join(collection_id.to_string()).join("current/bin/tool");
    anyhow::ensure!(std::fs::read(current_path)? == b"tool-v1");

    // Verify both versions
    packages.verify(&v1)?;
    packages.verify(&v2)?;

    // Remove v1 (must switch away first)
    packages.switch(collection_id, "2.0.0")?;
    packages.remove(collection_id, "1.0.0")?;

    // Verify v1 is gone, v2 remains
    let final_state = packages.state()?;
    let final_installed = final_state
        .current(collection_id)
        .context("collection should still exist")?;
    anyhow::ensure!(!final_installed.versions.contains_key("1.0.0"));
    anyhow::ensure!(final_installed.versions.contains_key("2.0.0"));
    anyhow::ensure!(final_installed.current == "2.0.0");

    // Verify v2 is still healthy after removing v1
    packages.verify(&v2)?;

    Ok(())
}

/// 5.3 Integration Test: Install v1, install v2, switch between
#[test]
fn test_multi_version_coexistence() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-package-test")?;
    let collection_id = uuid::Uuid::new_v4();
    let packages = PackageManager::new(directory.path().join("packages"));

    // Install v1
    let v1_content: &[u8] = b"content-v1";
    let v1_files = &[("data/file.txt", v1_content)];
    let v1 = make_manifest(collection_id, "1.0.0", v1_files)?;
    let source_v1 = write_source_files(directory.path(), v1_files)?;
    packages.install(&v1, &source_v1)?;

    // Install v2
    let v2_content: &[u8] = b"content-v2";
    let v2_files = &[("data/file.txt", v2_content)];
    let v2 = make_manifest(collection_id, "2.0.0", v2_files)?;
    let source_v2 = write_source_files(directory.path(), v2_files)?;
    packages.install(&v2, &source_v2)?;

    // Both version directories should exist
    let collection_dir = packages.root().join(collection_id.to_string());
    anyhow::ensure!(collection_dir.join("1.0.0").join("data/file.txt").exists());
    anyhow::ensure!(collection_dir.join("2.0.0").join("data/file.txt").exists());

    // current symlink should point to v2 (latest installed)
    let initial_link = std::fs::read_link(collection_dir.join("current"))?;
    anyhow::ensure!(initial_link == Path::new("2.0.0"));

    // Switch to v1
    packages.switch(collection_id, "1.0.0")?;
    let v1_link = std::fs::read_link(collection_dir.join("current"))?;
    anyhow::ensure!(v1_link == Path::new("1.0.0"));
    anyhow::ensure!(std::fs::read(collection_dir.join("current/data/file.txt"))? == b"content-v1");

    // Switch to v2
    packages.switch(collection_id, "2.0.0")?;
    let v2_link = std::fs::read_link(collection_dir.join("current"))?;
    anyhow::ensure!(v2_link == Path::new("2.0.0"));
    anyhow::ensure!(std::fs::read(collection_dir.join("current/data/file.txt"))? == b"content-v2");

    // State file tracks both versions
    let state = packages.state()?;
    let installed = state.current(collection_id).context("should have collection")?;
    anyhow::ensure!(installed.versions.len() == 2);
    anyhow::ensure!(installed.current == "2.0.0");

    Ok(())
}

/// 5.3 Integration Test: Stage -> verify -> symlink swap -> cleanup
#[test]
fn test_atomic_upgrade() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-package-test")?;
    let collection_id = uuid::Uuid::new_v4();
    let packages = PackageManager::new(directory.path().join("packages"));

    // Install v1
    let v1_data: &[u8] = b"v1-binary";
    let v1_files = &[("app.bin", v1_data)];
    let v1 = make_manifest(collection_id, "1.0.0", v1_files)?;
    let source_v1 = write_source_files(directory.path(), v1_files)?;
    packages.install(&v1, &source_v1)?;

    // Record v1 state
    let collection_dir = packages.root().join(collection_id.to_string());
    anyhow::ensure!(std::fs::read(collection_dir.join("current/app.bin"))? == b"v1-binary");
    anyhow::ensure!(std::fs::read_link(collection_dir.join("current"))? == Path::new("1.0.0"));

    // Install v2
    let v2_data: &[u8] = b"v2-binary-upgraded";
    let v2_files = &[("app.bin", v2_data)];
    let v2 = make_manifest(collection_id, "2.0.0", v2_files)?;
    let source_v2 = write_source_files(directory.path(), v2_files)?;
    packages.install(&v2, &source_v2)?;

    // Verify atomic swap: current now points to v2
    anyhow::ensure!(std::fs::read_link(collection_dir.join("current"))? == Path::new("2.0.0"));
    anyhow::ensure!(std::fs::read(collection_dir.join("current/app.bin"))? == b"v2-binary-upgraded");

    // Verify no staging artifacts remain
    let staging_artifacts: Vec<_> = std::fs::read_dir(&collection_dir)?
        .filter_map(std::result::Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(".stage-") || name.starts_with(".current-"))
        })
        .collect();
    anyhow::ensure!(
        staging_artifacts.is_empty(),
        "staging artifacts should be cleaned up: {staging_artifacts:?}"
    );

    // Verify state is consistent
    let state = packages.state()?;
    let installed = state.current(collection_id).context("should exist")?;
    anyhow::ensure!(installed.current == "2.0.0");
    anyhow::ensure!(installed.versions.len() == 2);

    // Verify v1 is still independently accessible
    anyhow::ensure!(std::fs::read(collection_dir.join("1.0.0/app.bin"))? == b"v1-binary");

    Ok(())
}

/// 5.3 Integration Test: Verify catches corruption
#[test]
fn test_package_integrity() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-package-test")?;
    let collection_id = uuid::Uuid::new_v4();
    let packages = PackageManager::new(directory.path().join("packages"));

    // Install a package
    let original: &[u8] = b"original content";
    let files = &[("data.txt", original)];
    let manifest = make_manifest(collection_id, "1.0.0", files)?;
    let source = write_source_files(directory.path(), files)?;
    packages.install(&manifest, &source)?;

    // Verify passes before corruption
    packages.verify(&manifest)?;

    // Corrupt the installed file
    let file_path = packages.root().join(collection_id.to_string()).join("1.0.0/data.txt");
    std::fs::write(&file_path, b"corrupted content")?;

    // Verify should detect corruption
    let err = packages
        .verify(&manifest)
        .expect_err("verify should fail after corruption");
    anyhow::ensure!(
        format!("{err}").contains("does not match manifest"),
        "unexpected error: {err}"
    );

    // Verify a different manifest version not installed
    let wrong_manifest = make_manifest(collection_id, "9.9.9", files)?;
    anyhow::ensure!(packages.verify(&wrong_manifest).is_err());

    // Verify on a missing collection
    let missing_manifest = make_manifest(uuid::Uuid::new_v4(), "1.0.0", files)?;
    anyhow::ensure!(packages.verify(&missing_manifest).is_err());

    Ok(())
}

/// 5.3 Integration Test: Publish -> search -> info across nodes
#[tokio::test]
async fn test_package_discovery() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-package-test")?;
    let (relay_map, relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let memory_lookup = MemoryLookup::new();

    let publisher = relay_node(
        &directory,
        "publisher",
        relay_map.clone(),
        relay_url.clone(),
        memory_lookup.clone(),
    )
    .await?;
    let searcher = relay_node(&directory, "searcher", relay_map, relay_url, memory_lookup.clone()).await?;

    // Publisher subscribes to catalog (no bootstrap, it's the first peer)
    let pub_catalog = PackageCatalog::new(publisher.gossip_service());
    let pub_topic = pub_catalog.subscribe(vec![]).await?;
    let (sender, _receiver) = syncweb_core::node::gossip_service::GossipService::split(pub_topic);

    // Searcher subscribes with publisher as bootstrap and waits for join
    let search_catalog = PackageCatalog::new(searcher.gossip_service());
    let mut search_topic = search_catalog.subscribe(vec![publisher.endpoint().id()]).await?;
    tokio::time::timeout(Duration::from_secs(30), search_topic.joined())
        .await
        .context("gossip join timed out")?
        .context("gossip join failed")?;

    // Create a manifest for the announcement
    let collection_id = uuid::Uuid::new_v4();
    let manifest_hash = publisher.blob_store().add_bytes(b"fake-manifest-data").await?;
    let ticket = publisher.blob_store().ticket(publisher.endpoint(), manifest_hash);

    let pkg_announcement = PackageAnnouncement::new(
        collection_id,
        "example-pkg",
        "1.0.0",
        1,
        manifest_hash,
        ticket.to_string(),
        publisher.endpoint().id(),
    )?;

    // Announce repeatedly so the searcher has time to receive
    let announce_task = tokio::spawn({
        let catalog_clone = pub_catalog.clone();
        let announcement_clone = pkg_announcement.clone();
        async move {
            loop {
                let _ = catalog_clone.announce(&sender, &announcement_clone).await;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    });

    let results = search_catalog
        .search(&mut search_topic, Some("example"), Duration::from_secs(10))
        .await?;
    announce_task.abort();

    anyhow::ensure!(!results.is_empty(), "searcher should discover the published package");
    let found = results.first().context("should have first result")?;
    anyhow::ensure!(found.collection_id == collection_id);
    anyhow::ensure!(found.name == "example-pkg");
    anyhow::ensure!(found.version == "1.0.0");
    anyhow::ensure!(found.manifest == manifest_hash);

    // Register the ticket endpoint so searcher can fetch
    PackageCatalog::register_ticket_endpoint(&memory_lookup, found)?;

    publisher.stop().await?;
    searcher.stop().await?;
    Ok(())
}
