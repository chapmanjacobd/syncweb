use anyhow::ensure;

use syncweb_core::{
    ProviderCapabilities, ProviderContent, ProviderLease, ProviderLeaseOptions,
    node::identity::IdentityManager,
    node::iroh_node::{DiscoveryConfig, IrohNode, RelayMode},
};

use crate::test_utils::TestDirectory;

async fn test_node(
    directory: &TestDirectory,
    name: &str,
    relay_map: Option<iroh::RelayMap>,
) -> anyhow::Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    let relay_mode = relay_map.map_or(RelayMode::Default, |map| RelayMode::Custom { map, insecure: true });
    Ok(IrohNode::new(
        identity,
        root.join("data"),
        relay_mode,
        crate::test_utils::empty_member_keys(),
        DiscoveryConfig::disabled(),
    )
    .await?)
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

#[tokio::test]
async fn test_publish_and_find_provider_lease() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-lease-test")?;
    let node = test_node(&directory, "node", None).await?;
    let doc = node.docs_engine().create_namespace().await?;
    let hash = iroh_blobs::Hash::from_bytes([7; 32]);
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&node.endpoint().secret_key().to_bytes());
    let mut lease = ProviderLease::new(
        node.endpoint().id(),
        ProviderLeaseOptions::new(
            None,
            doc.id(),
            None,
            vec![ProviderContent::blob(hash)],
            ProviderCapabilities::new(true, false),
            10,
            20,
        ),
    );
    lease.sign(&signing_key)?;

    node.topic_tracker()
        .publish_provider_lease(doc.id(), lease.clone())
        .await?;
    let leases = node.topic_tracker().find_provider_leases(doc.id()).await?;
    ensure!(leases.len() == 1);
    ensure!(leases[0] == lease);
    ensure!(node.refresh_provider_leases(doc.id(), 15, None).await? == 1);
    ensure!(
        node.provider_peer_snapshot(&hash)?
            .is_some_and(|providers| providers.contains(&node.endpoint().id()))
    );

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
