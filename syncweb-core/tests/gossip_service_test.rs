use std::path::{Path, PathBuf};
use std::time::Duration;

use iroh::address_lookup::memory::MemoryLookup;
use n0_future::StreamExt;
use syncweb_core::node::identity::IdentityManager;
use syncweb_core::node::iroh_node::{IrohNode, RelayMode};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("syncweb-services-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).expect("create test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove test directory");
    }
}

async fn test_node(
    directory: &TestDirectory,
    name: &str,
    relay_map: Option<iroh::RelayMap>,
    address_lookup: Option<MemoryLookup>,
) -> IrohNode {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key")).expect("create identity");
    let relay_mode = match relay_map {
        Some(map) => RelayMode::Custom { map, insecure: true },
        None => RelayMode::Default,
    };
    match address_lookup {
        Some(lookup) => IrohNode::new_with_address_lookup(identity, root.join("data"), relay_mode, lookup)
            .await
            .expect("start node"),
        None => IrohNode::new(identity, root.join("data"), relay_mode)
            .await
            .expect("start node"),
    }
}

#[tokio::test]
async fn test_subscribe_publish() {
    let directory = TestDirectory::new();
    let (relay_map, relay_url, _server) = iroh::test_utils::run_relay_server().await.unwrap();
    let memory_lookup = MemoryLookup::new();
    let first = test_node(
        &directory,
        "first",
        Some(relay_map.clone()),
        Some(memory_lookup.clone()),
    )
    .await;
    let second = test_node(&directory, "second", Some(relay_map), Some(memory_lookup.clone())).await;

    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(first.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(second.endpoint().id()).with_relay_url(relay_url));

    let topic = iroh_gossip::TopicId::from_bytes(rand::random());

    let first_topic = first
        .gossip_service()
        .subscribe(topic, vec![])
        .await
        .expect("subscribe first node");
    let (first_sender, _first_receiver) = syncweb_core::node::gossip_service::GossipService::split(first_topic);

    let mut second_topic = second
        .gossip_service()
        .subscribe(topic, vec![first.endpoint().id()])
        .await
        .expect("subscribe second node");
    tokio::time::timeout(Duration::from_secs(30), second_topic.joined())
        .await
        .expect("gossip join timed out")
        .expect("gossip join failed");

    let _publish_task = tokio::spawn(async move {
        loop {
            let _ = first_sender
                .broadcast(bytes::Bytes::from_static(b"simple message"))
                .await;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    let received = tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(event) = second_topic.next().await {
            if let Ok(iroh_gossip::api::Event::Received(message)) = event {
                return Some(message.content);
            }
        }
        None
    })
    .await
    .expect("receive timed out")
    .expect("stream closed");

    assert_eq!(received.as_ref(), b"simple message");

    first.stop().await.expect("stop first node");
    second.stop().await.expect("stop second node");
}

#[tokio::test]
async fn test_multiple_subscribers() {
    let directory = TestDirectory::new();
    let (relay_map, relay_url, _server) = iroh::test_utils::run_relay_server().await.unwrap();
    let memory_lookup = MemoryLookup::new();
    let first = test_node(
        &directory,
        "first",
        Some(relay_map.clone()),
        Some(memory_lookup.clone()),
    )
    .await;
    let second = test_node(
        &directory,
        "second",
        Some(relay_map.clone()),
        Some(memory_lookup.clone()),
    )
    .await;
    let third = test_node(&directory, "third", Some(relay_map), Some(memory_lookup.clone())).await;

    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(first.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(second.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(third.endpoint().id()).with_relay_url(relay_url));

    let topic = iroh_gossip::TopicId::from_bytes(rand::random());

    let first_topic = first
        .gossip_service()
        .subscribe(topic, vec![])
        .await
        .expect("subscribe first node");
    let (first_sender, _first_receiver) = syncweb_core::node::gossip_service::GossipService::split(first_topic);
    let mut second_topic = second
        .gossip_service()
        .subscribe(topic, vec![first.endpoint().id()])
        .await
        .expect("subscribe second node");
    tokio::time::timeout(Duration::from_secs(30), second_topic.joined())
        .await
        .expect("gossip join timed out")
        .expect("gossip join failed");
    let mut third_topic = third
        .gossip_service()
        .subscribe(topic, vec![second.endpoint().id()])
        .await
        .expect("subscribe third node");
    tokio::time::timeout(Duration::from_secs(30), third_topic.joined())
        .await
        .expect("gossip join timed out")
        .expect("gossip join failed");

    let _publish_task = tokio::spawn(async move {
        loop {
            let _ = first_sender
                .broadcast(bytes::Bytes::from_static(b"phase one message"))
                .await;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    let second_received = tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(event) = second_topic.next().await {
            if let Ok(iroh_gossip::api::Event::Received(message)) = event {
                return Some(message.content);
            }
        }
        None
    })
    .await
    .expect("second gossip receive timed out")
    .expect("second gossip stream closed");
    let third_received = tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(event) = third_topic.next().await {
            if let Ok(iroh_gossip::api::Event::Received(message)) = event {
                return Some(message.content);
            }
        }
        None
    })
    .await
    .expect("third gossip receive timed out")
    .expect("third gossip stream closed");
    assert_eq!(second_received.as_ref(), b"phase one message");
    assert_eq!(third_received.as_ref(), b"phase one message");

    first.stop().await.expect("stop first node");
    second.stop().await.expect("stop second node");
    third.stop().await.expect("stop third node");
}
