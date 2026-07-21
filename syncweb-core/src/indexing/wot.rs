//! Local Web-of-Trust policy and signed metadata for indexed content.
//!
//! Content hashes establish integrity, while this module records who made a
//! claim and lets each indexer decide which claims are trusted. Records are
//! signed before they enter the SQLite/FTS index; moderation and revocation
//! decisions remain local policy state.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use iroh_blobs::Hash;
use iroh_docs::NamespaceId;
use serde::{Deserialize, Serialize};

use crate::{
    error::{Result, SyncwebError},
    indexing::{IndexingDatabase, IndexingService},
};

const METADATA_CONTEXT: &[u8] = b"syncweb/wot/metadata/v1\0";
const DELEGATION_CONTEXT: &[u8] = b"syncweb/wot/delegation/v1\0";
const REVOCATION_CONTEXT: &[u8] = b"syncweb/wot/revocation/v1\0";
const MODERATION_CONTEXT: &[u8] = b"syncweb/wot/moderation/v1\0";
const ATTESTATION_CONTEXT: &[u8] = b"syncweb/wot/attestation/v1\0";

/// Signed metadata appended to a content hash.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MetadataEntry {
    pub content: Hash,
    pub key: String,
    pub value: String,
    pub author: String,
    pub sequence: u64,
    pub created_at: u64,
    pub signature: Option<String>,
}

/// Alias emphasizing that the entry belongs to the Web-of-Trust layer.
pub type WotMetadata = MetadataEntry;

impl MetadataEntry {
    /// Create and sign metadata with an Ed25519 author key.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata fields are invalid or signing fails.
    pub fn new(
        content: Hash,
        key: impl Into<String>,
        value: impl Into<String>,
        sequence: u64,
        signing_key: &SigningKey,
    ) -> Result<Self> {
        Self::new_with_time(content, key, value, sequence, current_epoch_seconds(), signing_key)
    }

    /// Create and sign metadata with an explicit timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata fields are invalid or signing fails.
    pub fn new_with_time(
        content: Hash,
        key: impl Into<String>,
        value: impl Into<String>,
        sequence: u64,
        created_at: u64,
        signing_key: &SigningKey,
    ) -> Result<Self> {
        let mut entry = Self {
            content,
            key: key.into(),
            value: value.into(),
            author: author_id(&signing_key.verifying_key()),
            sequence,
            created_at,
            signature: None,
        };
        entry.sign(signing_key)?;
        Ok(entry)
    }

    /// Create unsigned metadata for a specific author identity.
    ///
    /// Call [`Self::sign`] before accepting or serializing the entry.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata fields are invalid.
    pub fn unsigned(
        content: Hash,
        key: impl Into<String>,
        value: impl Into<String>,
        sequence: u64,
        created_at: u64,
        author: &VerifyingKey,
    ) -> Result<Self> {
        let entry = Self {
            content,
            key: key.into(),
            value: value.into(),
            author: author_id(author),
            sequence,
            created_at,
            signature: None,
        };
        entry.validate()?;
        Ok(entry)
    }

    /// Sign the entry with its declared author.
    ///
    /// # Errors
    ///
    /// Returns an error if the signing key does not match the author.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        ensure_signer(&self.author, &signing_key.verifying_key())?;
        self.signature = Some(sign_text(METADATA_CONTEXT, &self.unsigned_bytes()?, signing_key));
        Ok(())
    }

    /// Verify the metadata signature and structure.
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata or signature is invalid.
    pub fn verify_signature(&self) -> Result<()> {
        self.validate()?;
        verify_text(
            METADATA_CONTEXT,
            &self.unsigned_bytes()?,
            &self.author,
            self.signature.as_deref(),
        )
    }

    /// Validate non-cryptographic metadata fields.
    ///
    /// # Errors
    ///
    /// Returns an error if a required field or author identity is invalid.
    pub fn validate(&self) -> Result<()> {
        if self.key.trim().is_empty() {
            return Err(SyncwebError::InvalidConfig(
                "Web-of-Trust metadata key cannot be empty".to_owned(),
            ));
        }
        if self.sequence == 0 {
            return Err(SyncwebError::InvalidConfig(
                "Web-of-Trust metadata sequence must be greater than zero".to_owned(),
            ));
        }
        let _author = parse_author(&self.author)?;
        Ok(())
    }

    /// Encode the entry as JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the entry is invalid or cannot be serialized.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self)
            .map_err(|error| SyncwebError::operation("failed to serialize Web-of-Trust metadata", error))
    }

    /// Decode an entry from JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the entry is malformed.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let entry: Self = serde_json::from_slice(bytes.as_ref())
            .map_err(|error| SyncwebError::operation("failed to deserialize Web-of-Trust metadata", error))?;
        entry.validate()?;
        Ok(entry)
    }

    fn unsigned_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        signed_bytes(METADATA_CONTEXT, &unsigned)
    }
}

