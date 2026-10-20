//! Admission, revocation, and moderation policy for ownerless communities.
//!
//! These policies are deliberately local and portable. A community can
//! exchange signed revocations through its normal document or CAR paths
//! without requiring an always-online coordinator, while each node retains
//! control over which issuers it trusts and which contribution limits it
//! enforces.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt,
};

use ed25519_dalek::SigningKey;
use iroh::PublicKey;
use serde::{Deserialize, Serialize};

use crate::{
    Result, SyncwebError,
    signature::{public_key_hex, sign_hex, verify_hex},
};

use super::community::{CommunityId, CommunityInvitation};

const COMMUNITY_REVOCATION_CONTEXT: &[u8] = b"syncweb/community-revocation/v1\0";
const REVOCATION_NONCE_BYTES: usize = 16;
const MAX_REVOCATION_REASON_BYTES: usize = 512;
const DEFAULT_AUDIT_CAPACITY: usize = 256;
pub const COMMUNITY_REVOCATION_VERSION: u8 = 1;

/// A signed revocation that can target a member or one invitation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CommunityRevocation {
    pub version: u8,
    pub community_id: String,
    pub issuer: String,
    pub subject: Option<String>,
    pub invitation_nonce: Option<String>,
    pub issued_at: u64,
    pub reason: String,
    pub nonce: String,
    pub signature: String,
}

impl CommunityRevocation {
    /// Construct an unsigned revocation.
    ///
    /// At least one of `subject` or `invitation_nonce` must be supplied before
    /// the revocation can be signed.
    #[must_use]
    pub fn new(
        community: CommunityId,
        issuer: PublicKey,
        subject: Option<PublicKey>,
        invitation_nonce: Option<String>,
        issued_at: u64,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            version: COMMUNITY_REVOCATION_VERSION,
            community_id: community.to_string(),
            issuer: issuer.to_string(),
            subject: subject.map(|key| key.to_string()),
            invitation_nonce,
            issued_at,
            reason: reason.into(),
            nonce: hex::encode(rand::random::<[u8; REVOCATION_NONCE_BYTES]>()),
            signature: String::new(),
        }
    }

    /// Sign the revocation with the issuer's identity key.
    ///
    /// # Errors
    ///
    /// Returns an error when the key does not match the issuer or the
    /// revocation is malformed.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        self.validate_structure()?;
        if self.issuer != public_key_hex(signing_key) {
            return Err(SyncwebError::InvalidSignature(
                "signing key does not match community revocation issuer".to_owned(),
            ));
        }
        self.signature = sign_hex(signing_key, &self.signed_bytes()?);
        Ok(())
    }

    /// Verify the issuer signature without applying time or local trust policy.
    ///
    /// # Errors
    ///
    /// Returns an error when the revocation is malformed or unsigned.
    pub fn verify(&self) -> Result<()> {
        self.validate_structure()?;
        if self.signature.is_empty() {
            return Err(SyncwebError::MissingSignature("community revocation".to_owned()));
        }
        verify_hex(&self.issuer, &self.signed_bytes()?, &self.signature)
    }

    /// Validate authenticity, freshness, and optional community scope.
    ///
    /// # Errors
    ///
    /// Returns an error when the revocation is malformed, future-dated,
    /// outside the expected community, or incorrectly signed.
    pub fn validate(&self, now: u64, expected_community: Option<CommunityId>) -> Result<()> {
        self.validate_structure()?;
        let community_id: CommunityId = self.community_id.parse()?;
        if expected_community.is_some_and(|expected| expected != community_id) {
            return Err(SyncwebError::AccessDenied(
                "community revocation is for a different community".to_owned(),
            ));
        }
        if self.issued_at > now {
            return Err(SyncwebError::InvalidConfig(
                "community revocation is issued in the future".to_owned(),
            ));
        }
        self.verify()
    }

    /// Return whether this revocation applies to an invitation.
    #[must_use]
    pub fn applies_to(&self, invitation: &CommunityInvitation) -> bool {
        if self.community_id != invitation.community_id {
            return false;
        }
        let subject_matches = self
            .subject
            .as_deref()
            .is_none_or(|subject| invitation.subject.as_deref() == Some(subject));
        let nonce_matches = self
            .invitation_nonce
            .as_deref()
            .is_none_or(|nonce| nonce == invitation.nonce);
        subject_matches && nonce_matches
    }

    fn validate_structure(&self) -> Result<()> {
        if self.version != COMMUNITY_REVOCATION_VERSION {
            return Err(SyncwebError::InvalidTicket(format!(
                "unsupported community revocation version {}",
                self.version
            )));
        }
        let _: CommunityId = self.community_id.parse()?;
        self.issuer
            .parse::<PublicKey>()
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid revocation issuer: {error}")))?;
        if let Some(subject) = &self.subject {
            subject
                .parse::<PublicKey>()
                .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid revocation subject: {error}")))?;
        }
        if self.subject.is_none() && self.invitation_nonce.is_none() {
            return Err(SyncwebError::InvalidTicket(
                "community revocation must target a subject or invitation".to_owned(),
            ));
        }
        if let Some(invitation_nonce) = &self.invitation_nonce {
            let nonce = hex::decode(invitation_nonce).map_err(|error| {
                SyncwebError::InvalidTicket(format!("invalid revocation invitation nonce: {error}"))
            })?;
            if nonce.len() != 16 {
                return Err(SyncwebError::InvalidTicket(format!(
                    "revocation invitation nonce must contain 16 bytes, got {}",
                    nonce.len()
                )));
            }
        }
        let nonce = hex::decode(&self.nonce)
            .map_err(|error| SyncwebError::InvalidTicket(format!("invalid revocation nonce: {error}")))?;
        if nonce.len() != REVOCATION_NONCE_BYTES {
            return Err(SyncwebError::InvalidTicket(format!(
                "community revocation nonce must contain {REVOCATION_NONCE_BYTES} bytes, got {}",
                nonce.len()
            )));
        }
        if self.reason.trim().is_empty() {
            return Err(SyncwebError::InvalidConfig(
                "community revocation reason cannot be empty".to_owned(),
            ));
        }
        if self.reason.len() > MAX_REVOCATION_REASON_BYTES {
            return Err(SyncwebError::InvalidConfig(format!(
                "community revocation reason exceeds {MAX_REVOCATION_REASON_BYTES} bytes"
            )));
        }
        Ok(())
    }

    fn signed_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature.clear();
        let message = serde_json::to_vec(&unsigned)
            .map_err(|error| SyncwebError::operation("failed to serialize community revocation", error))?;
        let mut signed = Vec::with_capacity(COMMUNITY_REVOCATION_CONTEXT.len().saturating_add(message.len()));
        signed.extend_from_slice(COMMUNITY_REVOCATION_CONTEXT);
        signed.extend_from_slice(&message);
        Ok(signed)
    }
}

