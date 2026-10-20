//! Portable community identities and signed invitations.
//!
//! A community invitation is deliberately self-contained: the same bytes can
//! be placed in a QR code, a file, a CAR manifest, or a `syncweb://` URI.
//! Membership capabilities are independent so a discovery-only or read-only
//! invitation cannot accidentally grant write, pinning, relay, or contribution
//! authority.

use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use ed25519_dalek::SigningKey;
use iroh::PublicKey;
use serde::{Deserialize, Serialize};

use crate::{
    Result, SyncwebError,
    signature::{public_key_hex, sign_hex, verify_hex},
};

const COMMUNITY_INVITATION_CONTEXT: &[u8] = b"syncweb/community-invitation/v1\0";
const COMMUNITY_URI_PREFIX: &str = "syncweb://community/";
const COMMUNITY_ID_BYTES: usize = 32;
const COMMUNITY_NONCE_BYTES: usize = 16;
const MAX_INVITATION_BYTES: usize = 64 * 1024;
const MAX_NAME_BYTES: usize = 128;
pub const COMMUNITY_INVITATION_VERSION: u8 = 1;

/// Stable, content-independent identity for a communal network.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CommunityId([u8; COMMUNITY_ID_BYTES]);

impl CommunityId {
    /// Generate a new random community identity.
    #[must_use]
    pub fn random() -> Self {
        Self(rand::random())
    }

    /// Construct an identity from its wire bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; COMMUNITY_ID_BYTES]) -> Self {
        Self(bytes)
    }

    /// Return the identity's wire bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; COMMUNITY_ID_BYTES] {
        &self.0
    }
}

impl Display for CommunityId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&hex::encode(self.0))
    }
}

impl FromStr for CommunityId {
    type Err = SyncwebError;

    fn from_str(value: &str) -> Result<Self> {
        let bytes = hex::decode(value)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid community ID: {error}")))?;
        let bytes: [u8; COMMUNITY_ID_BYTES] = bytes.try_into().map_err(|bytes: Vec<u8>| {
            SyncwebError::InvalidConfig(format!(
                "community ID must contain {COMMUNITY_ID_BYTES} bytes, got {}",
                bytes.len()
            ))
        })?;
        Ok(Self(bytes))
    }
}

/// Human-readable identity metadata carried by an invitation.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CommunityIdentity {
    pub id: CommunityId,
    pub name: String,
}

impl CommunityIdentity {
    /// Create a new community with a random identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the display name is empty, too long, or contains
    /// control characters.
    pub fn new(name: impl Into<String>) -> Result<Self> {
        Self::from_parts(CommunityId::random(), name)
    }

    /// Construct a community from an existing identity and display name.
    ///
    /// # Errors
    ///
    /// Returns an error when the display name is empty, too long, or contains
    /// control characters.
    pub fn from_parts(id: CommunityId, name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        validate_name(&name)?;
        Ok(Self { id, name })
    }
}

/// A capability that can be granted independently in a community invitation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum CommunityCapability {
    Discovery,
    Read,
    Write,
    Pin,
    Relay,
    Contribute,
}

/// A coarse operator-visible role carried by a community invitation.
///
/// Capabilities remain the authorization source of truth. Roles provide a
/// stable label for status and audit output, while validation prevents an
/// invitation from granting capabilities outside its declared role.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum CommunityRole {
    Member,
    Contributor,
    Relay,
    Moderator,
    Owner,
}

impl CommunityRole {
    /// Return whether this role permits a capability to be granted.
    #[must_use]
    pub const fn allows(self, capability: CommunityCapability) -> bool {
        match self {
            Self::Member => matches!(capability, CommunityCapability::Discovery | CommunityCapability::Read),
            Self::Contributor => matches!(
                capability,
                CommunityCapability::Discovery
                    | CommunityCapability::Read
                    | CommunityCapability::Pin
                    | CommunityCapability::Contribute
            ),
            Self::Relay => matches!(
                capability,
                CommunityCapability::Discovery | CommunityCapability::Read | CommunityCapability::Relay
            ),
            Self::Moderator => matches!(
                capability,
                CommunityCapability::Discovery
                    | CommunityCapability::Read
                    | CommunityCapability::Write
                    | CommunityCapability::Pin
                    | CommunityCapability::Relay
            ),
            Self::Owner => true,
        }
    }
}

/// Independent permissions carried by a community invitation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CommunityCapabilities {
    pub discovery: bool,
    pub read: bool,
    pub write: bool,
    pub pin: bool,
    pub relay: bool,
    pub contribute: bool,
}