/// A signed delegation from one trusted author to another.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TrustDelegation {
    pub delegator: String,
    pub delegate: String,
    pub scope: Option<Hash>,
    pub sequence: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub signature: Option<String>,
}

impl TrustDelegation {
    /// Create and sign a delegation.
    ///
    /// `scope` restricts the delegate to one content hash; `None` delegates
    /// generally.
    ///
    /// # Errors
    ///
    /// Returns an error if identities or timestamps are invalid.
    pub fn new(
        delegate: &VerifyingKey,
        scope: Option<Hash>,
        sequence: u64,
        expires_at: u64,
        signing_key: &SigningKey,
    ) -> Result<Self> {
        Self::new_with_time(
            delegate,
            scope,
            sequence,
            current_epoch_seconds(),
            expires_at,
            signing_key,
        )
    }

    /// Create and sign a delegation with an explicit issue time.
    ///
    /// # Errors
    ///
    /// Returns an error if identities or timestamps are invalid.
    pub fn new_with_time(
        delegate: &VerifyingKey,
        scope: Option<Hash>,
        sequence: u64,
        issued_at: u64,
        expires_at: u64,
        signing_key: &SigningKey,
    ) -> Result<Self> {
        let mut delegation = Self {
            delegator: author_id(&signing_key.verifying_key()),
            delegate: author_id(delegate),
            scope,
            sequence,
            issued_at,
            expires_at,
            signature: None,
        };
        delegation.sign(signing_key)?;
        Ok(delegation)
    }

    /// Sign the delegation with its declared delegator.
    ///
    /// # Errors
    ///
    /// Returns an error if the signing key does not match the delegator.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        ensure_signer(&self.delegator, &signing_key.verifying_key())?;
        self.signature = Some(sign_text(DELEGATION_CONTEXT, &self.unsigned_bytes()?, signing_key));
        Ok(())
    }

    /// Verify the delegation signature and structure.
    ///
    /// # Errors
    ///
    /// Returns an error if the delegation or signature is invalid.
    pub fn verify_signature(&self) -> Result<()> {
        self.validate()?;
        verify_text(
            DELEGATION_CONTEXT,
            &self.unsigned_bytes()?,
            &self.delegator,
            self.signature.as_deref(),
        )
    }

    /// Verify the delegation and require it to be active at `now`.
    ///
    /// # Errors
    ///
    /// Returns an error if the signature is invalid or the delegation expired.
    pub fn verify_at(&self, now: u64) -> Result<()> {
        self.verify_signature()?;
        if self.expires_at <= now {
            return Err(SyncwebError::InvalidConfig("trust delegation has expired".to_owned()));
        }
        Ok(())
    }

    /// Validate identities, sequence, and timestamps.
    ///
    /// # Errors
    ///
    /// Returns an error if a delegation field is invalid.
    pub fn validate(&self) -> Result<()> {
        if self.sequence == 0 {
            return Err(SyncwebError::InvalidConfig(
                "trust delegation sequence must be greater than zero".to_owned(),
            ));
        }
        if self.expires_at <= self.issued_at {
            return Err(SyncwebError::InvalidConfig(
                "trust delegation expiration must be after its issue time".to_owned(),
            ));
        }
        if self.delegator == self.delegate {
            return Err(SyncwebError::InvalidConfig(
                "trust delegation cannot delegate to itself".to_owned(),
            ));
        }
        let _delegator = parse_author(&self.delegator)?;
        let _delegate = parse_author(&self.delegate)?;
        Ok(())
    }

    fn unsigned_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        signed_bytes(DELEGATION_CONTEXT, &unsigned)
    }
}

/// The result of evaluating a local trust policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TrustDecision {
    TrustedRoot,
    TrustedDelegation,
    Untrusted,
    Revoked,
}

impl TrustDecision {
    #[must_use]
    pub const fn is_trusted(self) -> bool {
        matches!(self, Self::TrustedRoot | Self::TrustedDelegation)
    }
}

/// Local trust roots and cryptographically signed delegations.
#[derive(Clone, Debug, Default)]
pub struct TrustPolicy {
    roots: HashSet<String>,
    delegations: HashMap<String, Vec<TrustDelegation>>,
    revoked_authors: HashSet<String>,
}

impl TrustPolicy {
    /// Create an empty local policy.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a policy with one trusted root.
    #[must_use]
    pub fn with_root(signing_key: &SigningKey) -> Self {
        let mut policy = Self::new();
        policy.trust_author(&signing_key.verifying_key());
        policy
    }

    /// Trust an author directly as a local root.
    pub fn trust_author(&mut self, author: &VerifyingKey) {
        self.roots.insert(author_id(author));
        self.revoked_authors.remove(&author_id(author));
    }

    /// Revoke a local trust root or delegate identity.
    pub fn revoke_author(&mut self, author: &VerifyingKey) {
        self.revoked_authors.insert(author_id(author));
    }

    /// Add a signed delegation issued by an already trusted author.
    ///
    /// # Errors
    ///
    /// Returns an error if the delegation is invalid, expired, or not issued
    /// by an identity trusted for its scope.
    pub fn add_delegation(&mut self, delegation: TrustDelegation) -> Result<bool> {
        self.add_delegation_at(delegation, current_epoch_seconds())
    }

