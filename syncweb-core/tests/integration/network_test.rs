use anyhow::{Context, ensure};
use iroh::SecretKey;
use syncweb_core::{
    constants::NETWORK_MEMBERS_KEY,
    net::{
        NetworkManager, NetworkOptions, NetworkTicket,
        membership_doc::{SignedMemberList, write_member_list},
    },
    node::{
        identity::IdentityManager,
        iroh_node::{DiscoveryConfig, IrohNode, RelayMode},
    },
    storage::node_db::NodeDatabase,
};

use crate::test_utils::{TestDirectory, empty_member_keys};

async fn node(directory: &TestDirectory, name: &str) -> anyhow::Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    Ok(IrohNode::new(
        identity,
        root.join("data"),
        RelayMode::Default,
        empty_member_keys(),
        DiscoveryConfig::disabled(),
    )
    .await?)
}

async fn read_member_list(node: &IrohNode, doc: &iroh_docs::api::Doc) -> anyhow::Result<SignedMemberList> {
    let entry = node
        .docs_engine()
        .get_any(doc, NETWORK_MEMBERS_KEY)
        .await?
        .context("member list entry missing")?;
    let bytes = node.blob_store().get(entry.content_hash()).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[tokio::test]
async fn membership_doc_writer_signs_and_tracks_invites() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-network-membership")?;
    let alice = node(&directory, "alice").await?;
    let bob = node(&directory, "bob").await?;

    let db = NodeDatabase::open(directory.path().join("owner.db"))?;
    let mut manager = NetworkManager::new(db, alice.endpoint().id(), empty_member_keys())?;
    let id = manager.create("team", NetworkOptions::default())?;
    let network = manager.get(&id).context("network missing")?.clone();

    let docs = alice.docs_engine();
    let (doc, _write_ticket) = docs.create_or_open_namespace(None).await?;
    let author = docs.author().await?;
    let signing = ed25519_dalek::SigningKey::from_bytes(&alice.endpoint().secret_key().to_bytes());

    let initial_sequence = write_member_list(docs, alice.blob_store(), &doc, author, &signing, &network).await?;
    ensure!(initial_sequence == 1, "first member list should use sequence 1");

    let initial_list = read_member_list(&alice, &doc).await?;
    initial_list.verify()?;
    ensure!(initial_list.members.len() == 1);
    ensure!(initial_list.members.first().context("member entry missing")?.key == alice.endpoint().id().to_string());

    let member = bob.endpoint().id();
    manager.invite(id, member)?;
    let updated_network = manager.get(&id).context("network missing")?.clone();
    let next_sequence = write_member_list(docs, alice.blob_store(), &doc, author, &signing, &updated_network).await?;
    ensure!(next_sequence == 2, "second member list should use sequence 2");

    let updated_list = read_member_list(&alice, &doc).await?;
    updated_list.verify()?;
    ensure!(updated_list.members.len() == 2, "invited member should be listed");
    ensure!(
        updated_list
            .members
            .iter()
            .any(|entry| entry.key == bob.endpoint().id().to_string())
    );

    let read_ticket = docs.share_ticket(&doc, false).await?;
    let bob_doc = bob.docs_engine().import_ticket(read_ticket).await?;
    ensure!(bob_doc.id() == doc.id(), "imported membership doc should match");

    alice.stop().await?;
    bob.stop().await?;
    Ok(())
}

#[test]
fn network_lifecycle_persists_and_tickets_round_trip() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-network-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root)?;
    let result = (|| -> anyhow::Result<()> {
        let owner = SecretKey::generate().public();
        let member = SecretKey::generate().public();
        let owner_db = NodeDatabase::open(root.join("owner.db"))?;
        let mut owner_manager = NetworkManager::new(owner_db.clone(), owner, empty_member_keys())?;
        let id = owner_manager.create("work", NetworkOptions::default().with_label("Work").invite_only(true))?;
        let ticket = owner_manager.invite(id, member)?;
        let encoded = ticket.to_string();
        let decoded: NetworkTicket = encoded.parse()?;
        anyhow::ensure!(decoded == ticket);

        let member_db = NodeDatabase::open(root.join("member.db"))?;
        let mut member_manager = NetworkManager::new(member_db, member, empty_member_keys())?;
        anyhow::ensure!(member_manager.join(decoded)? == id);
        anyhow::ensure!(
            member_manager
                .get(&id)
                .is_some_and(|network| network.is_member(&member))
        );

        owner_manager.kick(id, &member)?;
        anyhow::ensure!(!owner_manager.get(&id).is_some_and(|network| network.is_member(&member)));
        drop(owner_manager);
        let reloaded = NetworkManager::new(owner_db, owner, empty_member_keys())?;
        anyhow::ensure!(reloaded.list().len() == 1);
        Ok(())
    })();
    std::fs::remove_dir_all(root)?;
    result
}

#[test]
fn test_network_create_rejects_empty_name() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-net-empty-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).context("unwrap failed")?;
    let result = (|| -> anyhow::Result<()> {
        let owner = SecretKey::generate().public();
        let db = NodeDatabase::open(root.join("node.db"))?;
        let mut manager = NetworkManager::new(db, owner, empty_member_keys()).context("unwrap failed")?;

        let res = manager.create("", NetworkOptions::default());
        ensure!(res.is_err());

        let res_spaces = manager.create("  ", NetworkOptions::default());
        ensure!(res_spaces.is_err());
        Ok(())
    })();
    std::fs::remove_dir_all(root).context("unwrap failed")?;
    result
}

