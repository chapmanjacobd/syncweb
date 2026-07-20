use anyhow::{Context, Result};
use iroh::{PublicKey, SecretKey};
use std::path::{Path, PathBuf};

const SYNCTHING_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

#[derive(Clone)]
pub struct IdentityManager {
    secret_key: SecretKey,
    path: PathBuf,
}

impl IdentityManager {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let secret_key = if path.exists() {
            let bytes = std::fs::read(&path)
                .with_context(|| format!("failed to read identity from {}", path.display()))?;
            let arr: [u8; 32] = bytes
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid secret key file: expected 32 bytes"))?;
            SecretKey::from_bytes(&arr)
        } else {
            let secret_key = SecretKey::generate();
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create identity directory {}", parent.display())
                })?;
            }
            write_secret_key(&path, secret_key.to_bytes())?;
            secret_key
        };

        Ok(Self { secret_key, path })
    }

    pub fn node_id(&self) -> PublicKey {
        self.secret_key.public()
    }

    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn write_secret_key(path: &Path, bytes: [u8; 32]) -> Result<()> {
    use std::io::Write;

    let temporary_path = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let result = (|| -> Result<()> {
        let mut file = options.open(&temporary_path).with_context(|| {
            format!(
                "failed to create temporary identity {}",
                temporary_path.display()
            )
        })?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary_path, path)
            .with_context(|| format!("failed to persist identity to {}", path.display()))
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&temporary_path);
    }
    result
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeviceId(PublicKey);

impl DeviceId {
    pub fn from_node_id(node_id: PublicKey) -> Self {
        Self(node_id)
    }

    pub fn from_syncthing(value: &str) -> Result<Self> {
        let compact: String = value
            .chars()
            .filter(|character| *character != '-')
            .map(|character| character.to_ascii_uppercase())
            .collect();
        if compact.len() != 56 || !compact.is_ascii() {
            anyhow::bail!("invalid Syncthing device ID: expected 56 base32 characters");
        }

        let mut encoded = String::with_capacity(52);
        for chunk in compact.as_bytes().chunks_exact(14) {
            let payload = &chunk[..13];
            if luhn32(payload)? != chunk[13] {
                anyhow::bail!("invalid Syncthing device ID checksum");
            }
            encoded.push_str(std::str::from_utf8(payload)?);
        }

        let decoded = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &encoded)
            .ok_or_else(|| anyhow::anyhow!("invalid Syncthing device ID base32 encoding"))?;
        let bytes: [u8; 32] = decoded
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid Syncthing device ID key length"))?;
        let node_id = PublicKey::from_bytes(&bytes)
            .context("invalid Ed25519 public key in Syncthing device ID")?;
        Ok(Self(node_id))
    }

    pub fn to_syncthing(&self) -> String {
        let encoded = base32::encode(
            base32::Alphabet::Rfc4648 { padding: false },
            self.0.as_bytes(),
        );
        let mut checked = String::with_capacity(56);
        for chunk in encoded.as_bytes().chunks(13) {
            checked.push_str(std::str::from_utf8(chunk).expect("base32 output is ASCII"));
            checked.push(char::from(
                luhn32(chunk).expect("base32 output uses the Syncthing alphabet"),
            ));
        }

        checked
            .as_bytes()
            .chunks(7)
            .map(|chunk| std::str::from_utf8(chunk).expect("device ID is ASCII"))
            .collect::<Vec<_>>()
            .join("-")
    }

    pub fn node_id(&self) -> PublicKey {
        self.0
    }
}

fn luhn32(input: &[u8]) -> Result<u8> {
    let mut factor = 2;
    let mut sum = 0;
    for character in input.iter().rev() {
        let codepoint = SYNCTHING_ALPHABET
            .iter()
            .position(|candidate| candidate == character)
            .ok_or_else(|| anyhow::anyhow!("invalid Syncthing device ID base32 character"))?;
        let addend = factor * codepoint;
        sum += addend / SYNCTHING_ALPHABET.len() + addend % SYNCTHING_ALPHABET.len();
        factor = if factor == 2 { 1 } else { 2 };
    }
    let remainder = sum % SYNCTHING_ALPHABET.len();
    Ok(SYNCTHING_ALPHABET[(SYNCTHING_ALPHABET.len() - remainder) % SYNCTHING_ALPHABET.len()])
}