    /// Add a signed delegation using an explicit validation time.
    ///
    /// # Errors
    ///
    /// Returns an error if the delegation is invalid, expired, or not issued
    /// by an identity trusted for its scope.
    pub fn add_delegation_at(&mut self, delegation: TrustDelegation, now: u64) -> Result<bool> {
        delegation.verify_at(now)?;
        let scope = delegation.scope;
        if !self.is_trusted_for_at(&delegation.delegator, scope.as_ref(), now) {
            return Err(SyncwebError::InvalidIdentity(
                "trust delegation issuer is not trusted for its scope".to_owned(),
            ));
        }
        let entries = self.delegations.entry(delegation.delegator.clone()).or_default();
        if entries.iter().any(|existing| {
            existing.delegate == delegation.delegate
                && existing.scope == delegation.scope
                && existing.sequence >= delegation.sequence
        }) {
            return Ok(false);
        }
        entries.retain(|existing| {
            !(existing.delegate == delegation.delegate
                && existing.scope == delegation.scope
                && existing.sequence < delegation.sequence)
        });
        entries.push(delegation);
        Ok(true)
    }

    /// Return the local trust decision for an author.
    #[must_use]
    pub fn evaluate(&self, author: &str) -> TrustDecision {
        self.evaluate_at(author, current_epoch_seconds())
    }

    /// Return the local trust decision at an explicit time.
    #[must_use]
    pub fn evaluate_at(&self, author: &str, now: u64) -> TrustDecision {
        self.evaluate_for_at(author, None, now)
    }

    /// Return the local trust decision for an author and optional content.
    #[must_use]
    pub fn evaluate_for(&self, author: &str, content: Option<&Hash>) -> TrustDecision {
        self.evaluate_for_at(author, content, current_epoch_seconds())
    }

    /// Return the local trust decision for an author and content at an
    /// explicit time.
    #[must_use]
    pub fn evaluate_for_at(&self, author: &str, content: Option<&Hash>, now: u64) -> TrustDecision {
        if self.revoked_authors.contains(author) {
            return TrustDecision::Revoked;
        }
        if self.roots.contains(author) {
            return TrustDecision::TrustedRoot;
        }
        if self.reaches_trusted_root(author, content, now, &mut HashSet::new()) {
            TrustDecision::TrustedDelegation
        } else {
            TrustDecision::Untrusted
        }
    }

    /// Return whether an author is trusted for the requested content.
    #[must_use]
    pub fn is_trusted_for(&self, author: &str, content: Option<&Hash>) -> bool {
        self.is_trusted_for_at(author, content, current_epoch_seconds())
    }

    /// Return whether an author is trusted at an explicit time.
    #[must_use]
    pub fn is_trusted_for_at(&self, author: &str, content: Option<&Hash>, now: u64) -> bool {
        self.evaluate_for_at(author, content, now).is_trusted()
    }

    fn reaches_trusted_root(
        &self,
        target: &str,
        content: Option<&Hash>,
        now: u64,
        visited: &mut HashSet<String>,
    ) -> bool {
        if self.revoked_authors.contains(target) {
            return false;
        }
        if self.roots.contains(target) {
            return true;
        }
        if !visited.insert(target.to_owned()) {
            return false;
        }
        self.delegations.iter().any(|(delegator, delegations)| {
            delegations.iter().any(|delegation| {
                if delegation.delegate != target
                    || delegation.expires_at <= now
                    || (delegation.scope.is_some() && delegation.scope.as_ref() != content)
                {
                    return false;
                }
                let mut branch_visited = visited.clone();
                self.reaches_trusted_root(delegator, content, now, &mut branch_visited)
            })
        })
    }
}

/// A publisher-signed self-revocation for a content hash.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RevocationRecord {
    pub content: Hash,
    pub publisher: String,
    pub sequence: u64,
    pub revoked_at: u64,
    pub reason: String,
    pub signature: Option<String>,
}

impl RevocationRecord {
    /// Create and sign a self-revocation.
    ///
    /// # Errors
    ///
    /// Returns an error if the sequence or reason is invalid.
    pub fn new(content: Hash, sequence: u64, reason: impl Into<String>, signing_key: &SigningKey) -> Result<Self> {
        let mut record = Self {
            content,
            publisher: author_id(&signing_key.verifying_key()),
            sequence,
            revoked_at: current_epoch_seconds(),
            reason: reason.into(),
            signature: None,
        };
        record.sign(signing_key)?;
        Ok(record)
    }

