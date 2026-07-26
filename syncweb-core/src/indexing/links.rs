//! Stable content links, signed mutable names, capabilities, and mirrors.
//!
//! Links are deliberately independent of the document and blob stores.  A
//! caller can persist the records in either store, or use [`LinkResolver`] as
//! a small in-memory resolver for local indexing and tests.  Immutable links
//! identify a hash directly; mutable links identify a signed, monotonically
//! advancing pointer.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    future::Future,
    pin::Pin,
    str::{FromStr, Split},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use iroh::{PublicKey, SecretKey};
use iroh_blobs::{Hash, ticket::BlobTicket};
use serde::{Deserialize, Serialize};

use super::IndexingDatabase;
use crate::{
    error::{Result, SyncwebError},
    gossip::SignedGossipMessage,
    indexing::ProviderLease,
};

const LINK_SIGNATURE_CONTEXT: &[u8] = b"syncweb/name-pointer/v1\0";
const LINK_SCHEME: &str = "syncweb://";

/// Return the current Unix epoch in seconds.
#[must_use]
pub fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

/// An immutable content-addressed link.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ContentLink {
    /// The content or manifest hash identified by this link.
    pub hash: Hash,
}

impl ContentLink {
    /// Create an immutable content link.
    #[must_use]
    pub const fn new(hash: Hash) -> Self {
        Self { hash }
    }

    /// Return the content hash.
    #[must_use]
    pub const fn hash(self) -> Hash {
        self.hash
    }

    /// Alias for [`Self::new`].
    #[must_use]
    pub const fn create(hash: Hash) -> Self {
        Self::new(hash)
    }

    /// Parse an immutable content URI.
    ///
    /// # Errors
    ///
    /// Returns an error if the URI or hash is malformed.
    pub fn parse(value: &str) -> Result<Self> {
        value.parse()
    }

    /// Return the canonical URI.
    #[must_use]
    pub fn uri(self) -> String {
        self.to_string()
    }

    /// Validate this link.
    ///
    /// # Errors
    ///
    /// This currently cannot fail, but is provided so all link kinds have a
    /// common validation API.
    pub const fn validate(self) -> Result<()> {
        Ok(())
    }
}

impl fmt::Display for ContentLink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{LINK_SCHEME}content/{}", self.hash)
    }
}

impl FromStr for ContentLink {
    type Err = SyncwebError;

    fn from_str(value: &str) -> Result<Self> {
        let hash_text = value
            .strip_prefix("syncweb://content/")
            .ok_or_else(|| SyncwebError::InvalidConfig("content link must use syncweb://content/".to_owned()))?;
        if hash_text.is_empty() || hash_text.contains('/') || hash_text.contains('?') || hash_text.contains('#') {
            return Err(SyncwebError::InvalidConfig(
                "content link has an invalid hash".to_owned(),
            ));
        }
        let hash = hash_text
            .parse::<Hash>()
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid content link hash: {error}")))?;
        Ok(Self::new(hash))
    }
}

/// Alias used by callers that prefer the term immutable link.
pub type ImmutableLink = ContentLink;

/// A stable publisher/alias portion of a mutable link.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NameLink {
    /// The publisher whose signed pointer is followed.
    pub publisher: PublicKey,
    /// The publisher-controlled alias.
    pub alias: String,
}

impl NameLink {
    /// Create a mutable name link.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty publisher alias or an alias that could
    /// change URI path semantics.
    pub fn new(publisher: PublicKey, alias: impl Into<String>) -> Result<Self> {
        let link = Self {
            publisher,
            alias: alias.into(),
        };
        link.validate()?;
        Ok(link)
    }

    /// Alias for [`Self::new`].
    ///
    /// # Errors
    ///
    /// Returns an error if the alias is invalid.
    pub fn create(publisher: PublicKey, alias: impl Into<String>) -> Result<Self> {
        Self::new(publisher, alias)
    }

    /// Parse a mutable name URI.
    ///
    /// # Errors
    ///
    /// Returns an error if the URI or publisher is malformed.
    pub fn parse(value: &str) -> Result<Self> {
        value.parse()
    }

    /// Return the canonical URI.
    #[must_use]
    pub fn uri(&self) -> String {
        self.to_string()
    }

    /// Validate the publisher alias.
    ///
    /// # Errors
    ///
    /// Returns an error if the alias is empty or contains URI delimiters.
    pub fn validate(&self) -> Result<()> {
        validate_path_segment(&self.alias, "mutable link alias")
    }
}

impl fmt::Display for NameLink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{LINK_SCHEME}name/{}/{}", self.publisher, self.alias)
    }
}

impl FromStr for NameLink {
    type Err = SyncwebError;

    fn from_str(value: &str) -> Result<Self> {
        let remainder = value
            .strip_prefix("syncweb://name/")
            .ok_or_else(|| SyncwebError::InvalidConfig("name link must use syncweb://name/".to_owned()))?;
        let mut parts = remainder.split('/');
        let publisher_text = next_part(&mut parts, "name link publisher")?;
        let alias = next_part(&mut parts, "name link alias")?;
        if parts.next().is_some() {
            return Err(SyncwebError::InvalidConfig(
                "name link has too many path segments".to_owned(),
            ));
        }
        let publisher = publisher_text
            .parse::<PublicKey>()
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid name link publisher: {error}")))?;
        Self::new(publisher, alias)
    }
}

/// Alias used by callers that prefer the term mutable link.
pub type MutableLink = NameLink;

