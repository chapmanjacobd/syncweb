use std::path::{Path, PathBuf};
use std::time::Duration;

use syncweb_core::node::identity::IdentityManager;
use syncweb_core::node::iroh_node::IrohNode;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("syncweb-node-test-{}", uuid::Uuid::new_v4()));
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
async fn test_endpoint_creation() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node").await;

    assert!(!node.endpoint().bound_sockets().is_empty());

    node.stop().await.expect("stop node");
}

#[tokio::test]
async fn test_router_setup() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node").await;

    assert!(node.is_running());
    let _ = node.blobs();
    let _ = node.docs();
    let _ = node.gossip();

    node.stop().await.expect("stop node");
}

#[tokio::test]
async fn test_protocol_registration() {
    let directory = TestDirectory::new();
    let server = test_node(&directory, "server").await;
    let client = test_node(&directory, "client").await;
    let server_address = server.endpoint().addr();

    for alpn in [
        iroh_blobs::protocol::ALPN,
        iroh_docs::ALPN,
        iroh_gossip::ALPN,
    ] {
        let connection = tokio::time::timeout(
            Duration::from_secs(5),
            client.endpoint().connect(server_address.clone(), alpn),
        )
        .await
        .expect("protocol connection timed out")
        .expect("registered protocol should connect");
        connection.close(0u32.into(), b"test complete");
    }

    client.stop().await.expect("stop client");
    server.stop().await.expect("stop server");
}

#[tokio::test]
async fn test_shutdown() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node").await;

    node.stop().await.expect("stop node");

    assert!(!node.is_running());
    assert!(node.endpoint().is_closed());
}

#[tokio::test]
async fn test_two_nodes_connect() {
    let directory = TestDirectory::new();
    let first = test_node(&directory, "first").await;
    let second = test_node(&directory, "second").await;

    let connection = tokio::time::timeout(
        Duration::from_secs(5),
        first
            .endpoint()
            .connect(second.endpoint().addr(), iroh_gossip::ALPN),
    )
    .await
    .expect("direct connection timed out")
    .expect("connect nodes directly");

    assert_eq!(connection.remote_id(), second.endpoint().id());

    first.stop().await.expect("stop first node");
    second.stop().await.expect("stop second node");
}