    /// Sign the revocation with the declared publisher.
    ///
    /// # Errors
    ///
    /// Returns an error if the signing key does not match the publisher.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        ensure_signer(&self.publisher, &signing_key.verifying_key())?;
        self.signature = Some(sign_text(REVOCATION_CONTEXT, &self.unsigned_bytes()?, signing_key));
        Ok(())
    }

    /// Verify the publisher signature.
    ///
    /// # Errors
    ///
    /// Returns an error if the record or signature is invalid.
    pub fn verify_signature(&self) -> Result<()> {
        self.validate()?;
        verify_text(
            REVOCATION_CONTEXT,
            &self.unsigned_bytes()?,
            &self.publisher,
            self.signature.as_deref(),
        )
    }

    /// Validate revocation fields.
    ///
    /// # Errors
    ///
    /// Returns an error if the sequence, reason, or publisher is invalid.
    pub fn validate(&self) -> Result<()> {
        if self.sequence == 0 {
            return Err(SyncwebError::InvalidConfig(
                "revocation sequence must be greater than zero".to_owned(),
            ));
        }
        if self.reason.trim().is_empty() {
            return Err(SyncwebError::InvalidConfig(
                "revocation reason cannot be empty".to_owned(),
            ));
        }
        let _publisher = parse_author(&self.publisher)?;
        Ok(())
    }

    fn unsigned_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        signed_bytes(REVOCATION_CONTEXT, &unsigned)
    }
}

/// Moderation outcomes that can be attached to a content hash.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ModerationAction {
    Show,
    Hide,
    Warn,
    Quarantine,
    Restore,
}

/// The evaluated moderation decision.
pub type ModerationDecision = ModerationAction;

impl ModerationAction {
    #[must_use]
    pub const fn hides_content(&self) -> bool {
        matches!(self, Self::Hide | Self::Quarantine)
    }

    #[must_use]
    pub const fn is_visible(self) -> bool {
        !self.hides_content()
    }
}

/// The area in which a moderation decision applies.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ModerationScope {
    #[default]
    Global,
    Network(String),
    Folder(NamespaceId),
    File {
        namespace_id: NamespaceId,
        key: Vec<u8>,
    },
}

impl ModerationScope {
    #[must_use]
    pub fn applies_to(&self, context: &ModerationContext) -> bool {
        match self {
            Self::Global => true,
            Self::Network(network) => context.network.as_deref() == Some(network),
            Self::Folder(namespace_id) => context.namespace_id == Some(*namespace_id),
            Self::File { namespace_id, key } => {
                context.namespace_id == Some(*namespace_id) && context.key.as_deref() == Some(key.as_slice())
            }
        }
    }

    #[must_use]
    pub const fn specificity(&self) -> u8 {
        match self {
            Self::Global => 0,
            Self::Network(_) => 1,
            Self::Folder(_) => 2,
            Self::File { .. } => 3,
        }
    }
}

/// Context used to evaluate scoped moderation records.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ModerationContext {
    pub content: Hash,
    pub network: Option<String>,
    pub namespace_id: Option<NamespaceId>,
    pub key: Option<Vec<u8>>,
}

impl ModerationContext {
    #[must_use]
    pub const fn new(content: Hash) -> Self {
        Self {
            content,
            network: None,
            namespace_id: None,
            key: None,
        }
    }

    #[must_use]
    pub fn in_network(mut self, network: impl Into<String>) -> Self {
        self.network = Some(network.into());
        self
    }

    #[must_use]
    pub fn for_file(mut self, namespace_id: NamespaceId, key: impl AsRef<[u8]>) -> Self {
        self.namespace_id = Some(namespace_id);
        self.key = Some(key.as_ref().to_vec());
        self
    }
}

/// A signed moderation decision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ModerationRecord {
    pub content: Hash,
    pub moderator: String,
    pub action: ModerationAction,
    #[serde(default)]
    pub scope: ModerationScope,
    pub sequence: u64,
    pub created_at: u64,
    pub reason: String,
    pub signature: Option<String>,
}

impl ModerationRecord {
    /// Create and sign a moderation record.
    ///
    /// # Errors
    ///
    /// Returns an error if moderation fields are invalid.
    pub fn new(
        content: Hash,
        action: ModerationAction,
        sequence: u64,
        reason: impl Into<String>,
        signing_key: &SigningKey,
    ) -> Result<Self> {
        let mut record = Self {
            content,
            moderator: author_id(&signing_key.verifying_key()),
            action,
            scope: ModerationScope::Global,
            sequence,
            created_at: current_epoch_seconds(),
            reason: reason.into(),
            signature: None,
        };
        record.sign(signing_key)?;
        Ok(record)
    }

    /// Create and sign a global moderation record with an explicit timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if moderation fields are invalid or signing fails.
    pub fn new_with_time(
        content: Hash,
        action: ModerationAction,
        sequence: u64,
        created_at: u64,
        reason: impl Into<String>,
        signing_key: &SigningKey,
    ) -> Result<Self> {
        Self::new_scoped_with_time(
            content,
            ModerationScope::Global,
            action,
            sequence,
            created_at,
            reason,
            signing_key,
        )
    }

