//! Historical provider reputation and signed trust observations.
//!
//! Provider leases describe what a node claims to serve now. Reputation is
//! deliberately separate: it records what this node has observed over time
//! and can be combined with the local Web-of-Trust policy at decision time.

use std::{
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use iroh::{PublicKey, SecretKey};
use iroh_blobs::Hash;
use iroh_gossip::{
    TopicId,
    api::{GossipSender, GossipTopic},
};
use serde::{Deserialize, Serialize};

use super::{resilience::FetchFailureKind, wot::TrustPolicy};
use crate::{
    error::{Result, SyncwebError},
    node::gossip_service::GossipService,
};

const REPUTATION_SIGNAL_CONTEXT: &[u8] = b"syncweb/provider-trust/v1\0";
const TRUST_STREAM_TOPIC_SEED: &[u8] = b"syncweb/provider-trust-stream/v1";
const DEFAULT_DECAY_HALF_LIFE: Duration = Duration::from_hours(24);
const DEFAULT_TEMPORARY_BAN: Duration = Duration::from_hours(1);
const DEFAULT_BACKOFF_FACTOR: f64 = 2.0;
const DEFAULT_MAX_BAN: Duration = Duration::from_hours(24 * 30);
const DEFAULT_SIGNAL_TTL: Duration = Duration::from_hours(168);
const DEFAULT_SIGNAL_BATCH_SIZE: usize = 64;
const AUTO_BAN_FAILURE_THRESHOLD: u32 = 3;

/// Historical fetch outcomes for one provider.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProviderReputation {
    pub provider: PublicKey,
    pub total_fetches: u64,
    pub successful_fetches: u64,
    pub failed_fetches: u64,
    pub consecutive_failures: u32,
    pub last_success_at: Option<u64>,
    pub last_failure_at: Option<u64>,
}

impl ProviderReputation {
    #[must_use]
    pub const fn new(provider: PublicKey) -> Self {
        Self {
            provider,
            total_fetches: 0,
            successful_fetches: 0,
            failed_fetches: 0,
            consecutive_failures: 0,
            last_success_at: None,
            last_failure_at: None,
        }
    }

    pub const fn record_success(&mut self, now: u64) {
        self.total_fetches = self.total_fetches.saturating_add(1);
        self.successful_fetches = self.successful_fetches.saturating_add(1);
        self.consecutive_failures = 0;
        self.last_success_at = Some(now);
    }

    pub const fn record_failure(&mut self, _kind: FetchFailureKind, now: u64) {
        self.total_fetches = self.total_fetches.saturating_add(1);
        self.failed_fetches = self.failed_fetches.saturating_add(1);
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.last_failure_at = Some(now);
    }

    /// Return a reliability score in `[0.0, 1.0]`.
    ///
    /// The standalone score uses the default 24-hour half-life. Stores apply
    /// their configured half-life and failure weight through the same formula.
    #[must_use]
    pub fn reliability_score(&self, now: u64) -> f64 {
        self.reliability_score_with(now, DEFAULT_DECAY_HALF_LIFE, 2.0)
    }

    #[must_use]
    pub fn is_reliable(&self, threshold: f64) -> bool {
        self.reliability_score(current_epoch_seconds()) > threshold
    }

    #[must_use]
    pub fn is_reliable_at(&self, threshold: f64, now: u64) -> bool {
        self.reliability_score(now) > threshold
    }

    #[must_use]
    pub const fn should_auto_ban(&self, consecutive_threshold: u32) -> bool {
        self.consecutive_failures >= consecutive_threshold
    }

    fn reliability_score_with(&self, now: u64, half_life: Duration, failure_weight: f64) -> f64 {
        if self.total_fetches == 0 {
            return 0.5;
        }
        let failure_count = u32::try_from(self.failed_fetches.min(u64::from(u32::MAX))).unwrap_or(u32::MAX);
        let success_count = u32::try_from(self.successful_fetches.min(u64::from(u32::MAX))).unwrap_or(u32::MAX);
        let failures = std::ops::Mul::mul(failure_weight.max(0.0), f64::from(failure_count));
        let denominator = std::ops::Add::add(f64::from(success_count), failures);
        let base = if denominator == 0.0 {
            0.5
        } else {
            std::ops::Div::div(f64::from(success_count), denominator)
        };
        let latest = self.last_success_at.max(self.last_failure_at).unwrap_or(now);
        let age_seconds = now.saturating_sub(latest).min(u64::from(u32::MAX));
        let age = f64::from(u32::try_from(age_seconds).unwrap_or(u32::MAX));
        let half_life_seconds = half_life.as_secs_f64().max(1.0);
        let decay = 0.5_f64.powf(std::ops::Div::div(age, half_life_seconds));
        std::ops::Sub::sub(base, 0.5).mul_add(decay, 0.5).clamp(0.0, 1.0)
    }
}

