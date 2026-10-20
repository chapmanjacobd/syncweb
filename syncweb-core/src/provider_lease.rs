//! Signed, expiring provider availability leases.
//!
//! Leases are advisory control-plane records. They identify the community and
//! folder in which a provider is valid, but callers must still fetch and
//! verify the referenced content before treating it as available.

use std::collections::HashSet;

use ed25519_dalek::SigningKey;
use iroh::PublicKey;
use iroh_blobs::Hash;
use iroh_docs::NamespaceId;
use serde::{Deserialize, Serialize};

use crate::{
    Result, SyncwebError,
    signature::{public_key_hex, sign_hex, verify_hex},
};

const PROVIDER_LEASE_CONTEXT: &[u8] = b"syncweb/provider-lease/v1\0";
pub const PROVIDER_LEASE_VERSION: u8 = 1;

/// A content identifier advertised by a provider.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ProviderContent {
    Blob { hash: String },
    Segment { hash: String, offset: u64, length: u64 },
}

impl ProviderContent {
    #[must_use]
    pub fn blob(hash: Hash) -> Self {
        Self::Blob { hash: hash.to_string() }
    }

    #[must_use]
    pub fn segment(hash: Hash, offset: u64, length: u64) -> Self {
        Self::Segment {
            hash: hash.to_string(),
            offset,
            length,
        }
    }
}

/// Capabilities that affect how a provider may be selected.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProviderCapabilities {
    pub supports_segments: bool,
    pub supports_relay: bool,
}

impl ProviderCapabilities {
    /// Construct a capability summary for discovery and provider leases.
    #[must_use]
    pub const fn new(supports_segments: bool, supports_relay: bool) -> Self {
        Self {
            supports_segments,
            supports_relay,
        }
    }
}

/// Unsigned fields used to construct a provider lease.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProviderLeaseOptions {
    pub community: Option<String>,
    pub folder: NamespaceId,
    pub content_root: Option<Hash>,
    pub contents: Vec<ProviderContent>,
    pub capabilities: ProviderCapabilities,
    pub observed_at: u64,
    pub expires_at: u64,
}

impl ProviderLeaseOptions {
    /// Create lease fields before signing them with [`ProviderLease::sign`].
    #[must_use]
    pub const fn new(
        community: Option<String>,
        folder: NamespaceId,
        content_root: Option<Hash>,
        contents: Vec<ProviderContent>,
        capabilities: ProviderCapabilities,
        observed_at: u64,
        expires_at: u64,
    ) -> Self {
        Self {
            community,
            folder,
            content_root,
            contents,
            capabilities,
            observed_at,
            expires_at,
        }
    }
}

/// A signed, bounded advertisement of provider availability.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProviderLease {
    pub version: u8,
    pub community: Option<String>,
    pub folder: String,
    pub provider: String,
    pub content_root: Option<String>,
    pub contents: Vec<ProviderContent>,
    pub capabilities: ProviderCapabilities,
    pub observed_at: u64,
    pub expires_at: u64,
    pub signature: String,
}

impl ProviderLease {
    /// Construct an unsigned lease for a provider and folder scope.
    #[must_use]
    pub fn new(provider: PublicKey, options: ProviderLeaseOptions) -> Self {
        Self {
            version: PROVIDER_LEASE_VERSION,
            community: options.community,
            folder: options.folder.to_string(),
            provider: hex::encode(provider.as_bytes()),
            content_root: options.content_root.map(|hash| hash.to_string()),
            contents: options.contents,
            capabilities: options.capabilities,
            observed_at: options.observed_at,
            expires_at: options.expires_at,
            signature: String::new(),
        }
    }