    /// Create and sign a moderation record for a specific scope.
    ///
    /// # Errors
    ///
    /// Returns an error if moderation fields are invalid.
    pub fn new_scoped(
        content: Hash,
        scope: ModerationScope,
        action: ModerationAction,
        sequence: u64,
        reason: impl Into<String>,
        signing_key: &SigningKey,
    ) -> Result<Self> {
        Self::new_scoped_with_time(
            content,
            scope,
            action,
            sequence,
            current_epoch_seconds(),
            reason,
            signing_key,
        )
    }

    /// Create and sign a scoped moderation record with an explicit timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if moderation fields are invalid.
    pub fn new_scoped_with_time(
        content: Hash,
        scope: ModerationScope,
        action: ModerationAction,
        sequence: u64,
        created_at: u64,
        reason: impl Into<String>,
        signing_key: &SigningKey,
    ) -> Result<Self> {
        let mut record = Self {
            content,
            moderator: author_id(&signing_key.verifying_key()),
            action,
            scope,
            sequence,
            created_at,
            reason: reason.into(),
            signature: None,
        };
        record.sign(signing_key)?;
        Ok(record)
    }

    /// Return a copy of this record with a different scope, re-signing it.
    ///
    /// # Errors
    ///
    /// Returns an error if the record cannot be signed.
    pub fn with_scope(mut self, scope: ModerationScope, signing_key: &SigningKey) -> Result<Self> {
        self.scope = scope;
        self.sign(signing_key)?;
        Ok(self)
    }

    /// Sign the moderation record.
    ///
    /// # Errors
    ///
    /// Returns an error if the signing key does not match the moderator.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        ensure_signer(&self.moderator, &signing_key.verifying_key())?;
        self.signature = Some(sign_text(MODERATION_CONTEXT, &self.unsigned_bytes()?, signing_key));
        Ok(())
    }

    /// Verify the moderation signature.
    ///
    /// # Errors
    ///
    /// Returns an error if the record or signature is invalid.
    pub fn verify_signature(&self) -> Result<()> {
        self.validate()?;
        verify_text(
            MODERATION_CONTEXT,
            &self.unsigned_bytes()?,
            &self.moderator,
            self.signature.as_deref(),
        )
    }

    /// Validate moderation fields.
    ///
    /// # Errors
    ///
    /// Returns an error if a field is invalid.
    pub fn validate(&self) -> Result<()> {
        if self.sequence == 0 {
            return Err(SyncwebError::InvalidConfig(
                "moderation sequence must be greater than zero".to_owned(),
            ));
        }
        if self.reason.trim().is_empty() {
            return Err(SyncwebError::InvalidConfig(
                "moderation reason cannot be empty".to_owned(),
            ));
        }
        if let ModerationScope::Network(network) = &self.scope
            && network.trim().is_empty()
        {
            return Err(SyncwebError::InvalidConfig(
                "moderation network scope cannot be empty".to_owned(),
            ));
        }
        if let ModerationScope::File { key, .. } = &self.scope
            && key.is_empty()
        {
            return Err(SyncwebError::InvalidConfig(
                "moderation file scope cannot be empty".to_owned(),
            ));
        }
        let _moderator = parse_author(&self.moderator)?;
        Ok(())
    }

    fn unsigned_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        signed_bytes(MODERATION_CONTEXT, &unsigned)
    }
}

/// Types of signed provenance metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AttestationKind {
    License,
    Provenance,
    Derivative,
    Other(String),
}

/// A signed license, provenance, or derivative attestation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Attestation {
    pub content: Hash,
    pub issuer: String,
    pub kind: AttestationKind,
    pub value: String,
    pub sequence: u64,
    pub issued_at: u64,
    pub signature: Option<String>,
}

impl Attestation {
    /// Create and sign an attestation.
    ///
    /// # Errors
    ///
    /// Returns an error if attestation fields are invalid.
    pub fn new(
        content: Hash,
        kind: AttestationKind,
        value: impl Into<String>,
        sequence: u64,
        signing_key: &SigningKey,
    ) -> Result<Self> {
        let mut attestation = Self {
            content,
            issuer: author_id(&signing_key.verifying_key()),
            kind,
            value: value.into(),
            sequence,
            issued_at: current_epoch_seconds(),
            signature: None,
        };
        attestation.sign(signing_key)?;
        Ok(attestation)
    }

    /// Sign the attestation with the declared issuer.
    ///
    /// # Errors
    ///
    /// Returns an error if the signing key does not match the issuer.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        ensure_signer(&self.issuer, &signing_key.verifying_key())?;
        self.signature = Some(sign_text(ATTESTATION_CONTEXT, &self.unsigned_bytes()?, signing_key));
        Ok(())
    }

    /// Verify the attestation signature.
    ///
    /// # Errors
    ///
    /// Returns an error if the attestation or signature is invalid.
    pub fn verify_signature(&self) -> Result<()> {
        self.validate()?;
        verify_text(
            ATTESTATION_CONTEXT,
            &self.unsigned_bytes()?,
            &self.issuer,
            self.signature.as_deref(),
        )
    }

    /// Validate attestation fields.
    ///
    /// # Errors
    ///
    /// Returns an error if a field is invalid.
    pub fn validate(&self) -> Result<()> {
        if self.sequence == 0 {
            return Err(SyncwebError::InvalidConfig(
                "attestation sequence must be greater than zero".to_owned(),
            ));
        }
        if self.value.trim().is_empty() {
            return Err(SyncwebError::InvalidConfig(
                "attestation value cannot be empty".to_owned(),
            ));
        }
        let _issuer = parse_author(&self.issuer)?;
        Ok(())
    }

    fn unsigned_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        signed_bytes(ATTESTATION_CONTEXT, &unsigned)
    }
}