/// A signed mutable pointer to an immutable manifest.
///
/// Every accepted pointer has a publisher, alias, sequence, and manifest
/// bound by an Ed25519 signature.  A resolver accepts only a sequence greater
/// than the currently accepted sequence for that publisher and alias.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MutablePointer {
    /// Publisher identity; this must match the signing key.
    pub publisher: PublicKey,
    /// Publisher-controlled alias.
    pub alias: String,
    /// Monotonically increasing update number.
    pub sequence: u64,
    /// Immutable manifest identified by this pointer.
    pub manifest: Hash,
    /// Optional semantic version used for version pinning.
    #[serde(default)]
    pub version: Option<String>,
    /// Signed provider leases for the manifest.
    #[serde(default)]
    pub providers: Vec<ProviderLease>,
    /// Hex-encoded Ed25519 signature over the pointer without this field.
    #[serde(default)]
    pub signature: Option<String>,
}

impl MutablePointer {
    /// Create an unsigned pointer without provider leases.
    ///
    /// Call [`Self::sign`] before registering it with a resolver.
    ///
    /// # Errors
    ///
    /// Returns an error if the alias, sequence, or provider data is invalid.
    pub fn new(publisher: PublicKey, alias: impl Into<String>, manifest: Hash, sequence: u64) -> Result<Self> {
        Self::new_with_providers(publisher, alias, manifest, sequence, Vec::new())
    }

    /// Create an unsigned pointer with provider leases.
    ///
    /// # Errors
    ///
    /// Returns an error if the alias, sequence, or provider data is invalid.
    pub fn new_with_providers(
        publisher: PublicKey,
        alias: impl Into<String>,
        manifest: Hash,
        sequence: u64,
        providers: Vec<ProviderLease>,
    ) -> Result<Self> {
        let pointer = Self {
            publisher,
            alias: alias.into(),
            sequence,
            manifest,
            version: None,
            providers,
            signature: None,
        };
        pointer.validate()?;
        Ok(pointer)
    }

    /// Create and sign a pointer with a Dalek signing key.
    ///
    /// # Errors
    ///
    /// Returns an error if the pointer is invalid or the signer does not
    /// match the publisher.
    pub fn signed(
        publisher: PublicKey,
        alias: impl Into<String>,
        manifest: Hash,
        sequence: u64,
        signing_key: &SigningKey,
    ) -> Result<Self> {
        let mut pointer = Self::new(publisher, alias, manifest, sequence)?;
        pointer.sign(signing_key)?;
        Ok(pointer)
    }

    /// Create and sign a pointer with an iroh secret key.
    ///
    /// # Errors
    ///
    /// Returns an error if the pointer is invalid or the signer does not
    /// match the publisher.
    pub fn signed_with_secret_key(
        publisher: PublicKey,
        alias: impl Into<String>,
        manifest: Hash,
        sequence: u64,
        secret_key: &SecretKey,
    ) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(&secret_key.to_bytes());
        Self::signed(publisher, alias, manifest, sequence, &signing_key)
    }

    /// Set a semantic version on this pointer.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self.signature = None;
        self
    }

    /// Add a provider lease to this pointer.
    #[must_use]
    pub fn with_provider(mut self, provider: ProviderLease) -> Self {
        self.providers.push(provider);
        self.signature = None;
        self
    }

    /// Add several provider leases to this pointer.
    #[must_use]
    pub fn with_providers(mut self, providers: impl IntoIterator<Item = ProviderLease>) -> Self {
        self.providers.extend(providers);
        self.signature = None;
        self
    }

    /// Return the corresponding stable name link.
    ///
    /// # Errors
    ///
    /// Returns an error if the alias is invalid.
    pub fn link(&self) -> Result<NameLink> {
        NameLink::new(self.publisher, self.alias.clone())
    }

    /// Return the immutable manifest hash targeted by this pointer.
    #[must_use]
    pub const fn manifest_hash(&self) -> Hash {
        self.manifest
    }

    /// Serialize the pointer without its signature.
    ///
    /// # Errors
    ///
    /// Returns an error if the pointer is invalid or cannot be serialized.
    pub fn unsigned_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        unsigned.validate()?;
        let mut bytes = LINK_SIGNATURE_CONTEXT.to_vec();
        let encoded = serde_json::to_vec(&unsigned)
            .map_err(|error| SyncwebError::operation("failed to serialize mutable link pointer", error))?;
        bytes.extend_from_slice(&encoded);
        Ok(bytes)
    }

    /// Sign this pointer with an Ed25519 key.
    ///
    /// # Errors
    ///
    /// Returns an error if the signer does not match the publisher or the
    /// pointer cannot be serialized.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        if signing_key.verifying_key().to_bytes() != *self.publisher.as_bytes() {
            return Err(SyncwebError::InvalidIdentity(
                "mutable link signer does not match publisher".to_owned(),
            ));
        }
        self.validate()?;
        self.signature = Some(hex::encode(signing_key.sign(&self.unsigned_bytes()?).to_bytes()));
        Ok(())
    }

    /// Verify the pointer's signature and all provider leases.
    ///
    /// # Errors
    ///
    /// Returns an error if the signature, pointer, or provider lease is
    /// invalid.
    pub fn verify_signature(&self) -> Result<()> {
        self.validate()?;
        let signature_text = self
            .signature
            .as_deref()
            .ok_or_else(|| SyncwebError::InvalidConfig("mutable link pointer must be signed".to_owned()))?;
        let signature_bytes = hex::decode(signature_text)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid mutable link signature: {error}")))?;
        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid mutable link signature: {error}")))?;
        let key = VerifyingKey::from_bytes(self.publisher.as_bytes())
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid mutable link publisher: {error}")))?;
        key.verify(&self.unsigned_bytes()?, &signature)
            .map_err(|error| SyncwebError::InvalidConfig(format!("mutable link signature is invalid: {error}")))?;
        for provider in &self.providers {
            provider.verify_signature()?;
        }
        Ok(())
    }

    /// Validate non-cryptographic pointer fields.
    ///
    /// # Errors
    ///
    /// Returns an error if a pointer field or provider lease is invalid.
    pub fn validate(&self) -> Result<()> {
        validate_path_segment(&self.alias, "mutable link alias")?;
        if self.sequence == 0 {
            return Err(SyncwebError::InvalidConfig(
                "mutable link sequence must be greater than zero".to_owned(),
            ));
        }
        if let Some(version) = &self.version
            && version.trim().is_empty()
        {
            return Err(SyncwebError::InvalidConfig(
                "mutable link version cannot be empty".to_owned(),
            ));
        }
        for provider in &self.providers {
            provider.validate()?;
            if provider.hash != self.manifest {
                return Err(SyncwebError::InvalidTicket(
                    "mutable link provider lease does not match its manifest".to_owned(),
                ));
            }
        }
        Ok(())
    }

    /// Encode the signed pointer as JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the pointer is unsigned, invalid, or cannot be
    /// serialized.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.verify_signature()?;
        serde_json::to_vec(self)
            .map_err(|error| SyncwebError::operation("failed to serialize mutable link pointer", error))
    }

    /// Decode and validate a signed pointer.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes do not contain a valid signed pointer.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let pointer: Self = serde_json::from_slice(bytes.as_ref())
            .map_err(|error| SyncwebError::operation("failed to deserialize mutable link pointer", error))?;
        pointer.verify_signature()?;
        Ok(pointer)
    }
}