impl CommunityCapabilities {
    /// Capabilities for a normal read-only participant.
    #[must_use]
    pub const fn read_only() -> Self {
        Self {
            discovery: true,
            read: true,
            write: false,
            pin: false,
            relay: false,
            contribute: false,
        }
    }

    /// All currently defined capabilities.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            discovery: true,
            read: true,
            write: true,
            pin: true,
            relay: true,
            contribute: true,
        }
    }

    /// Return whether this set grants the requested capability.
    #[must_use]
    pub const fn allows(self, capability: CommunityCapability) -> bool {
        match capability {
            CommunityCapability::Discovery => self.discovery,
            CommunityCapability::Read => self.read,
            CommunityCapability::Write => self.write,
            CommunityCapability::Pin => self.pin,
            CommunityCapability::Relay => self.relay,
            CommunityCapability::Contribute => self.contribute,
        }
    }

    #[must_use]
    const fn is_empty(self) -> bool {
        !(self.discovery || self.read || self.write || self.pin || self.relay || self.contribute)
    }
}

/// A signed, portable invitation to a community.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CommunityInvitation {
    pub version: u8,
    pub community_id: String,
    pub name: String,
    pub issuer: String,
    pub subject: Option<String>,
    pub capabilities: CommunityCapabilities,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<CommunityRole>,
    pub issued_at: u64,
    pub expires_at: Option<u64>,
    pub nonce: String,
    pub signature: String,
}

impl CommunityInvitation {
    /// Construct an unsigned invitation.
    #[must_use]
    pub fn new(
        community: &CommunityIdentity,
        issuer: PublicKey,
        subject: Option<PublicKey>,
        capabilities: CommunityCapabilities,
        issued_at: u64,
        expires_at: Option<u64>,
    ) -> Self {
        Self::new_with_role(
            community,
            issuer,
            subject,
            capabilities,
            inferred_role(capabilities),
            issued_at,
            expires_at,
        )
    }

    /// Construct an unsigned invitation with an explicit operator role.
    #[must_use]
    pub fn new_with_role(
        community: &CommunityIdentity,
        issuer: PublicKey,
        subject: Option<PublicKey>,
        capabilities: CommunityCapabilities,
        role: CommunityRole,
        issued_at: u64,
        expires_at: Option<u64>,
    ) -> Self {
        Self {
            version: COMMUNITY_INVITATION_VERSION,
            community_id: community.id.to_string(),
            name: community.name.clone(),
            issuer: issuer.to_string(),
            subject: subject.map(|key| key.to_string()),
            capabilities,
            role: Some(role),
            issued_at,
            expires_at,
            nonce: hex::encode(rand::random::<[u8; COMMUNITY_NONCE_BYTES]>()),
            signature: String::new(),
        }
    }

    /// Return this invitation's optional role label.
    #[must_use]
    pub const fn role(&self) -> Option<CommunityRole> {
        self.role
    }

