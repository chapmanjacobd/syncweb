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
    /// # Errors
    ///
    /// Returns an error if the identity file cannot be read, parsed, or if a new one cannot be written.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path_buf = path.into();
        let secret_key = if path_buf.exists() {
            let bytes = std::fs::read(&path_buf)
                .with_context(|| format!("failed to read identity from {}", path_buf.display()))?;
            let arr: [u8; 32] = bytes
                .try_into()
                .map_err(|_err| anyhow::anyhow!("invalid secret key file: expected 32 bytes"))?;
            SecretKey::from_bytes(&arr)
        } else {
            let secret_key = SecretKey::generate();
            if let Some(parent) = path_buf.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create identity directory {}", parent.display()))?;
            }
            write_secret_key(&path_buf, secret_key.to_bytes())?;
            secret_key
        };

        Ok(Self {
            secret_key,
            path: path_buf,
        })
    }

    #[must_use]
    pub fn node_id(&self) -> PublicKey {
        self.secret_key.public()
    }

    #[must_use]
    pub const fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    #[must_use]
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
        let mut file = options
            .open(&temporary_path)
            .with_context(|| format!("failed to create temporary identity {}", temporary_path.display()))?;
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
    #[must_use]
    pub const fn from_node_id(node_id: PublicKey) -> Self {
        Self(node_id)
    }

    /// # Errors
    ///
    /// Returns an error if the provided value is not a valid Syncthing device ID.
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
            let (&checksum, payload) = chunk.split_last().context("invalid chunk length")?;
            if luhn32(payload)? != checksum {
                anyhow::bail!("invalid Syncthing device ID checksum");
            }
            encoded.push_str(std::str::from_utf8(payload)?);
        }

        let decoded = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &encoded)
            .ok_or_else(|| anyhow::anyhow!("invalid Syncthing device ID base32 encoding"))?;
        let bytes: [u8; 32] = decoded
            .try_into()
            .map_err(|_err| anyhow::anyhow!("invalid Syncthing device ID key length"))?;
        let node_id = PublicKey::from_bytes(&bytes).context("invalid Ed25519 public key in Syncthing device ID")?;
        Ok(Self(node_id))
    }

    #[must_use]
    pub fn to_syncthing(&self) -> String {
        let encoded = base32::encode(base32::Alphabet::Rfc4648 { padding: false }, self.0.as_bytes());
        let mut checked = String::with_capacity(56);
        for chunk in encoded.as_bytes().chunks(13) {
            checked.push_str(std::str::from_utf8(chunk).unwrap_or(""));
            checked.push(char::from(luhn32(chunk).unwrap_or(0)));
        }

        checked
            .as_bytes()
            .chunks(7)
            .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
            .collect::<Vec<_>>()
            .join("-")
    }

    #[must_use]
    pub const fn node_id(&self) -> PublicKey {
        self.0
    }
}

fn luhn32(input: &[u8]) -> Result<u8> {
    let mut factor = 2_usize;
    let mut sum = 0_usize;
    let alpha_len = SYNCTHING_ALPHABET.len();
    for character in input.iter().rev() {
        let codepoint = SYNCTHING_ALPHABET
            .iter()
            .position(|candidate| candidate == character)
            .ok_or_else(|| anyhow::anyhow!("invalid Syncthing device ID base32 character"))?;

        let addend = factor
            .checked_mul(codepoint)
            .ok_or_else(|| anyhow::anyhow!("math overflow"))?;
        let addend_div = addend
            .checked_div(alpha_len)
            .ok_or_else(|| anyhow::anyhow!("div by zero"))?;
        let addend_rem = addend
            .checked_rem(alpha_len)
            .ok_or_else(|| anyhow::anyhow!("div by zero"))?;
        let addend_sum = addend_div
            .checked_add(addend_rem)
            .ok_or_else(|| anyhow::anyhow!("math overflow"))?;
        sum = sum
            .checked_add(addend_sum)
            .ok_or_else(|| anyhow::anyhow!("math overflow"))?;

        factor = if factor == 2 { 1 } else { 2 };
    }
    let remainder = sum
        .checked_rem(alpha_len)
        .ok_or_else(|| anyhow::anyhow!("div by zero"))?;
    let index_sub = alpha_len
        .checked_sub(remainder)
        .ok_or_else(|| anyhow::anyhow!("math underflow"))?;
    let index = index_sub
        .checked_rem(alpha_len)
        .ok_or_else(|| anyhow::anyhow!("div by zero"))?;

    SYNCTHING_ALPHABET
        .get(index)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("index out of bounds"))
}