/// Alias for the signed mutable pointer terminology used by the protocol.
pub type SignedMutablePointer = MutablePointer;

/// A bearer capability link to a private manifest.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PrivateLink {
    /// Immutable manifest granted by this capability.
    pub manifest: Hash,
    /// Secret read capability.  It is intentionally included in the URI.
    pub capability: [u8; 32],
    /// Unix timestamp after which the capability is no longer accepted.
    pub expires_at: u64,
}

impl PrivateLink {
    /// Generate a private capability link with a cryptographically random
    /// bearer token.
    ///
    /// # Errors
    ///
    /// Returns an error if the expiration is invalid.
    pub fn generate(manifest: Hash, expires_at: u64) -> Result<Self> {
        let capability = rand::random::<[u8; 32]>();
        Self::new(manifest, capability, expires_at)
    }

    /// Alias for [`Self::generate`].
    ///
    /// # Errors
    ///
    /// Returns an error if the expiration is invalid.
    pub fn create(manifest: Hash, expires_at: u64) -> Result<Self> {
        Self::generate(manifest, expires_at)
    }

    /// Parse a private capability URI.
    ///
    /// # Errors
    ///
    /// Returns an error if the URI, hash, capability, or expiration is
    /// malformed.
    pub fn parse(value: &str) -> Result<Self> {
        value.parse()
    }

    /// Create a private capability link.
    ///
    /// # Errors
    ///
    /// Returns an error if the capability or expiration is invalid.
    pub fn new(manifest: Hash, capability: [u8; 32], expires_at: u64) -> Result<Self> {
        let link = Self {
            manifest,
            capability,
            expires_at,
        };
        link.validate()?;
        Ok(link)
    }

    /// Return the canonical capability URI.
    #[must_use]
    pub fn uri(&self) -> String {
        self.to_string()
    }

    /// Validate the capability and expiry.
    ///
    /// # Errors
    ///
    /// Returns an error if the expiration is zero.
    pub fn validate(&self) -> Result<()> {
        if self.expires_at == 0 {
            return Err(SyncwebError::InvalidConfig(
                "private link expiration must be greater than zero".to_owned(),
            ));
        }
        Ok(())
    }

    /// Return whether this link is expired at `now`.
    #[must_use]
    pub const fn is_expired_at(&self, now: u64) -> bool {
        self.expires_at <= now
    }

    /// Return a stable key used by revocation lists.
    #[must_use]
    pub fn revocation_key(&self) -> (Hash, Vec<u8>) {
        (self.manifest, self.capability.to_vec())
    }
}

impl fmt::Display for PrivateLink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{LINK_SCHEME}private/{}/{}?expires={}",
            self.manifest, hex::encode(self.capability), self.expires_at
        )
    }
}

impl FromStr for PrivateLink {
    type Err = SyncwebError;

    fn from_str(value: &str) -> Result<Self> {
        let (path, query) = value
            .strip_prefix("syncweb://private/")
            .ok_or_else(|| SyncwebError::InvalidConfig("private link must use syncweb://private/".to_owned()))?
            .split_once('?')
            .ok_or_else(|| SyncwebError::InvalidConfig("private link must contain an expiration".to_owned()))?;
        let mut parts = path.split('/');
        let manifest = next_part(&mut parts, "private link manifest")?;
        let capability_hex = next_part(&mut parts, "private link capability")?;
        if parts.next().is_some() {
            return Err(SyncwebError::InvalidConfig(
                "private link has too many path segments".to_owned(),
            ));
        }
        let manifest_hash = manifest
            .parse::<Hash>()
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid private link manifest: {error}")))?;
        let capability_bytes = hex::decode(capability_hex)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid private link capability: {error}")))?;
        let capability: [u8; 32] = capability_bytes.try_into().map_err(|_| {
            SyncwebError::InvalidConfig("private link capability must be 32 bytes".to_owned())
        })?;
        let expires_text = query
            .strip_prefix("expires=")
            .ok_or_else(|| SyncwebError::InvalidConfig("private link query must contain expires".to_owned()))?;
        let expires_at = expires_text
            .parse::<u64>()
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid private link expiration: {error}")))?;
        Self::new(manifest_hash, capability, expires_at)
    }
}