    /// Sign the invitation with the community issuer's identity key.
    ///
    /// # Errors
    ///
    /// Returns an error when the signing key does not match the issuer or the
    /// invitation cannot be serialized.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        self.validate_structure()?;
        if self.issuer != public_key_hex(signing_key) {
            return Err(SyncwebError::InvalidSignature(
                "signing key does not match community invitation issuer".to_owned(),
            ));
        }
        self.signature = sign_hex(signing_key, &self.signed_bytes()?);
        Ok(())
    }

    /// Verify the issuer signature without applying local time or scope policy.
    ///
    /// # Errors
    ///
    /// Returns an error when the invitation is malformed or its signature is
    /// absent or invalid.
    pub fn verify(&self) -> Result<()> {
        self.validate_structure()?;
        if self.signature.is_empty() {
            return Err(SyncwebError::MissingSignature("community invitation".to_owned()));
        }
        verify_hex(&self.issuer, &self.signed_bytes()?, &self.signature)
    }

    /// Validate authenticity, freshness, and optional community scope.
    ///
    /// # Errors
    ///
    /// Returns an error when the invitation is malformed, outside the expected
    /// community, expired, future-dated, or not correctly signed.
    pub fn validate(&self, now: u64, expected_community: Option<CommunityId>) -> Result<()> {
        self.validate_structure()?;
        let community_id: CommunityId = self.community_id.parse()?;
        if expected_community.is_some_and(|expected| expected != community_id) {
            return Err(SyncwebError::AccessDenied(
                "community invitation is for a different community".to_owned(),
            ));
        }
        if self.issued_at > now {
            return Err(SyncwebError::InvalidConfig(
                "community invitation is issued in the future".to_owned(),
            ));
        }
        if let Some(expires_at) = self.expires_at {
            if expires_at <= self.issued_at {
                return Err(SyncwebError::InvalidConfig(
                    "community invitation expiry must be after issuance".to_owned(),
                ));
            }
            if now >= expires_at {
                return Err(SyncwebError::InvalidConfig(
                    "community invitation has expired".to_owned(),
                ));
            }
        }
        self.verify()
    }

    /// Serialize into bounded bytes suitable for a file, QR payload, or CAR block.
    ///
    /// # Errors
    ///
    /// Returns an error when the invitation is malformed or exceeds the wire
    /// size limit.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate_structure()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|error| SyncwebError::operation("failed to serialize community invitation", error))?;
        if bytes.len() > MAX_INVITATION_BYTES {
            return Err(SyncwebError::InvalidConfig(
                "community invitation exceeds the maximum wire size".to_owned(),
            ));
        }
        Ok(bytes)
    }

    /// Decode an invitation from file, QR, or CAR bytes.
    ///
    /// Signature and expiry policy are checked separately by [`Self::validate`].
    ///
    /// # Errors
    ///
    /// Returns an error when the bytes are oversized, malformed, or structurally
    /// invalid.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        if bytes.as_ref().len() > MAX_INVITATION_BYTES {
            return Err(SyncwebError::InvalidTicket(
                "community invitation exceeds the maximum wire size".to_owned(),
            ));
        }
        let invitation: Self = serde_json::from_slice(bytes.as_ref())
            .map_err(|error| SyncwebError::InvalidTicket(format!("invalid community invitation: {error}")))?;
        invitation.validate_structure()?;
        Ok(invitation)
    }

    fn validate_structure(&self) -> Result<()> {
        if self.version != COMMUNITY_INVITATION_VERSION {
            return Err(SyncwebError::InvalidTicket(format!(
                "unsupported community invitation version {}",
                self.version
            )));
        }
        validate_name(&self.name)?;
        let _: CommunityId = self.community_id.parse()?;
        self.issuer
            .parse::<PublicKey>()
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid community issuer: {error}")))?;
        if let Some(subject) = &self.subject {
            subject
                .parse::<PublicKey>()
                .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid community subject: {error}")))?;
        }
        let nonce = hex::decode(&self.nonce)
            .map_err(|error| SyncwebError::InvalidTicket(format!("invalid community invitation nonce: {error}")))?;
        if nonce.len() != COMMUNITY_NONCE_BYTES {
            return Err(SyncwebError::InvalidTicket(format!(
                "community invitation nonce must contain {COMMUNITY_NONCE_BYTES} bytes, got {}",
                nonce.len()
            )));
        }
        if self.capabilities.is_empty() {
            return Err(SyncwebError::InvalidTicket(
                "community invitation must grant at least one capability".to_owned(),
            ));
        }
        if let Some(role) = self.role {
            for capability in [
                CommunityCapability::Discovery,
                CommunityCapability::Read,
                CommunityCapability::Write,
                CommunityCapability::Pin,
                CommunityCapability::Relay,
                CommunityCapability::Contribute,
            ] {
                if self.capabilities.allows(capability) && !role.allows(capability) {
                    return Err(SyncwebError::InvalidTicket(format!(
                        "community invitation role {role:?} cannot grant {capability:?}"
                    )));
                }
            }
        }
        Ok(())
    }

    fn signed_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature.clear();
        let message = serde_json::to_vec(&unsigned)
            .map_err(|error| SyncwebError::operation("failed to serialize community invitation", error))?;
        let mut signed = Vec::with_capacity(COMMUNITY_INVITATION_CONTEXT.len().saturating_add(message.len()));
        signed.extend_from_slice(COMMUNITY_INVITATION_CONTEXT);
        signed.extend_from_slice(&message);
        Ok(signed)
    }
}

impl Display for CommunityInvitation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let bytes = self.to_bytes().map_err(|_| std::fmt::Error)?;
        write!(formatter, "{COMMUNITY_URI_PREFIX}{}", hex::encode(bytes))
    }
}

impl FromStr for CommunityInvitation {
    type Err = SyncwebError;

    fn from_str(value: &str) -> Result<Self> {
        let encoded = value.strip_prefix(COMMUNITY_URI_PREFIX).ok_or_else(|| {
            SyncwebError::InvalidTicket(format!("community invitation must start with {COMMUNITY_URI_PREFIX}"))
        })?;
        let bytes = hex::decode(encoded)
            .map_err(|error| SyncwebError::InvalidTicket(format!("invalid community invitation encoding: {error}")))?;
        Self::from_bytes(bytes)
    }
}

