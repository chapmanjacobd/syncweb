use std::time::Duration;

use anyhow::Context;
use iroh::address_lookup::memory::MemoryLookup;
use n0_future::StreamExt;
use syncweb_core::{
    folder::{
        Capability, CollectionEntry, CollectionManifest, CollectionStore, FolderManager, PackageManager, SyncMode,
    },
    node::{
        identity::IdentityManager,
        iroh_node::{DiscoveryConfig, IrohNode, RelayMode},
    },
    storage::node_db::NodeDatabase,
};

use crate::test_utils::TestDirectory;

async fn node(directory: &TestDirectory, name: &str) -> anyhow::Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    Ok(IrohNode::new(
        identity,
        root.join("data"),
        RelayMode::Default,
        crate::test_utils::empty_member_keys(),
        DiscoveryConfig::disabled(),
    )
    .await?)
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
    anyhow::ensure!(sr.mode().can_write_locally());
    anyhow::ensure!(sr.mode().can_receive());
    anyhow::ensure!(sr.mode().to_string() == "sendreceive");

    let so = manager.create(SyncMode::SendOnly).await?;
    anyhow::ensure!(so.mode().can_write_locally());
    anyhow::ensure!(!so.mode().can_receive());
    anyhow::ensure!(so.mode().to_string() == "sendonly");

    let ro = manager.create(SyncMode::ReceiveOnly).await?;
    anyhow::ensure!(!ro.mode().can_write_locally());
    anyhow::ensure!(ro.mode().can_receive());
    anyhow::ensure!(ro.mode().to_string() == "receiveonly");

    let re = manager.create(SyncMode::ReceiveEncrypted).await?;
    anyhow::ensure!(!re.mode().can_write_locally());
    anyhow::ensure!(re.mode().can_receive());
    anyhow::ensure!(re.mode().to_string() == "receiveencrypted");

    anyhow::ensure!(manager.list().await?.len() == 4);

    test_node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_public_blob_subscription_uses_blob_store() -> anyhow::Result<()> {
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
            DiscoveryConfig::disabled(),
            crate::test_utils::empty_member_keys(),
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
            DiscoveryConfig::disabled(),
            crate::test_utils::empty_member_keys(),
        )
        .await?
    };
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(first.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(second.endpoint().id()).with_relay_url(relay_url));

    let hash = first.blob_store().add_bytes(b"public subscription").await?;
    let ticket = first.blob_store().ticket(first.endpoint(), hash);
    let subscribed_hash = FolderManager::new(&second).subscribe_public(&ticket).await?;
    anyhow::ensure!(subscribed_hash == hash);
    anyhow::ensure!(second.blob_store().get(hash).await? == b"public subscription".as_slice());
    // No doc namespace should have been created
    anyhow::ensure!(second.docs_engine().inner().list().await?.count().await == 0);

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
        DiscoveryConfig::disabled(),
        crate::test_utils::empty_member_keys(),
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
        DiscoveryConfig::disabled(),
        crate::test_utils::empty_member_keys(),
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
        DiscoveryConfig::disabled(),
        crate::test_utils::empty_member_keys(),
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
        DiscoveryConfig::disabled(),
        crate::test_utils::empty_member_keys(),
    )
    .await?;

    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(node_a.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(node_b.endpoint().id()).with_relay_url(relay_url));

    let manager_a = FolderManager::new(&node_a);
    let folder_a = manager_a.create(SyncMode::SendOnly).await?;
    anyhow::ensure!(folder_a.mode().can_write_locally());
    anyhow::ensure!(!folder_a.mode().can_receive());

    folder_a.set_blob("doc.txt", b"sent from A").await?;

    node_a.topic_tracker().announce(folder_a.namespace_id()).await?;

    let ticket = folder_a.ticket(node_a.endpoint().addr(), true).await?;

    let manager_b = FolderManager::new(&node_b);
    let folder_b = manager_b.join(ticket.to_string(), SyncMode::ReceiveOnly).await?;
    anyhow::ensure!(!folder_b.mode().can_write_locally());
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

// ---------------------------------------------------------------------------
// Workflow-style tests using the World DSL
// ---------------------------------------------------------------------------

#[tokio::test]
async fn workflow_two_nodes_sync() -> anyhow::Result<()> {
    let world = crate::integration::world::World::new(&["alice", "bob"]).await?;
    let alice = world.device("alice")?;
    let bob = world.device("bob")?;

    let folder_a = alice.create_folder(SyncMode::SendReceive).await?;
    let hash = alice.write(&folder_a, "hello.txt", b"hello from alice").await?;

    let ticket = folder_a.folder.ticket(alice.endpoint().addr(), true).await?;

    let folder_b = bob.join_folder(&ticket.to_string(), SyncMode::ReceiveOnly).await?;

    let entry = bob.wait_entry(folder_b.namespace, "hello.txt").await?;
    anyhow::ensure!(entry.content_hash() == hash);

    let data = bob.wait_blob(hash).await?;
    anyhow::ensure!(data.as_ref() == b"hello from alice");

    alice.node().stop().await?;
    bob.node().stop().await?;
    Ok(())
}

