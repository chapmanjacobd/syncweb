use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use iroh_docs::NamespaceId;
use serde::{Deserialize, Serialize};

use crate::{
    Result, SyncwebError,
    constants::{MEMBER_LIST_SIGNATURE_CONTEXT, NETWORK_DOC_NAMESPACE_CONTEXT},
};

/// The canonical, owner-signed list of network members.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SignedMemberList {
    pub network_id: String,
    pub owner: String,
    pub sequence: u64,
    pub members: Vec<MemberEntry>,
    pub updated_at: u64,
    pub signature: String,
}

/// A single member entry in the member list.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MemberEntry {
    pub key: String,
    pub joined_at: u64,
    pub role: MemberRole,
}

/// Role of a member in the network.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MemberRole {
    Admin,
    Member,
}

impl SignedMemberList {
    /// Create a new empty member list for the given network and owner.
    #[must_use]
    pub fn new(network_id: &str, owner: &str) -> Self {
        Self {
            network_id: network_id.to_owned(),
            owner: owner.to_owned(),
            sequence: 0,
            members: Vec::new(),
            updated_at: current_unix_secs(),
            signature: String::new(),
        }
    }

    /// Sign this member list with the given signing key.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        let owner_hex = hex::encode(signing_key.verifying_key().to_bytes());
        if self.owner != owner_hex {
            return Err(SyncwebError::InvalidSignature(
                "signing key does not match owner field".to_owned(),
            ));
        }
        let message = self.serialize_unsigned()?;
        let mut signed_bytes = Vec::new();
        signed_bytes.extend_from_slice(MEMBER_LIST_SIGNATURE_CONTEXT);
        signed_bytes.extend_from_slice(&message);
        self.signature = hex::encode(signing_key.sign(&signed_bytes).to_bytes());
        Ok(())
    }

    /// Verify the signature on this member list.
    ///
    /// # Errors
    ///
    /// Returns an error if the signature is missing, malformed, or invalid.
    pub fn verify(&self) -> Result<()> {
        let key_bytes = hex::decode(&self.owner)
            .map_err(|error| SyncwebError::InvalidSignature(format!("invalid owner key hex: {error}")))?;
        let key_array: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_length_error| SyncwebError::InvalidSignature("owner key must be 32 bytes".to_owned()))?;
        let verifying_key = VerifyingKey::from_bytes(&key_array)
            .map_err(|error| SyncwebError::InvalidSignature(format!("invalid owner key: {error}")))?;
        let sig_bytes = hex::decode(&self.signature)
            .map_err(|error| SyncwebError::InvalidSignature(format!("invalid signature hex: {error}")))?;
        let sig_array: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_length_error| SyncwebError::InvalidSignature("signature must be 64 bytes".to_owned()))?;
        let signature = Signature::from_bytes(&sig_array);
        let message = self.serialize_unsigned()?;
        let mut signed_bytes = Vec::new();
        signed_bytes.extend_from_slice(MEMBER_LIST_SIGNATURE_CONTEXT);
        signed_bytes.extend_from_slice(&message);
        verifying_key
            .verify(&signed_bytes, &signature)
            .map_err(|error| SyncwebError::InvalidSignature(format!("signature verification failed: {error}")))
    }

    /// Serialize without the signature field.
    fn serialize_unsigned(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature = String::new();
        serde_json::to_vec(&unsigned).map_err(|error| SyncwebError::operation("failed to serialize member list", error))
    }
}

/// Derive the deterministic Iroh docs namespace for a network.
///
/// Used by the owner at network creation time. New members receive the
/// namespace via the `doc_ticket` in the `NetworkTicket`.
#[must_use]
pub fn network_doc_namespace(network_id: &[u8; 32], shared_secret: &[u8; 32]) -> NamespaceId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(NETWORK_DOC_NAMESPACE_CONTEXT);
    hasher.update(network_id);
    hasher.update(shared_secret);
    NamespaceId::from(hasher.finalize().as_bytes())
}

/// Network metadata stored in the membership doc.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NetworkInfo {
    pub name: String,
    pub label: String,
    pub created_at: u64,
}

fn current_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