/// Local trust policy for admitting invitations without an online owner.
#[derive(Clone, Debug)]
pub struct CommunityAdmissionPolicy {
    community: CommunityId,
    trusted_issuers: BTreeSet<String>,
    revocations: Vec<CommunityRevocation>,
}

impl CommunityAdmissionPolicy {
    /// Create an admission policy for a community.
    #[must_use]
    pub const fn new(community: CommunityId) -> Self {
        Self {
            community,
            trusted_issuers: BTreeSet::new(),
            revocations: Vec::new(),
        }
    }

    /// Trust an issuer for this local admission policy.
    pub fn trust_issuer(&mut self, issuer: PublicKey) {
        self.trusted_issuers.insert(issuer.to_string());
    }

    /// Stop trusting an issuer for future admissions.
    pub fn untrust_issuer(&mut self, issuer: &PublicKey) -> bool {
        self.trusted_issuers.remove(&issuer.to_string())
    }

    /// Add a signed revocation issued by a trusted community issuer.
    ///
    /// # Errors
    ///
    /// Returns an error when the revocation is invalid, scoped to another
    /// community, or signed by an issuer this policy does not trust.
    pub fn add_revocation(&mut self, revocation: CommunityRevocation, now: u64) -> Result<()> {
        revocation.validate(now, Some(self.community))?;
        if !self.trusted_issuers.contains(&revocation.issuer) {
            return Err(SyncwebError::AccessDenied(
                "community revocation issuer is not trusted".to_owned(),
            ));
        }
        if !self.revocations.contains(&revocation) {
            self.revocations.push(revocation);
        }
        Ok(())
    }

    /// Validate an invitation against trusted issuers and received revocations.
    ///
    /// # Errors
    ///
    /// Returns an error when the invitation is not trusted or has been revoked.
    pub fn admit(&self, invitation: &CommunityInvitation, now: u64) -> Result<()> {
        invitation.validate(now, Some(self.community))?;
        if !self.trusted_issuers.contains(&invitation.issuer) {
            return Err(SyncwebError::AccessDenied(
                "community invitation issuer is not trusted".to_owned(),
            ));
        }
        for revocation in &self.revocations {
            revocation.validate(now, Some(self.community))?;
            if revocation.applies_to(invitation) {
                return Err(SyncwebError::AccessDenied(format!(
                    "community invitation revoked: {}",
                    revocation.reason
                )));
            }
        }
        Ok(())
    }