#[tokio::test]
async fn workflow_bidirectional_sync() -> anyhow::Result<()> {
    let world = crate::integration::world::World::new(&["alice", "bob"]).await?;
    let alice = world.device("alice")?;
    let bob = world.device("bob")?;

    let folder_a = alice.create_folder(SyncMode::SendReceive).await?;
    alice.write(&folder_a, "from-alice.txt", b"alice's file").await?;

    let ticket = folder_a.folder.ticket(alice.endpoint().addr(), true).await?;

    let folder_b = bob.join_folder(&ticket.to_string(), SyncMode::SendReceive).await?;
    bob.write(&folder_b, "from-bob.txt", b"bob's file").await?;

    // Verify bob can read alice's file.
    let entry_a = bob.wait_entry(folder_b.namespace, "from-alice.txt").await?;
    let data = bob.wait_blob(entry_a.content_hash()).await?;
    anyhow::ensure!(data.as_ref() == b"alice's file");

    // Verify alice can read bob's file.
    let entry_b = alice.wait_entry(folder_a.namespace, "from-bob.txt").await?;
    let bob_data = alice.wait_blob(entry_b.content_hash()).await?;
    anyhow::ensure!(bob_data.as_ref() == b"bob's file");

    alice.node().stop().await?;
    bob.node().stop().await?;
    Ok(())
}

#[tokio::test]
async fn workflow_sendonly_receiveonly() -> anyhow::Result<()> {
    let world = crate::integration::world::World::new(&["sender", "receiver"]).await?;
    let sender = world.device("sender")?;
    let receiver = world.device("receiver")?;

    let folder_s = sender.create_folder(SyncMode::SendOnly).await?;
    let hash = sender.write(&folder_s, "data.bin", b"binary data").await?;

    let ticket = folder_s.folder.ticket(sender.endpoint().addr(), false).await?;

    let folder_r = receiver.join_folder(&ticket.to_string(), SyncMode::ReceiveOnly).await?;

    let entry = receiver.wait_entry(folder_r.namespace, "data.bin").await?;
    anyhow::ensure!(entry.content_hash() == hash);

    // ReceiveOnly cannot write
    let write_result = receiver.write(&folder_r, "should-fail.txt", b"nope").await;
    anyhow::ensure!(write_result.is_err());

    sender.node().stop().await?;
    receiver.node().stop().await?;
    Ok(())
}

#[tokio::test]
async fn workflow_world_devices_and_directory() -> anyhow::Result<()> {
    let world = crate::integration::world::World::new(&["alice", "bob"]).await?;

    let all = world.devices();
    anyhow::ensure!(all.len() == 2, "expected 2 devices, got {}", all.len());
    anyhow::ensure!(all.iter().any(|d| d.name == "alice"), "alice missing from devices()");
    anyhow::ensure!(all.iter().any(|d| d.name == "bob"), "bob missing from devices()");

    let dir = world.directory();
    anyhow::ensure!(dir.exists(), "directory should exist: {dir:?}");

    let alice = world.device("alice")?;
    let alice_dir = alice.dir();
    anyhow::ensure!(alice_dir.exists(), "alice dir should exist: {alice_dir:?}");

    Ok(())
}

#[tokio::test]
async fn workflow_list_entries_after_sync() -> anyhow::Result<()> {
    let world = crate::integration::world::World::new(&["alice", "bob"]).await?;
    let alice = world.device("alice")?;
    let bob = world.device("bob")?;

    let folder_a = alice.create_folder(SyncMode::SendReceive).await?;
    alice.write(&folder_a, "file1.txt", b"content1").await?;
    alice.write(&folder_a, "file2.txt", b"content2").await?;

    let ticket = folder_a.folder.ticket(alice.endpoint().addr(), true).await?;

    let folder_b = bob.join_folder(&ticket.to_string(), SyncMode::ReceiveOnly).await?;
    bob.wait_entry(folder_b.namespace, "file1.txt").await?;

    let entries = alice.list_entries(folder_a.namespace).await?;
    anyhow::ensure!(
        entries.len() >= 2,
        "alice should list >= 2 entries, got {}",
        entries.len()
    );

    let bob_entries = bob.list_entries(folder_b.namespace).await?;
    anyhow::ensure!(
        bob_entries.iter().any(|e| e.key() == b"file1.txt"),
        "bob should see file1.txt: {bob_entries:?}"
    );

    alice.node().stop().await?;
    bob.node().stop().await?;
    Ok(())
}