/// Alias for capability links.
pub type CapabilityLink = PrivateLink;

/// A direct provider ticket registered as a mirror.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Mirror {
    /// Hash served by this mirror.
    pub hash: Hash,
    /// Blob ticket for the mirror.
    pub ticket: BlobTicket,
}

impl Mirror {
    /// Construct a mirror from a blob ticket.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket does not match its hash.
    pub fn new(ticket: BlobTicket) -> Result<Self> {
        let mirror = Self {
            hash: ticket.hash(),
            ticket,
        };
        mirror.validate()?;
        Ok(mirror)
    }

    /// Validate that the ticket is consistent with the mirror hash.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket does not match its hash.
    pub fn validate(&self) -> Result<()> {
        if self.ticket.hash() != self.hash {
            return Err(SyncwebError::InvalidTicket(
                "mirror ticket does not match its content hash".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Result of resolving any public or private link.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct LinkResolution {
    /// Manifest or content hash identified by the link.
    pub manifest: Hash,
    /// Semantic version selected by a mutable link, if available.
    pub version: Option<String>,
    /// Mutable pointer sequence, if the link is mutable.
    pub sequence: Option<u64>,
    /// Signed provider leases advertised by the pointer or resolver.
    pub providers: Vec<ProviderLease>,
    /// Direct blob tickets in provider/mirror fallback order.
    pub tickets: Vec<BlobTicket>,
}

impl LinkResolution {
    /// Create a new resolution containing just a manifest hash.
    #[must_use]
    pub const fn new(manifest: Hash) -> Self {
        Self {
            manifest,
            version: None,
            sequence: None,
            providers: Vec::new(),
            tickets: Vec::new(),
        }
    }

    /// Return the resolved hash.
    #[must_use]
    pub const fn hash(&self) -> Hash {
        self.manifest
    }

    /// Return the resolved immutable manifest hash.
    #[must_use]
    pub const fn manifest_hash(&self) -> Hash {
        self.manifest
    }

    /// Return provider and mirror tickets in fallback order.
    #[must_use]
    pub fn provider_tickets(&self) -> &[BlobTicket] {
        &self.tickets
    }

    /// Fetch using the first provider that succeeds.
    ///
    /// # Errors
    ///
    /// Returns an error when no provider exists or every provider fails.
    pub fn fetch_with<T, Fetch>(&self, mut fetch: Fetch) -> Result<T>
    where
        Fetch: FnMut(&BlobTicket) -> Result<T>,
    {
        fetch_from_mirrors(&self.tickets, &mut fetch)
    }
}

/// Alias used by callers that prefer the shorter resolution name.
pub type ResolvedLink = LinkResolution;

/// A parsed syncweb stable link.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum Link {
    /// Immutable content link.
    Content(ContentLink),
    /// Mutable publisher name.
    Name(NameLink),
    /// Private bearer capability.
    Private(PrivateLink),
}

/// Alias for the public link enum.
pub type SyncwebLink = Link;

impl Link {
    /// Parse a stable link.
    ///
    /// # Errors
    ///
    /// Returns an error if the URI is not a supported syncweb link.
    pub fn parse(value: &str) -> Result<Self> {
        value.parse()
    }

    /// Return the link URI.
    #[must_use]
    pub fn uri(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Link {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Content(link) => link.fmt(formatter),
            Self::Name(link) => link.fmt(formatter),
            Self::Private(link) => link.fmt(formatter),
        }
    }
}

impl FromStr for Link {
    type Err = SyncwebError;

    fn from_str(value: &str) -> Result<Self> {
        if value.starts_with("syncweb://content/") {
            return Ok(Self::Content(value.parse()?));
        }
        if value.starts_with("syncweb://name/") {
            return Ok(Self::Name(value.parse()?));
        }
        if value.starts_with("syncweb://private/") {
            return Ok(Self::Private(value.parse()?));
        }
        Err(SyncwebError::InvalidConfig(
            "unsupported syncweb link; expected content, name, or private".to_owned(),
        ))
    }
}

/// Options for a link resolution operation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct ResolveOptions {
    /// Select an exact semantic version from a mutable pointer history.
    pub version: Option<String>,
    /// Evaluate expiry and provider leases at this timestamp.
    pub now: Option<u64>,
}

impl ResolveOptions {
    /// Select an exact version.
    #[must_use]
    pub fn version(version: impl Into<String>) -> Self {
        Self {
            version: Some(version.into()),
            now: None,
        }
    }

    /// Evaluate the link at an explicit time.
    #[must_use]
    pub const fn at(now: u64) -> Self {
        Self {
            version: None,
            now: Some(now),
        }
    }

    /// Set an exact version while retaining the other options.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
}

#[derive(Clone, Debug, Default)]
struct ResolverState {
    mirrors: HashMap<Hash, Vec<Mirror>>,
    pointers: HashMap<(PublicKey, String), PointerHistory>,
    revoked: HashSet<(Hash, Vec<u8>)>,
}

#[derive(Clone, Debug, Default)]
struct PointerHistory {
    current: Option<MutablePointer>,
    versions: BTreeMap<String, MutablePointer>,
}

/// Provider fetch hook for network-aware resolution.
pub type ProviderFetch =
    Arc<dyn Fn(NameLink) -> Pin<Box<dyn Future<Output = Result<LinkResolution>> + Send>> + Send + Sync>;

/// In-memory resolver and mirror registry.
#[derive(Clone)]
pub struct LinkResolver {
    state: Arc<Mutex<ResolverState>>,
    database: Option<IndexingDatabase>,
    provider_fetch: Option<ProviderFetch>,
}

impl fmt::Debug for LinkResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LinkResolver")
            .field("state", &self.state)
            .field("database", &self.database)
            .finish_non_exhaustive()
    }
}

impl Default for LinkResolver {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(ResolverState::default())),
            database: None,
            provider_fetch: None,
        }
    }
}

