use std::time::Duration;

use iroh::address_lookup::memory::MemoryLookup;
use n0_future::StreamExt;
use syncweb_core::node::identity::IdentityManager;
use syncweb_core::node::iroh_node::{IrohNode, RelayMode};

use crate::test_utils::{TestDirectory, test_node};

async fn test_node_with_lookup(
    directory: &TestDirectory,
    name: &str,
    relay_map: iroh::RelayMap,
    lookup: MemoryLookup,
) -> anyhow::Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    Ok(IrohNode::new_with_address_lookup(
        identity,
        root.join("data"),
        RelayMode::Custom {
            map: relay_map,
            insecure: true,
        },
        lookup,
    )
    .await?)
}

#[tokio::test]
async fn test_endpoint_creation() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-node-test")?;
    let node = test_node(&directory, "node").await?;

    anyhow::ensure!(!node.endpoint().bound_sockets().is_empty());

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_router_setup() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-node-test")?;
    let node = test_node(&directory, "node").await?;

    anyhow::ensure!(node.is_running());
    let _ = node.blobs();
    let _ = node.docs();
    let _ = node.gossip();

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_protocol_registration() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-node-test")?;
    let server = test_node(&directory, "server").await?;
    let client = test_node(&directory, "client").await?;
    let server_address = server.endpoint().addr();

    for alpn in [iroh_blobs::protocol::ALPN, iroh_docs::ALPN, iroh_gossip::ALPN] {
        let connection = tokio::time::timeout(
            Duration::from_secs(5),
            client.endpoint().connect(server_address.clone(), alpn),
        )
        .await??;
        connection.close(0_u32.into(), b"test complete");
    }

    client.stop().await?;
    server.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_shutdown() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-node-test")?;
    let node = test_node(&directory, "node").await?;

    node.stop().await?;

    anyhow::ensure!(!node.is_running());
    anyhow::ensure!(node.endpoint().is_closed());
    Ok(())
}

#[tokio::test]
async fn test_two_nodes_connect() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-node-test")?;
    let first = test_node(&directory, "first").await?;
    let second = test_node(&directory, "second").await?;

    let connection = tokio::time::timeout(
        Duration::from_secs(5),
        first.endpoint().connect(second.endpoint().addr(), iroh_gossip::ALPN),
    )
    .await??;

    anyhow::ensure!(connection.remote_id() == second.endpoint().id());

    first.stop().await?;
    second.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_node_discovery() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-node-test")?;
    let (relay_map, relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let memory_lookup = MemoryLookup::new();

    let first = test_node_with_lookup(&directory, "first", relay_map.clone(), memory_lookup.clone()).await?;
    let second = test_node_with_lookup(&directory, "second", relay_map, memory_lookup.clone()).await?;

    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(first.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(second.endpoint().id()).with_relay_url(relay_url));

    let topic = iroh_gossip::TopicId::from_bytes(rand::random());

    let first_topic = first
        .gossip_service()
        .subscribe(topic, vec![])
        .await
        .map_err(|e| anyhow::anyhow!("first subscribe: {e}"))?;
    let (_first_sender, mut first_receiver) = syncweb_core::node::gossip_service::GossipService::split(first_topic);

    let mut second_topic = second
        .gossip_service()
        .subscribe(topic, vec![first.endpoint().id()])
        .await
        .map_err(|e| anyhow::anyhow!("second subscribe: {e}"))?;
    tokio::time::timeout(Duration::from_secs(10), second_topic.joined())
        .await
        .map_err(|elapsed| anyhow::anyhow!("second node gossip join timed out: {elapsed}"))?
        .map_err(|e| anyhow::anyhow!("second node gossip join failed: {e}"))?;

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(event) = first_receiver.next().await
                && matches!(event, Ok(iroh_gossip::api::Event::NeighborUp(_)))
            {
                return anyhow::Ok(());
            }
        }
    })
    .await
    .map_err(|elapsed| anyhow::anyhow!("nodes did not discover each other via gossip within timeout: {elapsed}"))??;

    first.stop().await?;
    second.stop().await?;
    Ok(())
}