#[tokio::test]
async fn workflow_manager_lists_folders() -> anyhow::Result<()> {
    let world = crate::integration::world::World::new(&["alice"]).await?;
    let alice = world.device("alice")?;

    let handle = alice.create_folder(SyncMode::SendReceive).await?;
    let folders = handle.manager.list().await?;
    anyhow::ensure!(!folders.is_empty(), "manager should list at least one folder");

    let handle2 = alice.create_folder(SyncMode::SendReceive).await?;
    let folders2 = handle2.manager.list().await?;
    anyhow::ensure!(folders2.len() >= 2, "should have >= 2 folders after second create");

    alice.node().stop().await?;
    Ok(())
}

#[tokio::test]
async fn workflow_memory_lookup_has_endpoints() -> anyhow::Result<()> {
    let world = crate::integration::world::World::new(&["alice", "bob"]).await?;

    let alice = world.device("alice")?;
    let bob = world.device("bob")?;

    let alice_info = world.memory_lookup.get_endpoint_info(alice.endpoint().id());
    anyhow::ensure!(alice_info.is_some(), "alice endpoint should be in memory lookup");

    let bob_info = world.memory_lookup.get_endpoint_info(bob.endpoint().id());
    anyhow::ensure!(bob_info.is_some(), "bob endpoint should be in memory lookup");

    alice.node().stop().await?;
    bob.node().stop().await?;
    Ok(())
}

/// Publish an IIAB package, share to a third party while publisher is online,
/// publisher disconnects, third party verifies files on disk.
#[tokio::test]
async fn workflow_iiab_publish_share_after_disconnect() -> anyhow::Result<()> {
    let world = crate::integration::world::World::new(&["publisher", "third_party"]).await?;
    let publisher = world.device("publisher")?;
    let third_party = world.device("third_party")?;

    let folder = publisher.create_folder(SyncMode::SendReceive).await?;
    let ticket = folder.folder.ticket(publisher.endpoint().addr(), true).await?;
    let ns = folder.namespace;

    let mut manifest = CollectionManifest::new(uuid::Uuid::new_v4(), "1.0.0");

    let files: &[(&str, &[u8])] = &[
        ("index.html", b"<html><body>Wikipedia</body></html>"),
        ("data/articles.zim", b"zim-content-here"),
        ("data/thumbs.db", b"thumb-cache"),
    ];
    for (name, data) in files {
        let hash = publisher.write(&folder, name, data).await?;
        manifest
            .entries
            .push(CollectionEntry::new(hash, *name, u64::try_from(data.len())?)?);
    }

    let store = CollectionStore::new(
        folder.folder.doc().clone(),
        folder.folder.author(),
        publisher.node().blob_store().clone(),
        publisher.node().docs_engine().clone(),
    );
    let head = store.publish(&manifest, 1).await?;
    let manifest_ticket = publisher
        .node()
        .blob_store()
        .ticket(publisher.endpoint(), head.manifest)
        .to_string();

    third_party
        .join_folder(&ticket.to_string(), SyncMode::ReceiveOnly)
        .await?;
    third_party.wait_entry(ns, "index.html").await?;

    let node_db = NodeDatabase::open(world.directory().join("third_party.db"))?;
    let packages = PackageManager::new(world.directory().join("packages"), node_db);
    let blob_ticket = manifest_ticket.parse()?;
    let installed = packages
        .install_from_ticket(&blob_ticket, third_party.endpoint(), third_party.node().blob_store())
        .await?;
    anyhow::ensure!(installed.collection_id == manifest.collection_id);
    anyhow::ensure!(installed.version == "1.0.0");

    publisher.node().stop().await?;

    let state = packages.state()?;
    let info = state
        .current(installed.collection_id)
        .context("package should be installed")?;
    anyhow::ensure!(info.current == "1.0.0");

    let pkg_dir = packages.root().join(installed.collection_id.to_string()).join("1.0.0");
    anyhow::ensure!(pkg_dir.join("index.html").exists());
    anyhow::ensure!(std::fs::read(pkg_dir.join("index.html"))? == b"<html><body>Wikipedia</body></html>");
    anyhow::ensure!(pkg_dir.join("data/articles.zim").exists());
    anyhow::ensure!(std::fs::read(pkg_dir.join("data/articles.zim"))? == b"zim-content-here");
    anyhow::ensure!(pkg_dir.join("data/thumbs.db").exists());
    anyhow::ensure!(std::fs::read(pkg_dir.join("data/thumbs.db"))? == b"thumb-cache");

    packages.verify(&installed)?;

    third_party.node().stop().await?;
    Ok(())
}