impl LinkResolver {
    /// Create an empty resolver.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a resolver with a provider fetch hook for network resolution.
    ///
    /// The fetch function is called when a name link cannot be resolved
    /// locally, allowing the caller to query remote peers via iroh-docs.
    #[must_use]
    pub fn with_provider_fetch(fetch: ProviderFetch) -> Self {
        Self {
            state: Arc::new(Mutex::new(ResolverState::default())),
            database: None,
            provider_fetch: Some(fetch),
        }
    }

    /// Attempt to resolve a name link by querying remote peers.
    ///
    /// Returns `None` if no provider fetch hook is registered or the link
    /// is not a name link. The caller is responsible for falling back to
    /// local resolution.
    ///
    /// # Errors
    ///
    /// Returns an error if the remote fetch fails.
    pub async fn resolve_remote(&self, link: &Link) -> Result<Option<LinkResolution>> {
        match link {
            Link::Name(name) => {
                if let Some(ref fetch) = self.provider_fetch {
                    fetch(name.clone()).await.map(Some)
                } else {
                    Ok(None)
                }
            }
            Link::Content(_) | Link::Private(_) => Ok(None),
        }
    }

    /// Create a resolver backed by an indexing database.
    ///
    /// Pointers and mirrors are hydrated from the database on construction.
    /// Mutations are persisted through the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be read.
    pub fn with_database(database: IndexingDatabase) -> Result<Self> {
        let result = database.load_links()?;
        let mut state = ResolverState::default();
        for pointer in result.0 {
            let key = (pointer.publisher, pointer.alias.clone());
            let history = state.pointers.entry(key).or_default();
            if let Some(current) = &history.current
                && pointer.sequence <= current.sequence
            {
                continue;
            }
            if let Some(version) = &pointer.version {
                history.versions.insert(version.clone(), pointer.clone());
            }
            history.current = Some(pointer);
        }
        for ticket_str in result.1 {
            if let Ok(ticket) = ticket_str.parse::<iroh_blobs::ticket::BlobTicket>()
                && let Ok(mirror) = Mirror::new(ticket)
            {
                let mirrors = state.mirrors.entry(mirror.hash).or_default();
                if !mirrors.iter().any(|existing| existing.ticket == mirror.ticket) {
                    mirrors.push(mirror);
                }
            }
        }
        for link in result.2 {
            state.revoked.insert(link.revocation_key());
        }
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            database: Some(database),
            provider_fetch: None,
        })
    }

    /// Register a direct mirror ticket for a hash.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket is malformed.
    pub fn register_mirror(&self, ticket: BlobTicket) -> Result<()> {
        let mirror = Mirror::new(ticket)?;
        let mut state = self.lock_state()?;
        let mirrors = state.mirrors.entry(mirror.hash).or_default();
        if !mirrors.iter().any(|existing| existing.ticket == mirror.ticket) {
            mirrors.push(mirror.clone());
            if let Some(ref database) = self.database {
                let _ = database.upsert_link_mirror(&mirror.ticket.to_string());
            }
        }
        drop(state);
        Ok(())
    }

    /// Alias for [`Self::register_mirror`].
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket is malformed.
    pub fn add_mirror(&self, ticket: BlobTicket) -> Result<()> {
        self.register_mirror(ticket)
    }

    /// Register a provider ticket as a mirror.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket is malformed.
    pub fn register_provider_ticket(&self, ticket: BlobTicket) -> Result<()> {
        self.register_mirror(ticket)
    }

    /// Register a signed provider lease for a hash.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease signature, expiry, or ticket is invalid.
    pub fn register_provider_lease(&self, lease: ProviderLease) -> Result<()> {
        let now = current_epoch_seconds();
        let validated_lease = checked_lease(lease, now)?;
        let ticket = lease_ticket(&validated_lease)?;
        let mut state = self.lock_state()?;
        let mirrors = state.mirrors.entry(validated_lease.hash).or_default();
        if !mirrors.iter().any(|existing| existing.ticket == ticket) {
            mirrors.push(Mirror {
                hash: validated_lease.hash,
                ticket,
            });
        }
        drop(state);
        Ok(())
    }

    /// Return registered direct mirror tickets in registration order.
    ///
    /// # Errors
    ///
    /// Returns an error if the resolver state lock is poisoned.
    pub fn mirrors(&self, hash: Hash) -> Result<Vec<BlobTicket>> {
        let state = self.lock_state()?;
        Ok(state
            .mirrors
            .get(&hash)
            .map(|mirrors| mirrors.iter().map(|mirror| mirror.ticket.clone()).collect())
            .unwrap_or_default())
    }

    /// Accept a signed mutable pointer, enforcing monotonic sequence.
    ///
    /// # Errors
    ///
    /// Returns an error if the pointer is unsigned, invalid, or older than
    /// the current pointer.
    pub fn publish(&self, pointer: &MutablePointer) -> Result<()> {
        pointer.verify_signature()?;
        let key = (pointer.publisher, pointer.alias.clone());
        let mut state = self.lock_state()?;
        let history = state.pointers.entry(key).or_default();
        if let Some(current) = &history.current
            && pointer.sequence <= current.sequence
        {
            return Err(SyncwebError::InvalidConfig(format!(
                "mutable link sequence {} must be greater than {}",
                pointer.sequence, current.sequence
            )));
        }
        if let Some(version) = &pointer.version {
            if history.versions.contains_key(version) {
                return Err(SyncwebError::InvalidConfig(format!(
                    "mutable link version {version:?} is already published"
                )));
            }
            history.versions.insert(version.clone(), pointer.clone());
        }
        history.current = Some(pointer.clone());
        if let Some(ref database) = self.database {
            let _ = database.upsert_link_pointer(pointer);
        }
        drop(state);
        Ok(())
    }

    /// Alias for [`Self::publish`].
    ///
    /// # Errors
    ///
    /// Returns an error if the pointer is invalid or not monotonic.
    pub fn register_pointer(&self, pointer: &MutablePointer) -> Result<()> {
        self.publish(pointer)
    }

    /// Alias for [`Self::publish`].
    ///
    /// # Errors
    ///
    /// Returns an error if the pointer is invalid or not monotonic.
    pub fn update(&self, pointer: &MutablePointer) -> Result<()> {
        self.publish(pointer)
    }

    /// Return the current pointer for a mutable name.
    ///
    /// # Errors
    ///
    /// Returns an error if the link is invalid or the resolver state lock is
    /// poisoned.
    pub fn current_pointer(&self, link: &NameLink) -> Result<Option<MutablePointer>> {
        link.validate()?;
        let state = self.lock_state()?;
        Ok(state
            .pointers
            .get(&(link.publisher, link.alias.clone()))
            .and_then(|history| history.current.clone()))
    }

    /// Revoke a private capability.
    ///
    /// # Errors
    ///
    /// Returns an error if the link is invalid or the resolver state lock is
    /// poisoned.
    pub fn revoke(&self, link: &PrivateLink) -> Result<()> {
        link.validate()?;
        self.lock_state()?.revoked.insert(link.revocation_key());
        Ok(())
    }

    /// Alias for [`Self::revoke`].
    ///
    /// # Errors
    ///
    /// Returns an error if the link is invalid or the resolver state lock is
    /// poisoned.
    pub fn revoke_private(&self, link: &PrivateLink) -> Result<()> {
        self.revoke(link)
    }

    /// Return whether a private capability has been revoked.
    ///
    /// # Errors
    ///
    /// Returns an error if the link is invalid or the resolver state lock is
    /// poisoned.
    pub fn is_revoked(&self, link: &PrivateLink) -> Result<bool> {
        link.validate()?;
        Ok(self.lock_state()?.revoked.contains(&link.revocation_key()))
    }

    /// Resolve a link using the current time.
    ///
    /// # Errors
    ///
    /// Returns an error if the link is invalid, unavailable, expired, revoked,
    /// or has an invalid provider.
    pub fn resolve(&self, link: &Link) -> Result<LinkResolution> {
        self.resolve_with_options(link, &ResolveOptions::default())
    }

    /// Resolve a link at an explicit timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if the link is invalid, unavailable, expired, revoked,
    /// or has an invalid provider.
    pub fn resolve_at(&self, link: &Link, now: u64) -> Result<LinkResolution> {
        self.resolve_with_options(link, &ResolveOptions::at(now))
    }

    /// Resolve a mutable link to an exact semantic version.
    ///
    /// # Errors
    ///
    /// Returns an error if the link or requested version is unavailable.
    pub fn resolve_version(&self, link: &NameLink, version: &str) -> Result<LinkResolution> {
        self.resolve_with_options(&Link::Name(link.clone()), &ResolveOptions::version(version))
    }

    /// Resolve a link using explicit version and expiry options.
    ///
    /// # Errors
    ///
    /// Returns an error if the link is invalid, unavailable, expired, revoked,
    /// or has an invalid provider.
    pub fn resolve_with_options(&self, link: &Link, options: &ResolveOptions) -> Result<LinkResolution> {
        let now = options.now.unwrap_or_else(current_epoch_seconds);
        let state = self.lock_state()?;
        let resolution = match link {
            Link::Content(content) => {
                content.validate()?;
                resolution_for_hash(content.hash, None, None, Vec::new(), &state, now)
            }
            Link::Private(private) => {
                private.validate()?;
                if private.is_expired_at(now) {
                    return Err(SyncwebError::InvalidConfig("private link has expired".to_owned()));
                }
                if state.revoked.contains(&private.revocation_key()) {
                    return Err(SyncwebError::InvalidConfig("private link has been revoked".to_owned()));
                }
                resolution_for_hash(private.manifest, None, None, Vec::new(), &state, now)
            }
            Link::Name(name) => {
                name.validate()?;
                let history = state
                    .pointers
                    .get(&(name.publisher, name.alias.clone()))
                    .ok_or_else(|| SyncwebError::InvalidConfig("mutable link has no published pointer".to_owned()))?;
                let pointer = if let Some(version) = &options.version {
                    history.versions.get(version).ok_or_else(|| {
                        SyncwebError::InvalidConfig(format!("mutable link version {version:?} not found"))
                    })?
                } else {
                    history
                        .current
                        .as_ref()
                        .ok_or_else(|| SyncwebError::InvalidConfig("mutable link has no current pointer".to_owned()))?
                };
                resolution_for_hash(
                    pointer.manifest,
                    pointer.version.clone(),
                    Some(pointer.sequence),
                    pointer.providers.clone(),
                    &state,
                    now,
                )
            }
        };
        drop(state);
        resolution
    }

    /// Parse and resolve a link URI.
    ///
    /// # Errors
    ///
    /// Returns an error if the URI cannot be parsed or the link cannot be
    /// resolved.
    pub fn resolve_uri(&self, uri: &str) -> Result<LinkResolution> {
        let link = uri.parse::<Link>()?;
        self.resolve(&link)
    }

    /// Resolve and fetch a link, trying each provider and mirror in order.
    ///
    /// # Errors
    ///
    /// Returns the first successful fetch result, or an error if resolution
    /// fails or every provider fails.
    pub fn fetch_with_mirrors<T, Fetch>(&self, link: &Link, fetch: Fetch) -> Result<T>
    where
        Fetch: FnMut(&BlobTicket) -> Result<T>,
    {
        self.resolve(link)?.fetch_with(fetch)
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, ResolverState>> {
        self.state
            .lock()
            .map_err(|error| SyncwebError::operation("link resolver state lock poisoned", error))
    }
}

