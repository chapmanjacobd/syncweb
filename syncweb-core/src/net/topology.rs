//! Bounded topology policy for large communal networks.
//!
//! The rendezvous cache is an optional bootstrap aid. It stores short-lived
//! inventory summaries and provider identities, but never becomes an
//! authorization boundary or a required source of truth. Peers can discard a
//! cache and fall back to direct discovery, gossip, or relay paths.

use std::{collections::BTreeMap, time::Duration};

use blake3;
use iroh::PublicKey;
use iroh_blobs::Hash;

use super::community::CommunityId;
use crate::{Result, SyncwebError};

const MAX_FOLDER_BYTES: usize = 256;

/// Limits for an optional local rendezvous/cache-peer role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CachePeerConfig {
    /// Maximum number of provider summaries retained by the cache peer.
    pub max_records: usize,
    /// Maximum number of collection roots represented by one summary.
    pub max_roots_per_record: usize,
    /// Maximum number of content items represented by one summary.
    pub max_content_items_per_record: usize,
}

impl Default for CachePeerConfig {
    fn default() -> Self {
        Self {
            max_records: 512,
            max_roots_per_record: 64,
            max_content_items_per_record: 1_000_000,
        }
    }
}

impl CachePeerConfig {
    fn validate(self) -> Result<Self> {
        if self.max_records == 0 {
            return Err(SyncwebError::InvalidConfig(
                "cache-peer maximum record count must be non-zero".to_owned(),
            ));
        }
        if self.max_roots_per_record == 0 {
            return Err(SyncwebError::InvalidConfig(
                "cache-peer maximum root count must be non-zero".to_owned(),
            ));
        }
        if self.max_content_items_per_record == 0 {
            return Err(SyncwebError::InvalidConfig(
                "cache-peer maximum content count must be non-zero".to_owned(),
            ));
        }
        Ok(self)
    }
}

/// A bounded, expiring inventory summary learned from one provider.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct InventorySummary {
    pub provider: PublicKey,
    pub community: Option<CommunityId>,
    pub folder: String,
    pub inventory_hash: Hash,
    pub root_count: usize,
    pub content_count: usize,
    pub observed_at: u64,
    pub expires_at: u64,
}