fn validate_name(name: &str) -> Result<()> {
    let length = name.len();
    if name.trim().is_empty() {
        return Err(SyncwebError::InvalidConfig("community name cannot be empty".to_owned()));
    }
    if length > MAX_NAME_BYTES {
        return Err(SyncwebError::InvalidConfig(format!(
            "community name exceeds {MAX_NAME_BYTES} bytes"
        )));
    }
    if name.chars().any(char::is_control) {
        return Err(SyncwebError::InvalidConfig(
            "community name cannot contain control characters".to_owned(),
        ));
    }
    Ok(())
}

const fn inferred_role(capabilities: CommunityCapabilities) -> CommunityRole {
    if capabilities.contribute && (capabilities.write || capabilities.relay) {
        CommunityRole::Owner
    } else if capabilities.contribute || capabilities.pin {
        CommunityRole::Contributor
    } else if capabilities.write {
        CommunityRole::Moderator
    } else if capabilities.relay {
        CommunityRole::Relay
    } else {
        CommunityRole::Member
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn public_key(signing_key: &SigningKey) -> PublicKey {
        PublicKey::from_bytes(&signing_key.verifying_key().to_bytes()).expect("public key")
    }

    #[test]
    fn signed_invitation_round_trips_as_uri_and_bytes() {
        let issuer = key(1);
        let subject = key(2);
        let community = CommunityIdentity::new("Offline classroom").expect("valid community");
        let mut invitation = CommunityInvitation::new(
            &community,
            public_key(&issuer),
            Some(public_key(&subject)),
            CommunityCapabilities::read_only(),
            100,
            Some(200),
        );
        invitation.sign(&issuer).expect("invitation should sign");

        let uri = invitation.to_string();
        let decoded: CommunityInvitation = uri.parse().expect("URI should decode");
        assert_eq!(decoded, invitation);
        decoded
            .validate(150, Some(community.id))
            .expect("invitation should validate");
        let bytes = invitation.to_bytes().expect("invitation should encode");
        assert_eq!(
            CommunityInvitation::from_bytes(bytes).expect("bytes should decode"),
            invitation
        );
    }

    #[test]
    fn capabilities_are_independent() {
        let capabilities = CommunityCapabilities {
            discovery: true,
            read: true,
            write: false,
            pin: true,
            relay: false,
            contribute: false,
        };
        assert!(capabilities.allows(CommunityCapability::Discovery));
        assert!(capabilities.allows(CommunityCapability::Read));
        assert!(capabilities.allows(CommunityCapability::Pin));
        assert!(!capabilities.allows(CommunityCapability::Write));
        assert!(!capabilities.allows(CommunityCapability::Relay));
        assert!(!capabilities.allows(CommunityCapability::Contribute));
    }

    #[test]
    fn roles_bound_capabilities_and_round_trip() {
        let issuer = key(5);
        let subject = key(6);
        let community = CommunityIdentity::new("Role test").expect("valid community");
        let mut invitation = CommunityInvitation::new_with_role(
            &community,
            public_key(&issuer),
            Some(public_key(&subject)),
            CommunityCapabilities {
                discovery: true,
                read: true,
                pin: true,
                ..CommunityCapabilities::default()
            },
            CommunityRole::Contributor,
            100,
            None,
        );
        invitation.sign(&issuer).expect("role invitation should sign");
        let decoded = CommunityInvitation::from_bytes(invitation.to_bytes().expect("encode")).expect("decode");
        assert_eq!(decoded.role(), Some(CommunityRole::Contributor));
        decoded
            .validate(100, Some(community.id))
            .expect("role invitation should validate");

        let mut invalid = CommunityInvitation::new_with_role(
            &community,
            public_key(&issuer),
            None,
            CommunityCapabilities::all(),
            CommunityRole::Member,
            100,
            None,
        );
        assert!(invalid.sign(&issuer).is_err());
    }

    #[test]
    fn validation_rejects_tampering_scope_and_expiry() {
        let issuer = key(3);
        let community = CommunityIdentity::new("Test").expect("valid community");
        let mut invitation = CommunityInvitation::new(
            &community,
            public_key(&issuer),
            None,
            CommunityCapabilities::all(),
            100,
            Some(200),
        );
        invitation.sign(&issuer).expect("invitation should sign");
        assert!(invitation.validate(200, Some(community.id)).is_err());
        assert!(invitation.validate(150, Some(CommunityId::random())).is_err());

        invitation.name = "Tampered".to_owned();
        assert!(invitation.verify().is_err());
    }

    #[test]
    fn malformed_invitation_is_rejected_before_signature_checks() {
        let issuer = key(4);
        let community = CommunityIdentity::new("Test").expect("valid community");
        let invitation = CommunityInvitation::new(
            &community,
            public_key(&issuer),
            None,
            CommunityCapabilities::default(),
            100,
            None,
        );
        assert!(invitation.to_bytes().is_err());
    }
}