/// Configuration for historical reputation scoring.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct ReputationConfig {
    pub min_samples: usize,
    pub decay_half_life: Duration,
    pub failure_weight: f64,
    pub temporary_ban_duration: Duration,
    pub auto_ban_backoff_factor: f64,
    pub max_auto_ban_duration: Duration,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            min_samples: 5,
            decay_half_life: DEFAULT_DECAY_HALF_LIFE,
            failure_weight: 2.0,
            temporary_ban_duration: DEFAULT_TEMPORARY_BAN,
            auto_ban_backoff_factor: DEFAULT_BACKOFF_FACTOR,
            max_auto_ban_duration: DEFAULT_MAX_BAN,
        }
    }
}

#[derive(Clone, Debug)]
struct AutoBan {
    until: u64,
    count: u32,
}

/// Local provider reputation database.
#[derive(Clone, Debug)]
pub struct ProviderReputationStore {
    reputations: HashMap<PublicKey, ProviderReputation>,
    config: ReputationConfig,
    policy: TrustPolicy,
    auto_bans: HashMap<PublicKey, AutoBan>,
    signal_sequences: HashMap<(PublicKey, PublicKey), u64>,
    pending_signals: Vec<ProviderTrustSignal>,
    max_signal_batch: usize,
    reporter: Option<PublicKey>,
    next_signal_sequence: u64,
}

impl Default for ProviderReputationStore {
    fn default() -> Self {
        Self::new(ReputationConfig::default())
    }
}

impl ProviderReputationStore {
    #[must_use]
    pub fn new(config: ReputationConfig) -> Self {
        Self {
            reputations: HashMap::new(),
            config,
            policy: TrustPolicy::new(),
            auto_bans: HashMap::new(),
            signal_sequences: HashMap::new(),
            pending_signals: Vec::new(),
            max_signal_batch: DEFAULT_SIGNAL_BATCH_SIZE,
            reporter: None,
            next_signal_sequence: 1,
        }
    }

    #[must_use]
    pub fn with_policy(config: ReputationConfig, policy: TrustPolicy) -> Self {
        let mut store = Self::new(config);
        store.policy = policy;
        store
    }

    #[must_use]
    pub const fn config(&self) -> &ReputationConfig {
        &self.config
    }

    pub fn set_policy(&mut self, policy: TrustPolicy) {
        self.policy = policy;
    }

    /// Configure the reporter identity used for reputation transitions.
    pub const fn set_reporter(&mut self, reporter: PublicKey) {
        self.reporter = Some(reporter);
    }

    /// # Errors
    ///
    /// Returns an error when the reporter identity is not a valid Ed25519 key.
    pub fn trust_reporter(&mut self, reporter: &PublicKey) -> Result<()> {
        let verifying_key = VerifyingKey::from_bytes(reporter.as_bytes())
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid trust reporter: {error}")))?;
        self.policy.trust_author(&verifying_key);
        Ok(())
    }

    #[must_use]
    pub fn reputation(&self, provider: PublicKey) -> ProviderReputation {
        self.reputations
            .get(&provider)
            .cloned()
            .unwrap_or_else(|| ProviderReputation::new(provider))
    }

    #[must_use]
    pub fn score(&self, provider: PublicKey, now: u64) -> f64 {
        let reputation = self.reputation(provider);
        if usize::try_from(reputation.total_fetches).unwrap_or(usize::MAX) < self.config.min_samples {
            0.5
        } else {
            reputation.reliability_score_with(now, self.config.decay_half_life, self.config.failure_weight)
        }
    }

    pub fn record_fetch_result(&mut self, provider: PublicKey, success: bool, kind: FetchFailureKind, now: u64) {
        let previous_score = self.score(provider, now);
        let reputation = self
            .reputations
            .entry(provider)
            .or_insert_with(|| ProviderReputation::new(provider));
        if success {
            reputation.record_success(now);
        } else {
            reputation.record_failure(kind, now);
            if reputation.should_auto_ban(AUTO_BAN_FAILURE_THRESHOLD) && !self.is_banned(provider, now) {
                self.apply_auto_ban(provider, now);
            }
        }
        let current_score = self.score(provider, now);
        let crossed_to_unreliable = previous_score >= 0.5 && current_score < 0.5;
        let crossed_to_reliable = previous_score < 0.5 && current_score >= 0.5;
        if (crossed_to_unreliable || crossed_to_reliable)
            && let Some(reporter) = self.reporter
        {
            let signal_kind = if crossed_to_reliable {
                TrustSignalKind::ObservedSuccess
            } else {
                TrustSignalKind::ObservedFailure
            };
            let signal = ProviderTrustSignal {
                provider,
                signal: signal_kind,
                hash: None,
                reporter,
                timestamp: now,
                sequence: self.next_signal_sequence,
                signature: None,
            };
            self.next_signal_sequence = self.next_signal_sequence.saturating_add(1);
            self.enqueue_signal(signal);
        }
    }