/// Try provider tickets in order until one succeeds.
///
/// # Errors
///
/// Returns an error when no ticket is supplied or every fetch attempt fails.
pub fn fetch_from_mirrors<T, Fetch>(tickets: &[BlobTicket], fetch: &mut Fetch) -> Result<T>
where
    Fetch: FnMut(&BlobTicket) -> Result<T>,
{
    if tickets.is_empty() {
        return Err(SyncwebError::InvalidConfig("link has no provider tickets".to_owned()));
    }
    let mut failures = Vec::new();
    for ticket in tickets {
        match fetch(ticket) {
            Ok(value) => return Ok(value),
            Err(error) => failures.push(error.to_string()),
        }
    }
    Err(SyncwebError::InvalidConfig(format!(
        "all mirror providers failed: {}",
        failures.join("; ")
    )))
}

fn resolution_for_hash(
    manifest: Hash,
    version: Option<String>,
    sequence: Option<u64>,
    providers: Vec<ProviderLease>,
    state: &ResolverState,
    now: u64,
) -> Result<LinkResolution> {
    let mut tickets = Vec::new();
    let mut active_providers = Vec::new();
    for provider in providers {
        if provider.expires_at <= now {
            continue;
        }
        provider.verify_at(now)?;
        let ticket = lease_ticket(&provider)?;
        if !tickets.contains(&ticket) {
            tickets.push(ticket);
        }
        active_providers.push(provider);
    }
    if let Some(mirrors) = state.mirrors.get(&manifest) {
        for mirror in mirrors {
            if !tickets.contains(&mirror.ticket) {
                tickets.push(mirror.ticket.clone());
            }
        }
    }
    Ok(LinkResolution {
        manifest,
        version,
        sequence,
        providers: active_providers,
        tickets,
    })
}

