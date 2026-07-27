use anyhow::{Context, ensure};
use iroh::SecretKey;
use syncweb_core::net::{NetworkManager, NetworkOptions, NetworkTicket};
use syncweb_core::storage::node_db::NodeDatabase;

use crate::test_utils::{empty_member_keys, no_public_network};

#[test]
fn network_lifecycle_persists_and_tickets_round_trip() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-network-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root)?;
    let owner = SecretKey::generate().public();
    let member = SecretKey::generate().public();
    let owner_db = NodeDatabase::open(root.join("owner.db"))?;
    let mut owner_manager = NetworkManager::new(owner_db.clone(), owner, empty_member_keys(), no_public_network())?;
    let id = owner_manager.create("work", NetworkOptions::default().with_label("Work").invite_only(true))?;
    let ticket = owner_manager.invite(id, member)?;
    let encoded = ticket.to_string();
    let decoded: NetworkTicket = encoded.parse()?;
    anyhow::ensure!(decoded == ticket);

    let member_db = NodeDatabase::open(root.join("member.db"))?;
    let mut member_manager = NetworkManager::new(member_db, member, empty_member_keys(), no_public_network())?;
    anyhow::ensure!(member_manager.join(decoded)? == id);
    anyhow::ensure!(
        member_manager
            .get(&id)
            .is_some_and(|network| network.is_member(&member))
    );

    owner_manager.kick(id, &member)?;
    anyhow::ensure!(!owner_manager.get(&id).is_some_and(|network| network.is_member(&member)));
    drop(owner_manager);
    let reloaded = NetworkManager::new(owner_db, owner, empty_member_keys(), no_public_network())?;
    anyhow::ensure!(reloaded.list().len() == 1);

    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_network_create_rejects_empty_name() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-net-empty-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).context("unwrap failed")?;
    let owner = SecretKey::generate().public();
    let db = NodeDatabase::open(root.join("node.db"))?;
    let mut manager = NetworkManager::new(db, owner, empty_member_keys(), no_public_network()).context("unwrap failed")?;

    let result = manager.create("", NetworkOptions::default());
    ensure!(result.is_err());

    let result_spaces = manager.create("  ", NetworkOptions::default());
    ensure!(result_spaces.is_err());

    std::fs::remove_dir_all(root).context("unwrap failed")?;
    Ok(())
}

#[test]
fn test_network_invite_rejects_non_owner() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-net-owner-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).context("unwrap failed")?;
    let owner = SecretKey::generate().public();
    let other = SecretKey::generate().public();
    let db = NodeDatabase::open(root.join("node.db"))?;
    let mut manager_owner = NetworkManager::new(db.clone(), owner, empty_member_keys(), no_public_network()).context("unwrap failed")?;
    let mut manager_other = NetworkManager::new(db, other, empty_member_keys(), no_public_network()).context("unwrap failed")?;

    let id = manager_owner
        .create("test", NetworkOptions::default())
        .context("unwrap failed")?;

    // Other node cannot invite.
    let result = manager_other.invite(id, SecretKey::generate().public());
    ensure!(result.is_err());

    std::fs::remove_dir_all(root).context("unwrap failed")?;
    Ok(())
}

#[test]
fn test_network_kick_owner_rejected() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-net-kick-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).context("unwrap failed")?;
    let owner = SecretKey::generate().public();
    let db = NodeDatabase::open(root.join("node.db"))?;
    let mut manager = NetworkManager::new(db, owner, empty_member_keys(), no_public_network()).context("unwrap failed")?;

    let id = manager
        .create("test", NetworkOptions::default())
        .context("unwrap failed")?;
    let result = manager.kick(id, &owner);
    ensure!(result.is_err());

    std::fs::remove_dir_all(root).context("unwrap failed")?;
    Ok(())
}

#[test]
fn test_network_leave_removes_network() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-net-leave-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).context("unwrap failed")?;
    let owner = SecretKey::generate().public();
    let db = NodeDatabase::open(root.join("node.db"))?;
    let mut manager = NetworkManager::new(db, owner, empty_member_keys(), no_public_network()).context("unwrap failed")?;

    let id = manager
        .create("test", NetworkOptions::default())
        .context("unwrap failed")?;
    anyhow::ensure!(manager.list().len() == 1);

    manager.leave(id).context("unwrap failed")?;
    ensure!(manager.list().is_empty());
    ensure!(manager.get(&id).is_none());

    std::fs::remove_dir_all(root).context("unwrap failed")?;
    Ok(())
}

#[test]
fn test_network_folder_membership() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-net-folder-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).context("unwrap failed")?;
    let owner = SecretKey::generate().public();
    let db = NodeDatabase::open(root.join("node.db"))?;
    let mut manager = NetworkManager::new(db, owner, empty_member_keys(), no_public_network()).context("unwrap failed")?;

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

    std::fs::remove_dir_all(root).context("unwrap failed")?;
    Ok(())
}

#[test]
fn test_network_ticket_round_trip_deterministic() -> anyhow::Result<()> {
    let owner = SecretKey::generate().public();
    let member = SecretKey::generate().public();
    let root = std::env::temp_dir().join(format!("syncweb-net-ticket-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root)?;

    let db = NodeDatabase::open(root.join("node.db"))?;
    let mut manager = NetworkManager::new(db, owner, empty_member_keys(), no_public_network())?;
    let id = manager.create("roundtrip", NetworkOptions::default().with_label("RT"))?;
    let ticket = manager.invite(id, member)?;

    // Parse multiple times to ensure determinism.
    let encoded = ticket.to_string();
    let first: NetworkTicket = encoded.parse()?;
    let second: NetworkTicket = encoded.parse()?;
    anyhow::ensure!(first == second);
    anyhow::ensure!(first.name == "roundtrip");
    anyhow::ensure!(first.label == "RT");
    anyhow::ensure!(first.is_invite_only() == ticket.is_invite_only());

    std::fs::remove_dir_all(root)?;
    Ok(())
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
