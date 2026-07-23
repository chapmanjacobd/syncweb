use std::{fs, path::Path};

use anyhow::Result;
use iroh_blobs::Hash;
use syncweb_core::{
    folder::{FolderManager, SyncMode},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
    snapshot::{Snapshot, SnapshotEntry, SnapshotStore},
};

use crate::test_utils::TestDirectory;

async fn test_node(directory: &TestDirectory) -> Result<IrohNode> {
    let identity = IdentityManager::new(directory.path().join("identity.key"))?;
    Ok(IrohNode::new(identity, directory.path().join("data"), RelayMode::Default).await?)
}

async fn test_node_with_relay(directory: &TestDirectory, name: &str, relay_map: iroh::RelayMap) -> Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    Ok(IrohNode::new(
        identity,
        root.join("data"),
        RelayMode::Custom {
            map: relay_map,
            insecure: true,
        },
    )
    .await?)
}

#[test]
fn snapshot_round_trip_and_diff() -> Result<()> {
    let first = Snapshot::new(
        None,
        vec![
            SnapshotEntry::new("a.txt", Hash::new(b"old"), 3)?,
            SnapshotEntry::new("removed.txt", Hash::new(b"gone"), 4)?,
        ],
        Some("before".to_owned()),
    );
    let second = Snapshot::new(
        None,
        vec![
            SnapshotEntry::new("a.txt", Hash::new(b"new"), 3)?,
            SnapshotEntry::new("added.txt", Hash::new(b"new file"), 8)?,
        ],
        Some("after".to_owned()),
    );
    anyhow::ensure!(Snapshot::from_bytes(first.to_bytes()?)? == first);
    let diff = first.diff(&second)?;
    anyhow::ensure!(diff.added.len() == 1);
    anyhow::ensure!(diff.removed.len() == 1);
    anyhow::ensure!(diff.modified.len() == 1);
    Ok(())
}

