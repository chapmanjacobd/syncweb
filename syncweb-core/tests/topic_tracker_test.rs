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