fn lease_ticket(lease: &ProviderLease) -> Result<BlobTicket> {
    lease
        .ticket
        .parse::<BlobTicket>()
        .map_err(|error| SyncwebError::InvalidTicket(format!("invalid provider ticket: {error}")))
}

fn checked_lease(lease: ProviderLease, now: u64) -> Result<ProviderLease> {
    lease.verify_at(now)?;
    Ok(lease)
}

fn validate_path_segment(value: &str, field: &str) -> Result<()> {
    if value.is_empty() || value == "." || value == ".." {
        return Err(SyncwebError::InvalidConfig(format!(
            "{field} cannot be empty or dot-like"
        )));
    }
    if value
        .chars()
        .any(|character| character.is_control() || matches!(character, '/' | '?' | '#' | '%'))
    {
        return Err(SyncwebError::InvalidConfig(format!("{field} contains URI delimiters")));
    }
    Ok(())
}

fn next_part<'a>(parts: &mut Split<'a, char>, field: &str) -> Result<&'a str> {
    let part = parts
        .next()
        .ok_or_else(|| SyncwebError::InvalidConfig(format!("{field} is missing")))?;
    if part.is_empty() {
        return Err(SyncwebError::InvalidConfig(format!("{field} is empty")));
    }
    Ok(part)
}

impl SignedGossipMessage for PrivateLink {
    fn verify_signature(&self) -> Result<()> {
        Ok(())
    }
}

/// Deterministic gossip topic for broadcasting link revocations.
pub const REVOCATION_GOSSIP_TOPIC: &[u8] = b"syncweb/link-revocations/v1";