    pub fn record_success(&mut self, provider: PublicKey, now: u64) {
        self.record_fetch_result(provider, true, FetchFailureKind::Unknown, now);
    }

    pub fn record_failure(&mut self, provider: PublicKey, kind: FetchFailureKind, now: u64) {
        self.record_fetch_result(provider, false, kind, now);
    }

    #[must_use]
    pub fn is_banned(&self, provider: PublicKey, now: u64) -> bool {
        self.auto_bans.get(&provider).is_some_and(|ban| ban.until > now)
    }

    #[must_use]
    pub fn auto_ban_until(&self, provider: PublicKey) -> Option<u64> {
        self.auto_bans.get(&provider).map(|ban| ban.until)
    }

    #[must_use]
    pub fn auto_ban_count(&self, provider: PublicKey) -> u32 {
        self.auto_bans.get(&provider).map_or(0, |ban| ban.count)
    }

    /// Clear the active timeout while retaining its backoff history.
    pub fn rejoin(&mut self, provider: PublicKey, now: u64) -> bool {
        self.auto_bans.get(&provider).is_some_and(|ban| ban.until <= now)
    }

    #[must_use]
    pub fn should_skip_provider(&self, provider: PublicKey, now: u64, threshold: f64) -> bool {
        self.is_banned(provider, now) || self.score(provider, now) < threshold
    }

    /// Return all known providers, ranked by score and then hash distance.
    #[must_use]
    pub fn rank_providers(&self, now: u64, hash: Hash) -> Vec<PublicKey> {
        let providers = self.reputations.keys().copied().collect::<Vec<_>>();
        self.rank_provider_list(now, hash, &providers)
    }

