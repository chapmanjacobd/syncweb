use std::path::{Path, PathBuf};

use anyhow::Result;
use syncweb_core::{
    folder::{CollectionManifest, CollectionStore, FolderManager, SyncMode},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Result<Self> {
        let path = std::env::temp_dir().join(format!("syncweb-collection-pub-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _result = std::fs::remove_dir_all(&self.0);
    }
}

async fn test_node(directory: &TestDirectory, name: &str) -> Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    Ok(IrohNode::new(identity, root.join("data"), RelayMode::Default).await?)
}

#[tokio::test]
async fn test_collection_publish_uses_copy_mode() -> Result<()> {
    let directory = TestDirectory::new()?;
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