#[tokio::test]
async fn snapshot_path_restore_pins_and_removes_stale_files() -> Result<()> {
    let directory = TestDirectory::new("syncweb-snapshot-test")?;
    let source = directory.path().join("source");
    fs::create_dir_all(source.join("sub"))?;
    fs::write(source.join("a.txt"), b"alpha")?;
    fs::write(source.join("sub/b.txt"), b"beta")?;

    let node = test_node(&directory).await?;
    let store = SnapshotStore::new(node.blob_store().clone());
    let snapshot = store.create_from_path(&source, 1, None).await?;
    let destination = directory.path().join("destination");
    fs::create_dir_all(&destination)?;
    fs::write(destination.join("stale.txt"), b"stale")?;
    store.restore_to_path(&snapshot, &destination).await?;

    anyhow::ensure!(fs::read(destination.join("a.txt"))? == b"alpha");
    anyhow::ensure!(fs::read(destination.join("sub/b.txt"))? == b"beta");
    anyhow::ensure!(!destination.join("stale.txt").exists());
    anyhow::ensure!(store.list().await?.iter().any(|item| item.id == snapshot.id));
    anyhow::ensure!(
        node.blob_store()
            .list_pins("syncweb/snapshot/")
            .await?
            .iter()
            .any(|(name, _)| name.contains("/blob/"))
    );
    store.delete(snapshot.id).await?;
    anyhow::ensure!(store.list().await?.is_empty());
    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_snapshot_sharing() -> Result<()> {
    let directory = TestDirectory::new("syncweb-snapshot-test")?;
    let (relay_map, _relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let first = test_node_with_relay(&directory, "first", relay_map.clone()).await?;
    let second = test_node_with_relay(&directory, "second", relay_map).await?;
    let source = directory.path().join("source");
    fs::create_dir_all(&source)?;
    fs::write(source.join("shared.txt"), b"shared")?;

    let first_store = SnapshotStore::new(first.blob_store().clone());
    let snapshot = first_store.create_from_path(&source, 1, None).await?;
    let ticket = first_store.ticket(first.endpoint(), &snapshot).await?;
    let second_store = SnapshotStore::new(second.blob_store().clone());
    let imported = second_store.import_ticket(second.endpoint(), &ticket).await?;

    anyhow::ensure!(imported == snapshot);
    let entry = snapshot
        .entries
        .first()
        .ok_or_else(|| anyhow::anyhow!("snapshot has no entries"))?;
    anyhow::ensure!(second.blob_store().get(entry.hash).await? == b"shared".as_slice());
    first.stop().await?;
    second.stop().await?;
    Ok(())
}

#[tokio::test]
async fn folder_snapshot_restores_document_entries() -> Result<()> {
    let directory = TestDirectory::new("syncweb-snapshot-test")?;
    let node = test_node(&directory).await?;
    let folder = FolderManager::new(&node).create(SyncMode::SendReceive).await?;
    folder.set_blob(b"a.txt", b"old").await?;
    let store = SnapshotStore::with_docs(node.blob_store().clone(), node.docs_engine().clone());
    let snapshot = store.create_for_folder(&folder, None).await?;
    folder.set_blob(b"a.txt", b"new").await?;
    folder.set_blob(b"extra.txt", b"extra").await?;
    store.restore_for_folder(&folder, &snapshot).await?;

    let restored = node
        .docs_engine()
        .get_any(folder.doc(), b"a.txt")
        .await?
        .ok_or_else(|| anyhow::anyhow!("restored entry is missing"))?;
    let extra = node.docs_engine().get_any(folder.doc(), b"extra.txt").await?;
    anyhow::ensure!(node.blob_store().get(restored.content_hash()).await? == b"old".as_slice());
    anyhow::ensure!(extra.is_none());
    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_create_snapshot() -> Result<()> {
    let directory = TestDirectory::new("syncweb-snapshot-test")?;
    let source = directory.path().join("source");
    fs::create_dir_all(source.join("sub"))?;
    fs::write(source.join("a.txt"), b"hello")?;
    fs::write(source.join("sub/b.txt"), b"world")?;

    let node = test_node(&directory).await?;
    let store = SnapshotStore::new(node.blob_store().clone());
    let snapshot = store
        .create_from_path(&source, 1, Some("test snapshot".to_owned()))
        .await?;

    anyhow::ensure!(snapshot.schema_version == 1);
    anyhow::ensure!(snapshot.description.as_deref() == Some("test snapshot"));
    anyhow::ensure!(snapshot.file_count == 2);
    anyhow::ensure!(snapshot.total_size == 10);
    anyhow::ensure!(snapshot.id == snapshot.root_hash);
    anyhow::ensure!(snapshot.entries.len() == 2);

    let entry_a = snapshot
        .entries
        .iter()
        .find(|entry| entry.path == Path::new("a.txt"))
        .ok_or_else(|| anyhow::anyhow!("a.txt entry missing"))?;
    anyhow::ensure!(entry_a.size == 5);
    anyhow::ensure!(node.blob_store().has(entry_a.hash).await?);

    let entry_b = snapshot
        .entries
        .iter()
        .find(|entry| entry.path == Path::new("sub/b.txt"))
        .ok_or_else(|| anyhow::anyhow!("sub/b.txt entry missing"))?;
    anyhow::ensure!(entry_b.size == 5);
    anyhow::ensure!(node.blob_store().has(entry_b.hash).await?);

    snapshot.validate()?;
    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_snapshot_pin_gc() -> Result<()> {
    let directory = TestDirectory::new("syncweb-snapshot-test")?;
    let source = directory.path().join("source");
    fs::create_dir_all(&source)?;
    fs::write(source.join("pinned.txt"), b"keep me")?;

    let node = test_node(&directory).await?;
    let store = SnapshotStore::new(node.blob_store().clone());
    let snapshot = store.create_from_path(&source, 1, None).await?;
    let entry = snapshot
        .entries
        .first()
        .ok_or_else(|| anyhow::anyhow!("snapshot has no entries"))?;

    let has_blob_pins = node
        .blob_store()
        .list_pins("syncweb/snapshot/")
        .await?
        .into_iter()
        .any(|(name, _)| name.contains("/blob/"));
    anyhow::ensure!(has_blob_pins, "blob pins should exist after snapshot creation");
    anyhow::ensure!(
        node.blob_store().has(entry.hash).await?,
        "blob should be accessible while pinned"
    );

    store.delete(snapshot.id).await?;
    let pins_after = node
        .blob_store()
        .list_pins(&format!("syncweb/snapshot/{}/blob/", snapshot.id))
        .await?;
    anyhow::ensure!(
        pins_after.is_empty(),
        "blob pins should be removed after snapshot delete"
    );

    let manifest_pins = node
        .blob_store()
        .list_pins(&format!("syncweb/snapshot/{}/manifest", snapshot.id))
        .await?;
    anyhow::ensure!(
        manifest_pins.is_empty(),
        "manifest pin should be removed after snapshot delete"
    );
    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_backup_restore_cycle() -> Result<()> {
    let directory = TestDirectory::new("syncweb-snapshot-test")?;
    let source = directory.path().join("source");
    fs::create_dir_all(source.join("sub"))?;
    fs::write(source.join("a.txt"), b"original_a")?;
    fs::write(source.join("sub/b.txt"), b"original_b")?;
    fs::write(source.join("c.txt"), b"original_c")?;

    let node = test_node(&directory).await?;
    let store = SnapshotStore::new(node.blob_store().clone());
    let snapshot = store.create_from_path(&source, 1, Some("backup".to_owned())).await?;

    fs::write(source.join("a.txt"), b"modified_a")?;
    fs::write(source.join("sub/b.txt"), b"modified_b")?;
    fs::write(source.join("new.txt"), b"new_file")?;
    fs::remove_file(source.join("c.txt"))?;

    anyhow::ensure!(fs::read(source.join("a.txt"))? == b"modified_a");
    anyhow::ensure!(!source.join("c.txt").exists());
    anyhow::ensure!(source.join("new.txt").exists());

    store.restore_to_path(&snapshot, &source).await?;

    anyhow::ensure!(fs::read(source.join("a.txt"))? == b"original_a");
    anyhow::ensure!(fs::read(source.join("sub/b.txt"))? == b"original_b");
    anyhow::ensure!(fs::read(source.join("c.txt"))? == b"original_c");
    anyhow::ensure!(!source.join("new.txt").exists(), "new.txt should be removed by restore");

    node.stop().await?;
    Ok(())
}
