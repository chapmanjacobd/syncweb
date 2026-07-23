mod test_utils;

use std::time::Duration;

use anyhow::Context;
use iroh::address_lookup::memory::MemoryLookup;
use syncweb_core::{
    folder::{Capability, CollectionEntry, CollectionManifest, CollectionStore, FolderManager, SyncMode},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
};

use crate::test_utils::TestDirectory;

async fn node(directory: &TestDirectory, name: &str) -> anyhow::Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    Ok(IrohNode::new(identity, root.join("data"), RelayMode::Default).await?)
}

#[tokio::test]
async fn create_join_list_and_drop_folder() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-folder-test")?;
    let first = node(&directory, "first").await?;
    let second = node(&directory, "second").await?;
    let first_manager = FolderManager::new(&first);
    let folder = first_manager.create(SyncMode::SendReceive).await?;
    let ticket = folder.ticket(first.endpoint().addr(), true).await?;

    let second_manager = FolderManager::new(&second);
    let joined = second_manager.join(ticket.to_string(), SyncMode::ReceiveOnly).await?;
    anyhow::ensure!(joined.namespace_id() == folder.namespace_id());
    anyhow::ensure!(second_manager.list().await?.len() == 1);

    second_manager.drop(joined.namespace_id()).await?;
    anyhow::ensure!(second_manager.list().await?.is_empty());

    first.stop().await?;
    second.stop().await?;
    Ok(())
}