    /// Sign the lease with the provider's identity key.
    ///
    /// # Errors
    ///
    /// Returns an error when the signing key does not match the advertised
    /// provider or the lease cannot be serialized.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        if self.provider != public_key_hex(signing_key) {
            return Err(SyncwebError::InvalidSignature(
                "signing key does not match provider field".to_owned(),
            ));
        }
        let message = self.unsigned_bytes()?;
        let mut signed = Vec::new();
        signed.extend_from_slice(PROVIDER_LEASE_CONTEXT);
        signed.extend_from_slice(&message);
        self.signature = sign_hex(signing_key, &signed);
        Ok(())
    }

    /// Verify the provider signature without applying local scope or expiry
    /// policy.
    ///
    /// # Errors
    ///
    /// Returns an error when the signature is absent, malformed, or invalid.
    pub fn verify(&self) -> Result<()> {
        if self.signature.is_empty() {
            return Err(SyncwebError::MissingSignature("provider lease".to_owned()));
        }
        let message = self.unsigned_bytes()?;
        let mut signed = Vec::new();
        signed.extend_from_slice(PROVIDER_LEASE_CONTEXT);
        signed.extend_from_slice(&message);
        verify_hex(&self.provider, &signed, &self.signature)
    }

    /// Decode the provider identity after validating its wire representation.
    ///
    /// # Errors
    ///
    /// Returns an error when the provider field is not a 32-byte public key.
    pub fn provider_id(&self) -> Result<PublicKey> {
        let encoded = hex::decode(&self.provider)
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid provider node ID: {error}")))?;
        let key_bytes: [u8; 32] = encoded.try_into().map_err(|bytes: Vec<u8>| {
            SyncwebError::InvalidIdentity(format!("provider node ID must contain 32 bytes, got {}", bytes.len()))
        })?;
        PublicKey::from_bytes(&key_bytes)
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid provider node ID: {error}")))
    }

    /// Decode the unique blob identifiers advertised by this lease.
    ///
    /// The optional content root is included because it is itself a
    /// content-addressed availability observation. Segment entries contribute
    /// their containing blob hash.
    ///
    /// # Errors
    ///
    /// Returns an error when an advertised hash is malformed.
    pub fn content_hashes(&self) -> Result<Vec<Hash>> {
        let mut seen = HashSet::new();
        let mut hashes = Vec::new();
        if let Some(root) = &self.content_root {
            let hash = parse_hash(root, "content root")?;
            if seen.insert(hash) {
                hashes.push(hash);
            }
        }
        for content in &self.contents {
            let value = match content {
                ProviderContent::Blob { hash } | ProviderContent::Segment { hash, .. } => hash,
            };
            let hash = parse_hash(value, "provider content")?;
            if seen.insert(hash) {
                hashes.push(hash);
            }
        }
        Ok(hashes)
    }

    /// Verify authenticity, structure, freshness, and local scope.
    ///
    /// `community` and `folder` are the local scope. Exact matching is
    /// intentional: an unscoped daemon does not accept a named-community lease,
    /// and a named community does not accept an unscoped lease.
    ///
    /// # Errors
    ///
    /// Returns an error when the lease is malformed, expired, outside the
    /// local scope, or has an invalid signature.
    pub fn validate(&self, now: u64, community: Option<&str>, folder: &str) -> Result<()> {
        if self.version != PROVIDER_LEASE_VERSION {
            return Err(SyncwebError::InvalidConfig(format!(
                "unsupported provider lease version {}",
                self.version
            )));
        }
        if self.folder != folder {
            return Err(SyncwebError::AccessDenied(
                "provider lease is for a different folder".to_owned(),
            ));
        }
        if self.community.as_deref() != community {
            return Err(SyncwebError::AccessDenied(
                "provider lease is outside the local community".to_owned(),
            ));
        }
        if self.observed_at >= self.expires_at {
            return Err(SyncwebError::InvalidConfig(
                "provider lease expiry must be after observation".to_owned(),
            ));
        }
        if self.observed_at > now {
            return Err(SyncwebError::InvalidConfig(
                "provider lease observation is in the future".to_owned(),
            ));
        }
        if now >= self.expires_at {
            return Err(SyncwebError::InvalidConfig("provider lease has expired".to_owned()));
        }
        self.provider_id()?;
        if self.folder.is_empty() {
            return Err(SyncwebError::InvalidConfig(
                "provider lease folder cannot be empty".to_owned(),
            ));
        }
        if let Some(root) = &self.content_root {
            validate_hash(root, "content root")?;
        }
        for content in &self.contents {
            match content {
                ProviderContent::Blob { hash } => validate_hash(hash, "blob")?,
                ProviderContent::Segment { hash, offset, length } => {
                    validate_hash(hash, "segment blob")?;
                    if *length == 0 {
                        return Err(SyncwebError::InvalidConfig(
                            "provider segment length must be non-zero".to_owned(),
                        ));
                    }
                    offset.checked_add(*length).ok_or_else(|| {
                        SyncwebError::InvalidConfig("provider segment range overflows u64".to_owned())
                    })?;
                }
            }
        }
        if self.content_root.is_none() && self.contents.is_empty() {
            return Err(SyncwebError::InvalidConfig(
                "provider lease must advertise a content root or content identifiers".to_owned(),
            ));
        }
        self.verify()
    }

    fn unsigned_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature.clear();
        serde_json::to_vec(&unsigned)
            .map_err(|error| SyncwebError::operation("failed to serialize provider lease", error))
    }
}