    #[must_use]
    pub fn rank_provider_list(&self, now: u64, hash: Hash, providers: &[PublicKey]) -> Vec<PublicKey> {
        let mut unique = HashSet::new();
        let mut ranked = providers
            .iter()
            .copied()
            .filter(|provider| unique.insert(*provider))
            .map(|provider| (self.score(provider, now), provider))
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .0
                .total_cmp(&left.0)
                .then_with(|| xor_distance(hash, right.1).cmp(&xor_distance(hash, left.1)))
                .then_with(|| left.1.as_bytes().cmp(right.1.as_bytes()))
        });
        ranked.into_iter().map(|(_, provider)| provider).collect()
    }

    pub fn purge_stale(&mut self, now: u64, ttl: Duration) {
        let ttl_seconds = ttl.as_secs();
        self.reputations.retain(|_, reputation| {
            let last = reputation.last_success_at.max(reputation.last_failure_at);
            last.is_some_and(|timestamp| now.saturating_sub(timestamp) <= ttl_seconds)
        });
        self.auto_bans.retain(|_, ban| ban.until > now || ban.count > 0);
    }

    /// Queue one observation, coalescing identical observations in the batch.
    /// # Errors
    ///
    /// Returns an error when the signal structure is invalid.
    pub fn queue_trust_signal(&mut self, signal: ProviderTrustSignal) -> Result<bool> {
        signal.validate()?;
        Ok(self.enqueue_signal(signal))
    }

    fn enqueue_signal(&mut self, signal: ProviderTrustSignal) -> bool {
        if self.pending_signals.iter().any(|existing| {
            existing.provider == signal.provider
                && existing.reporter == signal.reporter
                && existing.signal == signal.signal
                && existing.hash == signal.hash
        }) {
            return false;
        }
        if self.pending_signals.len() >= self.max_signal_batch {
            self.pending_signals.remove(0);
        }
        self.pending_signals.push(signal);
        true
    }

    #[must_use]
    pub fn pending_trust_signals(&self) -> &[ProviderTrustSignal] {
        &self.pending_signals
    }

    pub fn take_pending_trust_signals(&mut self) -> Vec<ProviderTrustSignal> {
        std::mem::take(&mut self.pending_signals)
    }

    pub fn set_max_signal_batch(&mut self, max_signal_batch: usize) {
        self.max_signal_batch = max_signal_batch.max(1);
        let keep_from = self.pending_signals.len().saturating_sub(self.max_signal_batch);
        if keep_from > 0 {
            self.pending_signals.drain(..keep_from);
        }
    }

    /// Accept a signed observation from a locally trusted reporter.
    /// # Errors
    ///
    /// Returns an error when the signal is malformed, expired, or unsigned.
    pub fn ingest_trust_signal(&mut self, signal: ProviderTrustSignal) -> Result<bool> {
        signal.verify()?;
        let ProviderTrustSignal {
            provider,
            signal: kind,
            hash,
            reporter,
            timestamp,
            sequence,
            signature,
        } = signal;
        drop(signature);
        let reporter_id = hex::encode(reporter.as_bytes());
        if !self.policy.is_trusted_for_at(&reporter_id, hash.as_ref(), timestamp) {
            return Ok(false);
        }
        let sequence_key = (reporter, provider);
        if self
            .signal_sequences
            .get(&sequence_key)
            .is_some_and(|last_sequence| *last_sequence >= sequence)
        {
            return Ok(false);
        }
        self.signal_sequences.insert(sequence_key, sequence);
        match kind {
            TrustSignalKind::ObservedSuccess => {
                self.record_fetch_result(provider, true, FetchFailureKind::Unknown, timestamp);
            }
            TrustSignalKind::ObservedFailure => {
                self.record_fetch_result(provider, false, FetchFailureKind::Unknown, timestamp);
            }
            TrustSignalKind::ObservedCorruption => {
                self.record_fetch_result(provider, false, FetchFailureKind::Corruption, timestamp);
            }
        }
        Ok(true)
    }

    /// Publish a signed signal to the provider-trust stream.
    ///
    /// # Errors
    ///
    /// Returns an error when the signal is invalid or gossip rejects the
    /// publication.
    pub async fn publish_signal(
        &self,
        gossip: &GossipService,
        sender: &GossipSender,
        signal: &ProviderTrustSignal,
    ) -> Result<()> {
        signal.verify()?;
        gossip.publish(sender, signal.to_bytes()?).await
    }

    /// Subscribe to the provider-trust stream.
    ///
    /// # Errors
    ///
    /// Returns an error when the gossip subscription cannot be created.
    pub async fn subscribe_trust_stream(
        &self,
        gossip: &GossipService,
        bootstrap: Vec<PublicKey>,
    ) -> Result<GossipTopic> {
        gossip.subscribe(trust_stream_topic(), bootstrap).await
    }

    fn apply_auto_ban(&mut self, provider: PublicKey, now: u64) {
        let previous_count = self.auto_bans.get(&provider).map_or(0, |ban| ban.count);
        let count = previous_count.saturating_add(1);
        let exponent = i32::try_from(count.saturating_sub(1)).unwrap_or(i32::MAX);
        let scaled = std::ops::Mul::mul(
            self.config.temporary_ban_duration.as_secs_f64(),
            self.config.auto_ban_backoff_factor.max(1.0).powi(exponent),
        );
        let duration = Duration::from_secs_f64(scaled.min(self.config.max_auto_ban_duration.as_secs_f64()));
        self.auto_bans.insert(
            provider,
            AutoBan {
                until: now.saturating_add(duration.as_secs()),
                count,
            },
        );
    }
}

/// A fetch or integrity observation published by a trusted reporter.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProviderTrustSignal {
    pub provider: PublicKey,
    pub signal: TrustSignalKind,
    pub hash: Option<Hash>,
    pub reporter: PublicKey,
    pub timestamp: u64,
    pub sequence: u64,
    pub signature: Option<String>,
}

/// Kinds of automated provider observations.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TrustSignalKind {
    ObservedSuccess,
    ObservedFailure,
    ObservedCorruption,
}

impl ProviderTrustSignal {
    /// Create an unsigned signal and validate its structure.
    ///
    /// # Errors
    ///
    /// Returns an error when the signal structure is invalid.
    pub fn new(
        provider: PublicKey,
        kind: TrustSignalKind,
        hash: Option<Hash>,
        reporter: PublicKey,
        timestamp: u64,
        sequence: u64,
    ) -> Result<Self> {
        let signal = Self {
            provider,
            signal: kind,
            hash,
            reporter,
            timestamp,
            sequence,
            signature: None,
        };
        signal.validate()?;
        Ok(signal)
    }

