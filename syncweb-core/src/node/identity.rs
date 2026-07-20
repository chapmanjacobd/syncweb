use hkdf::Hkdf;
use iroh::{PublicKey, SecretKey};
use iroh_docs::NamespaceId;
use sha2::Sha256;
use std::path::{Path, PathBuf};

use crate::error::{Result, SyncwebError};

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
            let bytes = std::fs::read(&path_buf).map_err(|error| SyncwebError::identity(&path_buf, error))?;
            let arr: [u8; 32] = bytes.try_into().map_err(|error: Vec<u8>| {
                SyncwebError::InvalidIdentity(format!("expected 32 bytes, got {} bytes", error.len()))
            })?;
            SecretKey::from_bytes(&arr)
        } else {
            let secret_key = SecretKey::generate();
            if let Some(parent) = path_buf.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent).map_err(|error| SyncwebError::identity(&path_buf, error))?;
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

    /// Derive a per-folder author secret key from the master key and namespace ID.
    ///
    /// Uses HKDF-SHA256 with the namespace ID as salt and the master secret key
    /// as input key material, producing a deterministic 32-byte key unique to each folder.
    ///
    /// # Errors
    ///
    /// Returns an error if HKDF expansion fails (should never happen with SHA-256 and 32-byte output).
    pub fn derive_folder_key(&self, namespace_id: NamespaceId) -> Result<SecretKey> {
        let hk = Hkdf::<Sha256>::new(Some(namespace_id.as_bytes()), &self.secret_key.to_bytes());
        let mut key_bytes = [0_u8; 32];
        hk.expand(b"syncweb-folder-author", &mut key_bytes)
            .map_err(|e| SyncwebError::KeyDerivation(e.to_string()))?;
        Ok(SecretKey::from_bytes(&key_bytes))
    }

    /// Derive a per-folder [`iroh_docs::Author`] from the master key.
    ///
    /// The author's signing key is deterministically derived from the master key
    /// and the folder's namespace ID via HKDF, ensuring each folder has a unique
    /// author identity without storing additional key material.
    ///
    /// # Errors
    ///
    /// Returns an error if key derivation fails.
    pub fn derive_folder_author(&self, namespace_id: NamespaceId) -> Result<iroh_docs::Author> {
        let folder_key = self.derive_folder_key(namespace_id)?;
        Ok(iroh_docs::Author::from_bytes(&folder_key.to_bytes()))
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
            .map_err(|error| SyncwebError::identity(path, error))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary_path, path).map_err(|error| SyncwebError::identity(path, error))
    })();

    if result.is_err()
        && let Err(error) = std::fs::remove_file(&temporary_path)
    {
        tracing::warn!(
            path = %temporary_path.display(),
            ?error,
            "failed to clean up temporary identity file"
        );
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
            return Err(SyncwebError::InvalidDeviceId(
                "expected 56 base32 characters".to_owned(),
            ));
        }

        let mut encoded = String::with_capacity(52);
        for chunk in compact.as_bytes().chunks_exact(14) {
            let (&checksum, payload) = chunk
                .split_last()
                .ok_or_else(|| SyncwebError::InvalidDeviceId("invalid chunk length".to_owned()))?;
            if luhn32(payload)? != checksum {
                return Err(SyncwebError::InvalidDeviceId("invalid checksum".to_owned()));
            }
            encoded.push_str(
                std::str::from_utf8(payload)
                    .map_err(|error| SyncwebError::InvalidDeviceId(format!("invalid base32 encoding: {error}")))?,
            );
        }

        let decoded = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &encoded)
            .ok_or_else(|| SyncwebError::InvalidDeviceId("invalid base32 encoding".to_owned()))?;
        let bytes: [u8; 32] = decoded.try_into().map_err(|error: Vec<u8>| {
            SyncwebError::InvalidDeviceId(format!("invalid public key length: {} bytes", error.len()))
        })?;
        let node_id =
            PublicKey::from_bytes(&bytes).map_err(|error| SyncwebError::InvalidDeviceId(error.to_string()))?;
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
            .ok_or_else(|| SyncwebError::InvalidDeviceId("invalid base32 character".to_owned()))?;

        let addend = factor
            .checked_mul(codepoint)
            .ok_or_else(|| SyncwebError::InvalidDeviceId("math overflow".to_owned()))?;
        let addend_div = addend
            .checked_div(alpha_len)
            .ok_or_else(|| SyncwebError::InvalidDeviceId("division by zero".to_owned()))?;
        let addend_rem = addend
            .checked_rem(alpha_len)
            .ok_or_else(|| SyncwebError::InvalidDeviceId("division by zero".to_owned()))?;
        let addend_sum = addend_div
            .checked_add(addend_rem)
            .ok_or_else(|| SyncwebError::InvalidDeviceId("math overflow".to_owned()))?;
        sum = sum
            .checked_add(addend_sum)
            .ok_or_else(|| SyncwebError::InvalidDeviceId("math overflow".to_owned()))?;

        factor = if factor == 2 { 1 } else { 2 };
    }
    let remainder = sum
        .checked_rem(alpha_len)
        .ok_or_else(|| SyncwebError::InvalidDeviceId("division by zero".to_owned()))?;
    let index_sub = alpha_len
        .checked_sub(remainder)
        .ok_or_else(|| SyncwebError::InvalidDeviceId("math underflow".to_owned()))?;
    let index = index_sub
        .checked_rem(alpha_len)
        .ok_or_else(|| SyncwebError::InvalidDeviceId("division by zero".to_owned()))?;

    SYNCTHING_ALPHABET
        .get(index)
        .copied()
        .ok_or_else(|| SyncwebError::InvalidDeviceId("index out of bounds".to_owned()))
}