#[derive(Clone, Debug, Default)]
struct WotState {
    revocations: HashMap<Hash, RevocationRecord>,
    moderation: HashMap<Hash, Vec<ModerationRecord>>,
    attestations: HashMap<(Hash, String, u64), Attestation>,
}

/// Indexing-layer service for trusted metadata and local governance records.
#[derive(Clone)]
pub struct WotService {
    database: IndexingDatabase,
    policy: Arc<Mutex<TrustPolicy>>,
    state: Arc<Mutex<WotState>>,
}

impl std::fmt::Debug for WotService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("WotService").finish_non_exhaustive()
    }
}

impl WotService {
    /// Create a Web-of-Trust service using an existing indexing service.
    #[must_use]
    pub fn new(indexing: &IndexingService, policy: TrustPolicy) -> Self {
        Self::with_database(indexing.database().clone(), policy)
    }

    /// Create a service backed by an indexing database.
    #[must_use]
    pub fn with_database(database: IndexingDatabase, policy: TrustPolicy) -> Self {
        Self {
            database,
            policy: Arc::new(Mutex::new(policy)),
            state: Arc::new(Mutex::new(WotState::default())),
        }
    }

    /// Create an isolated in-memory Web-of-Trust service.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` cannot initialize.
    pub fn in_memory(policy: TrustPolicy) -> Result<Self> {
        Ok(Self::with_database(IndexingDatabase::in_memory()?, policy))
    }

    #[must_use]
    pub const fn database(&self) -> &IndexingDatabase {
        &self.database
    }

    /// Return a copy of the current local trust policy.
    ///
    /// # Errors
    ///
    /// Returns an error if policy state is poisoned.
    pub fn policy(&self) -> Result<TrustPolicy> {
        Ok(self
            .policy
            .lock()
            .map_err(|error| SyncwebError::operation("Web-of-Trust policy lock poisoned", error))?
            .clone())
    }

    /// Add a local trusted root.
    ///
    /// # Errors
    ///
    /// Returns an error if policy state is poisoned.
    pub fn trust_author(&self, author: &VerifyingKey) -> Result<()> {
        self.policy
            .lock()
            .map_err(|error| SyncwebError::operation("Web-of-Trust policy lock poisoned", error))?
            .trust_author(author);
        Ok(())
    }

    /// Add a signed delegation to the local policy.
    ///
    /// # Errors
    ///
    /// Returns an error if the delegation is invalid or its issuer is not
    /// locally trusted.
    pub fn add_delegation(&self, delegation: TrustDelegation) -> Result<bool> {
        self.add_delegation_at(delegation, current_epoch_seconds())
    }

    /// Add a signed delegation using an explicit validation time.
    ///
    /// # Errors
    ///
    /// Returns an error if the delegation is invalid or its issuer is not
    /// locally trusted.
    pub fn add_delegation_at(&self, delegation: TrustDelegation, now: u64) -> Result<bool> {
        self.policy
            .lock()
            .map_err(|error| SyncwebError::operation("Web-of-Trust policy lock poisoned", error))?
            .add_delegation_at(delegation, now)
    }

    /// Append metadata after signature and local trust checks.
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata is not signed, trusted, or the content
    /// has been self-revoked.
    pub fn append_metadata(&self, entry: &MetadataEntry) -> Result<bool> {
        entry.verify_signature()?;
        let now = current_epoch_seconds();
        let policy = self
            .policy
            .lock()
            .map_err(|error| SyncwebError::operation("Web-of-Trust policy lock poisoned", error))?;
        if !policy.is_trusted_for_at(&entry.author, Some(&entry.content), now) {
            return Err(SyncwebError::InvalidIdentity(
                "metadata author is not trusted for this content".to_owned(),
            ));
        }
        drop(policy);
        if self.is_revoked(&entry.content)? {
            return Err(SyncwebError::InvalidConfig(
                "metadata cannot be appended to self-revoked content".to_owned(),
            ));
        }
        self.database.append_wot_metadata(entry)
    }

