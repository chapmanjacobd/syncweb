use std::path::{Path, PathBuf};
use std::time::Duration;

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
) -> IrohNode {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key")).expect("create identity");
    let relay_mode = match relay_map {
        Some(map) => RelayMode::Custom {
            map,
            insecure: true,
        },
        None => RelayMode::Default,
    };
    IrohNode::new(identity, root.join("data"), relay_mode)
        .await
        .expect("start node")
}

#[tokio::test]
async fn test_add_bytes() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node", None).await;
    let hash = node
        .blob_store()
        .add_bytes(b"phase one blob")
        .await
        .expect("add blob");

    assert!(node.blob_store().has(hash).await.expect("check blob"));

    node.stop().await.expect("stop node");
}

#[tokio::test]
async fn test_has_blob() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node", None).await;
    let hash = node
        .blob_store()
        .add_bytes(b"blob data")
        .await
        .expect("add");
    assert!(node.blob_store().has(hash).await.unwrap());

    let fake_hash = iroh_blobs::Hash::from_bytes([0u8; 32]);
    assert!(!node.blob_store().has(fake_hash).await.unwrap());

    node.stop().await.unwrap();
}

#[tokio::test]
async fn test_get_blob() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node", None).await;
    let hash = node
        .blob_store()
        .add_bytes(b"blob data")
        .await
        .expect("add");

    let bytes = node.blob_store().get(hash).await.expect("get");
    assert_eq!(bytes, b"blob data".as_slice());

    node.stop().await.unwrap();
}

#[tokio::test]
async fn test_blob_ticket() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node", None).await;
    let hash = node
        .blob_store()
        .add_bytes(b"blob data")
        .await
        .expect("add");

    let ticket = node.blob_store().ticket(node.endpoint(), hash);
    assert_eq!(ticket.hash(), hash);
    assert!(ticket.to_string().starts_with("blob"));

    node.stop().await.unwrap();
}

#[tokio::test]
async fn test_add_file() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node", None).await;
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
    let (relay_map, _relay_url, _server) = iroh::test_utils::run_relay_server().await.unwrap();
    let first = test_node(&directory, "first", Some(relay_map.clone())).await;
    let second = test_node(&directory, "second", Some(relay_map)).await;
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
async fn test_create_namespace() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node", None).await;
    let doc = node
        .docs_engine()
        .create_namespace()
        .await
        .expect("create namespace");
    assert!(!doc.id().to_string().is_empty());
    node.stop().await.expect("stop node");
}

#[tokio::test]
async fn test_set_get_entry() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node", None).await;
    let doc = node.docs_engine().create_namespace().await.unwrap();
    let author = node.docs_engine().author().await.unwrap();

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

    node.stop().await.expect("stop node");
}

#[tokio::test]
async fn test_watch_entries() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node", None).await;
    let doc = node.docs_engine().create_namespace().await.unwrap();
    let author = node.docs_engine().author().await.unwrap();

    let mut events = node
        .docs_engine()
        .watch(&doc)
        .await
        .expect("watch document");

    node.docs_engine()
        .set(&doc, author, b"key2", b"value2")
        .await
        .expect("set document entry");

    let event = tokio::time::timeout(std::time::Duration::from_secs(5), events.next())
        .await
        .expect("watch event timed out")
        .expect("watch stream closed");

    match event {
        Ok(iroh_docs::engine::LiveEvent::InsertLocal { entry }) => {
            assert_eq!(entry.key(), b"key2".as_slice());
        }
        _ => panic!("unexpected event: {:?}", event),
    }

    node.stop().await.expect("stop node");
}

#[tokio::test]
async fn test_author_from_secret() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node", None).await;

    // Test creating (exporting) and importing an author
    let original_author_id = node.docs_engine().author().await.expect("get author");
    let author_secret = node
        .docs_engine()
        .export_author(original_author_id)
        .await
        .expect("export author")
        .expect("author exists");

    let secret_str = author_secret.to_string();

    let parsed_author = std::str::FromStr::from_str(&secret_str).expect("parse author secret");
    let imported_author_id = node
        .docs_engine()
        .import_author(parsed_author)
        .await
        .expect("import author");

    assert_eq!(original_author_id, imported_author_id);
    node.stop().await.expect("stop node");
}

#[tokio::test]
async fn test_subscribe_publish() {
    let directory = TestDirectory::new();
    let (relay_map, _relay_url, _server) = iroh::test_utils::run_relay_server().await.unwrap();
    let first = test_node(&directory, "first", Some(relay_map.clone())).await;
    let second = test_node(&directory, "second", Some(relay_map)).await;
    let topic = iroh_gossip::TopicId::from_bytes(rand::random());

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
    tokio::time::timeout(Duration::from_secs(60), second_topic.joined())
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

    let received = tokio::time::timeout(Duration::from_secs(60), async {
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
    let (relay_map, _relay_url, _server) = iroh::test_utils::run_relay_server().await.unwrap();
    let first = test_node(&directory, "first", Some(relay_map.clone())).await;
    let second = test_node(&directory, "second", Some(relay_map.clone())).await;
    let third = test_node(&directory, "third", Some(relay_map)).await;
    let topic = iroh_gossip::TopicId::from_bytes(rand::random());

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
    tokio::time::timeout(Duration::from_secs(60), second_topic.joined())
        .await
        .expect("gossip join timed out")
        .expect("gossip join failed");
    let mut third_topic = third
        .gossip_service()
        .subscribe(topic, vec![second.endpoint().id()])
        .await
        .expect("subscribe third node");
    tokio::time::timeout(Duration::from_secs(60), third_topic.joined())
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

    let second_received = tokio::time::timeout(Duration::from_secs(60), async {
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
    let third_received = tokio::time::timeout(Duration::from_secs(60), async {
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
async fn test_announce_topic() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node", None).await;
    let doc = node
        .docs_engine()
        .create_namespace()
        .await
        .expect("create namespace");

    node.topic_tracker()
        .announce(doc.id())
        .await
        .expect("announce topic");

    node.stop().await.expect("stop node");
}

#[tokio::test]
async fn test_find_peers() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node", None).await;
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
fn test_bubble_detection() {
    assert!(matches!(
        distributed_topic_tracker::Config::default()
            .merge_config()
            .bubble_merge(),
        distributed_topic_tracker::BubbleMergeConfig::Enabled(_)
    ));
}
