//! Lease-based availability and replication for the indexing service.
//!
//! The resilience layer observes signed provider claims without changing the
//! document synchronization protocol. When a blob falls below its configured
//! replication budget, it can use the advertised blob tickets to fetch and
//! pin the content through the normal blob store.

use std::{
    collections::{HashMap, HashSet},
    io,
    str::FromStr,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use iroh::{Endpoint, PublicKey, SecretKey};
use iroh_blobs::{Hash, ticket::BlobTicket};
use iroh_gossip::{
    TopicId,
    api::{Event, GossipSender, GossipTopic},
};
use n0_future::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::{sync::watch, task::JoinHandle};

use crate::{
    error::{Result, SyncwebError},
    node::{blob_store::BlobStore, gossip_service::GossipService},
};

const RESILIENCE_TOPIC_SEED: &[u8] = b"syncweb/provider-leases/v1";
const PROVIDER_LEASE_SIGNATURE_CONTEXT: &[u8] = b"syncweb/provider-lease/v1\0";
const DEFAULT_OBSERVATION_TTL: Duration = Duration::from_mins(5);
const DEFAULT_MAX_JITTER: Duration = Duration::from_secs(30);
const DEFAULT_RESPONSIBLE_PEERS: usize = 1;
const DEFAULT_MAX_FAILURES_PER_PROVIDER: usize = 128;
const DEFAULT_FAILURE_TTL: Duration = Duration::from_hours(24);
const DEFINITIVE_FAILURE_THRESHOLD: u32 = 3;
const STREAM_VALIDATION_CHUNK_SIZE: usize = 64 * 1024;
const REPLICATION_PIN_PREFIX: &str = "syncweb/replication/";

/// The broad cause of a failed provider fetch.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FetchFailureKind {
    NotFound,
    ConnectionRefused,
    Timeout,
    Corruption,
    Unknown,
}

impl FetchFailureKind {
    /// Classify an error at the resilience boundary.
    ///
    /// `SyncwebError::Operation` intentionally retains the underlying
    /// library error as display text, so classification uses stable
    /// lower-case error phrases rather than coupling this layer to iroh's
    /// private error types.
    #[must_use]
    pub fn from_syncweb_error(error: &SyncwebError) -> Self {
        Self::from_message(error.to_string())
    }

    /// Alias for [`Self::from_syncweb_error`].
    #[must_use]
    pub fn classify(error: &SyncwebError) -> Self {
        Self::from_syncweb_error(error)
    }

    /// Alias for [`Self::from_syncweb_error`].
    #[must_use]
    pub fn from_error(error: &SyncwebError) -> Self {
        Self::from_syncweb_error(error)
    }

    /// Classify a displayable error message.
    #[must_use]
    pub fn from_message(message: impl AsRef<str>) -> Self {
        let message_text = message.as_ref().to_ascii_lowercase();
        if [
            "not found",
            "does not exist",
            "no such file",
            "missing blob",
            "unknown hash",
            "404",
        ]
        .iter()
        .any(|phrase| message_text.contains(phrase))
        {
            return Self::NotFound;
        }
        if [
            "hash mismatch",
            "checksum mismatch",
            "integrity",
            "corrupt",
            "invalid data",
        ]
        .iter()
        .any(|phrase| message_text.contains(phrase))
        {
            return Self::Corruption;
        }
        if [
            "connection refused",
            "connection reset",
            "network is unreachable",
            "no route to host",
            "unreachable",
        ]
        .iter()
        .any(|phrase| message_text.contains(phrase))
        {
            return Self::ConnectionRefused;
        }
        if ["timed out", "timeout", "deadline exceeded"]
            .iter()
            .any(|phrase| message_text.contains(phrase))
        {
            return Self::Timeout;
        }
        Self::Unknown
    }

    #[must_use]
    pub const fn is_definitive(self) -> bool {
        matches!(self, Self::NotFound | Self::Corruption)
    }

    #[must_use]
    pub const fn is_transient(self) -> bool {
        !self.is_definitive()
    }
}

impl From<&SyncwebError> for FetchFailureKind {
    fn from(error: &SyncwebError) -> Self {
        Self::from_syncweb_error(error)
    }
}

/// A provider-specific failure observed while fetching one blob.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FetchFailure {
    pub kind: FetchFailureKind,
    pub provider: PublicKey,
    pub hash: Hash,
    pub timestamp: u64,
    pub error_detail: String,
}

impl FetchFailure {
    /// Construct a failure timestamped with the current epoch time.
    #[must_use]
    pub fn new(kind: FetchFailureKind, provider: PublicKey, hash: Hash, error_detail: impl Into<String>) -> Self {
        Self::new_at(kind, provider, hash, current_epoch_seconds(), error_detail)
    }

    /// Construct a failure with an explicit timestamp.
    #[must_use]
    pub fn new_at(
        kind: FetchFailureKind,
        provider: PublicKey,
        hash: Hash,
        timestamp: u64,
        error_detail: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            provider,
            hash,
            timestamp,
            error_detail: error_detail.into(),
        }
    }

    /// Construct and classify a failure from a core error.
    #[must_use]
    pub fn from_syncweb_error(provider: PublicKey, hash: Hash, error: &SyncwebError) -> Self {
        Self::new(
            FetchFailureKind::from_syncweb_error(error),
            provider,
            hash,
            error.to_string(),
        )
    }

    /// Alias for [`Self::from_syncweb_error`].
    #[must_use]
    pub fn from_error(provider: PublicKey, hash: Hash, error: &SyncwebError) -> Self {
        Self::from_syncweb_error(provider, hash, error)
    }

    /// Encode this failure as JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|error| SyncwebError::operation("failed to serialize fetch failure", error))
    }

    /// Decode a failure from JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON is malformed.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        serde_json::from_slice(bytes.as_ref())
            .map_err(|error| SyncwebError::operation("failed to deserialize fetch failure", error))
    }
}

/// Validate a provider stream using bounded memory.
///
/// The reader is consumed in fixed-size chunks. At most one chunk is held in
/// memory, and an extra byte is read only after the expected size to reject
/// oversized responses before they can be accepted or buffered.
///
/// # Errors
///
/// Returns an error if the stream is too long, truncated, cannot be read, or
/// does not hash to `expected_hash`.
pub async fn validate_bounded_fetch<R>(mut reader: R, expected_size: u64, expected_hash: Hash) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    let mut hasher = blake3::Hasher::new();
    let mut remaining = expected_size;
    let mut buffer = vec![0_u8; STREAM_VALIDATION_CHUNK_SIZE];
    while remaining > 0 {
        let requested = usize::try_from(remaining).map_or(STREAM_VALIDATION_CHUNK_SIZE, |remaining_size| {
            remaining_size.min(STREAM_VALIDATION_CHUNK_SIZE)
        });
        let read_buffer = buffer
            .get_mut(..requested)
            .ok_or_else(|| SyncwebError::InvalidConfig("invalid bounded fetch buffer size".to_owned()))?;
        let read = reader.read(read_buffer).await?;
        if read == 0 {
            return Err(SyncwebError::operation(
                "fetched blob validation failed",
                io::Error::new(io::ErrorKind::UnexpectedEof, "provider stream is truncated"),
            ));
        }
        let data = buffer
            .get(..read)
            .ok_or_else(|| SyncwebError::InvalidConfig("invalid bounded fetch read size".to_owned()))?;
        hasher.update(data);
        remaining = remaining.saturating_sub(u64::try_from(read).unwrap_or(u64::MAX));
    }

    let read_buffer = buffer
        .get_mut(..1)
        .ok_or_else(|| SyncwebError::InvalidConfig("invalid bounded fetch buffer size".to_owned()))?;
    let read = reader.read(read_buffer).await?;
    if read != 0 {
        return Err(SyncwebError::operation(
            "fetched blob validation failed",
            io::Error::new(io::ErrorKind::InvalidData, "provider stream exceeds expected size"),
        ));
    }

    let actual_hash = Hash::from_bytes(*hasher.finalize().as_bytes());
    if actual_hash != expected_hash {
        return Err(SyncwebError::operation(
            "fetched blob validation failed",
            io::Error::new(io::ErrorKind::InvalidData, "provider stream hash mismatch"),
        ));
    }
    Ok(())
}