    /// Search trusted metadata while applying local revocation and moderation.
    ///
    /// # Errors
    ///
    /// Returns an error if the query or index is invalid.
    pub fn search_metadata(&self, query: &str, limit: usize) -> Result<Vec<MetadataEntry>> {
        let records = self.database.search_wot_metadata(query, limit)?;
        let state = self
            .state
            .lock()
            .map_err(|error| SyncwebError::operation("Web-of-Trust state lock poisoned", error))?;
        Ok(records
            .into_iter()
            .filter(|entry| !state.revocations.contains_key(&entry.content))
            .filter(|entry| {
                state.moderation.get(&entry.content).is_none_or(|moderation_records| {
                    moderation_records
                        .iter()
                        .filter(|record| matches!(&record.scope, ModerationScope::Global))
                        .max_by_key(|record| record.sequence)
                        .is_none_or(|record| !record.action.hides_content())
                })
            })
            .collect())
    }

    /// Alias for [`Self::search_metadata`].
    ///
    /// # Errors
    ///
    /// Returns an error if the query or index is invalid.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<MetadataEntry>> {
        self.search_metadata(query, limit)
    }

    /// Accept a publisher's self-revocation.
    ///
    /// # Errors
    ///
    /// Returns an error if the signature is invalid or an older sequence was
    /// already recorded.
    pub fn revoke_self(&self, record: RevocationRecord) -> Result<bool> {
        record.verify_signature()?;
        let mut state = self
            .state
            .lock()
            .map_err(|error| SyncwebError::operation("Web-of-Trust state lock poisoned", error))?;
        if state
            .revocations
            .get(&record.content)
            .is_some_and(|existing| existing.sequence >= record.sequence)
        {
            return Ok(false);
        }
        state.revocations.insert(record.content, record);
        drop(state);
        Ok(true)
    }

    /// Return whether a content hash has a valid self-revocation.
    ///
    /// # Errors
    ///
    /// Returns an error if state is poisoned.
    pub fn is_revoked(&self, content: &Hash) -> Result<bool> {
        Ok(self
            .state
            .lock()
            .map_err(|error| SyncwebError::operation("Web-of-Trust state lock poisoned", error))?
            .revocations
            .contains_key(content))
    }

    /// Accept a trusted moderation record.
    ///
    /// # Errors
    ///
    /// Returns an error if the signature is invalid, the moderator is not
    /// trusted, or an older sequence was already recorded.
    pub fn apply_moderation(&self, record: ModerationRecord) -> Result<bool> {
        record.verify_signature()?;
        let now = current_epoch_seconds();
        if !self
            .policy()?
            .is_trusted_for_at(&record.moderator, Some(&record.content), now)
        {
            return Err(SyncwebError::InvalidIdentity(
                "moderator is not trusted for this content".to_owned(),
            ));
        }
        let mut state = self
            .state
            .lock()
            .map_err(|error| SyncwebError::operation("Web-of-Trust state lock poisoned", error))?;
        let records = state.moderation.entry(record.content).or_default();
        if records
            .iter()
            .filter(|existing| existing.scope == record.scope)
            .any(|existing| existing.sequence >= record.sequence)
        {
            return Ok(false);
        }
        records.push(record);
        drop(state);
        Ok(true)
    }

    /// Return the latest moderation record for a content hash.
    ///
    /// # Errors
    ///
    /// Returns an error if state is poisoned.
    pub fn moderation(&self, content: &Hash) -> Result<Option<ModerationRecord>> {
        Ok(self
            .state
            .lock()
            .map_err(|error| SyncwebError::operation("Web-of-Trust state lock poisoned", error))?
            .moderation
            .get(content)
            .and_then(|records| {
                records
                    .iter()
                    .max_by_key(|record| (record.sequence, record.scope.specificity()))
                    .cloned()
            }))
    }

    /// Return all moderation records for a content hash, newest first.
    ///
    /// # Errors
    ///
    /// Returns an error if state is poisoned.
    pub fn moderation_records(&self, content: &Hash) -> Result<Vec<ModerationRecord>> {
        let mut records = self
            .state
            .lock()
            .map_err(|error| SyncwebError::operation("Web-of-Trust state lock poisoned", error))?
            .moderation
            .get(content)
            .cloned()
            .unwrap_or_default();
        records.sort_by(|left, right| {
            right
                .sequence
                .cmp(&left.sequence)
                .then_with(|| right.scope.specificity().cmp(&left.scope.specificity()))
        });
        Ok(records)
    }

    /// List all moderation records, optionally restricted to one content hash.
    ///
    /// # Errors
    ///
    /// Returns an error if state is poisoned.
    pub fn list_moderation(&self, content: Option<&Hash>) -> Result<Vec<ModerationRecord>> {
        let state = self
            .state
            .lock()
            .map_err(|error| SyncwebError::operation("Web-of-Trust state lock poisoned", error))?;
        let mut records = state
            .moderation
            .iter()
            .filter(|(hash, _)| content.is_none_or(|requested| requested == *hash))
            .flat_map(|(_, records)| records.iter().cloned())
            .collect::<Vec<_>>();
        drop(state);
        records.sort_by(|left, right| {
            right
                .sequence
                .cmp(&left.sequence)
                .then_with(|| right.scope.specificity().cmp(&left.scope.specificity()))
        });
        Ok(records)
    }

    /// Evaluate the latest applicable moderation decision for a context.
    ///
    /// More recent decisions win; when sequence numbers tie, the more
    /// specific scope wins.
    ///
    /// # Errors
    ///
    /// Returns an error if state is poisoned.
    pub fn moderation_decision(&self, context: &ModerationContext) -> Result<ModerationAction> {
        let selected_record = self
            .state
            .lock()
            .map_err(|error| SyncwebError::operation("Web-of-Trust state lock poisoned", error))?
            .moderation
            .get(&context.content)
            .and_then(|records| {
                records
                    .iter()
                    .filter(|record| record.scope.applies_to(context))
                    .max_by_key(|record| (record.sequence, record.scope.specificity()))
                    .cloned()
            });
        Ok(selected_record.map_or(ModerationAction::Show, |selected| selected.action))
    }

    /// Alias for [`Self::moderation_decision`].
    ///
    /// # Errors
    ///
    /// Returns an error if state is poisoned.
    pub fn evaluate_moderation(&self, context: &ModerationContext) -> Result<ModerationAction> {
        self.moderation_decision(context)
    }

    /// Store an attestation after cryptographic and local trust checks.
    ///
    /// # Errors
    ///
    /// Returns an error if the attestation is invalid or its issuer is not
    /// trusted.
    pub fn append_attestation(&self, attestation: Attestation) -> Result<bool> {
        attestation.verify_signature()?;
        let now = current_epoch_seconds();
        if !self
            .policy()?
            .is_trusted_for_at(&attestation.issuer, Some(&attestation.content), now)
        {
            return Err(SyncwebError::InvalidIdentity(
                "attestation issuer is not trusted for this content".to_owned(),
            ));
        }
        let mut state = self
            .state
            .lock()
            .map_err(|error| SyncwebError::operation("Web-of-Trust state lock poisoned", error))?;
        let key = (attestation.content, attestation.issuer.clone(), attestation.sequence);
        if state.attestations.contains_key(&key) {
            return Ok(false);
        }
        state.attestations.insert(key, attestation);
        drop(state);
        Ok(true)
    }

    /// Verify an attestation's cryptographic signature without applying local
    /// trust policy.
    ///
    /// # Errors
    ///
    /// Returns an error if the attestation is malformed or tampered with.
    pub fn verify_attestation(&self, attestation: &Attestation) -> Result<()> {
        attestation.verify_signature()
    }
}