#[test]
fn test_network_invite_rejects_non_owner() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-net-owner-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).context("unwrap failed")?;
    let result = (|| -> anyhow::Result<()> {
        let owner = SecretKey::generate().public();
        let other = SecretKey::generate().public();
        let db = NodeDatabase::open(root.join("node.db"))?;
        let mut manager_owner = NetworkManager::new(db.clone(), owner, empty_member_keys()).context("unwrap failed")?;
        let mut manager_other = NetworkManager::new(db, other, empty_member_keys()).context("unwrap failed")?;

        let id = manager_owner
            .create("test", NetworkOptions::default())
            .context("unwrap failed")?;

        let invite_result = manager_other.invite(id, SecretKey::generate().public());
        ensure!(invite_result.is_err());
        Ok(())
    })();
    std::fs::remove_dir_all(root).context("unwrap failed")?;
    result
}

#[test]
fn test_network_kick_owner_rejected() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-net-kick-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).context("unwrap failed")?;
    let result = (|| -> anyhow::Result<()> {
        let owner = SecretKey::generate().public();
        let db = NodeDatabase::open(root.join("node.db"))?;
        let mut manager = NetworkManager::new(db, owner, empty_member_keys()).context("unwrap failed")?;

        let id = manager
            .create("test", NetworkOptions::default())
            .context("unwrap failed")?;
        let kick_result = manager.kick(id, &owner);
        ensure!(kick_result.is_err());
        Ok(())
    })();
    std::fs::remove_dir_all(root).context("unwrap failed")?;
    result
}

#[test]
fn test_network_leave_removes_network() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-net-leave-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).context("unwrap failed")?;
    let result = (|| -> anyhow::Result<()> {
        let owner = SecretKey::generate().public();
        let db = NodeDatabase::open(root.join("node.db"))?;
        let mut manager = NetworkManager::new(db, owner, empty_member_keys()).context("unwrap failed")?;

        let id = manager
            .create("test", NetworkOptions::default())
            .context("unwrap failed")?;
        anyhow::ensure!(manager.list().len() == 1);

        manager.leave(id).context("unwrap failed")?;
        ensure!(manager.list().is_empty());
        ensure!(manager.get(&id).is_none());
        Ok(())
    })();
    std::fs::remove_dir_all(root).context("unwrap failed")?;
    result
}

#[test]
fn test_network_folder_membership() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-net-folder-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).context("unwrap failed")?;
    let result = (|| -> anyhow::Result<()> {
        let owner = SecretKey::generate().public();
        let db = NodeDatabase::open(root.join("node.db"))?;
        let mut manager = NetworkManager::new(db, owner, empty_member_keys()).context("unwrap failed")?;

        let id = manager
            .create("test", NetworkOptions::default())
            .context("unwrap failed")?;
        let folder = iroh_docs::NamespaceId::default();

        manager.add_folder(id, folder).context("unwrap failed")?;
        let network = manager.get(&id).context("unwrap failed")?;
        ensure!(network.folders.contains(&folder));

        manager.remove_folder(id, folder).context("unwrap failed")?;
        let network_after = manager.get(&id).context("unwrap failed")?;
        ensure!(!network_after.folders.contains(&folder));
        Ok(())
    })();
    std::fs::remove_dir_all(root).context("unwrap failed")?;
    result
}

#[test]
fn test_network_ticket_round_trip_deterministic() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-net-ticket-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root)?;
    let result = (|| -> anyhow::Result<()> {
        let owner = SecretKey::generate().public();
        let member = SecretKey::generate().public();

        let db = NodeDatabase::open(root.join("node.db"))?;
        let mut manager = NetworkManager::new(db, owner, empty_member_keys())?;
        let id = manager.create("roundtrip", NetworkOptions::default().with_label("RT"))?;
        let ticket = manager.invite(id, member)?;

        let encoded = ticket.to_string();
        let first: NetworkTicket = encoded.parse()?;
        let second: NetworkTicket = encoded.parse()?;
        anyhow::ensure!(first == second);
        anyhow::ensure!(first.name == "roundtrip");
        anyhow::ensure!(first.label == "RT");
        anyhow::ensure!(first.is_invite_only() == ticket.is_invite_only());
        Ok(())
    })();
    std::fs::remove_dir_all(root)?;
    result
}

#[test]
fn test_network_id_from_name_is_stable() -> anyhow::Result<()> {
    let a = syncweb_core::net::NetworkId::from_name("hello");
    let b = syncweb_core::net::NetworkId::from_name("hello");
    let c = syncweb_core::net::NetworkId::from_name("world");
    anyhow::ensure!(a == b);
    anyhow::ensure!(a != c);
    Ok(())
}

#[test]
fn test_network_id_hex_round_trip() -> anyhow::Result<()> {
    let id = syncweb_core::net::NetworkId::from_name("test");
    let hex = id.to_string();
    let parsed: syncweb_core::net::NetworkId = hex.parse().context("unwrap failed")?;
    anyhow::ensure!(id == parsed);
    Ok(())
}
