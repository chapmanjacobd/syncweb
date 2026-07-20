use std::path::{Path, PathBuf};

use syncweb_core::node::identity::IdentityManager;
use syncweb_core::node::iroh_node::{IrohNode, RelayMode};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Result<Self, std::io::Error> {
        let path = std::env::temp_dir().join(format!("syncweb-services-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn test_node(
    directory: &TestDirectory,
    name: &str,
    relay_map: Option<iroh::RelayMap>,
) -> anyhow::Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    let relay_mode = relay_map.map_or(RelayMode::Default, |map| RelayMode::Custom { map, insecure: true });
    Ok(IrohNode::new(identity, root.join("data"), relay_mode).await?)
}

#[tokio::test]
async fn test_add_bytes() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let node = test_node(&directory, "node", None).await?;
    let hash = node.blob_store().add_bytes(b"phase one blob").await?;

    assert!(node.blob_store().has(hash).await?);

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_has_blob() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let node = test_node(&directory, "node", None).await?;
    let hash = node.blob_store().add_bytes(b"blob data").await?;
    assert!(node.blob_store().has(hash).await?);

    let fake_hash = iroh_blobs::Hash::from_bytes([0_u8; 32]);
    assert!(!node.blob_store().has(fake_hash).await?);

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_get_blob() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let node = test_node(&directory, "node", None).await?;
    let hash = node.blob_store().add_bytes(b"blob data").await?;

    let bytes = node.blob_store().get(hash).await?;
    assert_eq!(bytes, b"blob data".as_slice());

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_blob_ticket() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let node = test_node(&directory, "node", None).await?;
    let hash = node.blob_store().add_bytes(b"blob data").await?;

    let ticket = node.blob_store().ticket(node.endpoint(), hash);
    assert_eq!(ticket.hash(), hash);
    assert!(ticket.to_string().starts_with("blob"));

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_add_file() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let node = test_node(&directory, "node", None).await?;
    let path = directory.path().join("input.txt");
    std::fs::write(&path, b"file blob")?;

    let hash = node.blob_store().add_file(&path).await?;
    assert_eq!(node.blob_store().get(hash).await?, b"file blob".as_slice());

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_two_nodes_sync_blob() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let (relay_map, _relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let first = test_node(&directory, "first", Some(relay_map.clone())).await?;
    let second = test_node(&directory, "second", Some(relay_map)).await?;
    let hash = first.blob_store().add_bytes(b"shared blob").await?;
    let ticket = first.blob_store().ticket(first.endpoint(), hash);

    second.blob_store().fetch(second.endpoint(), &ticket).await?;
    assert_eq!(second.blob_store().get(hash).await?, b"shared blob".as_slice());

    first.stop().await?;
    second.stop().await?;
    Ok(())
}