fn signed_bytes<T: Serialize>(context: &[u8], value: &T) -> Result<Vec<u8>> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| SyncwebError::operation("failed to serialize signed Web-of-Trust record", error))?;
    let mut bytes = Vec::with_capacity(context.len().saturating_add(encoded.len()));
    bytes.extend_from_slice(context);
    bytes.extend_from_slice(&encoded);
    Ok(bytes)
}

fn sign_text(context: &[u8], unsigned: &[u8], signing_key: &SigningKey) -> String {
    let mut bytes = Vec::with_capacity(context.len().saturating_add(unsigned.len()));
    bytes.extend_from_slice(context);
    bytes.extend_from_slice(unsigned);
    hex::encode(signing_key.sign(&bytes).to_bytes())
}

fn verify_text(context: &[u8], unsigned: &[u8], author: &str, signature: Option<&str>) -> Result<()> {
    let signature_text = signature
        .ok_or_else(|| SyncwebError::InvalidConfig("signed Web-of-Trust record has no signature".to_owned()))?;
    let signature_bytes = hex::decode(signature_text)
        .map_err(|error| SyncwebError::InvalidConfig(format!("invalid Web-of-Trust signature encoding: {error}")))?;
    let parsed_signature = Signature::from_slice(&signature_bytes)
        .map_err(|error| SyncwebError::InvalidConfig(format!("invalid Web-of-Trust signature: {error}")))?;
    let verifying_key = parse_author(author)?;
    let mut bytes = Vec::with_capacity(context.len().saturating_add(unsigned.len()));
    bytes.extend_from_slice(context);
    bytes.extend_from_slice(unsigned);
    verifying_key
        .verify(&bytes, &parsed_signature)
        .map_err(|error| SyncwebError::InvalidConfig(format!("Web-of-Trust signature is invalid: {error}")))
}

fn ensure_signer(author: &str, signing_key: &VerifyingKey) -> Result<()> {
    if author == author_id(signing_key) {
        Ok(())
    } else {
        Err(SyncwebError::InvalidIdentity(
            "Web-of-Trust signer does not match the declared author".to_owned(),
        ))
    }
}

fn author_id(author: &VerifyingKey) -> String {
    hex::encode(author.to_bytes())
}

fn parse_author(author: &str) -> Result<VerifyingKey> {
    let bytes = hex::decode(author)
        .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid Web-of-Trust author encoding: {error}")))?;
    let key_bytes: [u8; 32] = bytes.try_into().map_err(|error: Vec<u8>| {
        SyncwebError::InvalidIdentity(format!("Web-of-Trust author must be 32 bytes, got {}", error.len()))
    })?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid Web-of-Trust author key: {error}")))
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