impl InventorySummary {
    /// Validate this summary against the cache limits and local clock.
    ///
    /// # Errors
    ///
    /// Returns an error when the summary is malformed, future-dated, expired,
    /// or exceeds the configured bounded representation.
    pub fn validate(&self, now: u64, config: CachePeerConfig) -> Result<()> {
        config.validate()?;
        if self.folder.is_empty() {
            return Err(SyncwebError::InvalidConfig(
                "cache-peer inventory folder cannot be empty".to_owned(),
            ));
        }
        if self.folder.len() > MAX_FOLDER_BYTES {
            return Err(SyncwebError::InvalidConfig(format!(
                "cache-peer inventory folder exceeds {MAX_FOLDER_BYTES} bytes"
            )));
        }
        if self.observed_at > now {
            return Err(SyncwebError::InvalidConfig(
                "cache-peer inventory observation is in the future".to_owned(),
            ));
        }
        if self.expires_at <= self.observed_at || now >= self.expires_at {
            return Err(SyncwebError::InvalidConfig(
                "cache-peer inventory lease is expired or invalid".to_owned(),
            ));
        }
        if self.root_count > config.max_roots_per_record {
            return Err(SyncwebError::InvalidConfig(
                "cache-peer inventory root count exceeds the configured bound".to_owned(),
            ));
        }
        if self.content_count > config.max_content_items_per_record {
            return Err(SyncwebError::InvalidConfig(
                "cache-peer inventory content count exceeds the configured bound".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CacheKey {
    provider: [u8; 32],
    community: Option<CommunityId>,
}

impl CacheKey {
    fn new(provider: PublicKey, community: Option<CommunityId>) -> Self {
        Self {
            provider: *provider.as_bytes(),
            community,
        }
    }
}

/// An optional local rendezvous/cache-peer store.
///
/// The cache is intentionally local and disposable. It helps a node exchange
/// bounded provider summaries and bootstrap candidates, while direct discovery
/// remains the fallback when the cache is unavailable.
#[derive(Clone, Debug)]
pub struct RendezvousCache {
    community: Option<CommunityId>,
    config: CachePeerConfig,
    records: BTreeMap<CacheKey, InventorySummary>,
}

impl RendezvousCache {
    /// Create a cache scoped to one open or named community.
    ///
    /// `None` represents the open daemon scope; it never matches a named
    /// community.
    ///
    /// # Errors
    ///
    /// Returns an error when any configured bound is zero.
    pub fn new(community: Option<CommunityId>, config: CachePeerConfig) -> Result<Self> {
        Ok(Self {
            community,
            config: config.validate()?,
            records: BTreeMap::new(),
        })
    }

    /// Insert a newer provider summary and evict the least useful records.
    ///
    /// A stale update cannot replace a newer observation. The cache keeps at
    /// most one summary per provider and community scope.
    ///
    /// # Errors
    ///
    /// Returns an error when the summary is outside the local scope or fails
    /// validation.
    pub fn upsert(&mut self, summary: InventorySummary, now: u64) -> Result<()> {
        summary.validate(now, self.config)?;
        if summary.community != self.community {
            return Err(SyncwebError::AccessDenied(
                "cache-peer inventory is outside the local community".to_owned(),
            ));
        }
        let key = CacheKey::new(summary.provider, summary.community);
        let replace = self
            .records
            .get(&key)
            .is_none_or(|existing| newer_summary(&summary, existing));
        if replace {
            self.records.insert(key, summary);
        }
        self.evict_to_capacity();
        Ok(())
    }

    /// Remove expired summaries and return the number removed.
    pub fn purge_expired(&mut self, now: u64) -> usize {
        let before = self.records.len();
        self.records.retain(|_, summary| now < summary.expires_at);
        before.saturating_sub(self.records.len())
    }

    /// Return a deterministic, bounded sample of valid bootstrap candidates.
    #[must_use]
    pub fn bootstrap_peers(&mut self, now: u64, limit: usize, seed: u64) -> Vec<PublicKey> {
        self.purge_expired(now);
        let mut peers = self.records.values().collect::<Vec<_>>();
        peers.sort_by(|left, right| {
            peer_rank(left.provider, seed)
                .cmp(&peer_rank(right.provider, seed))
                .then(left.provider.as_bytes().cmp(right.provider.as_bytes()))
        });
        peers.into_iter().take(limit).map(|summary| summary.provider).collect()
    }

    /// Return the currently valid summaries, ordered by provider identity.
    #[must_use]
    pub fn summaries(&mut self, now: u64) -> Vec<InventorySummary> {
        self.purge_expired(now);
        self.records.values().cloned().collect()
    }

    /// Return the number of cached summaries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Return whether the cache has no summaries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Return this cache's community scope.
    #[must_use]
    pub const fn community(&self) -> Option<CommunityId> {
        self.community
    }

    fn evict_to_capacity(&mut self) {
        while self.records.len() > self.config.max_records {
            let oldest_key = self
                .records
                .iter()
                .min_by(|(_, left), (_, right)| {
                    left.observed_at
                        .cmp(&right.observed_at)
                        .then_with(|| left.expires_at.cmp(&right.expires_at))
                        .then_with(|| left.provider.as_bytes().cmp(right.provider.as_bytes()))
                })
                .map(|(key, _)| *key);
            let Some(evicted_key) = oldest_key else {
                break;
            };
            self.records.remove(&evicted_key);
        }
    }
}

const fn newer_summary(candidate: &InventorySummary, existing: &InventorySummary) -> bool {
    candidate.observed_at > existing.observed_at
        || (candidate.observed_at == existing.observed_at && candidate.expires_at > existing.expires_at)
}

fn peer_rank(provider: PublicKey, seed: u64) -> u64 {
    let mut input = [0_u8; 40];
    input[..32].copy_from_slice(provider.as_bytes());
    input[32..].copy_from_slice(&seed.to_le_bytes());
    let digest = blake3::hash(&input);
    let Some(prefix) = digest.as_bytes().first_chunk::<8>() else {
        return 0;
    };
    u64::from_le_bytes(*prefix)
}

/// Result of comparing the community scope observed on two reconnecting
/// partitions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CommunityMergeDecision {
    /// The partitions have the same explicit identity, or are both open.
    Merge,
    /// The partitions have different explicit identities or one is open.
    KeepSeparate,
}

/// Decide whether two partitions may exchange communal state.
#[must_use]
pub fn community_merge_decision(
    local_scope: Option<CommunityId>,
    remote_scope: Option<CommunityId>,
) -> CommunityMergeDecision {
    match (local_scope, remote_scope) {
        (None, None) => CommunityMergeDecision::Merge,
        (Some(local_id), Some(remote_id)) if local_id == remote_id => CommunityMergeDecision::Merge,
        _ => CommunityMergeDecision::KeepSeparate,
    }
}

/// Bounded, deterministic reconnect backoff for partition merges.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PartitionMergePolicy {
    initial_backoff: Duration,
    maximum_backoff: Duration,
}

impl PartitionMergePolicy {
    /// Create a policy with exponential backoff and a hard maximum.
    ///
    /// # Errors
    ///
    /// Returns an error when either duration is zero or the maximum is lower
    /// than the initial delay.
    pub fn new(initial_backoff: Duration, maximum_backoff: Duration) -> Result<Self> {
        if initial_backoff.is_zero() || maximum_backoff < initial_backoff {
            return Err(SyncwebError::InvalidConfig(
                "partition merge backoff must have a non-zero initial delay and a maximum at least as large".to_owned(),
            ));
        }
        Ok(Self {
            initial_backoff,
            maximum_backoff,
        })
    }

    /// Return a stable delay for one partition pair and retry attempt.
    ///
    /// The pair-specific jitter prevents every node from reconnecting at the
    /// same instant while preserving deterministic tests and bounded retries.
    #[must_use]
    pub fn reconnect_delay(&self, local_partition: u64, remote_partition: u64, attempt: u32) -> Duration {
        let base = exponential_delay(self.initial_backoff, self.maximum_backoff, attempt);
        let mut pair_bytes = [0_u8; 16];
        pair_bytes[..8].copy_from_slice(&local_partition.min(remote_partition).to_le_bytes());
        pair_bytes[8..].copy_from_slice(&local_partition.max(remote_partition).to_le_bytes());
        let pair_digest = blake3::hash(&pair_bytes);
        let pair_seed = pair_digest
            .as_bytes()
            .first_chunk::<8>()
            .map_or(0, |prefix| u64::from_le_bytes(*prefix));
        let jitter_limit = base.as_nanos().saturating_div(4);
        let jitter = if jitter_limit == 0 {
            0
        } else {
            let divisor = u64::try_from(jitter_limit.saturating_add(1)).unwrap_or(u64::MAX);
            u128::from(pair_seed.checked_rem(divisor).unwrap_or_default())
        };
        let nanos = u128::from(base.as_secs())
            .saturating_mul(1_000_000_000)
            .saturating_add(u128::from(base.subsec_nanos()))
            .saturating_add(jitter)
            .min(self.maximum_backoff.as_nanos());
        Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
    }
}

fn exponential_delay(initial: Duration, maximum: Duration, attempt: u32) -> Duration {
    let mut nanos = initial.as_nanos();
    let maximum_nanos = maximum.as_nanos();
    for _ in 0..attempt {
        nanos = nanos.saturating_mul(2).min(maximum_nanos);
        if nanos == maximum_nanos {
            break;
        }
    }
    Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn provider(seed: u8) -> PublicKey {
        let mut bytes = [0_u8; 32];
        bytes[0] = seed;
        let signing_key = SigningKey::from_bytes(&bytes);
        PublicKey::from_bytes(&signing_key.verifying_key().to_bytes()).expect("public key")
    }

    fn summary(provider: PublicKey, observed_at: u64, expires_at: u64) -> InventorySummary {
        InventorySummary {
            provider,
            community: None,
            folder: "folder".to_owned(),
            inventory_hash: Hash::from_bytes([provider.as_bytes().first().copied().unwrap_or_default(); 32]),
            root_count: 1,
            content_count: 2,
            observed_at,
            expires_at,
        }
    }

    #[test]
    fn cache_is_scoped_bounded_and_expires_records() {
        let mut cache = RendezvousCache::new(
            None,
            CachePeerConfig {
                max_records: 2,
                ..CachePeerConfig::default()
            },
        )
        .expect("cache config should be valid");
        cache
            .upsert(summary(provider(1), 1, 10), 2)
            .expect("summary should fit");
        cache
            .upsert(summary(provider(2), 2, 10), 2)
            .expect("summary should fit");
        cache
            .upsert(summary(provider(3), 3, 10), 3)
            .expect("summary should fit");
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.summaries(10).len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn stale_updates_do_not_replace_newer_observations() {
        let peer = provider(1);
        let mut cache = RendezvousCache::new(None, CachePeerConfig::default()).expect("cache config should be valid");
        cache.upsert(summary(peer, 10, 20), 11).expect("summary should fit");
        cache
            .upsert(summary(peer, 9, 30), 11)
            .expect("stale summary should be ignored");
        assert_eq!(cache.summaries(11)[0].expires_at, 20);
    }

    #[test]
    fn bootstrap_sampling_is_deterministic_and_bounded() {
        let mut cache = RendezvousCache::new(None, CachePeerConfig::default()).expect("cache config should be valid");
        for seed in 1..=4 {
            cache
                .upsert(summary(provider(seed), 1, 10), 2)
                .expect("summary should fit");
        }
        let first = cache.bootstrap_peers(2, 2, 7);
        let second = cache.bootstrap_peers(2, 2, 7);
        assert_eq!(first, second);
        assert_eq!(first.len(), 2);
    }

    #[test]
    fn community_identity_controls_partition_merge() {
        let community = CommunityId::from_bytes([1; 32]);
        assert_eq!(community_merge_decision(None, None), CommunityMergeDecision::Merge);
        assert_eq!(
            community_merge_decision(Some(community), Some(community)),
            CommunityMergeDecision::Merge
        );
        assert_eq!(
            community_merge_decision(None, Some(community)),
            CommunityMergeDecision::KeepSeparate
        );
        assert_eq!(
            community_merge_decision(Some(community), Some(CommunityId::from_bytes([2; 32]))),
            CommunityMergeDecision::KeepSeparate
        );
    }

    #[test]
    fn partition_backoff_is_pair_stable_and_bounded() {
        let policy = PartitionMergePolicy::new(Duration::from_secs(2), Duration::from_secs(10))
            .expect("backoff config should be valid");
        let first = policy.reconnect_delay(3, 9, 0);
        assert_eq!(first, policy.reconnect_delay(9, 3, 0));
        assert!(first >= Duration::from_secs(2));
        assert!(first <= Duration::from_secs(10));
        assert!(policy.reconnect_delay(3, 9, 20) <= Duration::from_secs(10));
        assert!(PartitionMergePolicy::new(Duration::ZERO, Duration::from_secs(1)).is_err());
    }
}