/// Upgrade a package to a new version that reuses the same file paths with
/// new content.  Both versions must coexist and the `current` symlink must
/// point at the latest.
#[tokio::test]
async fn workflow_iiab_update_same_paths() -> anyhow::Result<()> {
    let world = crate::integration::world::World::new(&["publisher", "consumer"]).await?;
    let publisher = world.device("publisher")?;
    let consumer = world.device("consumer")?;

    let folder = publisher.create_folder(SyncMode::SendReceive).await?;
    let ticket = folder.folder.ticket(publisher.endpoint().addr(), true).await?;
    let ns = folder.namespace;
    let collection_id = uuid::Uuid::new_v4();

    let v1_files: &[(&str, &[u8])] = &[("README.md", b"v1 readme"), ("bin/tool", b"tool-v1")];
    let mut v1_manifest = CollectionManifest::new(collection_id, "1.0.0");
    for (name, data) in v1_files {
        let hash = publisher.write(&folder, name, data).await?;
        v1_manifest
            .entries
            .push(CollectionEntry::new(hash, *name, u64::try_from(data.len())?)?);
    }
    let store = CollectionStore::new(
        folder.folder.doc().clone(),
        folder.folder.author(),
        publisher.node().blob_store().clone(),
        publisher.node().docs_engine().clone(),
    );
    let head_v1 = store.publish(&v1_manifest, 1).await?;
    let ticket_v1 = publisher
        .node()
        .blob_store()
        .ticket(publisher.endpoint(), head_v1.manifest)
        .to_string();

    consumer.join_folder(&ticket.to_string(), SyncMode::ReceiveOnly).await?;
    consumer.wait_entry(ns, "README.md").await?;

    let node_db = NodeDatabase::open(world.directory().join("consumer.db"))?;
    let packages = PackageManager::new(world.directory().join("packages"), node_db);
    let installed_v1 = packages
        .install_from_ticket(&ticket_v1.parse()?, consumer.endpoint(), consumer.node().blob_store())
        .await?;
    anyhow::ensure!(installed_v1.version == "1.0.0");

    let pkg_root = packages.root().join(collection_id.to_string());
    anyhow::ensure!(std::fs::read(pkg_root.join("current/README.md"))? == b"v1 readme");
    anyhow::ensure!(std::fs::read(pkg_root.join("current/bin/tool"))? == b"tool-v1");

    let v2_files: &[(&str, &[u8])] = &[("README.md", b"v2 readme -- updated"), ("bin/tool", b"tool-v2")];
    let mut v2_manifest = CollectionManifest::new(collection_id, "2.0.0");
    v2_manifest.parent = Some(head_v1.manifest);
    v2_manifest.changelog = Some("Same paths, new content".into());
    for (name, data) in v2_files {
        let hash = publisher.write(&folder, name, data).await?;
        v2_manifest
            .entries
            .push(CollectionEntry::new(hash, *name, u64::try_from(data.len())?)?);
    }
    let head_v2 = store.publish(&v2_manifest, 2).await?;
    let ticket_v2 = publisher
        .node()
        .blob_store()
        .ticket(publisher.endpoint(), head_v2.manifest)
        .to_string();

    let installed_v2 = packages
        .install_from_ticket(&ticket_v2.parse()?, consumer.endpoint(), consumer.node().blob_store())
        .await?;
    anyhow::ensure!(installed_v2.version == "2.0.0");

    let state = packages.state()?;
    let info = state.current(collection_id).context("collection should be installed")?;
    anyhow::ensure!(info.current == "2.0.0");
    anyhow::ensure!(info.versions.contains_key("1.0.0"));
    anyhow::ensure!(info.versions.contains_key("2.0.0"));

    anyhow::ensure!(std::fs::read(pkg_root.join("current/README.md"))? == b"v2 readme -- updated");
    anyhow::ensure!(std::fs::read(pkg_root.join("current/bin/tool"))? == b"tool-v2");

    anyhow::ensure!(std::fs::read(pkg_root.join("1.0.0/README.md"))? == b"v1 readme");
    anyhow::ensure!(std::fs::read(pkg_root.join("2.0.0/README.md"))? == b"v2 readme -- updated");

    packages.verify(&installed_v1)?;
    packages.verify(&installed_v2)?;

    let initial_link = std::fs::read_link(pkg_root.join("current"))?;
    anyhow::ensure!(initial_link == std::path::Path::new("2.0.0"));

    packages.switch(collection_id, "1.0.0")?;
    let switched_link = std::fs::read_link(pkg_root.join("current"))?;
    anyhow::ensure!(switched_link == std::path::Path::new("1.0.0"));
    anyhow::ensure!(std::fs::read(pkg_root.join("current/README.md"))? == b"v1 readme");

    packages.switch(collection_id, "2.0.0")?;
    let final_link = std::fs::read_link(pkg_root.join("current"))?;
    anyhow::ensure!(final_link == std::path::Path::new("2.0.0"));

    publisher.node().stop().await?;
    consumer.node().stop().await?;
    Ok(())
}
