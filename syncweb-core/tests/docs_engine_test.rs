use std::path::{Path, PathBuf};

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

async fn test_node(directory: &TestDirectory, name: &str, relay_map: Option<iroh::RelayMap>) -> IrohNode {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key")).expect("create identity");
    let relay_mode = match relay_map {
        Some(map) => RelayMode::Custom { map, insecure: true },
        None => RelayMode::Default,
    };
    IrohNode::new(identity, root.join("data"), relay_mode)
        .await
        .expect("start node")
}

#[tokio::test]
async fn test_create_namespace() {
    let directory = TestDirectory::new();
    let node = test_node(&directory, "node", None).await;
    let doc = node.docs_engine().create_namespace().await.expect("create namespace");
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

    let mut events = node.docs_engine().watch(&doc).await.expect("watch document");

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
        _ => panic!("unexpected event: {event:?}"),
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
