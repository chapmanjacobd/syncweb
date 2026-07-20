use std::path::PathBuf;

use syncweb_core::node::identity::{DeviceId, IdentityManager};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Result<Self, std::io::Error> {
        let path = std::env::temp_dir().join(format!("syncweb-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }

    fn identity_path(&self) -> PathBuf {
        self.0.join("nested").join("identity.key")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn test_generate_node_id() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let identity = IdentityManager::new(directory.identity_path())?;
    let encoded = base32::encode(
        base32::Alphabet::Rfc4648 { padding: false },
        identity.node_id().as_bytes(),
    );
    assert_eq!(encoded.len(), 52);
    Ok(())
}

#[test]
fn test_persist_secret_key() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let path = directory.identity_path();
    let identity = IdentityManager::new(&path)?;
    assert_eq!(std::fs::read(path)?, identity.secret_key().to_bytes());
    Ok(())
}

#[test]
fn test_load_existing_identity() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let path = directory.identity_path();
    let original = IdentityManager::new(&path)?;
    let loaded = IdentityManager::new(&path)?;
    assert_eq!(loaded.secret_key().to_bytes(), original.secret_key().to_bytes());
    Ok(())
}

#[test]
fn test_node_id_derivation() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let identity = IdentityManager::new(directory.identity_path())?;
    assert_eq!(identity.node_id(), identity.secret_key().public());
    Ok(())
}

#[test]
fn test_device_id_conversion() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let identity = IdentityManager::new(directory.identity_path())?;
    let device_id = DeviceId::from_node_id(identity.node_id());
    let syncthing_id = device_id.to_syncthing();
    assert_eq!(syncthing_id.len(), 63);
    assert_eq!(syncthing_id.replace('-', "").len(), 56);
    assert_eq!(DeviceId::from_syncthing(&syncthing_id)?.node_id(), identity.node_id());
    Ok(())
}

#[test]
fn test_persistent_identity_across_restarts() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let path = directory.identity_path();
    let first_node_id = IdentityManager::new(&path)?.node_id();
    let restarted_node_id = IdentityManager::new(&path)?.node_id();
    assert_eq!(restarted_node_id, first_node_id);
    Ok(())
}

#[test]
fn test_rejects_invalid_device_id_checksum() -> anyhow::Result<()> {
    let directory = TestDirectory::new()?;
    let identity = IdentityManager::new(directory.identity_path())?;
    let mut syncthing_id = DeviceId::from_node_id(identity.node_id()).to_syncthing();
    syncthing_id.replace_range(..1, if syncthing_id.starts_with('A') { "B" } else { "A" });
    assert!(DeviceId::from_syncthing(&syncthing_id).is_err());
    Ok(())
}
