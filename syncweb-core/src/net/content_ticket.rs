//! Signed, expiring content tickets scoped to an ownerless community.
//!
//! The underlying Iroh blob ticket remains responsible for endpoint and blob
//! addressing. This envelope adds the community admission boundary without
//! requiring a central tracker.

use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use ed25519_dalek::SigningKey;
use iroh::PublicKey;
use iroh_blobs::{Hash, ticket::BlobTicket};
use serde::{Deserialize, Serialize};

use crate::{
    Result, SyncwebError,
    signature::{public_key_hex, sign_hex, verify_hex},
};

use super::community::CommunityId;

const COMMUNITY_CONTENT_TICKET_CONTEXT: &[u8] = b"syncweb/community-content-ticket/v1\0";
const COMMUNITY_CONTENT_TICKET_URI_PREFIX: &str = "syncweb://content/";
const MAX_CONTENT_TICKET_BYTES: usize = 64 * 1024;
pub const COMMUNITY_CONTENT_TICKET_VERSION: u8 = 1;

/// A signed Iroh blob ticket that is valid only inside one community scope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CommunityContentTicket {
    pub version: u8,
    pub community_id: String,
    pub issuer: String,
    pub ticket: String,
    pub issued_at: u64,
    pub expires_at: Option<u64>,
    pub signature: String,
}

impl CommunityContentTicket {
    /// Construct an unsigned community-scoped content ticket.
    #[must_use]
    pub fn new(
        community: CommunityId,
        issuer: PublicKey,
        ticket: &BlobTicket,
        issued_at: u64,
        expires_at: Option<u64>,
    ) -> Self {
        Self {
            version: COMMUNITY_CONTENT_TICKET_VERSION,
            community_id: community.to_string(),
            issuer: issuer.to_string(),
            ticket: ticket.to_string(),
            issued_at,
            expires_at,
            signature: String::new(),
        }
    }

    /// Sign the ticket envelope with a trusted community issuer key.
    ///
    /// # Errors
    ///
    /// Returns an error when the key does not match the issuer or the payload
    /// is malformed.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        self.validate_structure()?;
        if self.issuer != public_key_hex(signing_key) {
            return Err(SyncwebError::InvalidSignature(
                "signing key does not match content ticket issuer".to_owned(),
            ));
        }
        self.signature = sign_hex(signing_key, &self.signed_bytes()?);
        Ok(())
    }

    /// Verify the issuer signature without applying local scope or expiry policy.
    ///
    /// # Errors
    ///
    /// Returns an error when the payload or signature is invalid.
    pub fn verify(&self) -> Result<()> {
        self.validate_structure()?;
        if self.signature.is_empty() {
            return Err(SyncwebError::MissingSignature("community content ticket".to_owned()));
        }
        verify_hex(&self.issuer, &self.signed_bytes()?, &self.signature)
    }

    /// Validate authenticity, freshness, and exact community scope.
    ///
    /// # Errors
    ///
    /// Returns an error when the ticket is malformed, expired, future-dated,
    /// outside the expected community, or incorrectly signed.
    pub fn validate(&self, now: u64, expected_community: CommunityId) -> Result<()> {
        self.validate_structure()?;
        let community: CommunityId = self.community_id.parse()?;
        if community != expected_community {
            return Err(SyncwebError::AccessDenied(
                "content ticket is outside the local community".to_owned(),
            ));
        }
        if self.issued_at > now {
            return Err(SyncwebError::InvalidConfig(
                "content ticket is issued in the future".to_owned(),
            ));
        }
        if let Some(expires_at) = self.expires_at {
            if expires_at <= self.issued_at {
                return Err(SyncwebError::InvalidConfig(
                    "content ticket expiry must be after issuance".to_owned(),
                ));
            }
            if now >= expires_at {
                return Err(SyncwebError::InvalidConfig("content ticket has expired".to_owned()));
            }
        }
        self.verify()
    }

    /// Decode the underlying Iroh blob ticket.
    ///
    /// # Errors
    ///
    /// Returns an error when the embedded ticket is malformed.
    pub fn blob_ticket(&self) -> Result<BlobTicket> {
        self.ticket
            .parse()
            .map_err(|error| SyncwebError::InvalidTicket(format!("invalid embedded blob ticket: {error}")))
    }

    /// Return the content hash addressed by the embedded ticket.
    ///
    /// # Errors
    ///
    /// Returns an error when the embedded ticket is malformed.
    pub fn content_hash(&self) -> Result<Hash> {
        Ok(self.blob_ticket()?.hash())
    }

    /// Serialize into bounded bytes suitable for a file or CAR metadata block.
    ///
    /// # Errors
    ///
    /// Returns an error when the ticket is malformed or too large.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate_structure()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|error| SyncwebError::operation("failed to serialize community content ticket", error))?;
        if bytes.len() > MAX_CONTENT_TICKET_BYTES {
            return Err(SyncwebError::InvalidConfig(
                "community content ticket exceeds the maximum wire size".to_owned(),
            ));
        }
        Ok(bytes)
    }

    /// Decode a ticket from file or CAR metadata.
    ///
    /// Signature and scope policy are checked separately by [`Self::validate`].
    ///
    /// # Errors
    ///
    /// Returns an error when the bytes are oversized, malformed, or structurally
    /// invalid.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        if bytes.as_ref().len() > MAX_CONTENT_TICKET_BYTES {
            return Err(SyncwebError::InvalidTicket(
                "community content ticket exceeds the maximum wire size".to_owned(),
            ));
        }
        let ticket: Self = serde_json::from_slice(bytes.as_ref())
            .map_err(|error| SyncwebError::InvalidTicket(format!("invalid community content ticket: {error}")))?;
        ticket.validate_structure()?;
        Ok(ticket)
    }

    fn validate_structure(&self) -> Result<()> {
        if self.version != COMMUNITY_CONTENT_TICKET_VERSION {
            return Err(SyncwebError::InvalidTicket(format!(
                "unsupported community content ticket version {}",
                self.version
            )));
        }
        let _: CommunityId = self.community_id.parse()?;
        self.issuer
            .parse::<PublicKey>()
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid content ticket issuer: {error}")))?;
        if self.ticket.trim().is_empty() {
            return Err(SyncwebError::InvalidTicket(
                "community content ticket must contain a blob ticket".to_owned(),
            ));
        }
        self.blob_ticket()?;
        if self.expires_at.is_some_and(|expires_at| expires_at <= self.issued_at) {
            return Err(SyncwebError::InvalidConfig(
                "content ticket expiry must be after issuance".to_owned(),
            ));
        }
        Ok(())
    }

    fn signed_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature.clear();
        let message = serde_json::to_vec(&unsigned)
            .map_err(|error| SyncwebError::operation("failed to serialize community content ticket", error))?;
        let mut signed = Vec::with_capacity(COMMUNITY_CONTENT_TICKET_CONTEXT.len().saturating_add(message.len()));
        signed.extend_from_slice(COMMUNITY_CONTENT_TICKET_CONTEXT);
        signed.extend_from_slice(&message);
        Ok(signed)
    }
}

