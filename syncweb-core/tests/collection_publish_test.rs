mod test_utils;

use anyhow::Result;
use syncweb_core::folder::{CollectionManifest, CollectionStore, FolderManager, SyncMode};

use crate::test_utils::{TestDirectory, test_node};

#[tokio::test]
async fn test_collection_publish_uses_copy_mode() -> Result<()> {
    let directory = TestDirectory::new("syncweb-collection-pub-test")?;
    let node = test_node(&directory, "node").await?;
    let folder = FolderManager::new(&node).create(SyncMode::SendReceive).await?;

    let content = b"collection publish blob";
    let path = directory.path().join("source.txt");
    std::fs::write(&path, content)?;

    let hash = node.blob_store().add_file(&path).await?;
    anyhow::ensure!(&*node.blob_store().get(hash).await? == content);

    let mut manifest = CollectionManifest::new(uuid::Uuid::new_v4(), "1.0.0");
    manifest.entries.push(syncweb_core::folder::CollectionEntry::new(
        hash,
        "source.txt",
        u64::try_from(content.len())?,
    )?);

    let store = CollectionStore::new(
        folder.doc().clone(),
        folder.author(),
        node.blob_store().clone(),
        node.docs_engine().clone(),
    );
    let head = store.publish(&manifest, 1).await?;
    anyhow::ensure!(head.sequence == 1);
    anyhow::ensure!(node.blob_store().has(head.manifest).await?);
    anyhow::ensure!(node.blob_store().has(hash).await?);

    let loaded = store.load(head.manifest).await?;
    anyhow::ensure!(loaded == manifest);

    node.stop().await?;
    Ok(())
}
