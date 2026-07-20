use std::path::{Path, PathBuf};

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
