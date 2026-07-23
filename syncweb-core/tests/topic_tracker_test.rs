mod test_utils;

use anyhow::ensure;

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
async fn test_announce_topic() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let doc = node.docs_engine().create_namespace().await?;

    node.topic_tracker().announce(doc.id()).await?;

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_find_peers() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let doc = node.docs_engine().create_namespace().await?;

    node.topic_tracker().announce(doc.id()).await?;
    let peers = node.topic_tracker().find_peers(doc.id()).await?;
    anyhow::ensure!(peers.is_empty());

    node.stop().await?;
    Ok(())
}

#[test]
fn test_bubble_detection() -> anyhow::Result<()> {
    ensure!(matches!(
        distributed_topic_tracker::Config::default()
            .merge_config()
            .bubble_merge(),
        distributed_topic_tracker::BubbleMergeConfig::Enabled(_)
    ));
    Ok(())
}
