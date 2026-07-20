use std::path::PathBuf;

use syncweb_core::{
    folder::{Capability, FolderManager, SyncMode},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("syncweb-folder-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).expect("create test directory");
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove test directory");
    }
}

async fn node(directory: &TestDirectory, name: &str) -> IrohNode {
    let root = directory.0.join(name);
    let identity = IdentityManager::new(root.join("identity.key")).expect("create identity");
    IrohNode::new(identity, root.join("data"), RelayMode::Default)
        .await
        .expect("start node")
}

#[tokio::test]
async fn create_join_list_and_drop_folder() {
    let directory = TestDirectory::new();
    let first = node(&directory, "first").await;
    let second = node(&directory, "second").await;
    let first_manager = FolderManager::new(&first);
    let folder = first_manager
        .create(SyncMode::SendReceive)
        .await
        .expect("create folder");
    let ticket = folder
        .ticket(first.endpoint().addr(), true)
        .await
        .expect("create ticket");

    let second_manager = FolderManager::new(&second);
    let joined = second_manager
        .join(ticket.to_string(), SyncMode::ReceiveOnly)
        .await
        .expect("join folder");
    assert_eq!(joined.namespace_id(), folder.namespace_id());
    assert_eq!(second_manager.list().await.expect("list folders").len(), 1);

    second_manager.drop(joined.namespace_id()).await.expect("drop folder");
    assert!(second_manager.list().await.expect("list folders").is_empty());

    first.stop().await.expect("stop first node");
    second.stop().await.expect("stop second node");
}

#[tokio::test]
async fn modes_enforce_local_writes_and_capabilities() {
    let directory = TestDirectory::new();
    let node = node(&directory, "node").await;
    let manager = FolderManager::new(&node);
    let receive_only = manager
        .create(SyncMode::ReceiveOnly)
        .await
        .expect("create receive-only folder");
    assert!(receive_only.set_blob("file", "data").await.is_err());

    let writable = manager
        .create(SyncMode::SendReceive)
        .await
        .expect("create writable folder");
    writable.grant(node.endpoint().id(), Capability::Write).await;
    assert!(writable.can_write_as(node.endpoint().id()).await);
    let hash = writable.set_blob("file", "data").await.expect("store blob");
    let entry = node
        .docs_engine()
        .get(writable.doc(), writable.author(), "file")
        .await
        .expect("read entry")
        .expect("entry exists");
    assert_eq!(entry.content_hash(), hash);
    assert_eq!(entry.content_len(), 4);

    node.stop().await.expect("stop node");
}
