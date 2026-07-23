use std::path::{Path, PathBuf};

use syncweb_core::node::{
    identity::IdentityManager,
    iroh_node::{IrohNode, RelayMode},
};

pub struct TestDirectory(PathBuf);

impl TestDirectory {
    /// Create a new test directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created.
    pub fn new(name: &str) -> anyhow::Result<Self> {
        let dir = std::env::temp_dir().join(format!("{}-{}", name, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            eprintln!("failed to remove test directory {}: {error}", self.0.display());
        }
    }
}

/// Create a test Iroh node within the given directory.
///
/// # Errors
///
/// Returns an error if the identity cannot be loaded or the node cannot connect.
pub async fn test_node(directory: &TestDirectory, name: &str) -> anyhow::Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    Ok(IrohNode::new(identity, root.join("data"), RelayMode::Default).await?)
}