/// Alias for [`validate_bounded_fetch`].
///
/// # Errors
///
/// Returns an error if the stream fails bounded size or hash validation.
pub async fn validate_fetch_stream<R>(reader: R, expected_size: u64, expected_hash: Hash) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    validate_bounded_fetch(reader, expected_size, expected_hash).await
}

/// Alias for [`validate_bounded_fetch`].
///
/// # Errors
///
/// Returns an error if the stream fails bounded size or hash validation.
pub async fn validate_bounded_stream<R>(reader: R, expected_size: u64, expected_hash: Hash) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    validate_bounded_fetch(reader, expected_size, expected_hash).await
}

/// A signed claim that a provider currently serves a blob.
///
/// The blob ticket is included in the signed payload so a lease cannot be
/// redirected to a different hash or provider after it is published.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProviderLease {
    pub hash: Hash,
    pub provider: PublicKey,
    pub ticket: String,
    pub sequence: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub signature: Option<String>,
}

impl ProviderLease {
    /// Create an unsigned lease using the current time as its issue time.
    ///
    /// Call [`Self::sign`] before publishing or tracking the lease.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket is invalid or does not match the hash.
    pub fn new(hash: Hash, ticket: impl Into<String>, sequence: u64, expires_at: u64) -> Result<Self> {
        Self::new_with_times(hash, ticket, sequence, current_epoch_seconds(), expires_at)
    }

    /// Create an unsigned lease with explicit timestamps.
    ///
    /// Explicit timestamps are useful when restoring persisted leases or
    /// testing expiry behavior.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket is invalid, does not match the hash, or
    /// the lease timestamps are invalid.
    pub fn new_with_times(
        hash: Hash,
        ticket: impl Into<String>,
        sequence: u64,
        issued_at: u64,
        expires_at: u64,
    ) -> Result<Self> {
        let ticket_text = ticket.into();
        let parsed_ticket = parse_ticket(&ticket_text)?;
        if parsed_ticket.hash() != hash {
            return Err(SyncwebError::InvalidTicket(
                "provider lease ticket does not match its blob hash".to_owned(),
            ));
        }
        let lease = Self {
            hash,
            provider: parsed_ticket.addr().id,
            ticket: ticket_text,
            sequence,
            issued_at,
            expires_at,
            signature: None,
        };
        lease.validate()?;
        Ok(lease)
    }

    /// Create and sign a lease with an iroh node secret key.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket is invalid, the key does not own the
    /// advertised provider, or the lease cannot be serialized.
    pub fn signed(
        hash: Hash,
        ticket: impl Into<String>,
        sequence: u64,
        expires_at: u64,
        secret_key: &SecretKey,
    ) -> Result<Self> {
        let mut lease = Self::new(hash, ticket, sequence, expires_at)?;
        lease.sign_with_secret_key(secret_key)?;
        Ok(lease)
    }

    /// Sign this lease with an Ed25519 signing key.
    ///
    /// The signing key must correspond to the provider encoded in the ticket.
    ///
    /// # Errors
    ///
    /// Returns an error if the key does not own the advertised provider or the
    /// unsigned lease cannot be serialized.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        if signing_key.verifying_key().to_bytes() != *self.provider.as_bytes() {
            return Err(SyncwebError::InvalidIdentity(
                "provider lease signer does not match the ticket provider".to_owned(),
            ));
        }
        self.signature = Some(hex::encode(signing_key.sign(&self.unsigned_bytes()?).to_bytes()));
        Ok(())
    }

    /// Sign this lease with an iroh node secret key.
    ///
    /// # Errors
    ///
    /// Returns an error if the key does not own the advertised provider or the
    /// lease cannot be serialized.
    pub fn sign_with_secret_key(&mut self, secret_key: &SecretKey) -> Result<()> {
        let signing_key = SigningKey::from_bytes(&secret_key.to_bytes());
        self.sign(&signing_key)
    }

    /// Return whether the lease carries a signature.
    #[must_use]
    pub const fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    /// Verify the lease signature without checking expiry.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease structure or signature is invalid.
    pub fn verify_signature(&self) -> Result<()> {
        self.validate()?;
        let signature_text = self
            .signature
            .as_deref()
            .ok_or_else(|| SyncwebError::InvalidConfig("provider lease must contain a signature".to_owned()))?;
        let signature_bytes = hex::decode(signature_text).map_err(|error| {
            SyncwebError::InvalidConfig(format!("invalid provider lease signature encoding: {error}"))
        })?;
        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid provider lease signature: {error}")))?;
        let verifying_key = VerifyingKey::from_bytes(self.provider.as_bytes())
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid provider lease identity: {error}")))?;
        verifying_key
            .verify(&self.unsigned_bytes()?, &signature)
            .map_err(|error| SyncwebError::InvalidConfig(format!("provider lease signature is invalid: {error}")))
    }

    /// Verify the signature and require the lease to be active at `now`.
    ///
    /// # Errors
    ///
    /// Returns an error if the signature is invalid or the lease has expired.
    pub fn verify_at(&self, now: u64) -> Result<()> {
        self.verify_signature()?;
        if self.expires_at <= now {
            return Err(SyncwebError::InvalidConfig("provider lease has expired".to_owned()));
        }
        Ok(())
    }

    /// Return whether the lease is expired at `now`.
    #[must_use]
    pub const fn is_expired_at(&self, now: u64) -> bool {
        self.expires_at <= now
    }

    /// Encode the lease as JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease is malformed or cannot be serialized.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|error| SyncwebError::operation("failed to serialize provider lease", error))
    }

    /// Decode a lease from JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease is malformed or cannot be decoded.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let lease: Self = serde_json::from_slice(bytes.as_ref())
            .map_err(|error| SyncwebError::operation("failed to deserialize provider lease", error))?;
        lease.validate()?;
        Ok(lease)
    }

    /// Validate the non-cryptographic lease fields.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket, sequence, or timestamps are invalid.
    pub fn validate(&self) -> Result<()> {
        if self.sequence == 0 {
            return Err(SyncwebError::InvalidConfig(
                "provider lease sequence must be greater than zero".to_owned(),
            ));
        }
        if self.expires_at <= self.issued_at {
            return Err(SyncwebError::InvalidConfig(
                "provider lease expiration must be after its issue time".to_owned(),
            ));
        }
        let ticket = parse_ticket(&self.ticket)?;
        if ticket.hash() != self.hash {
            return Err(SyncwebError::InvalidTicket(
                "provider lease ticket does not match its blob hash".to_owned(),
            ));
        }
        if ticket.addr().id != self.provider {
            return Err(SyncwebError::InvalidIdentity(
                "provider lease provider does not match its ticket".to_owned(),
            ));
        }
        Ok(())
    }

    fn unsigned_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        let encoded = serde_json::to_vec(&unsigned)
            .map_err(|error| SyncwebError::operation("failed to serialize unsigned provider lease", error))?;
        let mut signed_bytes = Vec::with_capacity(PROVIDER_LEASE_SIGNATURE_CONTEXT.len().saturating_add(encoded.len()));
        signed_bytes.extend_from_slice(PROVIDER_LEASE_SIGNATURE_CONTEXT);
        signed_bytes.extend_from_slice(&encoded);
        Ok(signed_bytes)
    }
}