impl Display for CommunityContentTicket {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let bytes = self.to_bytes().map_err(|error| {
            tracing::debug!(%error, "failed to serialize community content ticket for display");
            std::fmt::Error
        })?;
        write!(formatter, "{COMMUNITY_CONTENT_TICKET_URI_PREFIX}{}", hex::encode(bytes))
    }
}

impl FromStr for CommunityContentTicket {
    type Err = SyncwebError;

    fn from_str(value: &str) -> Result<Self> {
        let encoded = value.strip_prefix(COMMUNITY_CONTENT_TICKET_URI_PREFIX).ok_or_else(|| {
            SyncwebError::InvalidTicket(format!(
                "community content ticket must start with {COMMUNITY_CONTENT_TICKET_URI_PREFIX}"
            ))
        })?;
        let bytes = hex::decode(encoded).map_err(|error| {
            SyncwebError::InvalidTicket(format!("invalid community content ticket encoding: {error}"))
        })?;
        Self::from_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::EndpointAddr;
    use iroh_blobs::{BlobFormat, ticket::BlobTicket};

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn public_key(signing_key: &SigningKey) -> PublicKey {
        PublicKey::from_bytes(&signing_key.verifying_key().to_bytes()).expect("public key")
    }

    fn blob_ticket(hash: [u8; 32]) -> BlobTicket {
        let node_id = public_key(&key(9));
        BlobTicket::new(EndpointAddr::new(node_id), Hash::from_bytes(hash), BlobFormat::Raw)
    }

    #[test]
    fn signed_ticket_round_trips_and_enforces_scope() {
        let issuer = key(1);
        let community = CommunityId::from_bytes([2; 32]);
        let mut ticket =
            CommunityContentTicket::new(community, public_key(&issuer), &blob_ticket([7; 32]), 100, Some(200));
        ticket.sign(&issuer).expect("ticket should sign");

        let decoded: CommunityContentTicket = ticket.to_string().parse().expect("ticket should round trip");
        decoded.validate(150, community).expect("ticket should validate");
        assert_eq!(decoded.content_hash().expect("hash"), Hash::from_bytes([7; 32]));
        assert!(decoded.validate(150, CommunityId::from_bytes([3; 32])).is_err());
    }

    #[test]
    fn tampering_and_expiry_are_rejected() {
        let issuer = key(4);
        let community = CommunityId::from_bytes([5; 32]);
        let mut ticket =
            CommunityContentTicket::new(community, public_key(&issuer), &blob_ticket([7; 32]), 100, Some(200));
        ticket.sign(&issuer).expect("ticket should sign");
        ticket.ticket = blob_ticket([8; 32]).to_string();
        assert!(ticket.verify().is_err());
        let expired_ticket =
            CommunityContentTicket::new(community, public_key(&issuer), &blob_ticket([7; 32]), 100, Some(200));
        assert!(expired_ticket.validate(200, community).is_err());
    }
}