    /// Return the community scope enforced by this policy.
    #[must_use]
    pub const fn community(&self) -> CommunityId {
        self.community
    }

    /// Return the trusted issuer identities.
    #[must_use]
    pub const fn trusted_issuers(&self) -> &BTreeSet<String> {
        &self.trusted_issuers
    }

    /// Return all accepted revocations.
    #[must_use]
    pub fn revocations(&self) -> &[CommunityRevocation] {
        &self.revocations
    }
}

/// A bounded contribution rate limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommunityRateLimit {
    window_secs: u64,
    max_bytes: u64,
}

impl CommunityRateLimit {
    /// Construct a rate limit with a non-zero window and byte budget.
    ///
    /// # Errors
    ///
    /// Returns an error when either limit is zero.
    pub fn new(window_secs: u64, max_bytes: u64) -> Result<Self> {
        if window_secs == 0 || max_bytes == 0 {
            return Err(SyncwebError::InvalidConfig(
                "community rate limits must have non-zero window and byte values".to_owned(),
            ));
        }
        Ok(Self { window_secs, max_bytes })
    }

    #[must_use]
    pub const fn window_secs(self) -> u64 {
        self.window_secs
    }

    #[must_use]
    pub const fn max_bytes(self) -> u64 {
        self.max_bytes
    }
}

/// An operator-visible result of a moderation decision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CommunityAuditEvent {
    pub node: String,
    pub timestamp: u64,
    pub object_size: u64,
    pub accepted: bool,
    pub reason: Option<String>,
}

/// Local moderation hooks for community requests.
#[derive(Clone, Debug)]
pub struct CommunityModerationPolicy {
    denylisted_nodes: BTreeSet<String>,
    max_object_size: Option<u64>,
    rate_limit: Option<CommunityRateLimit>,
    usage: BTreeMap<String, (u64, u64)>,
    audit_capacity: usize,
    audit: VecDeque<CommunityAuditEvent>,
}

impl Default for CommunityModerationPolicy {
    fn default() -> Self {
        Self {
            denylisted_nodes: BTreeSet::new(),
            max_object_size: None,
            rate_limit: None,
            usage: BTreeMap::new(),
            audit_capacity: DEFAULT_AUDIT_CAPACITY,
            audit: VecDeque::new(),
        }
    }
}

impl CommunityModerationPolicy {
    /// Set or clear the maximum object size accepted by this policy.
    pub const fn set_max_object_size(&mut self, max_bytes: Option<u64>) {
        self.max_object_size = max_bytes;
    }

    /// Set or clear the contribution rate limit.
    pub fn set_rate_limit(&mut self, rate_limit: Option<CommunityRateLimit>) {
        self.rate_limit = rate_limit;
        self.usage.clear();
    }

    /// Deny a node before evaluating size or rate limits.
    pub fn deny_node(&mut self, node: PublicKey) {
        self.denylisted_nodes.insert(node.to_string());
    }

    /// Remove a node from the denylist.
    pub fn allow_node(&mut self, node: &PublicKey) -> bool {
        self.denylisted_nodes.remove(&node.to_string())
    }

    /// Set the bounded number of audit events retained in memory.
    pub fn set_audit_capacity(&mut self, capacity: usize) {
        self.audit_capacity = capacity;
        while self.audit.len() > capacity {
            self.audit.pop_front();
        }
    }

    /// Check a contribution and record both accepted and rejected decisions.
    ///
    /// # Errors
    ///
    /// Returns access denied when the node, object size, or rate budget is not
    /// allowed. The rejection is still recorded in the audit log.
    pub fn authorize(&mut self, node: PublicKey, object_size: u64, now: u64) -> Result<()> {
        let node_id = node.to_string();
        if self.denylisted_nodes.contains(&node_id) {
            return self.reject(node_id, object_size, now, "node is denylisted");
        }
        if self.max_object_size.is_some_and(|limit| object_size > limit) {
            return self.reject(node_id, object_size, now, "object exceeds community size limit");
        }
        if let Some(limit) = self.rate_limit {
            let (previous_window_start, previous_used) = self.usage.get(&node_id).copied().unwrap_or((now, 0));
            let (window_start, used) = if now.saturating_sub(previous_window_start) >= limit.window_secs {
                (now, 0)
            } else {
                (previous_window_start, previous_used)
            };
            let next_used_result = used
                .checked_add(object_size)
                .ok_or_else(|| SyncwebError::AccessDenied("community contribution rate budget overflow".to_owned()));
            let next_used = match next_used_result {
                Ok(value) => value,
                Err(error) => {
                    return self.reject(node_id, object_size, now, &error.to_string());
                }
            };
            if next_used > limit.max_bytes {
                return self.reject(node_id, object_size, now, "community contribution rate limit exceeded");
            }
            self.usage.insert(node_id.clone(), (window_start, next_used));
        }
        self.record(CommunityAuditEvent {
            node: node_id,
            timestamp: now,
            object_size,
            accepted: true,
            reason: None,
        });
        Ok(())
    }