/// A lease update accepted by a [`ProviderLeaseTracker`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LeaseUpdate {
    Inserted,
    Replaced,
    IgnoredOlder,
}

impl LeaseUpdate {
    #[must_use]
    pub const fn changed(self) -> bool {
        matches!(self, Self::Inserted | Self::Replaced)
    }
}

/// Verified and locally observed providers for one blob.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AvailabilityHealth {
    pub verified: usize,
    pub local: usize,
    pub verified_providers: Vec<PublicKey>,
    pub local_providers: Vec<PublicKey>,
}

impl AvailabilityHealth {
    #[must_use]
    pub const fn verified_count(&self) -> usize {
        self.verified
    }

    #[must_use]
    pub const fn verified_lease_count(&self) -> usize {
        self.verified
    }

    #[must_use]
    pub const fn local_count(&self) -> usize {
        self.local
    }

    #[must_use]
    pub const fn local_observation_count(&self) -> usize {
        self.local
    }
}

/// In-memory tracker for signed provider leases and local observations.
#[derive(Clone, Debug)]
pub struct ProviderLeaseTracker {
    leases: HashMap<Hash, HashMap<PublicKey, ProviderLease>>,
    observations: HashMap<Hash, HashMap<PublicKey, u64>>,
    failures: HashMap<Hash, HashMap<PublicKey, FailureRecord>>,
    failure_totals: HashMap<Hash, HashMap<PublicKey, u64>>,
    definitive_streaks: HashMap<Hash, HashMap<PublicKey, u32>>,
    max_failures_per_provider: usize,
}

impl Default for ProviderLeaseTracker {
    fn default() -> Self {
        Self::with_max_failures_per_provider(DEFAULT_MAX_FAILURES_PER_PROVIDER)
    }
}

/// Bounded failure history and aggregate counters for one provider and blob.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FailureRecord {
    pub failures: Vec<FetchFailure>,
    pub consecutive_failures: u32,
    pub last_failure_at: u64,
    pub first_failure_at: u64,
}

impl ProviderLeaseTracker {
    /// Create an empty tracker with a bounded failure history.
    #[must_use]
    pub fn with_max_failures_per_provider(max_failures_per_provider: usize) -> Self {
        Self {
            leases: HashMap::new(),
            observations: HashMap::new(),
            failures: HashMap::new(),
            failure_totals: HashMap::new(),
            definitive_streaks: HashMap::new(),
            max_failures_per_provider,
        }
    }

    /// Return the failure-detail cap for each `(hash, provider)` pair.
    #[must_use]
    pub const fn max_failures_per_provider(&self) -> usize {
        self.max_failures_per_provider
    }

    /// Set the failure-detail cap and evict oldest details if necessary.
    pub fn set_max_failures_per_provider(&mut self, max_failures_per_provider: usize) {
        self.max_failures_per_provider = max_failures_per_provider;
        for providers in self.failures.values_mut() {
            for record in providers.values_mut() {
                let keep_from = record.failures.len().saturating_sub(max_failures_per_provider);
                if keep_from > 0 {
                    record.failures.drain(..keep_from);
                }
            }
        }
    }

    /// Track an active, signed lease using the current time.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease signature or expiry is invalid.
    pub fn track(&mut self, lease: ProviderLease) -> Result<LeaseUpdate> {
        self.track_at(lease, current_epoch_seconds())
    }

    /// Track an active, signed lease at an explicit time.
    ///
    /// Older sequence numbers from the same provider are ignored. A new
    /// sequence replaces the provider's previous lease for this blob.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease signature or expiry is invalid.
    pub fn track_at(&mut self, lease: ProviderLease, now: u64) -> Result<LeaseUpdate> {
        lease.verify_at(now)?;
        let providers = self.leases.entry(lease.hash).or_default();
        let update = match providers.get(&lease.provider) {
            None => LeaseUpdate::Inserted,
            Some(existing) if lease.sequence > existing.sequence => LeaseUpdate::Replaced,
            Some(_) => LeaseUpdate::IgnoredOlder,
        };
        if update.changed() {
            providers.insert(lease.provider, lease);
        }
        Ok(update)
    }

    /// Alias for [`Self::track`].
    ///
    /// # Errors
    ///
    /// Returns an error if the lease signature or expiry is invalid.
    pub fn record_lease(&mut self, lease: ProviderLease) -> Result<LeaseUpdate> {
        self.track(lease)
    }

    /// Alias for [`Self::track`].
    ///
    /// # Errors
    ///
    /// Returns an error if the lease signature or expiry is invalid.
    pub fn add_lease(&mut self, lease: ProviderLease) -> Result<LeaseUpdate> {
        self.track(lease)
    }

    /// Record a provider failure using the current epoch time.
    pub fn record_failure(&mut self, hash: Hash, provider: PublicKey, failure: FetchFailure) {
        self.record_failure_at(hash, provider, failure, current_epoch_seconds());
    }

    /// Record a provider failure at an explicit time.
    ///
    /// Failure details are capped per provider, while aggregate counts remain
    /// available through [`Self::failure_count`].
    pub fn record_failure_at(&mut self, hash: Hash, provider: PublicKey, failure: FetchFailure, now: u64) {
        self.purge_stale_failures(now, DEFAULT_FAILURE_TTL);
        let existing_total = self
            .failure_totals
            .get(&hash)
            .and_then(|providers| providers.get(&provider))
            .copied()
            .unwrap_or(0);
        let previous_kind = self
            .failures
            .get(&hash)
            .and_then(|providers| providers.get(&provider))
            .and_then(|record| record.failures.last())
            .map(|item| item.kind);
        let previous_consecutive = self
            .failures
            .get(&hash)
            .and_then(|providers| providers.get(&provider))
            .map_or(0, |record| record.consecutive_failures);
        let record = self
            .failures
            .entry(hash)
            .or_default()
            .entry(provider)
            .or_insert_with(|| FailureRecord {
                failures: Vec::new(),
                consecutive_failures: 0,
                last_failure_at: now,
                first_failure_at: now,
            });
        if existing_total == 0 {
            record.first_failure_at = now;
        }
        record.last_failure_at = now;
        record.consecutive_failures = previous_consecutive.saturating_add(1);
        let definitive_streak = self
            .definitive_streaks
            .entry(hash)
            .or_default()
            .entry(provider)
            .or_default();
        *definitive_streak = if failure.kind.is_definitive() {
            if previous_kind.is_some_and(FetchFailureKind::is_definitive) {
                definitive_streak.saturating_add(1)
            } else {
                1
            }
        } else {
            0
        };
        record.failures.push(failure);
        let keep_from = record.failures.len().saturating_sub(self.max_failures_per_provider);
        if keep_from > 0 {
            record.failures.drain(..keep_from);
        }
        let total = self
            .failure_totals
            .entry(hash)
            .or_default()
            .entry(provider)
            .or_default();
        *total = existing_total.saturating_add(1);
    }

    /// Return the bounded failure details for a provider and blob.
    #[must_use]
    pub fn failure_record(&self, hash: &Hash, provider: &PublicKey) -> Option<&FailureRecord> {
        self.failures.get(hash).and_then(|providers| providers.get(provider))
    }

