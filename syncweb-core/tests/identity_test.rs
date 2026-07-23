mod test_utils;

use syncweb_core::node::identity::{DeviceId, IdentityManager};

use crate::test_utils::TestDirectory;

#[test]
fn test_generate_node_id() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-test")?;
    let identity = IdentityManager::new(directory.path().join("nested").join("identity.key"))?;
    let encoded = base32::encode(
        base32::Alphabet::Rfc4648 { padding: false },
        identity.node_id().as_bytes(),
    );
    anyhow::ensure!(encoded.len() == 52);
    Ok(())
}

#[test]
fn test_persist_secret_key() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-test")?;
    let path = directory.path().join("nested").join("identity.key");
    let identity = IdentityManager::new(&path)?;
    let content = std::fs::read_to_string(&path)?;
    let encoded = base32::encode(
        base32::Alphabet::Rfc4648 { padding: false },
        &identity.secret_key().to_bytes(),
    );
    anyhow::ensure!(content == encoded);
    Ok(())
}

#[test]
fn test_load_existing_identity() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-test")?;
    let path = directory.path().join("nested").join("identity.key");
    let original = IdentityManager::new(&path)?;
    let loaded = IdentityManager::new(&path)?;
    anyhow::ensure!(loaded.secret_key().to_bytes() == original.secret_key().to_bytes());
    Ok(())
}

#[test]
fn test_node_id_derivation() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-test")?;
    let identity = IdentityManager::new(directory.path().join("nested").join("identity.key"))?;
    anyhow::ensure!(identity.node_id() == identity.secret_key().public());
    Ok(())
}

#[test]
fn test_device_id_conversion() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-test")?;
    let identity = IdentityManager::new(directory.path().join("nested").join("identity.key"))?;
    let device_id = DeviceId::from_node_id(identity.node_id());
    let syncthing_id = device_id.to_syncthing();
    anyhow::ensure!(syncthing_id.len() == 63);
    anyhow::ensure!(syncthing_id.replace('-', "").len() == 56);
    anyhow::ensure!(DeviceId::from_syncthing(&syncthing_id)?.node_id() == identity.node_id());
    Ok(())
}

#[test]
fn test_persistent_identity_across_restarts() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-test")?;
    let path = directory.path().join("nested").join("identity.key");
    let first_node_id = IdentityManager::new(&path)?.node_id();
    let restarted_node_id = IdentityManager::new(&path)?.node_id();
    anyhow::ensure!(restarted_node_id == first_node_id);
    Ok(())
}

#[test]
fn test_rejects_invalid_device_id_checksum() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-test")?;
    let identity = IdentityManager::new(directory.path().join("nested").join("identity.key"))?;
    let mut syncthing_id = DeviceId::from_node_id(identity.node_id()).to_syncthing();
    syncthing_id.replace_range(..1, if syncthing_id.starts_with('A') { "B" } else { "A" });
    anyhow::ensure!(DeviceId::from_syncthing(&syncthing_id).is_err());
    Ok(())
}