#[tokio::test]
async fn modes_enforce_local_writes_and_capabilities() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-folder-test")?;
    let node = node(&directory, "node").await?;
    let manager = FolderManager::new(&node);
    let receive_only = manager.create(SyncMode::ReceiveOnly).await?;
    anyhow::ensure!(receive_only.set_blob("file", "data").await.is_err());

    let writable = manager.create(SyncMode::SendReceive).await?;
    writable.grant(node.endpoint().id(), Capability::Write).await;
    anyhow::ensure!(writable.can_write_as(node.endpoint().id()).await);
    let hash = writable.set_blob("file", "data").await?;
    let entry = node
        .docs_engine()
        .get(writable.doc(), writable.author(), "file")
        .await?
        .context("entry exists")?;
    anyhow::ensure!(entry.content_hash() == hash);
    anyhow::ensure!(entry.content_len() == 4);

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_sync_modes() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-folder-test")?;
    let test_node = node(&directory, "node").await?;
    let manager = FolderManager::new(&test_node);

    let sr = manager.create(SyncMode::SendReceive).await?;
    anyhow::ensure!(sr.mode().can_write());
    anyhow::ensure!(sr.mode().can_receive());
    anyhow::ensure!(!sr.mode().is_public());
    anyhow::ensure!(sr.mode().to_string() == "sendreceive");

    let so = manager.create(SyncMode::SendOnly).await?;
    anyhow::ensure!(so.mode().can_write());
    anyhow::ensure!(!so.mode().can_receive());
    anyhow::ensure!(!so.mode().is_public());
    anyhow::ensure!(so.mode().to_string() == "sendonly");

    let ro = manager.create(SyncMode::ReceiveOnly).await?;
    anyhow::ensure!(!ro.mode().can_write());
    anyhow::ensure!(ro.mode().can_receive());
    anyhow::ensure!(!ro.mode().is_public());
    anyhow::ensure!(ro.mode().to_string() == "receiveonly");

    let re = manager.create(SyncMode::ReceiveEncrypted).await?;
    anyhow::ensure!(!re.mode().can_write());
    anyhow::ensure!(re.mode().can_receive());
    anyhow::ensure!(!re.mode().is_public());
    anyhow::ensure!(re.mode().to_string() == "receiveencrypted");

    let pro = manager.create(SyncMode::PublicReadOnly).await?;
    anyhow::ensure!(!pro.mode().can_write());
    anyhow::ensure!(pro.mode().can_receive());
    anyhow::ensure!(pro.mode().is_public());
    anyhow::ensure!(pro.mode().to_string() == "publicreadonly");

    anyhow::ensure!(manager.list().await?.len() == 5);

    test_node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_public_readonly_blob_ticket_requires_no_folder_capability() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-folder-test")?;
    let node = node(&directory, "node").await?;
    let manager = FolderManager::new(&node);
    let folder = manager.create(SyncMode::PublicReadOnly).await?;
    let hash = node.blob_store().add_bytes(b"public content").await?;

    let ticket = folder.publish_blob(node.endpoint().addr(), hash).await?;
    anyhow::ensure!(ticket.hash() == hash);
    anyhow::ensure!(
        node.blob_store()
            .is_pinned(format!("syncweb/public/{}/{}", folder.namespace_id(), hash), hash)
            .await?
    );
    anyhow::ensure!(!folder.can_write_as(node.endpoint().id()).await);

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_public_readonly_mode_persists_after_manager_restart() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-folder-test")?;
    let node = node(&directory, "node").await?;
    let manager = FolderManager::new(&node);
    let folder = manager.create(SyncMode::PublicReadOnly).await?;

    let restarted_manager = FolderManager::new(&node);
    let restored = restarted_manager.get(folder.namespace_id()).await?;
    anyhow::ensure!(restored.mode() == SyncMode::PublicReadOnly);
    anyhow::ensure!(!restored.mode().can_write());

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_public_blob_subscription_creates_readonly_folder() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-folder-test")?;
    let (relay_map, relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let memory_lookup = MemoryLookup::new();
    let first = {
        let root = directory.path().join("publisher");
        let identity = IdentityManager::new(root.join("identity.key"))?;
        IrohNode::new_with_address_lookup(
            identity,
            root.join("data"),
            RelayMode::Custom {
                map: relay_map.clone(),
                insecure: true,
            },
            memory_lookup.clone(),
        )
        .await?
    };
    let second = {
        let root = directory.path().join("subscriber");
        let identity = IdentityManager::new(root.join("identity.key"))?;
        IrohNode::new_with_address_lookup(
            identity,
            root.join("data"),
            RelayMode::Custom {
                map: relay_map,
                insecure: true,
            },
            memory_lookup.clone(),
        )
        .await?
    };
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(first.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(second.endpoint().id()).with_relay_url(relay_url));

    let hash = first.blob_store().add_bytes(b"public subscription").await?;
    let ticket = first.blob_store().ticket(first.endpoint(), hash);
    let folder = FolderManager::new(&second).subscribe_public(&ticket).await?;
    anyhow::ensure!(folder.mode() == SyncMode::PublicReadOnly);
    let entry = second
        .docs_engine()
        .get(folder.doc(), folder.author(), "public/content")
        .await?
        .context("public content entry")?;
    anyhow::ensure!(entry.content_hash() == hash);
    anyhow::ensure!(second.blob_store().get(hash).await? == b"public subscription".as_slice());

    first.stop().await?;
    second.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_collection_head_is_persisted_and_monotonic() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-folder-test")?;
    let node = node(&directory, "node").await?;
    let folder = FolderManager::new(&node).create(SyncMode::SendReceive).await?;
    let collection_id = uuid::Uuid::new_v4();
    let content = node.blob_store().add_bytes(b"collection v1").await?;
    let mut manifest = CollectionManifest::new(collection_id, "1.0.0");
    manifest.entries.push(CollectionEntry::new(content, "file", 13)?);
    let store = CollectionStore::new(
        folder.doc().clone(),
        folder.author(),
        node.blob_store().clone(),
        node.docs_engine().clone(),
    );

    let first_head = store.publish(&manifest, 1).await?;
    anyhow::ensure!(store.head(collection_id).await? == Some(first_head));
    anyhow::ensure!(store.publish(&manifest, 1).await.is_err());

    manifest.version = "1.1.0".to_owned();
    manifest.parent = Some(first_head.manifest);
    let second_head = store.publish(&manifest, 2).await?;
    anyhow::ensure!(second_head.sequence == 2);
    anyhow::ensure!(store.head(collection_id).await? == Some(second_head));

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_capability_map() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-folder-test")?;
    let test_node = node(&directory, "node").await?;
    let manager = FolderManager::new(&test_node);
    let folder = manager.create(SyncMode::SendReceive).await?;

    let admin_key = iroh::SecretKey::generate().public();
    let write_key = iroh::SecretKey::generate().public();
    let read_key = iroh::SecretKey::generate().public();
    let unknown_key = iroh::SecretKey::generate().public();

    folder.grant(admin_key, Capability::Admin).await;
    folder.grant(write_key, Capability::Write).await;
    folder.grant(read_key, Capability::Read).await;

    anyhow::ensure!(folder.capability(admin_key).await == Some(Capability::Admin));
    anyhow::ensure!(folder.capability(write_key).await == Some(Capability::Write));
    anyhow::ensure!(folder.capability(read_key).await == Some(Capability::Read));
    anyhow::ensure!(folder.capability(unknown_key).await == None);

    anyhow::ensure!(Capability::Admin.can_write());
    anyhow::ensure!(Capability::Write.can_write());
    anyhow::ensure!(!Capability::Read.can_write());

    test_node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_accept_folder() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-folder-test")?;
    let test_node = node(&directory, "node").await?;
    let manager = FolderManager::new(&test_node);
    let folder = manager.create(SyncMode::SendReceive).await?;
    let ns = folder.namespace_id();

    let accepted = manager.accept(ns).await?;
    anyhow::ensure!(accepted.namespace_id() == ns);

    let listed = manager.list().await?;
    anyhow::ensure!(listed.len() >= 1);
    anyhow::ensure!(listed.iter().any(|f| f.namespace_id() == ns));

    test_node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_accept_returns_existing_if_already_managed() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-folder-test")?;
    let test_node = node(&directory, "node").await?;
    let manager = FolderManager::new(&test_node);

    let folder = manager.create(SyncMode::SendReceive).await?;
    let ns = folder.namespace_id();

    let accepted = manager.accept(ns).await?;
    anyhow::ensure!(accepted.namespace_id() == ns);
    anyhow::ensure!(manager.list().await?.len() == 1);

    test_node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_two_nodes_sync_files() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-folder-test")?;
    let (relay_map, relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let memory_lookup = MemoryLookup::new();

    let root_a = directory.path().join("node_a");
    let identity_a = IdentityManager::new(root_a.join("identity.key"))?;
    let node_a = IrohNode::new_with_address_lookup(
        identity_a,
        root_a.join("data"),
        RelayMode::Custom {
            map: relay_map.clone(),
            insecure: true,
        },
        memory_lookup.clone(),
    )
    .await?;

    let root_b = directory.path().join("node_b");
    let identity_b = IdentityManager::new(root_b.join("identity.key"))?;
    let node_b = IrohNode::new_with_address_lookup(
        identity_b,
        root_b.join("data"),
        RelayMode::Custom {
            map: relay_map,
            insecure: true,
        },
        memory_lookup.clone(),
    )
    .await?;

    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(node_a.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(node_b.endpoint().id()).with_relay_url(relay_url));

    let manager_a = FolderManager::new(&node_a);
    let folder_a = manager_a.create(SyncMode::SendReceive).await?;

    folder_a.grant(node_a.endpoint().id(), Capability::Admin).await;
    let hash = folder_a.set_blob("hello.txt", b"hello from A").await?;

    node_a.topic_tracker().announce(folder_a.namespace_id()).await?;

    let ticket = folder_a.ticket(node_a.endpoint().addr(), true).await?;

    let manager_b = FolderManager::new(&node_b);
    let folder_b = manager_b.join(ticket.to_string(), SyncMode::ReceiveOnly).await?;

    node_b.topic_tracker().announce(folder_b.namespace_id()).await?;

    let entry = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Some(entry) = node_b
                .docs_engine()
                .get(folder_b.doc(), folder_a.author(), "hello.txt")
                .await?
            {
                return anyhow::Ok(entry);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .context("timed out waiting for entry sync")?
    .context("entry should exist on B")?;
    anyhow::ensure!(entry.content_hash() == hash);

    let blob_bytes = node_b.blob_store().get(hash).await?;
    anyhow::ensure!(blob_bytes.as_ref() == b"hello from A");

    node_a.stop().await?;
    node_b.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_sendonly_receiveonly_sync() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-folder-test")?;
    let (relay_map, relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let memory_lookup = MemoryLookup::new();

    let root_a = directory.path().join("sender");
    let identity_a = IdentityManager::new(root_a.join("identity.key"))?;
    let node_a = IrohNode::new_with_address_lookup(
        identity_a,
        root_a.join("data"),
        RelayMode::Custom {
            map: relay_map.clone(),
            insecure: true,
        },
        memory_lookup.clone(),
    )
    .await?;

    let root_b = directory.path().join("receiver");
    let identity_b = IdentityManager::new(root_b.join("identity.key"))?;
    let node_b = IrohNode::new_with_address_lookup(
        identity_b,
        root_b.join("data"),
        RelayMode::Custom {
            map: relay_map,
            insecure: true,
        },
        memory_lookup.clone(),
    )
    .await?;

    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(node_a.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(node_b.endpoint().id()).with_relay_url(relay_url));

    let manager_a = FolderManager::new(&node_a);
    let folder_a = manager_a.create(SyncMode::SendOnly).await?;
    anyhow::ensure!(folder_a.mode().can_write());
    anyhow::ensure!(!folder_a.mode().can_receive());

    folder_a.set_blob("doc.txt", b"sent from A").await?;

    node_a.topic_tracker().announce(folder_a.namespace_id()).await?;

    let ticket = folder_a.ticket(node_a.endpoint().addr(), true).await?;

    let manager_b = FolderManager::new(&node_b);
    let folder_b = manager_b.join(ticket.to_string(), SyncMode::ReceiveOnly).await?;
    anyhow::ensure!(!folder_b.mode().can_write());
    anyhow::ensure!(folder_b.mode().can_receive());

    node_b.topic_tracker().announce(folder_b.namespace_id()).await?;

    let entry = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Some(entry) = node_b
                .docs_engine()
                .get(folder_b.doc(), folder_a.author(), "doc.txt")
                .await?
            {
                return anyhow::Ok(entry);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .context("timed out waiting for entry sync")?
    .context("entry should sync to B")?;
    anyhow::ensure!(entry.content_len() == 11);

    node_a.stop().await?;
    node_b.stop().await?;
    Ok(())
}

#[test]
fn test_namespace_key_derivation() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("syncweb-keyder-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory)?;
    let identity = IdentityManager::new(directory.join("key"))?;

    let ns_a = iroh_docs::NamespaceId::from([1_u8; 32]);
    let ns_b = iroh_docs::NamespaceId::from([2_u8; 32]);

    let key_a1 = identity.derive_folder_key(ns_a)?;
    let key_a2 = identity.derive_folder_key(ns_a)?;
    let key_b = identity.derive_folder_key(ns_b)?;

    anyhow::ensure!(key_a1.to_bytes() == key_a2.to_bytes());
    anyhow::ensure!(key_a1.to_bytes() != key_b.to_bytes());
    anyhow::ensure!(key_a1.to_bytes() != identity.secret_key().to_bytes());

    let author_a = identity.derive_folder_author(ns_a)?;
    let author_a2 = identity.derive_folder_author(ns_a)?;
    anyhow::ensure!(author_a.id() == author_a2.id());

    let author_b = identity.derive_folder_author(ns_b)?;
    anyhow::ensure!(author_a.id() != author_b.id());

    std::fs::remove_dir_all(&directory).context("failed to remove test directory")?;
    Ok(())
}