/// Deterministic gossip topic ID for link revocations.
#[must_use]
pub fn revocation_topic_id() -> iroh_gossip::TopicId {
    iroh_gossip::TopicId::from_bytes(*blake3::hash(REVOCATION_GOSSIP_TOPIC).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::{EndpointAddr, SecretKey};
    use iroh_blobs::BlobFormat;

    fn ticket(seed: u8, hash: Hash) -> BlobTicket {
        let secret = SecretKey::from_bytes(&[seed; 32]);
        BlobTicket::new(EndpointAddr::new(secret.public()), hash, BlobFormat::Raw)
    }

    fn pointer(seed: u8, alias: &str, hash: Hash, sequence: u64, version: &str) -> Result<MutablePointer> {
        let secret = SecretKey::from_bytes(&[seed; 32]);
        let mut pointer = MutablePointer::new(secret.public(), alias, hash, sequence)?.with_version(version);
        pointer.sign(&SigningKey::from_bytes(&secret.to_bytes()))?;
        Ok(pointer)
    }

    #[test]
    fn immutable_links_round_trip() {
        let hash = Hash::from_bytes([7; 32]);
        let link = ContentLink::new(hash);
        assert_eq!(link.to_string(), format!("syncweb://content/{hash}"));
        assert_eq!(
            link.to_string()
                .parse::<ContentLink>()
                .expect("content link should parse"),
            link
        );
        assert_eq!(
            link.to_string().parse::<Link>().expect("link should parse"),
            Link::Content(link)
        );
    }

    #[test]
    fn signed_names_are_monotonic_and_version_pinnable() {
        let hash_one = Hash::from_bytes([1; 32]);
        let hash_two = Hash::from_bytes([2; 32]);
        let resolver = LinkResolver::new();
        let first = pointer(1, "dataset", hash_one, 1, "1.0.0").expect("pointer should build");
        let name = first.link().expect("pointer should have a link");
        resolver.publish(&first).expect("first pointer should publish");
        assert_eq!(
            resolver
                .resolve(&Link::Name(name.clone()))
                .expect("name should resolve")
                .manifest,
            hash_one
        );
        let pinned = resolver
            .resolve_version(&name, "1.0.0")
            .expect("version should resolve");
        assert_eq!(pinned.manifest, hash_one);
        let second = pointer(1, "dataset", hash_two, 2, "2.0.0").expect("pointer should build");
        resolver.publish(&second).expect("second pointer should publish");
        assert_eq!(
            resolver
                .resolve(&Link::Name(name.clone()))
                .expect("name should resolve")
                .manifest,
            hash_two
        );
        assert!(resolver.publish(&first).is_err());
        assert_eq!(
            resolver
                .resolve_version(&name, "1.0.0")
                .expect("version should resolve")
                .manifest,
            hash_one
        );
    }

    #[test]
    fn mutable_resolution_includes_verified_provider_tickets() {
        let hash = Hash::from_bytes([8; 32]);
        let provider_secret = SecretKey::from_bytes(&[8; 32]);
        let provider_ticket = ticket(8, hash);
        let mut lease = ProviderLease::new_with_times(hash, provider_ticket.to_string(), 1, 0, u64::MAX)
            .expect("provider lease should build");
        lease
            .sign_with_secret_key(&provider_secret)
            .expect("provider lease should sign");

        let publisher_secret = SecretKey::from_bytes(&[9; 32]);
        let mut pointer =
            MutablePointer::new_with_providers(publisher_secret.public(), "dataset", hash, 1, vec![lease])
                .expect("pointer should build")
                .with_version("1.0.0");
        pointer
            .sign(&SigningKey::from_bytes(&publisher_secret.to_bytes()))
            .expect("pointer should sign");

        let resolver = LinkResolver::new();
        let name = pointer.link().expect("pointer should have a link");
        resolver.publish(&pointer).expect("pointer should publish");
        let resolution = resolver.resolve(&Link::Name(name)).expect("name should resolve");
        assert_eq!(resolution.manifest_hash(), hash);
        assert_eq!(resolution.providers.len(), 1);
        assert_eq!(resolution.provider_tickets(), &[provider_ticket]);
    }

    #[test]
    fn private_links_expire_and_revoke() {
        let hash = Hash::from_bytes([3; 32]);
        let cap: [u8; 32] = [1u8; 32];
        let link = PrivateLink::new(hash, cap, 20).expect("private link should build");
        let resolver = LinkResolver::new();
        assert_eq!(
            resolver
                .resolve_at(&Link::Private(link.clone()), 10)
                .expect("private link should resolve")
                .manifest,
            hash
        );
        resolver.revoke(&link).expect("private link should revoke");
        assert!(resolver.resolve_at(&Link::Private(link.clone()), 10).is_err());
        assert!(resolver.resolve_at(&Link::Private(link), 20).is_err());
    }

    #[test]
    fn private_link_generates_random_capability() {
        let hash = Hash::from_bytes([4; 32]);
        let link = PrivateLink::generate(hash, 100).expect("should generate");
        assert_ne!(link.capability, [0u8; 32], "capability should not be zeroed");
        assert_eq!(link.manifest, hash);
        assert_eq!(link.expires_at, 100);
        // Two generated links should differ
        let link2 = PrivateLink::generate(hash, 100).expect("should generate");
        assert_ne!(link.capability, link2.capability, "capabilities should be random");
    }

    #[test]
    fn private_link_uri_round_trip() {
        let hash = Hash::from_bytes([5; 32]);
        let cap: [u8; 32] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
        ];
        let link = PrivateLink::new(hash, cap, 200).expect("should build");
        let uri = link.to_string();
        assert!(uri.starts_with("syncweb://private/"));
        assert!(uri.contains("?expires=200"));
        let parsed: PrivateLink = uri.parse().expect("should parse");
        assert_eq!(parsed.manifest, hash);
        assert_eq!(parsed.capability, cap);
        assert_eq!(parsed.expires_at, 200);
    }

    #[test]
    fn mirror_fallback_tries_alternates() {
        let hash = Hash::from_bytes([4; 32]);
        let resolver = LinkResolver::new();
        let first = ticket(4, hash);
        let second = ticket(5, hash);
        resolver
            .register_mirror(first.clone())
            .expect("first mirror should register");
        resolver.register_mirror(second).expect("second mirror should register");
        let link = Link::Content(ContentLink::new(hash));
        let value = resolver
            .fetch_with_mirrors(&link, |candidate| {
                if candidate == &first {
                    Err(SyncwebError::InvalidConfig("first mirror unavailable".to_owned()))
                } else {
                    Ok("second mirror")
                }
            })
            .expect("fallback should find a mirror");
        assert_eq!(value, "second mirror");
    }
}