    /// Return the aggregate failure count for a provider and blob.
    #[must_use]
    pub fn failure_count(&self, hash: &Hash, provider: &PublicKey) -> usize {
        self.failure_totals
            .get(hash)
            .and_then(|providers| providers.get(provider))
            .copied()
            .map_or(0, |count| usize::try_from(count).unwrap_or(usize::MAX))
    }

    /// Return the current consecutive definitive-failure count.
    #[must_use]
    pub fn consecutive_failures(&self, hash: &Hash, provider: &PublicKey) -> u32 {
        self.failure_record(hash, provider)
            .map_or(0, |record| record.consecutive_failures)
    }

    /// Return whether a provider has crossed the definitive-failure threshold.
    #[must_use]
    pub fn is_definitively_failed(&self, hash: &Hash, provider: &PublicKey) -> bool {
        self.definitive_streaks
            .get(hash)
            .and_then(|providers| providers.get(provider))
            .is_some_and(|streak| *streak >= DEFINITIVE_FAILURE_THRESHOLD)
    }

    /// Clear all failure history for a provider after a successful fetch.
    pub fn clear_failures_for_provider(&mut self, hash: &Hash, provider: &PublicKey) {
        if let Some(providers) = self.failures.get_mut(hash) {
            providers.remove(provider);
            if providers.is_empty() {
                self.failures.remove(hash);
            }
        }
        if let Some(providers) = self.failure_totals.get_mut(hash) {
            providers.remove(provider);
            if providers.is_empty() {
                self.failure_totals.remove(hash);
            }
        }
        if let Some(providers) = self.definitive_streaks.get_mut(hash) {
            providers.remove(provider);
            if providers.is_empty() {
                self.definitive_streaks.remove(hash);
            }
        }
    }

    /// Remove failure records whose last observation is outside `ttl`.
    pub fn purge_stale_failures(&mut self, now: u64, ttl: Duration) {
        let ttl_seconds = ttl.as_secs();
        let mut stale = Vec::new();
        self.failures.retain(|hash, providers| {
            providers.retain(|provider, record| {
                let keep = now.saturating_sub(record.last_failure_at) <= ttl_seconds;
                if !keep {
                    stale.push((*hash, *provider));
                }
                keep
            });
            !providers.is_empty()
        });
        for (hash, provider) in stale {
            if let Some(totals) = self.failure_totals.get_mut(&hash) {
                totals.remove(&provider);
            }
            if let Some(streaks) = self.definitive_streaks.get_mut(&hash) {
                streaks.remove(&provider);
            }
        }
        self.failure_totals.retain(|_, providers| !providers.is_empty());
        self.definitive_streaks.retain(|_, providers| !providers.is_empty());
    }

    /// Record that a provider was observed serving a blob locally.
    pub fn observe_provider(&mut self, hash: Hash, provider: PublicKey) {
        self.observe_provider_at(hash, provider, current_epoch_seconds());
    }

    /// Record a provider observation at an explicit time.
    pub fn observe_provider_at(&mut self, hash: Hash, provider: PublicKey, observed_at: u64) {
        self.observations.entry(hash).or_default().insert(provider, observed_at);
    }

    /// Return active leases for a blob.
    #[must_use]
    pub fn leases(&self, hash: &Hash) -> Vec<ProviderLease> {
        self.leases_at(hash, current_epoch_seconds())
    }

