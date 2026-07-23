mod test_utils;

use std::time::Duration;

use anyhow::Context;
use iroh::address_lookup::memory::MemoryLookup;
use n0_future::StreamExt;
use syncweb_core::node::identity::IdentityManager;
use syncweb_core::node::iroh_node::{IrohNode, RelayMode};

use crate::test_utils::TestDirectory;

async fn publish_repeatedly(sender: iroh_gossip::api::GossipSender, message: &'static [u8]) -> anyhow::Result<()> {
    loop {
        sender
            .broadcast(bytes::Bytes::copy_from_slice(message))
            .await
            .context("failed to broadcast gossip message")?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

async fn test_node(
    directory: &TestDirectory,
    name: &str,
    relay_map: Option<iroh::RelayMap>,
    address_lookup: Option<MemoryLookup>,
) -> anyhow::Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    let relay_mode = relay_map.map_or(RelayMode::Default, |map| RelayMode::Custom { map, insecure: true });
    match address_lookup {
        Some(lookup) => Ok(IrohNode::new_with_address_lookup(identity, root.join("data"), relay_mode, lookup).await?),
        None => Ok(IrohNode::new(identity, root.join("data"), relay_mode).await?),
    }
}

#[tokio::test]
async fn test_subscribe_publish() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let (relay_map, relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let memory_lookup = MemoryLookup::new();
    let first = test_node(
        &directory,
        "first",
        Some(relay_map.clone()),
        Some(memory_lookup.clone()),
    )
    .await?;
    let second = test_node(&directory, "second", Some(relay_map), Some(memory_lookup.clone())).await?;

    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(first.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(second.endpoint().id()).with_relay_url(relay_url));

    let topic = iroh_gossip::TopicId::from_bytes(rand::random());

    let first_topic = first
        .gossip_service()
        .subscribe(topic, vec![])
        .await
        .context("subscribe first node")?;
    let (first_sender, _first_receiver) = syncweb_core::node::gossip_service::GossipService::split(first_topic);

    let mut second_topic = second
        .gossip_service()
        .subscribe(topic, vec![first.endpoint().id()])
        .await
        .context("subscribe second node")?;
    tokio::time::timeout(Duration::from_secs(30), second_topic.joined())
        .await
        .context("gossip join timed out")?
        .context("gossip join failed")?;

    let mut publish_task = tokio::spawn(publish_repeatedly(first_sender, b"simple message"));

    let received_result = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::select! {
            result = &mut publish_task => {
                result.context("gossip publisher task failed")??;
                anyhow::bail!("gossip publisher task exited unexpectedly");
            }
            received = async {
                while let Some(event) = second_topic.next().await {
                    if let Ok(iroh_gossip::api::Event::Received(message)) = event {
                        return Some(message.content);
                    }
                }
                None
            }
            => anyhow::Ok(received),
        }
    })
    .await;
    publish_task.abort();
    let received = received_result
        .context("receive timed out")??
        .context("stream closed")?;

    anyhow::ensure!(received.as_ref() == b"simple message");

    first.stop().await?;
    second.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_multiple_subscribers() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let (relay_map, relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let memory_lookup = MemoryLookup::new();
    let first = test_node(
        &directory,
        "first",
        Some(relay_map.clone()),
        Some(memory_lookup.clone()),
    )
    .await?;
    let second = test_node(
        &directory,
        "second",
        Some(relay_map.clone()),
        Some(memory_lookup.clone()),
    )
    .await?;
    let third = test_node(&directory, "third", Some(relay_map), Some(memory_lookup.clone())).await?;

    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(first.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(second.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(third.endpoint().id()).with_relay_url(relay_url));

    let topic = iroh_gossip::TopicId::from_bytes(rand::random());

    let first_topic = first
        .gossip_service()
        .subscribe(topic, vec![])
        .await
        .context("subscribe first node")?;
    let (first_sender, _first_receiver) = syncweb_core::node::gossip_service::GossipService::split(first_topic);
    let mut second_topic = second
        .gossip_service()
        .subscribe(topic, vec![first.endpoint().id()])
        .await
        .context("subscribe second node")?;
    tokio::time::timeout(Duration::from_secs(30), second_topic.joined())
        .await
        .context("gossip join timed out")?
        .context("gossip join failed")?;
    let mut third_topic = third
        .gossip_service()
        .subscribe(topic, vec![second.endpoint().id()])
        .await
        .context("subscribe third node")?;
    tokio::time::timeout(Duration::from_secs(30), third_topic.joined())
        .await
        .context("gossip join timed out")?
        .context("gossip join failed")?;

    let mut publish_task = tokio::spawn(publish_repeatedly(first_sender, b"phase one message"));

    let received_result = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::select! {
            result = &mut publish_task => {
                result.context("gossip publisher task failed")??;
                anyhow::bail!("gossip publisher task exited unexpectedly");
            }
            received = async {
                while let Some(event) = second_topic.next().await {
                    if let Ok(iroh_gossip::api::Event::Received(message)) = event {
                        return Some(message.content);
                    }
                }
                None
            } => anyhow::Ok(received),
        }
    })
    .await;
    publish_task.abort();
    let second_received = received_result
        .context("second gossip receive timed out")??
        .context("second gossip stream closed")?;
    let third_received = tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(event) = third_topic.next().await {
            if let Ok(iroh_gossip::api::Event::Received(message)) = event {
                return Some(message.content);
            }
        }
        None
    })
    .await
    .context("third gossip receive timed out")?
    .context("third gossip stream closed")?;
    anyhow::ensure!(second_received.as_ref() == b"phase one message");
    anyhow::ensure!(third_received.as_ref() == b"phase one message");

    first.stop().await?;
    second.stop().await?;
    third.stop().await?;
    Ok(())
}
