use iroh::SecretKey;
use std::path::{Path, PathBuf};
use anyhow::Result;

#[derive(Clone)]
pub struct IdentityManager {
    secret_key: SecretKey,
    path: PathBuf,
}

impl IdentityManager {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if path.exists() {
            let bytes = std::fs::read(&path)?;
            let arr: [u8; 32] = bytes.try_into().map_err(|_| {
                anyhow::anyhow!("invalid secret key file: expected 32 bytes")
            })?;
            let secret_key = SecretKey::from_bytes(&arr);
            Ok(Self { secret_key, path })
        } else {
            let secret_key = SecretKey::generate();
            let bytes = secret_key.to_bytes();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, bytes)?;
            Ok(Self { secret_key, path })
        }
    }

    pub fn node_id(&self) -> iroh::PublicKey {
        self.secret_key.public()
    }

    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