    /// Return active leases for a blob at an explicit time.
    #[must_use]
    pub fn leases_at(&self, hash: &Hash, now: u64) -> Vec<ProviderLease> {
        let mut leases = self
            .leases
            .get(hash)
            .map(|providers| {
                providers
                    .values()
                    .filter(|lease| !lease.is_expired_at(now))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        leases.sort_by(|left, right| left.provider.as_bytes().cmp(right.provider.as_bytes()));
        leases
    }

    /// Return active provider identities for a blob.
    #[must_use]
    pub fn providers(&self, hash: &Hash) -> Vec<PublicKey> {
        self.leases(hash).into_iter().map(|lease| lease.provider).collect()
    }

    /// Return the number of active, previously verified leases.
    #[must_use]
    pub fn verified_count(&self, hash: &Hash) -> usize {
        self.leases(hash).len()
    }

    /// Build a verified-vs-local availability report.
    #[must_use]
    pub fn health(&self, hash: &Hash, observation_ttl: Duration) -> AvailabilityHealth {
        self.health_at(hash, current_epoch_seconds(), observation_ttl)
    }

    /// Build an availability report at an explicit time.
    #[must_use]
    pub fn health_at(&self, hash: &Hash, now: u64, observation_ttl: Duration) -> AvailabilityHealth {
        let verified_providers = self
            .leases_at(hash, now)
            .into_iter()
            .map(|lease| lease.provider)
            .collect::<Vec<_>>();
        let ttl = observation_ttl.as_secs();
        let mut local_providers = self
            .observations
            .get(hash)
            .map(|observations| {
                observations
                    .iter()
                    .filter(|(_, observed_at)| now.saturating_sub(**observed_at) <= ttl)
                    .map(|(provider, _)| *provider)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        local_providers.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
        AvailabilityHealth {
            verified: verified_providers.len(),
            local: local_providers.len(),
            verified_providers,
            local_providers,
        }
    }

    /// Remove expired leases and stale local observations.
    pub fn purge(&mut self, now: u64, observation_ttl: Duration) {
        self.leases.retain(|_, providers| {
            providers.retain(|_, lease| !lease.is_expired_at(now));
            !providers.is_empty()
        });
        let ttl = observation_ttl.as_secs();
        self.observations.retain(|_, providers| {
            providers.retain(|_, observed_at| now.saturating_sub(*observed_at) <= ttl);
            !providers.is_empty()
        });
        self.purge_stale_failures(now, DEFAULT_FAILURE_TTL);
    }
}

/// Replication policy used by [`ResilienceService`].
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ReplicationBudget {
    pub min_providers: usize,
    pub responsible_peers: usize,
    pub max_jitter: Duration,
    pub observation_ttl: Duration,
}

impl ReplicationBudget {
    /// Create a budget with the default responsibility and timing settings.
    #[must_use]
    pub const fn new(min_providers: usize) -> Self {
        Self {
            min_providers,
            responsible_peers: DEFAULT_RESPONSIBLE_PEERS,
            max_jitter: DEFAULT_MAX_JITTER,
            observation_ttl: DEFAULT_OBSERVATION_TTL,
        }
    }

    /// Set the number of closest peers selected for a fetch.
    #[must_use]
    pub const fn with_responsible_peers(mut self, peers: usize) -> Self {
        self.responsible_peers = peers;
        self
    }

    /// Set the maximum deterministic jitter before a fetch.
    #[must_use]
    pub const fn with_max_jitter(mut self, jitter: Duration) -> Self {
        self.max_jitter = jitter;
        self
    }

    /// Set the lifetime of local provider observations.
    #[must_use]
    pub const fn with_observation_ttl(mut self, ttl: Duration) -> Self {
        self.observation_ttl = ttl;
        self
    }
}

impl Default for ReplicationBudget {
    fn default() -> Self {
        Self::new(3)
    }
}

/// Configuration for the resilience service.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ResilienceConfig {
    pub budget: ReplicationBudget,
    pub max_failures_per_provider: usize,
}

impl ResilienceConfig {
    #[must_use]
    pub const fn new(budget: ReplicationBudget) -> Self {
        Self {
            budget,
            max_failures_per_provider: DEFAULT_MAX_FAILURES_PER_PROVIDER,
        }
    }

    /// Set the maximum retained failure details per provider and blob.
    #[must_use]
    pub const fn with_max_failures_per_provider(mut self, max_failures_per_provider: usize) -> Self {
        self.max_failures_per_provider = max_failures_per_provider;
        self
    }
}

impl Default for ResilienceConfig {
    fn default() -> Self {
        Self::new(ReplicationBudget::default())
    }
}

/// Result of an attempted availability repair.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ReplicationResult {
    pub hash: Hash,
    pub health_before: AvailabilityHealth,
    pub health_after: AvailabilityHealth,
    pub selected_providers: Vec<PublicKey>,
    pub fetched_from: Vec<PublicKey>,
    pub failed_from: Vec<(PublicKey, FetchFailureKind)>,
    pub invalidated_leases: Vec<PublicKey>,
    pub pinned: bool,
    pub short_circuited: bool,
}

/// Result of waiting for a responsible fetch slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FetchWait {
    Ready,
    ShortCircuited,
}

/// Indexing-layer manager for leases, health, jitter, and repair fetches.
#[derive(Clone)]
pub struct ResilienceService {
    config: ResilienceConfig,
    tracker: Arc<Mutex<ProviderLeaseTracker>>,
    generations: Arc<Mutex<HashMap<Hash, watch::Sender<u64>>>>,
}

impl std::fmt::Debug for ResilienceService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResilienceService")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl ResilienceService {
    /// Create a resilience service from explicit configuration.
    #[must_use]
    pub fn new(config: ResilienceConfig) -> Self {
        let tracker = ProviderLeaseTracker::with_max_failures_per_provider(config.max_failures_per_provider);
        Self {
            config,
            tracker: Arc::new(Mutex::new(tracker)),
            generations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a resilience service using a replication budget.
    #[must_use]
    pub fn with_budget(budget: ReplicationBudget) -> Self {
        Self::new(ResilienceConfig::new(budget))
    }

    #[must_use]
    pub const fn config(&self) -> &ResilienceConfig {
        &self.config
    }

    /// Track a verified lease and wake pending fetches when it is new.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease is invalid or the service state lock is
    /// poisoned.
    pub fn record_lease(&self, lease: ProviderLease) -> Result<LeaseUpdate> {
        let hash = lease.hash;
        let update = self
            .tracker
            .lock()
            .map_err(|error| SyncwebError::operation("provider lease tracker lock poisoned", error))?
            .track(lease)?;
        if update.changed() {
            self.bump_generation(hash)?;
        }
        Ok(update)
    }

    /// Alias for [`Self::record_lease`].
    ///
    /// # Errors
    ///
    /// Returns an error if the lease is invalid or the service state lock is
    /// poisoned.
    pub fn track_lease(&self, lease: ProviderLease) -> Result<LeaseUpdate> {
        self.record_lease(lease)
    }

    /// Alias for [`Self::record_lease`].
    ///
    /// # Errors
    ///
    /// Returns an error if the lease is invalid or the service state lock is
    /// poisoned.
    pub fn ingest_lease(&self, lease: ProviderLease) -> Result<LeaseUpdate> {
        self.record_lease(lease)
    }

    /// Record a locally observed provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the service state lock is poisoned.
    pub fn observe_provider(&self, hash: Hash, provider: PublicKey) -> Result<()> {
        self.tracker
            .lock()
            .map_err(|error| SyncwebError::operation("provider lease tracker lock poisoned", error))?
            .observe_provider(hash, provider);
        Ok(())
    }

    /// Return the current health report for a blob.
    ///
    /// # Errors
    ///
    /// Returns an error if the service state lock is poisoned.
    pub fn health(&self, hash: &Hash) -> Result<AvailabilityHealth> {
        Ok(self
            .tracker
            .lock()
            .map_err(|error| SyncwebError::operation("provider lease tracker lock poisoned", error))?
            .health(hash, self.config.budget.observation_ttl))
    }

    /// Return whether a blob is below the configured verified lease budget.
    ///
    /// # Errors
    ///
    /// Returns an error if the service state lock is poisoned.
    pub fn needs_replication(&self, hash: &Hash) -> Result<bool> {
        Ok(self.health(hash)?.verified < self.config.budget.min_providers)
    }

    /// Return the closest active providers selected by consistent hashing.
    ///
    /// # Errors
    ///
    /// Returns an error if the service state lock is poisoned.
    pub fn responsible_providers(&self, hash: &Hash) -> Result<Vec<PublicKey>> {
        let providers = self
            .tracker
            .lock()
            .map_err(|error| SyncwebError::operation("provider lease tracker lock poisoned", error))?
            .providers(hash);
        Ok(consistent_hashing_selection(
            *hash,
            &providers,
            self.config.budget.responsible_peers,
        ))
    }

    /// Record a fetch failure in the shared provider tracker.
    ///
    /// # Errors
    ///
    /// Returns an error if the service state lock is poisoned.
    pub fn record_failure(&self, hash: Hash, provider: PublicKey, failure: FetchFailure) -> Result<()> {
        self.record_failure_at(hash, provider, failure, current_epoch_seconds())
    }

    /// Record a fetch failure at an explicit time.
    ///
    /// # Errors
    ///
    /// Returns an error if the service state lock is poisoned.
    pub fn record_failure_at(&self, hash: Hash, provider: PublicKey, failure: FetchFailure, now: u64) -> Result<()> {
        self.tracker
            .lock()
            .map_err(|error| SyncwebError::operation("provider lease tracker lock poisoned", error))?
            .record_failure_at(hash, provider, failure, now);
        Ok(())
    }

    /// Clear a provider's recorded failures after a successful fetch.
    ///
    /// # Errors
    ///
    /// Returns an error if the service state lock is poisoned.
    pub fn clear_failures_for_provider(&self, hash: &Hash, provider: &PublicKey) -> Result<()> {
        self.tracker
            .lock()
            .map_err(|error| SyncwebError::operation("provider lease tracker lock poisoned", error))?
            .clear_failures_for_provider(hash, provider);
        Ok(())
    }

    /// Return the deterministic jitter assigned to one provider.
    #[must_use]
    pub fn jitter_delay(&self, hash: Hash, provider: PublicKey) -> Duration {
        jitter_delay(hash, provider, self.config.budget.max_jitter)
    }

    /// Wait for a provider's jitter slot, cancelling when a newer lease is
    /// observed for the same blob.
    ///
    /// # Errors
    ///
    /// Returns an error if service state cannot be accessed.
    pub async fn wait_for_fetch_slot(&self, hash: Hash, provider: PublicKey) -> Result<FetchWait> {
        let mut generation = self.generation_receiver(hash)?;
        let delay = self.jitter_delay(hash, provider);
        tokio::select! {
            () = tokio::time::sleep(delay) => Ok(FetchWait::Ready),
            changed = generation.changed() => {
                changed.map_err(|error| SyncwebError::operation("provider lease cancellation channel closed", error))?;
                Ok(FetchWait::ShortCircuited)
            }
        }
    }

    /// Fetch and pin a blob when its verified availability is below budget.
    ///
    /// Providers are tried in consistent-hash order and each fetch is delayed
    /// by its provider-specific jitter. A newly accepted lease short-circuits
    /// the pending fetch instead of creating a thundering herd.
    ///
    /// # Errors
    ///
    /// Returns an error if state cannot be read, a selected provider ticket is
    /// invalid, or pinning fails. Provider fetch failures are returned in
    /// [`ReplicationResult::failed_from`] so another provider can be tried.
    pub async fn ensure_replication(
        &self,
        endpoint: &Endpoint,
        blobs: &BlobStore,
        hash: Hash,
    ) -> Result<ReplicationResult> {
        let health_before = self.health(&hash)?;
        let mut result = ReplicationResult {
            hash,
            health_before: health_before.clone(),
            health_after: health_before.clone(),
            selected_providers: Vec::new(),
            fetched_from: Vec::new(),
            failed_from: Vec::new(),
            invalidated_leases: Vec::new(),
            pinned: false,
            short_circuited: false,
        };
        if health_before.verified >= self.config.budget.min_providers {
            return Ok(result);
        }

        let leases = self
            .tracker
            .lock()
            .map_err(|error| SyncwebError::operation("provider lease tracker lock poisoned", error))?
            .leases(&hash);
        let providers = leases.iter().map(|lease| lease.provider).collect::<Vec<_>>();
        result.selected_providers =
            consistent_hashing_selection(hash, &providers, self.config.budget.responsible_peers);
        if result.selected_providers.is_empty() {
            return Ok(result);
        }

        if blobs.has(hash).await? {
            blobs.pin(replication_pin_name(hash), hash).await?;
            result.pinned = true;
            self.observe_provider(hash, endpoint.secret_key().public())?;
            result.health_after = self.health(&hash)?;
            return Ok(result);
        }
        let selected = result.selected_providers.clone();
        for provider in selected {
            match self.wait_for_fetch_slot(hash, provider).await? {
                FetchWait::ShortCircuited => {
                    result.short_circuited = true;
                    result.health_after = self.health(&hash)?;
                    return Ok(result);
                }
                FetchWait::Ready => {}
            }
            let Some(lease) = leases.iter().find(|lease| lease.provider == provider) else {
                continue;
            };
            let ticket = parse_ticket(&lease.ticket)?;
            match blobs.fetch(endpoint, &ticket).await {
                Ok(()) => {
                    result.fetched_from.push(provider);
                    self.clear_failures_for_provider(&hash, &provider)?;
                    break;
                }
                Err(error) => {
                    let kind = FetchFailureKind::from_syncweb_error(&error);
                    let failure = FetchFailure::from_syncweb_error(provider, hash, &error);
                    self.record_failure(hash, provider, failure)?;
                    result.failed_from.push((provider, kind));
                }
            }
        }
        if result.fetched_from.is_empty() {
            result.health_after = self.health(&hash)?;
            return Ok(result);
        }

        blobs.pin(replication_pin_name(hash), hash).await?;
        result.pinned = true;
        self.observe_provider(hash, endpoint.secret_key().public())?;
        result.health_after = self.health(&hash)?;
        Ok(result)
    }

    /// Alias for [`Self::ensure_replication`].
    ///
    /// # Errors
    ///
    /// Returns an error if the repair fetch fails.
    pub async fn replicate(&self, endpoint: &Endpoint, blobs: &BlobStore, hash: Hash) -> Result<ReplicationResult> {
        self.ensure_replication(endpoint, blobs, hash).await
    }

    /// Subscribe to the provider-lease gossip topic.
    ///
    /// # Errors
    ///
    /// Returns an error if the gossip subscription cannot be created.
    pub async fn subscribe(&self, gossip: &GossipService, bootstrap: Vec<PublicKey>) -> Result<GossipTopic> {
        gossip.subscribe(resilience_topic(), bootstrap).await
    }

    /// Publish a signed provider lease to gossip.
    ///
    /// # Errors
    ///
    /// Returns an error if the lease is unsigned, invalid, or cannot be sent.
    pub async fn announce(&self, gossip: &GossipService, sender: &GossipSender, lease: &ProviderLease) -> Result<()> {
        lease.verify_at(current_epoch_seconds())?;
        gossip.publish(sender, lease.to_bytes()?).await
    }

    /// Consume provider leases until the topic closes or `timeout` expires.
    ///
    /// A timeout is a normal end condition because gossip has no finite
    /// response boundary.
    ///
    /// # Errors
    ///
    /// Returns an error if a gossip event or lease is malformed.
    pub async fn consume_gossip(&self, topic: &mut GossipTopic, timeout: Duration) -> Result<usize> {
        let mut tracked = 0_usize;
        let receive = async {
            while let Some(event) = topic.next().await {
                if let Event::Received(message) =
                    event.map_err(|error| SyncwebError::operation("provider lease gossip event failed", error))?
                {
                    let lease = ProviderLease::from_bytes(message.content)?;
                    self.record_lease(lease)?;
                    tracked = tracked.saturating_add(1);
                }
            }
            Ok::<(), SyncwebError>(())
        };
        if let Ok(result) = tokio::time::timeout(timeout, receive).await {
            result?;
        }
        Ok(tracked)
    }

    /// Spawn a background provider-lease gossip consumer.
    #[must_use]
    pub fn spawn_gossip_listener(&self, mut topic: GossipTopic) -> JoinHandle<Result<usize>> {
        let service = self.clone();
        tokio::spawn(async move { service.consume_gossip(&mut topic, Duration::MAX).await })
    }

    fn generation_receiver(&self, hash: Hash) -> Result<watch::Receiver<u64>> {
        let mut generations = self
            .generations
            .lock()
            .map_err(|error| SyncwebError::operation("provider lease generation lock poisoned", error))?;
        Ok(generations
            .entry(hash)
            .or_insert_with(|| watch::channel(0_u64).0)
            .subscribe())
    }

    fn bump_generation(&self, hash: Hash) -> Result<()> {
        {
            let mut generations = self
                .generations
                .lock()
                .map_err(|error| SyncwebError::operation("provider lease generation lock poisoned", error))?;
            let sender = generations.entry(hash).or_insert_with(|| watch::channel(0_u64).0);
            sender.send_modify(|generation| *generation = generation.saturating_add(1));
            drop(generations);
        }
        Ok(())
    }
}

/// The deterministic gossip topic used for provider leases.
#[must_use]
pub fn resilience_topic() -> TopicId {
    TopicId::from_bytes(*blake3::hash(RESILIENCE_TOPIC_SEED).as_bytes())
}

/// Select the `count` providers closest to a blob's hash.
///
/// The XOR distance is compared lexicographically, making selection stable
/// across nodes without a central coordinator.
#[must_use]
pub fn consistent_hashing_selection(hash: Hash, providers: &[PublicKey], count: usize) -> Vec<PublicKey> {
    if count == 0 {
        return Vec::new();
    }
    let mut unique = HashSet::new();
    let mut ranked = providers
        .iter()
        .copied()
        .filter(|provider| unique.insert(*provider))
        .map(|provider| (xor_distance(hash, provider), provider))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.as_bytes().cmp(right.1.as_bytes())));
    ranked.truncate(count);
    ranked.into_iter().map(|(_, provider)| provider).collect()
}