    /// Return the retained moderation audit events in chronological order.
    pub fn audit_events(&self) -> impl Iterator<Item = &CommunityAuditEvent> {
        self.audit.iter()
    }

    fn reject(&mut self, node: String, object_size: u64, now: u64, reason: &str) -> Result<()> {
        self.record(CommunityAuditEvent {
            node,
            timestamp: now,
            object_size,
            accepted: false,
            reason: Some(reason.to_owned()),
        });
        Err(SyncwebError::AccessDenied(reason.to_owned()))
    }

    fn record(&mut self, event: CommunityAuditEvent) {
        if self.audit_capacity == 0 {
            return;
        }
        if self.audit.len() == self.audit_capacity {
            self.audit.pop_front();
        }
        self.audit.push_back(event);
    }
}

impl fmt::Display for CommunityRateLimit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} bytes per {} seconds", self.max_bytes, self.window_secs)
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
    fn ownerless_admission_accepts_trusted_invitation_and_honors_revocation() {
        let issuer = key(1);
        let subject = key(2);
        let community = CommunityId::from_bytes([7; 32]);
        let identity = super::super::community::CommunityIdentity::from_parts(community, "LAN").expect("identity");
        let mut invitation = CommunityInvitation::new(
            &identity,
            public_key(&issuer),
            Some(public_key(&subject)),
            super::super::community::CommunityCapabilities::read_only(),
            100,
            None,
        );
        invitation.sign(&issuer).expect("invitation should sign");

        let mut policy = CommunityAdmissionPolicy::new(community);
        policy.trust_issuer(public_key(&issuer));
        policy.admit(&invitation, 101).expect("trusted invitation should admit");

        let mut revocation = CommunityRevocation::new(
            community,
            public_key(&issuer),
            Some(public_key(&subject)),
            None,
            102,
            "device left the classroom",
        );
        revocation.sign(&issuer).expect("revocation should sign");
        policy
            .add_revocation(revocation, 103)
            .expect("revocation should be accepted");
        assert!(policy.admit(&invitation, 103).is_err());
    }

    #[test]
    fn moderation_enforces_denylist_size_and_rate_with_bounded_audit() {
        let node = public_key(&key(3));
        let mut policy = CommunityModerationPolicy::default();
        policy.set_max_object_size(Some(10));
        policy.set_rate_limit(Some(CommunityRateLimit::new(60, 15).expect("valid limit")));

        policy.authorize(node, 10, 100).expect("first contribution should pass");
        assert!(policy.authorize(node, 6, 101).is_err());
        assert!(policy.authorize(node, 11, 102).is_err());
        policy.deny_node(node);
        assert!(policy.authorize(node, 1, 103).is_err());

        let events = policy.audit_events().collect::<Vec<_>>();
        assert_eq!(events.len(), 4);
        assert!(events.first().expect("first audit event should exist").accepted);
        assert_eq!(
            events
                .get(1)
                .expect("second audit event should exist")
                .reason
                .as_deref(),
            Some("community contribution rate limit exceeded")
        );
        assert_eq!(
            events.get(2).expect("third audit event should exist").reason.as_deref(),
            Some("object exceeds community size limit")
        );
        assert_eq!(
            events
                .get(3)
                .expect("fourth audit event should exist")
                .reason
                .as_deref(),
            Some("node is denylisted")
        );
    }

    #[test]
    fn revocation_rejects_invalid_targets_and_untrusted_issuers() {
        let issuer = key(4);
        let community = CommunityId::from_bytes([8; 32]);
        let mut invalid_revocation =
            CommunityRevocation::new(community, public_key(&issuer), None, None, 100, "invalid");
        assert!(invalid_revocation.sign(&issuer).is_err());

        let mut policy = CommunityAdmissionPolicy::new(community);
        let mut untrusted_revocation = CommunityRevocation::new(
            community,
            public_key(&issuer),
            Some(public_key(&key(5))),
            None,
            100,
            "not trusted",
        );
        untrusted_revocation.sign(&issuer).expect("revocation should sign");
        assert!(policy.add_revocation(untrusted_revocation, 100).is_err());
    }
}
