use ed25519_dalek::SigningKey;
use iroh_docs::{AuthorId, api::Doc};
use serde::{Deserialize, Serialize};

use crate::{
    Result, SyncwebError,
    constants::{MEMBER_LIST_SIGNATURE_CONTEXT, NETWORK_MEMBERS_KEY},
    node::{blob_store::BlobStore, docs_engine::DocsEngine},
    parsing::current_unix_secs,
};

use super::network::Network;

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

    /// Build the member list from a network's current membership.
    ///
    /// The owner is encoded as the hex of the node's ed25519 public key, which
    /// matches what [`SignedMemberList::sign`] and [`SignedMemberList::verify`]
    /// expect.
    #[must_use]
    pub fn from_network(network: &Network) -> Self {
        let owner = hex::encode(network.owner.as_bytes());
        let now = current_unix_secs();
        let mut members = network
            .members
            .iter()
            .map(|key| MemberEntry {
                key: key.to_string(),
                joined_at: now,
                role: if *key == network.owner {
                    MemberRole::Admin
                } else {
                    MemberRole::Member
                },
            })
            .collect::<Vec<_>>();
        members.sort_by(|left, right| left.key.cmp(&right.key));
        Self {
            network_id: network.id.to_string(),
            owner,
            sequence: 0,
            members,
            updated_at: now,
            signature: String::new(),
        }
    }

    /// Sign this member list with the given signing key.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        let owner_hex = crate::signature::public_key_hex(signing_key);
        if self.owner != owner_hex {
            return Err(SyncwebError::InvalidSignature(
                "signing key does not match owner field".to_owned(),
            ));
        }
        let message = self.serialize_unsigned()?;
        let mut signed_bytes = Vec::new();
        signed_bytes.extend_from_slice(MEMBER_LIST_SIGNATURE_CONTEXT);
        signed_bytes.extend_from_slice(&message);
        self.signature = crate::signature::sign_hex(signing_key, &signed_bytes);
        Ok(())
    }

    /// Verify the signature on this member list.
    ///
    /// # Errors
    ///
    /// Returns an error if the signature is missing, malformed, or invalid.
    pub fn verify(&self) -> Result<()> {
        let message = self.serialize_unsigned()?;
        let mut signed_bytes = Vec::new();
        signed_bytes.extend_from_slice(MEMBER_LIST_SIGNATURE_CONTEXT);
        signed_bytes.extend_from_slice(&message);
        crate::signature::verify_hex(&self.owner, &signed_bytes, &self.signature)
    }

    /// Serialize without the signature field.
    fn serialize_unsigned(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature = String::new();
        serde_json::to_vec(&unsigned).map_err(|error| SyncwebError::operation("failed to serialize member list", error))
    }
}

/// Network metadata stored in the membership doc.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NetworkInfo {
    pub name: String,
    pub label: String,
    pub created_at: u64,
}

/// Sign and publish the current member list for a network into its membership
/// doc under `sys/network/members`.
///
/// The writer is the network owner: `signing` must match `network.owner`.
/// Returns the next list sequence number used.
///
/// # Errors
///
/// Returns an error if the previous list cannot be read, the list cannot be
/// signed, or the entry cannot be written.
pub async fn write_member_list(
    docs: &DocsEngine,
    blobs: &BlobStore,
    doc: &Doc,
    author: AuthorId,
    signing: &SigningKey,
    network: &Network,
) -> Result<u64> {
    let sequence = next_member_list_sequence(docs, blobs, doc).await?;
    let mut list = SignedMemberList::from_network(network);
    list.sequence = sequence;
    list.sign(signing)?;
    let bytes =
        serde_json::to_vec(&list).map_err(|error| SyncwebError::operation("failed to serialize member list", error))?;
    docs.set(doc, author, NETWORK_MEMBERS_KEY, bytes).await?;
    Ok(sequence)
}

/// Return the sequence number to use for the next member list write.
async fn next_member_list_sequence(docs: &DocsEngine, blobs: &BlobStore, doc: &Doc) -> Result<u64> {
    let Some(entry) = docs.get_any(doc, NETWORK_MEMBERS_KEY).await? else {
        return Ok(1);
    };
    let bytes = blobs.get(entry.content_hash()).await?;
    let current: SignedMemberList = serde_json::from_slice(&bytes)
        .map_err(|error| SyncwebError::operation("failed to deserialize member list", error))?;
    Ok(current.sequence.saturating_add(1))
}