/// Return the XOR distance between a blob hash and a provider identity.
#[must_use]
pub fn xor_distance(hash: Hash, provider: PublicKey) -> [u8; 32] {
    let mut distance = [0_u8; 32];
    for ((byte, hash_byte), provider_byte) in distance.iter_mut().zip(hash.as_bytes()).zip(provider.as_bytes()) {
        *byte = *hash_byte ^ *provider_byte;
    }
    distance
}

/// Return a stable pseudo-random delay in `[0, max]` for a provider.
///
/// The delay is derived from both identities rather than from wall-clock
/// state, so every node assigns the same slot to a provider while different
/// providers normally receive different slots.
#[must_use]
pub fn jitter_delay(hash: Hash, provider: PublicKey, max: Duration) -> Duration {
    let max_millis = u64::try_from(max.as_millis()).unwrap_or(u64::MAX);
    if max_millis == 0 {
        return Duration::ZERO;
    }
    let mut seed = Vec::with_capacity(64);
    seed.extend_from_slice(hash.as_bytes());
    seed.extend_from_slice(provider.as_bytes());
    let digest = blake3::hash(&seed);
    let mut value_bytes = [0_u8; 8];
    for (target, source) in value_bytes.iter_mut().zip(digest.as_bytes().iter().take(8)) {
        *target = *source;
    }
    let value = u64::from_le_bytes(value_bytes);
    let divisor = max_millis.saturating_add(1);
    Duration::from_millis(value.checked_rem(divisor).unwrap_or(0))
}