    /// Create and sign a signal using the current time.
    ///
    /// # Errors
    ///
    /// Returns an error when the signal structure is invalid or signing fails.
    pub fn new_with_time(
        provider: PublicKey,
        kind: TrustSignalKind,
        hash: Option<Hash>,
        sequence: u64,
        signing_key: &SigningKey,
    ) -> Result<Self> {
        let reporter = PublicKey::from_bytes(&signing_key.verifying_key().to_bytes())
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid trust reporter: {error}")))?;
        let mut created = Self::new(provider, kind, hash, reporter, current_epoch_seconds(), sequence)?;
        created.sign(signing_key)?;
        Ok(created)
    }

    /// # Errors
    ///
    /// Returns an error when the signing key does not match the reporter.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        if self.reporter.as_bytes() != signing_key.verifying_key().as_bytes() {
            return Err(SyncwebError::InvalidIdentity(
                "provider trust signal signer does not match reporter".to_owned(),
            ));
        }
        self.signature = Some(hex::encode(signing_key.sign(&self.unsigned_bytes()?).to_bytes()));
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error when the signing key does not match the reporter.
    pub fn sign_with_secret_key(&mut self, secret_key: &SecretKey) -> Result<()> {
        self.sign(&SigningKey::from_bytes(&secret_key.to_bytes()))
    }

    /// # Errors
    ///
    /// Returns an error when the signal is malformed, expired, or unsigned.
    pub fn verify(&self) -> Result<()> {
        self.verify_at(current_epoch_seconds())
    }

    /// # Errors
    ///
    /// Returns an error when the signal is malformed, expired, or unsigned.
    pub fn verify_at(&self, now: u64) -> Result<()> {
        self.validate()?;
        if self.timestamp > now {
            return Err(SyncwebError::InvalidConfig(
                "provider trust signal is from the future".to_owned(),
            ));
        }
        if now.saturating_sub(self.timestamp) > DEFAULT_SIGNAL_TTL.as_secs() {
            return Err(SyncwebError::InvalidConfig(
                "provider trust signal has expired".to_owned(),
            ));
        }
        let signature_text = self
            .signature
            .as_deref()
            .ok_or_else(|| SyncwebError::InvalidConfig("provider trust signal must contain a signature".to_owned()))?;
        let signature_bytes = hex::decode(signature_text)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid provider trust signature: {error}")))?;
        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid provider trust signature: {error}")))?;
        let key = VerifyingKey::from_bytes(self.reporter.as_bytes())
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid provider trust reporter: {error}")))?;
        key.verify(&self.unsigned_bytes()?, &signature)
            .map_err(|error| SyncwebError::InvalidConfig(format!("provider trust signature is invalid: {error}")))
    }

    /// # Errors
    ///
    /// Returns an error when an identity or sequence is invalid.
    pub fn validate(&self) -> Result<()> {
        if self.sequence == 0 {
            return Err(SyncwebError::InvalidConfig(
                "provider trust signal sequence must be greater than zero".to_owned(),
            ));
        }
        let _provider = VerifyingKey::from_bytes(self.provider.as_bytes())
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid provider identity: {error}")))?;
        let _reporter = VerifyingKey::from_bytes(self.reporter.as_bytes())
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid reporter identity: {error}")))?;
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error when the signal is malformed or serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self)
            .map_err(|error| SyncwebError::operation("failed to serialize provider trust signal", error))
    }

    /// # Errors
    ///
    /// Returns an error when the bytes do not contain a valid signal.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let signal: Self = serde_json::from_slice(bytes.as_ref())
            .map_err(|error| SyncwebError::operation("failed to deserialize provider trust signal", error))?;
        signal.validate()?;
        Ok(signal)
    }

    fn unsigned_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        let encoded = serde_json::to_vec(&unsigned)
            .map_err(|error| SyncwebError::operation("failed to serialize unsigned provider trust signal", error))?;
        let mut bytes = Vec::with_capacity(REPUTATION_SIGNAL_CONTEXT.len().saturating_add(encoded.len()));
        bytes.extend_from_slice(REPUTATION_SIGNAL_CONTEXT);
        bytes.extend_from_slice(&encoded);
        Ok(bytes)
    }
}

#[must_use]
pub fn trust_stream_topic() -> TopicId {
    TopicId::from_bytes(*blake3::hash(TRUST_STREAM_TOPIC_SEED).as_bytes())
}

fn xor_distance(hash: Hash, provider: PublicKey) -> [u8; 32] {
    let mut distance = [0_u8; 32];
    for ((target, hash_byte), provider_byte) in distance.iter_mut().zip(hash.as_bytes()).zip(provider.as_bytes()) {
        *target = *hash_byte ^ *provider_byte;
    }
    distance
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
