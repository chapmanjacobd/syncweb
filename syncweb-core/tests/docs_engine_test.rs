mod test_utils;

use anyhow::Context;
use n0_future::StreamExt;
use syncweb_core::node::identity::IdentityManager;
use syncweb_core::node::iroh_node::{IrohNode, RelayMode};

use crate::test_utils::TestDirectory;

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
async fn test_create_namespace() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let doc = node.docs_engine().create_namespace().await?;
    anyhow::ensure!(!doc.id().to_string().is_empty());
    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_set_get_entry() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let doc = node.docs_engine().create_namespace().await?;
    let author = node.docs_engine().author().await?;

    node.docs_engine().set(&doc, author, b"key", b"value").await?;
    let entry = node
        .docs_engine()
        .get(&doc, author, b"key")
        .await?
        .context("entry exists")?;
    anyhow::ensure!(entry.key() == b"key".as_slice());

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_watch_entries() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let doc = node.docs_engine().create_namespace().await?;
    let author = node.docs_engine().author().await?;

    let mut events = node.docs_engine().watch(&doc).await?;

    node.docs_engine().set(&doc, author, b"key2", b"value2").await?;

    let event = tokio::time::timeout(std::time::Duration::from_secs(5), events.next())
        .await
        .context("watch event timed out")?
        .context("watch stream closed")?;

    match event {
        Ok(iroh_docs::engine::LiveEvent::InsertLocal { entry }) => {
            anyhow::ensure!(entry.key() == b"key2".as_slice());
        }
        other => anyhow::bail!("unexpected event: {other:?}"),
    }

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_author_from_secret() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;

    let original_author_id = node.docs_engine().author().await?;
    let author_secret = node
        .docs_engine()
        .export_author(original_author_id)
        .await?
        .context("author exists")?;

    let secret_str = author_secret.to_string();

    let parsed_author = std::str::FromStr::from_str(&secret_str)?;
    let imported_author_id = node.docs_engine().import_author(parsed_author).await?;

    anyhow::ensure!(original_author_id == imported_author_id);
    node.stop().await?;
    Ok(())
}
