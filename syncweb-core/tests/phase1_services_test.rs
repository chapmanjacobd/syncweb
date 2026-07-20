use std::path::{Path, PathBuf};
use std::time::Duration;

use n0_future::StreamExt;
use syncweb_core::node::identity::IdentityManager;
use syncweb_core::node::iroh_node::IrohNode;

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

async fn test_node(directory: &TestDirectory, name: &str) -> IrohNode {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key")).expect("create identity");
    IrohNode::new(identity, root.join("data"))
        .await
        .expect("start node")
}

#[tokio::test]
async fn test_blob_store_operations() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node").await;
    let hash = node
        .blob_store()
        .add_bytes(b"phase one blob")
        .await
        .expect("add blob");

    assert!(node.blob_store().has(hash).await.expect("check blob"));
    assert_eq!(
        node.blob_store().get(hash).await.expect("read blob"),
        b"phase one blob".as_slice()
    );

    let ticket = node.blob_store().ticket(node.endpoint(), hash);
    assert_eq!(ticket.hash(), hash);
    assert!(ticket.to_string().starts_with("blob"));

    node.stop().await.expect("stop node");
}

#[tokio::test]
async fn test_blob_store_add_file() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node").await;
    let path = directory.path().join("input.txt");
    std::fs::write(&path, b"file blob").expect("write input file");

    let hash = node.blob_store().add_file(&path).await.expect("add file");
    assert_eq!(
        node.blob_store().get(hash).await.expect("read file blob"),
        b"file blob".as_slice()
    );

    node.stop().await.expect("stop node");
}

#[tokio::test]
async fn test_two_nodes_sync_blob() {
    let directory = TestDirectory::new();
    let first = test_node(&directory, "first").await;
    let second = test_node(&directory, "second").await;
    let hash = first
        .blob_store()
        .add_bytes(b"shared blob")
        .await
        .expect("add shared blob");
    let ticket = first.blob_store().ticket(first.endpoint(), hash);

    second
        .blob_store()
        .fetch(second.endpoint(), &ticket)
        .await
        .expect("fetch shared blob");
    assert_eq!(
        second
            .blob_store()
            .get(hash)
            .await
            .expect("read shared blob"),
        b"shared blob".as_slice()
    );

    first.stop().await.expect("stop first node");
    second.stop().await.expect("stop second node");
}

#[tokio::test]
async fn test_docs_engine_operations() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node").await;
    let doc = node
        .docs_engine()
        .create_namespace()
        .await
        .expect("create namespace");
    let author = node.docs_engine().author().await.expect("get author");

    node.docs_engine()
        .set(&doc, author, b"key", b"value")
        .await
        .expect("set document entry");
    let entry = node
        .docs_engine()
        .get(&doc, author, b"key")
        .await
        .expect("get document entry")
        .expect("entry exists");
    assert_eq!(entry.key(), b"key".as_slice());

    let _events = node
        .docs_engine()
        .watch(&doc)
        .await
        .expect("watch document");
    node.stop().await.expect("stop node");
}

#[tokio::test]
async fn test_gossip_multiple_subscribers_receive() {
    let directory = TestDirectory::new();
    let first = test_node(&directory, "first").await;
    let second = test_node(&directory, "second").await;
    let third = test_node(&directory, "third").await;
    let topic = iroh_gossip::TopicId::from_bytes([7; 32]);

    let first_topic = first
        .gossip_service()
        .subscribe(topic, vec![])
        .await
        .expect("subscribe first node");
    let (first_sender, _first_receiver) =
        syncweb_core::node::gossip_service::GossipService::split(first_topic);
    let mut second_topic = second
        .gossip_service()
        .subscribe(topic, vec![first.endpoint().id()])
        .await
        .expect("subscribe second node");
    tokio::time::timeout(Duration::from_secs(5), second_topic.joined())
        .await
        .expect("gossip join timed out")
        .expect("gossip join failed");
    let mut third_topic = third
        .gossip_service()
        .subscribe(topic, vec![second.endpoint().id()])
        .await
        .expect("subscribe third node");
    tokio::time::timeout(Duration::from_secs(5), third_topic.joined())
        .await
        .expect("gossip join timed out")
        .expect("gossip join failed");

    first
        .gossip_service()
        .publish(&first_sender, b"phase one message")
        .await
        .expect("publish message");

    let second_received = tokio::time::timeout(Duration::from_secs(5), async {
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
    let third_received = tokio::time::timeout(Duration::from_secs(5), async {
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

#[tokio::test]
async fn test_topic_tracker_announce_and_find_peers() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node").await;
    let doc = node
        .docs_engine()
        .create_namespace()
        .await
        .expect("create namespace");

    node.topic_tracker()
        .announce(doc.id())
        .await
        .expect("announce topic");
    let peers = node
        .topic_tracker()
        .find_peers(doc.id())
        .await
        .expect("find peers");
    assert!(peers.is_empty());

    node.stop().await.expect("stop node");
}

#[test]
fn test_bubble_detection_strategy_is_enabled() {
    assert!(matches!(
        distributed_topic_tracker::Config::default()
            .merge_config()
            .bubble_merge(),
        distributed_topic_tracker::BubbleMergeConfig::Enabled(_)
    ));
}
