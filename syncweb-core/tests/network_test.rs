use iroh::SecretKey;
use syncweb_core::net::{NetworkManager, NetworkOptions, NetworkTicket};

#[test]
fn network_lifecycle_persists_and_tickets_round_trip() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-phase4-network-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root)?;
    let owner = SecretKey::generate().public();
    let member = SecretKey::generate().public();
    let path = root.join("networks.json");
    let mut owner_manager = NetworkManager::new(&path, owner)?;
    let id = owner_manager.create("work", NetworkOptions::default().with_label("Work").invite_only(true))?;
    let ticket = owner_manager.invite(id, member)?;
    let encoded = ticket.to_string();
    let decoded: NetworkTicket = encoded.parse()?;
    anyhow::ensure!(decoded == ticket);

    let member_path = root.join("member-networks.json");
    let mut member_manager = NetworkManager::new(&member_path, member)?;
    anyhow::ensure!(member_manager.join(decoded)? == id);
    anyhow::ensure!(
        member_manager
            .get(&id)
            .is_some_and(|network| network.is_member(&member))
    );

    owner_manager.kick(id, &member)?;
    anyhow::ensure!(!owner_manager.get(&id).is_some_and(|network| network.is_member(&member)));
    drop(owner_manager);
    let reloaded = NetworkManager::new(&path, owner)?;
    anyhow::ensure!(reloaded.list().len() == 1);

    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_network_create_rejects_empty_name() {
    let root = std::env::temp_dir().join(format!("syncweb-phase4-net-empty-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let owner = SecretKey::generate().public();
    let path = root.join("networks.json");
    let mut manager = NetworkManager::new(&path, owner).unwrap();

    let result = manager.create("", NetworkOptions::default());
    assert!(result.is_err());

    let result_spaces = manager.create("  ", NetworkOptions::default());
    assert!(result_spaces.is_err());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_network_invite_rejects_non_owner() {
    let root = std::env::temp_dir().join(format!("syncweb-phase4-net-owner-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let owner = SecretKey::generate().public();
    let other = SecretKey::generate().public();
    let path = root.join("networks.json");
    let mut manager_owner = NetworkManager::new(&path, owner).unwrap();
    let mut manager_other = NetworkManager::new(&path, other).unwrap();

    let id = manager_owner.create("test", NetworkOptions::default()).unwrap();

    // Other node cannot invite.
    let result = manager_other.invite(id, SecretKey::generate().public());
    assert!(result.is_err());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_network_kick_owner_rejected() {
    let root = std::env::temp_dir().join(format!("syncweb-phase4-net-kick-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let owner = SecretKey::generate().public();
    let path = root.join("networks.json");
    let mut manager = NetworkManager::new(&path, owner).unwrap();

    let id = manager.create("test", NetworkOptions::default()).unwrap();
    let result = manager.kick(id, &owner);
    assert!(result.is_err());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_network_leave_removes_network() {
    let root = std::env::temp_dir().join(format!("syncweb-phase4-net-leave-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let owner = SecretKey::generate().public();
    let path = root.join("networks.json");
    let mut manager = NetworkManager::new(&path, owner).unwrap();

    let id = manager.create("test", NetworkOptions::default()).unwrap();
    assert_eq!(manager.list().len(), 1);

    manager.leave(id).unwrap();
    assert!(manager.list().is_empty());
    assert!(manager.get(&id).is_none());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_network_folder_membership() {
    let root = std::env::temp_dir().join(format!("syncweb-phase4-net-folder-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let owner = SecretKey::generate().public();
    let path = root.join("networks.json");
    let mut manager = NetworkManager::new(&path, owner).unwrap();

    let id = manager.create("test", NetworkOptions::default()).unwrap();
    let folder = iroh_docs::NamespaceId::default();

    manager.add_folder(id, folder).unwrap();
    let network = manager.get(&id).unwrap();
    assert!(network.folders.contains(&folder));

    manager.remove_folder(id, folder).unwrap();
    let network_after = manager.get(&id).unwrap();
    assert!(!network_after.folders.contains(&folder));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_network_ticket_round_trip_deterministic() -> anyhow::Result<()> {
    let owner = SecretKey::generate().public();
    let member = SecretKey::generate().public();
    let root = std::env::temp_dir().join(format!("syncweb-phase4-net-ticket-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root)?;

    let path = root.join("networks.json");
    let mut manager = NetworkManager::new(&path, owner)?;
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
fn test_network_id_from_name_is_stable() {
    let a = syncweb_core::net::NetworkId::from_name("hello");
    let b = syncweb_core::net::NetworkId::from_name("hello");
    let c = syncweb_core::net::NetworkId::from_name("world");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_network_id_hex_round_trip() {
    let id = syncweb_core::net::NetworkId::from_name("test");
    let hex = id.to_string();
    let parsed: syncweb_core::net::NetworkId = hex.parse().unwrap();
    assert_eq!(id, parsed);
}