fn replication_pin_name(hash: Hash) -> String {
    format!("{REPLICATION_PIN_PREFIX}{hash}")
}

fn parse_ticket(ticket: &str) -> Result<BlobTicket> {
    BlobTicket::from_str(ticket).map_err(|error| SyncwebError::InvalidTicket(error.to_string()))
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, time::Duration};

    use anyhow::Result;
    use iroh::{EndpointAddr, SecretKey};
    use iroh_blobs::{BlobFormat, Hash};

    use super::*;

    fn signed_lease(seed: u8, hash: Hash, sequence: u64) -> Result<(ProviderLease, SecretKey)> {
        let secret_key = SecretKey::from_bytes(&[seed; 32]);
        let ticket = BlobTicket::new(EndpointAddr::new(secret_key.public()), hash, BlobFormat::Raw).to_string();
        let mut lease = ProviderLease::new_with_times(hash, ticket, sequence, 0, u64::MAX)?;
        lease.sign_with_secret_key(&secret_key)?;
        Ok((lease, secret_key))
    }

    #[test]
    fn provider_leases_are_signed_and_monotonic() -> Result<()> {
        let hash = Hash::from_bytes([7_u8; 32]);
        let (lease, provider_key) = signed_lease(1, hash, 1)?;
        let encoded = lease.to_bytes()?;
        let decoded = ProviderLease::from_bytes(encoded)?;
        decoded.verify_at(10)?;

        let mut tracker = ProviderLeaseTracker::default();
        anyhow::ensure!(
            tracker.track_at(decoded.clone(), 10)? == LeaseUpdate::Inserted,
            "first lease should be inserted"
        );
        anyhow::ensure!(
            tracker.track_at(decoded, 10)? == LeaseUpdate::IgnoredOlder,
            "same sequence should be ignored"
        );
        let (replacement, _) = signed_lease(1, hash, 2)?;
        anyhow::ensure!(
            tracker.track_at(replacement, 10)? == LeaseUpdate::Replaced,
            "newer sequence should replace the old lease"
        );
        anyhow::ensure!(tracker.verified_count(&hash) == 1, "one provider should remain tracked");
        tracker.observe_provider_at(hash, provider_key.public(), 10);
        let health = tracker.health_at(&hash, 20, Duration::from_secs(30));
        anyhow::ensure!(health.verified == 1, "one verified lease should be reported");
        anyhow::ensure!(health.local == 1, "one local observation should be reported");
        Ok(())
    }

    #[test]
    fn consistent_selection_and_jitter_are_stable() -> Result<()> {
        let hash = Hash::from_bytes([9_u8; 32]);
        let (_, first_key) = signed_lease(1, hash, 1)?;
        let (_, second_key) = signed_lease(2, hash, 1)?;
        let (_, third_key) = signed_lease(3, hash, 1)?;
        let providers = vec![third_key.public(), first_key.public(), second_key.public()];
        let selected = consistent_hashing_selection(hash, &providers, 2);
        anyhow::ensure!(
            selected == consistent_hashing_selection(hash, &providers, 2),
            "selection should be stable"
        );
        anyhow::ensure!(selected.len() == 2, "selection should honor its limit");
        anyhow::ensure!(
            selected.iter().all(|provider| providers.contains(provider)),
            "selection should only contain candidates"
        );

        let first_delay = jitter_delay(hash, first_key.public(), Duration::from_millis(500));
        let second_delay = jitter_delay(hash, second_key.public(), Duration::from_millis(500));
        anyhow::ensure!(
            first_delay <= Duration::from_millis(500),
            "first jitter should stay within the configured window"
        );
        anyhow::ensure!(
            second_delay <= Duration::from_millis(500),
            "second jitter should stay within the configured window"
        );
        anyhow::ensure!(
            first_delay != second_delay,
            "providers should receive different jitter slots"
        );
        Ok(())
    }

    #[tokio::test]
    async fn a_new_lease_short_circuits_a_pending_fetch() -> Result<()> {
        let hash = Hash::from_bytes([11_u8; 32]);
        let (first_lease, first_key) = signed_lease(1, hash, 1)?;
        let (second_lease, _) = signed_lease(2, hash, 1)?;
        let budget = ReplicationBudget::new(3).with_max_jitter(Duration::from_secs(1));
        let service = ResilienceService::with_budget(budget);
        service.record_lease(first_lease)?;
        let delay = service.jitter_delay(hash, first_key.public());
        if delay.is_zero() {
            return Ok(());
        }
        let waiting = service.clone();
        let task = tokio::spawn(async move { waiting.wait_for_fetch_slot(hash, first_key.public()).await });
        tokio::time::sleep(Duration::from_millis(10)).await;
        service.record_lease(second_lease)?;
        anyhow::ensure!(
            task.await
                .map_err(|error| SyncwebError::operation("fetch wait task failed", error))??
                == FetchWait::ShortCircuited,
            "new lease should cancel the pending fetch"
        );
        Ok(())
    }

    #[test]
    fn health_verified_vs_local_are_distinct() -> Result<()> {
        let hash = Hash::from_bytes([13_u8; 32]);
        let (lease, key) = signed_lease(1, hash, 1)?;

        let local_only = Hash::from_bytes([14_u8; 32]);
        let (_, local_key) = signed_lease(2, local_only, 1)?;

        let service = ResilienceService::with_budget(ReplicationBudget::new(1));
        service.record_lease(lease.clone())?;
        service.observe_provider(local_only, local_key.public())?;

        let health = service.health(&hash)?;
        anyhow::ensure!(health.verified == 1, "one verified lease should be present");
        anyhow::ensure!(health.local == 0, "no local observations for this hash");
        anyhow::ensure!(
            health.verified_providers == vec![key.public()],
            "verified providers should list the lease key"
        );

        let local_health = service.health(&local_only)?;
        anyhow::ensure!(local_health.verified == 0, "no verified leases for local-only hash");
        anyhow::ensure!(local_health.local == 1, "one local observation should be present");
        anyhow::ensure!(
            local_health.local_providers == vec![local_key.public()],
            "local providers should list the observed key"
        );

        anyhow::ensure!(
            !service.needs_replication(&hash)?,
            "min_providers=1 is satisfied by one verified lease"
        );
        anyhow::ensure!(
            service.needs_replication(&local_only)?,
            "hash with no verified leases should need replication"
        );

        let high_budget = ResilienceService::with_budget(ReplicationBudget::new(3));
        high_budget.record_lease(lease)?;
        anyhow::ensure!(
            high_budget.needs_replication(&hash)?,
            "min_providers=3 is not satisfied by one verified lease"
        );

        Ok(())
    }

    #[test]
    fn purge_removes_expired_leases_and_stale_observations() -> Result<()> {
        let hash = Hash::from_bytes([15_u8; 32]);
        let sk1 = SecretKey::from_bytes(&[1; 32]);
        let ticket1 = BlobTicket::new(EndpointAddr::new(sk1.public()), hash, BlobFormat::Raw).to_string();
        let mut lease_a = ProviderLease::new_with_times(hash, ticket1, 1, 10, 50)?;
        lease_a.sign_with_secret_key(&sk1)?;

        let sk2 = SecretKey::from_bytes(&[2; 32]);
        let ticket2 = BlobTicket::new(EndpointAddr::new(sk2.public()), hash, BlobFormat::Raw).to_string();
        let mut lease_b = ProviderLease::new_with_times(hash, ticket2, 1, 10, 200)?;
        lease_b.sign_with_secret_key(&sk2)?;

        let mut tracker = ProviderLeaseTracker::default();
        tracker.track_at(lease_a, 10)?;
        tracker.track_at(lease_b, 10)?;
        tracker.observe_provider_at(hash, sk1.public(), 10);
        anyhow::ensure!(tracker.leases_at(&hash, 15).len() == 2, "both leases tracked");

        tracker.purge(15, Duration::from_secs(5));
        anyhow::ensure!(tracker.leases_at(&hash, 15).len() == 2, "both active at t=15");

        tracker.purge(55, Duration::from_secs(5));
        anyhow::ensure!(tracker.leases_at(&hash, 55).len() == 1, "lease_a expired at t=55");

        tracker.purge(200, Duration::from_secs(5));
        anyhow::ensure!(tracker.leases_at(&hash, 200).is_empty(), "all expired at t=200");

        Ok(())
    }

    #[test]
    fn fetch_failure_kind_classification_and_definitiveness() {
        let not_found = SyncwebError::operation("fetch", "provider blob not found");
        let refused = SyncwebError::operation("fetch", "connection refused");
        let timeout = SyncwebError::operation("fetch", "request timed out");
        let corruption = SyncwebError::operation("fetch", "hash mismatch");
        let unknown = SyncwebError::operation("fetch", "unexpected provider response");

        assert_eq!(
            FetchFailureKind::from_syncweb_error(&not_found),
            FetchFailureKind::NotFound
        );
        assert_eq!(
            FetchFailureKind::from_syncweb_error(&refused),
            FetchFailureKind::ConnectionRefused
        );
        assert_eq!(
            FetchFailureKind::from_syncweb_error(&timeout),
            FetchFailureKind::Timeout
        );
        assert_eq!(
            FetchFailureKind::from_syncweb_error(&corruption),
            FetchFailureKind::Corruption
        );
        assert_eq!(
            FetchFailureKind::from_syncweb_error(&unknown),
            FetchFailureKind::Unknown
        );
        assert!(FetchFailureKind::NotFound.is_definitive());
        assert!(FetchFailureKind::Corruption.is_definitive());
        assert!(FetchFailureKind::Timeout.is_transient());
        assert!(!FetchFailureKind::Timeout.is_definitive());
    }

    #[test]
    fn fetch_failure_round_trip_and_timestamp() -> Result<()> {
        let hash = Hash::from_bytes([21_u8; 32]);
        let provider = SecretKey::from_bytes(&[21_u8; 32]).public();
        let before = current_epoch_seconds();
        let failure = FetchFailure::new(FetchFailureKind::Timeout, provider, hash, "timed out");
        let after = current_epoch_seconds();

        anyhow::ensure!(
            (before..=after).contains(&failure.timestamp),
            "failure timestamp should use the current epoch"
        );
        anyhow::ensure!(
            FetchFailure::from_bytes(failure.to_bytes()?)? == failure,
            "fetch failures should round-trip through JSON"
        );
        Ok(())
    }

    #[test]
    fn failure_tracking_counts_clears_and_purges() -> Result<()> {
        let hash = Hash::from_bytes([22_u8; 32]);
        let other_hash = Hash::from_bytes([23_u8; 32]);
        let provider = SecretKey::from_bytes(&[22_u8; 32]).public();
        let other_provider = SecretKey::from_bytes(&[23_u8; 32]).public();
        let mut tracker = ProviderLeaseTracker::with_max_failures_per_provider(2);

        for timestamp in 1..=3 {
            tracker.record_failure_at(
                hash,
                provider,
                FetchFailure::new_at(FetchFailureKind::NotFound, provider, hash, timestamp, "missing"),
                timestamp,
            );
        }
        tracker.record_failure_at(
            other_hash,
            provider,
            FetchFailure::new_at(FetchFailureKind::NotFound, provider, other_hash, 4, "missing"),
            4,
        );
        tracker.record_failure_at(
            hash,
            other_provider,
            FetchFailure::new_at(FetchFailureKind::NotFound, other_provider, hash, 4, "missing"),
            4,
        );

        let record = tracker.failure_record(&hash, &provider).expect("failure record");
        anyhow::ensure!(record.failures.len() == 2, "failure details should be capped");
        anyhow::ensure!(
            record.failures.first().is_some_and(|failure| failure.timestamp == 2),
            "oldest detail should be evicted"
        );
        anyhow::ensure!(
            tracker.failure_count(&hash, &provider) == 3,
            "aggregate count should be retained"
        );
        anyhow::ensure!(tracker.consecutive_failures(&hash, &provider) == 3);
        anyhow::ensure!(tracker.is_definitively_failed(&hash, &provider));
        anyhow::ensure!(!tracker.is_definitively_failed(&other_hash, &provider));
        anyhow::ensure!(!tracker.is_definitively_failed(&hash, &other_provider));

        tracker.clear_failures_for_provider(&hash, &provider);
        anyhow::ensure!(tracker.failure_record(&hash, &provider).is_none());
        anyhow::ensure!(tracker.failure_count(&hash, &provider) == 0);

        tracker.purge_stale_failures(10, Duration::from_secs(1));
        anyhow::ensure!(tracker.failure_record(&other_hash, &provider).is_none());
        anyhow::ensure!(tracker.failure_record(&hash, &other_provider).is_none());
        Ok(())
    }

    #[test]
    fn transient_failures_do_not_become_definitive() {
        let hash = Hash::from_bytes([24_u8; 32]);
        let provider = SecretKey::from_bytes(&[24_u8; 32]).public();
        let mut tracker = ProviderLeaseTracker::default();
        for timestamp in 1..=3 {
            tracker.record_failure_at(
                hash,
                provider,
                FetchFailure::new_at(FetchFailureKind::Timeout, provider, hash, timestamp, "timeout"),
                timestamp,
            );
        }
        assert_eq!(tracker.consecutive_failures(&hash, &provider), 3);
        assert!(!tracker.is_definitively_failed(&hash, &provider));
    }

    #[tokio::test]
    async fn bounded_fetch_validation_rejects_oversized_truncated_and_corrupt_streams() -> Result<()> {
        let data = b"bounded fetch";
        let hash = Hash::new(data);
        let expected_size = u64::try_from(data.len())?;
        validate_bounded_fetch(Cursor::new(data), expected_size, hash).await?;
        anyhow::ensure!(
            validate_bounded_fetch(Cursor::new(b"bounded fetch!"), expected_size, hash)
                .await
                .is_err(),
            "oversized streams should be rejected"
        );
        anyhow::ensure!(
            validate_bounded_fetch(Cursor::new(b"bounded"), expected_size, hash)
                .await
                .is_err(),
            "truncated streams should be rejected"
        );
        anyhow::ensure!(
            validate_bounded_fetch(Cursor::new(b"wrong fetch"), expected_size, hash)
                .await
                .is_err(),
            "hash mismatches should be rejected"
        );
        Ok(())
    }
}