fn validate_hash(value: &str, kind: &str) -> Result<()> {
    let bytes =
        hex::decode(value).map_err(|error| SyncwebError::InvalidConfig(format!("invalid {kind} hash: {error}")))?;
    if bytes.len() != 32 {
        return Err(SyncwebError::InvalidConfig(format!(
            "{kind} hash must contain 32 bytes, got {}",
            bytes.len()
        )));
    }
    Ok(())
}

fn parse_hash(value: &str, kind: &str) -> Result<Hash> {
    let encoded =
        hex::decode(value).map_err(|error| SyncwebError::InvalidConfig(format!("invalid {kind} hash: {error}")))?;
    let hash_bytes: [u8; 32] = encoded.try_into().map_err(|bytes: Vec<u8>| {
        SyncwebError::InvalidConfig(format!("{kind} hash must contain 32 bytes, got {}", bytes.len()))
    })?;
    Ok(Hash::from_bytes(hash_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn folder() -> NamespaceId {
        NamespaceId::from(&[7_u8; 32])
    }

    fn hash(seed: u8) -> Hash {
        Hash::from_bytes([seed; 32])
    }

    fn signed_lease() -> ProviderLease {
        let signing_key = key(1);
        let mut lease = ProviderLease::new(
            PublicKey::from_bytes(&signing_key.verifying_key().to_bytes()).expect("public key"),
            ProviderLeaseOptions {
                community: Some("community-a".to_owned()),
                folder: folder(),
                content_root: None,
                contents: vec![ProviderContent::blob(hash(2))],
                capabilities: ProviderCapabilities {
                    supports_segments: false,
                    supports_relay: true,
                },
                observed_at: 100,
                expires_at: 200,
            },
        );
        lease.sign(&signing_key).expect("sign lease");
        lease
    }

    #[test]
    fn signs_and_validates_with_matching_scope() {
        let lease = signed_lease();
        assert!(lease.validate(150, Some("community-a"), &folder().to_string()).is_ok());
        assert_eq!(lease.version, PROVIDER_LEASE_VERSION);
    }

    #[test]
    fn rejects_tampering_and_scope_mismatch() {
        let mut lease = signed_lease();
        lease.contents = vec![ProviderContent::blob(hash(3))];
        assert!(lease.verify().is_err());

        let lease = signed_lease();
        assert!(lease.validate(150, Some("community-b"), &folder().to_string()).is_err());
        assert!(
            lease
                .validate(150, Some("community-a"), &NamespaceId::from(&[8_u8; 32]).to_string())
                .is_err()
        );
    }

    #[test]
    fn rejects_expired_and_malformed_content() {
        let mut lease = signed_lease();
        assert!(lease.validate(200, Some("community-a"), &folder().to_string()).is_err());

        lease.contents = vec![ProviderContent::Segment {
            hash: "00".repeat(32),
            offset: u64::MAX,
            length: 1,
        }];
        lease.sign(&key(1)).expect("resign lease");
        assert!(lease.validate(150, Some("community-a"), &folder().to_string()).is_err());

        lease.contents = Vec::new();
        lease.content_root = None;
        lease.sign(&key(1)).expect("resign lease");
        assert!(lease.validate(150, Some("community-a"), &folder().to_string()).is_err());
    }

    #[test]
    fn rejects_future_dated_observation() {
        let mut lease = signed_lease();
        lease.observed_at = 151;
        lease.sign(&key(1)).expect("resign lease");
        assert!(lease.validate(150, Some("community-a"), &folder().to_string()).is_err());
    }

    #[test]
    fn rejects_wrong_signing_key() {
        let mut lease = signed_lease();
        assert!(lease.sign(&key(2)).is_err());
    }
}
