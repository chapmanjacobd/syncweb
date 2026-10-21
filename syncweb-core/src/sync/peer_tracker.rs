use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use iroh::PublicKey;
use iroh_blobs::Hash;

use crate::{
    error::Result,
    provider_lease::{ProviderCapabilities, ProviderLease},
};

/// The source of a provider availability observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ObservationSource {
    Transfer,
    Lease,
    OfflineImport,
}

impl ObservationSource {
    const fn priority(self) -> u8 {
        match self {
            Self::Transfer => 3,
            Self::OfflineImport => 2,
            Self::Lease => 1,
        }
    }
}

/// Metadata retained for a fresh provider observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProviderObservation {
    pub source: ObservationSource,
    /// Confidence is an application-defined score from 0 through 100.
    pub confidence: u8,
    /// Capabilities explicitly advertised by the provider.
    pub capabilities: ProviderCapabilities,
}

/// Bounded transfer history for one provider.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PeerTransferStats {
    pub completed: u64,
    pub failed: u64,
    pub bytes: u64,
    pub completed_duration: Duration,
}

impl PeerTransferStats {
    /// Return the average completed-transfer throughput in bytes per second.
    #[must_use]
    pub fn throughput_bytes_per_second(self) -> Option<u64> {
        let nanos = self.completed_duration.as_nanos();
        if self.completed == 0 || self.bytes == 0 || nanos == 0 {
            return None;
        }
        let bytes_per_second = u128::from(self.bytes)
            .saturating_mul(1_000_000_000)
            .checked_div(nanos)?;
        u64::try_from(bytes_per_second).ok()
    }
}

/// Policy used when the peer availability cache reaches its size limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum EvictionStrategy {
    Lru,
    Fifo,
}

#[derive(Clone, Copy, Debug)]
struct ObservationRecord {
    metadata: ProviderObservation,
    observed_at: u64,
}

#[derive(Clone, Copy, Debug)]
struct TransferRecord {
    stats: PeerTransferStats,
    last_updated: u64,
}

#[derive(Clone, Debug)]
struct Availability {
    peers: HashMap<PublicKey, ObservationRecord>,
    inserted_at: u64,
    accessed_at: u64,
    last_seen: Instant,
}

type NegativeKey = (Hash, PublicKey);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ProviderScore {
    known: bool,
    confidence: u8,
    source_priority: u8,
    completed: u64,
    failed: u64,
    throughput: u64,
}

/// Bounded cache mapping blob hashes to the peers observed serving them.
///
/// `len()` returns the number of unique blob hashes tracked, not the total
/// number of peers. Use [`peer_count`](Self::peer_count) for per-blob counts.
#[derive(Clone, Debug)]
pub struct PeerTracker {
    entries: HashMap<Hash, Availability>,
    negative: HashMap<NegativeKey, Instant>,
    transfer_stats: HashMap<PublicKey, TransferRecord>,
    max_cache_size: usize,
    max_peers_per_blob: usize,
    negative_cache_size: usize,
    strategy: EvictionStrategy,
    expiry: Duration,
    negative_expiry: Duration,
    clock: u64,
}

/// Default maximum number of provider observations retained for one blob.
pub const DEFAULT_MAX_PEERS_PER_BLOB: usize = 1024;

impl PeerTracker {
    #[must_use]
    pub fn new(max_cache_size: usize, strategy: EvictionStrategy) -> Self {
        Self::with_expiry(max_cache_size, strategy, Duration::from_mins(5))
    }

    #[must_use]
    pub fn with_expiry(max_cache_size: usize, strategy: EvictionStrategy, expiry: Duration) -> Self {
        Self::with_expiries(max_cache_size, strategy, expiry, expiry)
    }

    /// Create a tracker with separate positive-observation and failure
    /// cooldown expiry windows.
    #[must_use]
    pub fn with_expiries(
        max_cache_size: usize,
        strategy: EvictionStrategy,
        expiry: Duration,
        negative_expiry: Duration,
    ) -> Self {
        Self::with_limits(
            max_cache_size,
            DEFAULT_MAX_PEERS_PER_BLOB,
            strategy,
            expiry,
            negative_expiry,
        )
    }

    /// Create a tracker with explicit blob and per-blob peer limits.
    #[must_use]
    pub fn with_limits(
        max_cache_size: usize,
        max_peers_per_blob: usize,
        strategy: EvictionStrategy,
        expiry: Duration,
        negative_expiry: Duration,
    ) -> Self {
        Self {
            entries: HashMap::new(),
            negative: HashMap::new(),
            transfer_stats: HashMap::new(),
            max_cache_size,
            max_peers_per_blob: max_peers_per_blob.max(1),
            negative_cache_size: max_cache_size.saturating_mul(4).max(1),
            strategy,
            expiry,
            negative_expiry,
            clock: 0,
        }
    }

    pub fn record_peer(&mut self, blob_hash: Hash, node_id: PublicKey) {
        self.record_observation(blob_hash, node_id, ObservationSource::Transfer, 100);
    }

    /// Record one completed or failed transfer while retaining a bounded
    /// per-peer history for scheduling and observability.
    pub fn record_transfer(&mut self, node_id: PublicKey, success: bool, bytes: u64, elapsed: Duration) {
        self.clock = self.clock.saturating_add(1);
        let entry = self.transfer_stats.entry(node_id).or_insert(TransferRecord {
            stats: PeerTransferStats {
                completed: 0,
                failed: 0,
                bytes: 0,
                completed_duration: Duration::ZERO,
            },
            last_updated: self.clock,
        });
        if success {
            entry.stats.completed = entry.stats.completed.saturating_add(1);
            entry.stats.bytes = entry.stats.bytes.saturating_add(bytes);
            entry.stats.completed_duration = entry.stats.completed_duration.saturating_add(elapsed);
        } else {
            entry.stats.failed = entry.stats.failed.saturating_add(1);
        }
        entry.last_updated = self.clock;
        while self.transfer_stats.len() > self.max_cache_size.saturating_mul(4).max(1) {
            let oldest = self
                .transfer_stats
                .iter()
                .min_by_key(|(_, record)| record.last_updated)
                .map(|(peer, _)| *peer);
            if let Some(peer) = oldest {
                self.transfer_stats.remove(&peer);
            } else {
                break;
            }
        }
    }

    /// Return a provider's bounded transfer history, if it is still retained.
    #[must_use]
    pub fn transfer_stats(&self, node_id: &PublicKey) -> Option<PeerTransferStats> {
        self.transfer_stats.get(node_id).map(|record| record.stats)
    }

    /// Record a provider observation with its source and confidence.
    ///
    /// Stronger observations never get replaced by weaker lease or import
    /// metadata, while a newer observation can increase confidence.
    pub fn record_observation(
        &mut self,
        blob_hash: Hash,
        node_id: PublicKey,
        source: ObservationSource,
        confidence: u8,
    ) {
        self.record_observation_with_capabilities(
            blob_hash,
            node_id,
            source,
            confidence,
            ProviderCapabilities::default(),
        );
    }

    /// Record an observation together with capabilities from a signed lease.
    pub fn record_observation_with_capabilities(
        &mut self,
        blob_hash: Hash,
        node_id: PublicKey,
        source: ObservationSource,
        confidence: u8,
        capabilities: ProviderCapabilities,
    ) {
        self.purge_negative();
        self.negative.remove(&(blob_hash, node_id));
        self.clock = self.clock.saturating_add(1);
        let now = Instant::now();
        let observed_at = self.clock;
        let entry = self.entries.entry(blob_hash).or_insert_with(|| Availability {
            peers: HashMap::new(),
            inserted_at: observed_at,
            accessed_at: observed_at,
            last_seen: now,
        });
        let metadata = ProviderObservation {
            source,
            confidence: confidence.min(100),
            capabilities,
        };
        entry
            .peers
            .entry(node_id)
            .and_modify(|record| {
                if source.priority() >= record.metadata.source.priority() {
                    record.metadata.source = source;
                }
                record.metadata.confidence = record.metadata.confidence.max(metadata.confidence);
                record.metadata.capabilities.supports_segments |= metadata.capabilities.supports_segments;
                record.metadata.capabilities.supports_relay |= metadata.capabilities.supports_relay;
                record.observed_at = observed_at;
            })
            .or_insert(ObservationRecord { metadata, observed_at });
        entry.accessed_at = observed_at;
        entry.last_seen = now;
        self.evict_peers_to_limit(&blob_hash);
        self.evict_to_limit();
    }

    /// Accept a signed lease after enforcing its local scope and expiry.
    ///
    /// Returns the number of unique content identifiers added to the
    /// registry. A rejected lease does not modify the registry.
    ///
    /// # Errors
    ///
    /// Returns an error when the lease is expired, malformed, incorrectly
    /// signed, or outside the supplied community and folder scope.
    pub fn record_lease(
        &mut self,
        lease: &ProviderLease,
        now: u64,
        community: Option<&str>,
        folder: &str,
    ) -> Result<usize> {
        lease.validate(now, community, folder)?;
        let provider = lease.provider_id()?;
        let hashes = lease.content_hashes()?;
        for hash in &hashes {
            self.record_observation_with_capabilities(
                *hash,
                provider,
                ObservationSource::Lease,
                75,
                lease.capabilities,
            );
        }
        Ok(hashes.len())
    }

    /// Return capabilities advertised by a fresh provider observation.
    #[must_use]
    pub fn provider_capabilities(&self, blob_hash: &Hash, node_id: &PublicKey) -> Option<ProviderCapabilities> {
        let entry = self.entries.get(blob_hash)?;
        if entry.last_seen.elapsed() > self.expiry || self.is_negative(blob_hash, node_id) {
            return None;
        }
        entry.peers.get(node_id).map(|record| record.metadata.capabilities)
    }

    #[must_use]
    pub fn get_peers(&mut self, blob_hash: &Hash) -> Vec<PublicKey> {
        self.purge_negative();
        self.clock = self.clock.saturating_add(1);
        let negative = &self.negative;
        let negative_expiry = self.negative_expiry;
        let mut peers = self.entries.get_mut(blob_hash).map_or_else(Vec::new, |entry| {
            if entry.last_seen.elapsed() > self.expiry {
                return Vec::new();
            }
            if self.strategy == EvictionStrategy::Lru {
                entry.accessed_at = self.clock;
            }
            entry
                .peers
                .iter()
                .filter(|(peer, _)| {
                    negative
                        .get(&(*blob_hash, **peer))
                        .is_none_or(|failed_at| failed_at.elapsed() > negative_expiry)
                })
                .map(|(peer, _)| *peer)
                .collect()
        });
        peers.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
        peers
    }

    #[must_use]
    pub fn peer_count(&self, blob_hash: &Hash) -> usize {
        self.entries.get(blob_hash).map_or(0, |entry| {
            if entry.last_seen.elapsed() <= self.expiry {
                entry
                    .peers
                    .iter()
                    .filter(|(peer, _)| !self.is_negative(blob_hash, peer))
                    .count()
            } else {
                0
            }
        })
    }

    /// Return the currently fresh peers serving a blob, or `None` when the
    /// registry has no fresh observation for the blob.
    pub fn peer_snapshot(&mut self, blob_hash: &Hash) -> Option<Vec<PublicKey>> {
        if !self.entries.contains_key(blob_hash) || !self.is_fresh(blob_hash) {
            return None;
        }
        Some(self.get_peers(blob_hash))
    }

    /// Return fresh provider metadata for a blob, excluding failed peers.
    pub fn observations(&mut self, blob_hash: &Hash) -> Vec<(PublicKey, ProviderObservation)> {
        self.purge_negative();
        let Some(entry) = self.entries.get(blob_hash) else {
            return Vec::new();
        };
        if entry.last_seen.elapsed() > self.expiry {
            return Vec::new();
        }
        let mut observations = entry
            .peers
            .iter()
            .filter(|(peer, _)| !self.is_negative(blob_hash, peer))
            .map(|(peer, record)| (*peer, record.metadata))
            .collect::<Vec<_>>();
        observations.sort_by(|(left, _), (right, _)| left.as_bytes().cmp(right.as_bytes()));
        observations
    }

    /// Rank discovered providers for a blob.
    ///
    /// Fresh observations are preferred over unknown peers. Within the
    /// observed set, higher-confidence and stronger-source observations win,
    /// followed by completed-transfer history and throughput. Failed peers
    /// remain eligible after the retry cooldown but are placed last so a
    /// temporary failure does not permanently exclude them.
    pub fn rank_peers(&mut self, blob_hash: &Hash, peers: &[PublicKey]) -> Vec<PublicKey> {
        self.purge_negative();
        let mut ranked = peers.to_vec();
        ranked.sort_by(|left, right| {
            let left_score = self.provider_score(blob_hash, left);
            let right_score = self.provider_score(blob_hash, right);
            right_score
                .known
                .cmp(&left_score.known)
                .then(right_score.confidence.cmp(&left_score.confidence))
                .then(right_score.source_priority.cmp(&left_score.source_priority))
                .then(left_score.failed.cmp(&right_score.failed))
                .then(right_score.completed.cmp(&left_score.completed))
                .then(right_score.throughput.cmp(&left_score.throughput))
                .then(left.as_bytes().cmp(right.as_bytes()))
        });
        ranked.dedup();
        ranked
    }

    /// Remove one peer's positive observation for a single blob immediately.
    ///
    /// Unlike [`record_failure`](Self::record_failure), this does not retain
    /// a retry cooldown.
    pub fn remove_blob_peer(&mut self, blob_hash: &Hash, node_id: &PublicKey) -> bool {
        let Some(entry) = self.entries.get_mut(blob_hash) else {
            return false;
        };
        let removed = entry.peers.remove(node_id).is_some();
        if entry.peers.is_empty() {
            self.entries.remove(blob_hash);
        }
        removed
    }

    /// Record a failed request and suppress retries for the configured
    /// negative-cache cooldown. A later successful observation clears it.
    pub fn record_failure(&mut self, blob_hash: Hash, node_id: PublicKey) {
        self.purge_negative();
        self.negative.insert((blob_hash, node_id), Instant::now());
        self.evict_negative_to_limit();
    }

    /// Return whether a request to this peer should be attempted now.
    #[must_use]
    pub fn should_try_peer(&self, blob_hash: &Hash, node_id: &PublicKey) -> bool {
        !self.is_negative(blob_hash, node_id)
    }

    #[must_use]
    pub fn contains(&self, blob_hash: &Hash, node_id: &PublicKey) -> bool {
        self.entries
            .get(blob_hash)
            .is_some_and(|entry| entry.peers.contains_key(node_id) && !self.is_negative(blob_hash, node_id))
    }

    /// Remove a peer from all cached blob availability records.
    pub fn on_peer_disconnected(&mut self, node_id: &PublicKey) {
        self.entries.retain(|_, entry| {
            entry.peers.remove(node_id);
            !entry.peers.is_empty()
        });
    }

    #[must_use]
    pub fn is_fresh(&self, blob_hash: &Hash) -> bool {
        self.entries
            .get(blob_hash)
            .is_some_and(|entry| entry.last_seen.elapsed() <= self.expiry)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn tick_and_maybe_evict(&mut self) {
        let now = Instant::now();
        self.entries
            .retain(|_, entry| now.saturating_duration_since(entry.last_seen) <= self.expiry);
        self.purge_negative();
        self.evict_to_limit();
    }

    fn is_negative(&self, blob_hash: &Hash, node_id: &PublicKey) -> bool {
        self.negative
            .get(&(*blob_hash, *node_id))
            .is_some_and(|failed_at| failed_at.elapsed() <= self.negative_expiry)
    }

    fn provider_score(&self, blob_hash: &Hash, node_id: &PublicKey) -> ProviderScore {
        let observation = self.entries.get(blob_hash).and_then(|entry| {
            (entry.last_seen.elapsed() <= self.expiry)
                .then(|| entry.peers.get(node_id))
                .flatten()
                .copied()
        });
        let transfer = self.transfer_stats.get(node_id).map(|record| record.stats);
        ProviderScore {
            known: observation.is_some(),
            confidence: observation.map_or(0, |record| record.metadata.confidence),
            source_priority: observation.map_or(0, |record| record.metadata.source.priority()),
            completed: transfer.map_or(0, |stats| stats.completed),
            failed: transfer.map_or(0, |stats| stats.failed),
            throughput: transfer
                .and_then(PeerTransferStats::throughput_bytes_per_second)
                .unwrap_or(0),
        }
    }

    fn purge_negative(&mut self) {
        let expiry = self.negative_expiry;
        self.negative.retain(|_, failed_at| failed_at.elapsed() <= expiry);
        self.evict_negative_to_limit();
    }

    fn evict_negative_to_limit(&mut self) {
        while self.negative.len() > self.negative_cache_size {
            let oldest = self
                .negative
                .iter()
                .min_by_key(|(_, failed_at)| *failed_at)
                .map(|(key, _)| *key);
            if let Some(key) = oldest {
                self.negative.remove(&key);
            } else {
                break;
            }
        }
    }

    fn evict_to_limit(&mut self) {
        while self.entries.len() > self.max_cache_size {
            let oldest = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| match self.strategy {
                    EvictionStrategy::Lru => entry.accessed_at,
                    EvictionStrategy::Fifo => entry.inserted_at,
                })
                .map(|(hash, _)| *hash);
            if let Some(hash) = oldest {
                self.entries.remove(&hash);
            } else {
                break;
            }
        }
    }

    fn evict_peers_to_limit(&mut self, blob_hash: &Hash) {
        let Some(entry) = self.entries.get_mut(blob_hash) else {
            return;
        };
        while entry.peers.len() > self.max_peers_per_blob {
            let oldest = entry
                .peers
                .iter()
                .min_by_key(|(_, record)| record.observed_at)
                .map(|(peer, _)| *peer);
            if let Some(peer) = oldest {
                entry.peers.remove(&peer);
            } else {
                break;
            }
        }
        if entry.peers.is_empty() {
            self.entries.remove(blob_hash);
        }
    }
}

/// Compact peer presence cache using stable integer indices.
#[derive(Clone, Debug, Default)]
pub struct EfficientPeerCache {
    hash_to_indices: HashMap<Hash, Vec<u16>>,
    presence: HashMap<Hash, Vec<u64>>,
    peer_to_index: HashMap<PublicKey, u16>,
    index_peer: Vec<PublicKey>,
    last_seen: HashMap<PublicKey, Instant>,
}

impl EfficientPeerCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a peer can serve a blob.
    ///
    /// # Errors
    ///
    /// Returns an error after all `u16` peer indices have been allocated.
    pub fn add_presence(&mut self, hash: Hash, peer: PublicKey) -> crate::Result<()> {
        let index = if let Some(index) = self.peer_to_index.get(&peer) {
            *index
        } else {
            let index = u16::try_from(self.index_peer.len())
                .map_err(|error| crate::SyncwebError::operation("peer cache capacity exceeded", error))?;
            self.index_peer.push(peer);
            self.peer_to_index.insert(peer, index);
            index
        };
        self.last_seen.insert(peer, Instant::now());
        let indices = self.hash_to_indices.entry(hash).or_default();
        match indices.binary_search(&index) {
            Ok(_) => {}
            Err(position) => indices.insert(position, index),
        }
        let index_usize = usize::from(index);
        let word_index = index_usize.checked_div(64).unwrap_or(0);
        let bit_index = index_usize.checked_rem(64).unwrap_or(0);
        let mask = 1_u64.checked_shl(u32::try_from(bit_index).unwrap_or(0)).unwrap_or(0);
        let presence = self.presence.entry(hash).or_default();
        presence.resize(word_index.saturating_add(1), 0);
        if let Some(word) = presence.get_mut(word_index) {
            *word |= mask;
        }
        Ok(())
    }

    pub fn remove_presence(&mut self, hash: &Hash, peer: &PublicKey) -> bool {
        let Some(index) = self.peer_to_index.get(peer) else {
            return false;
        };
        let Some(indices) = self.hash_to_indices.get_mut(hash) else {
            return false;
        };
        let removed = indices.binary_search(index).is_ok_and(|position| {
            indices.remove(position);
            true
        });
        if indices.is_empty() {
            self.hash_to_indices.remove(hash);
            self.presence.remove(hash);
        }
        if removed {
            let index_usize = usize::from(*index);
            let word_index = index_usize.checked_div(64).unwrap_or(0);
            let bit_index = index_usize.checked_rem(64).unwrap_or(0);
            let mask = 1_u64.checked_shl(u32::try_from(bit_index).unwrap_or(0)).unwrap_or(0);
            if let Some(word) = self.presence.get_mut(hash).and_then(|words| words.get_mut(word_index)) {
                *word &= !mask;
            }
        }
        removed
    }

    #[must_use]
    pub fn has_peer(&self, hash: &Hash, peer: &PublicKey) -> bool {
        self.peer_to_index.get(peer).is_some_and(|index| {
            let index_usize = usize::from(*index);
            let word_index = index_usize.checked_div(64).unwrap_or(0);
            let bit_index = index_usize.checked_rem(64).unwrap_or(0);
            let mask = 1_u64.checked_shl(u32::try_from(bit_index).unwrap_or(0)).unwrap_or(0);
            self.presence
                .get(hash)
                .and_then(|words| words.get(word_index))
                .is_some_and(|word| *word & mask != 0)
        })
    }

    #[must_use]
    pub fn peers(&self, hash: &Hash) -> Vec<PublicKey> {
        self.hash_to_indices.get(hash).map_or_else(Vec::new, |indices| {
            indices
                .iter()
                .filter_map(|index| self.index_peer.get(usize::from(*index)).copied())
                .collect()
        })
    }

    /// Return the total number of *unique* peers registered across all blobs.
    /// For per-blob counts see [`active_peer_count`](Self::active_peer_count).
    #[must_use]
    pub const fn total_peers(&self) -> usize {
        self.index_peer.len()
    }

    #[must_use]
    pub fn active_peer_count(&self, hash: &Hash) -> usize {
        self.hash_to_indices.get(hash).map_or(0, Vec::len)
    }

    #[must_use]
    pub fn memory_usage(&self) -> usize {
        let word_bytes = self
            .presence
            .values()
            .map(|words| words.len().saturating_mul(8))
            .sum::<usize>();
        let index_bytes = self
            .hash_to_indices
            .values()
            .map(|indices| indices.len().saturating_mul(2))
            .sum::<usize>();
        word_bytes
            .saturating_add(index_bytes)
            .saturating_add(
                self.peer_to_index
                    .len()
                    .saturating_mul(std::mem::size_of::<PublicKey>().saturating_add(2)),
            )
            .saturating_add(self.index_peer.len().saturating_mul(std::mem::size_of::<PublicKey>()))
    }

    /// Remove a peer from every blob while retaining stable indices for peers
    /// that may be observed again later.
    pub fn remove_peer(&mut self, peer: &PublicKey) {
        if !self.peer_to_index.contains_key(peer) {
            return;
        }
        let hashes = self.hash_to_indices.keys().copied().collect::<Vec<_>>();
        for hash in hashes {
            let _removed = self.remove_presence(&hash, peer);
        }
        self.last_seen.remove(peer);
    }

    /// Remove peers whose `last_seen` timestamp exceeds the given TTL.
    ///
    /// Fully reclaims each removed peer's index, so the total number of
    /// tracked peers stays bounded.
    ///
    /// Returns the number of peers removed.
    pub fn retain_active(&mut self, ttl: Duration) -> usize {
        self.retain_active_at(ttl, Instant::now())
    }

    /// Like [`retain_active`](Self::retain_active) but with an explicit
    /// observation time for deterministic testing.
    fn retain_active_at(&mut self, ttl: Duration, now: Instant) -> usize {
        let stale: Vec<PublicKey> = self
            .last_seen
            .iter()
            .filter(|(_, last_seen)| now.saturating_duration_since(**last_seen) > ttl)
            .map(|(peer, _)| *peer)
            .collect();
        let count = stale.len();
        for peer in stale {
            self.remove_peer(&peer);
            if let Some(stale_index) = self.peer_to_index.remove(&peer) {
                let last_index = self.index_peer.len().saturating_sub(1);
                if usize::from(stale_index) != last_index
                    && let Some(last_peer) = self.index_peer.get(last_index).copied()
                {
                    // Update presence bitmask for the moved peer: its bit
                    // is still at last_index but must be moved to stale_index.
                    self.remap_presence(last_index, usize::from(stale_index));
                    if let Some(slot) = self.index_peer.get_mut(usize::from(stale_index)) {
                        *slot = last_peer;
                    }
                    self.peer_to_index.insert(last_peer, stale_index);
                }
                self.index_peer.pop();
            }
        }
        count
    }

    /// Move all presence bits from `old_index` to `new_index` after a
    /// swap-remove compaction of `index_peer`.
    fn remap_presence(&mut self, old_index: usize, new_index: usize) {
        let old_u16 = u16::try_from(old_index).unwrap_or(u16::MAX);
        let new_u16 = u16::try_from(new_index).unwrap_or(u16::MAX);
        for (hash, indices) in &mut self.hash_to_indices {
            // binary_search returns a valid index in the Ok case
            let Ok(pos) = indices.binary_search(&old_u16) else {
                continue;
            };
            if let Some(slot) = indices.get_mut(pos) {
                *slot = new_u16;
            }
            let mut p = pos;
            while p > 0 {
                let prev = p.wrapping_sub(1);
                if indices.get(prev).is_none_or(|&v| v <= new_u16) {
                    break;
                }
                indices.swap(prev, p);
                p = prev;
            }
            // Update bitvector: clear old bit, set new bit
            if let Some(words) = self.presence.get_mut(hash) {
                let old_word = old_index.checked_div(64).unwrap_or(0);
                let old_bit = old_index.checked_rem(64).unwrap_or(0);
                if let Some(word) = words.get_mut(old_word) {
                    *word &= !(1_u64 << old_bit);
                }
                let new_word = new_index.checked_div(64).unwrap_or(0);
                let new_bit = new_index.checked_rem(64).unwrap_or(0);
                if new_word >= words.len() {
                    words.resize(new_word.saturating_add(1), 0);
                }
                if let Some(word) = words.get_mut(new_word) {
                    *word |= 1_u64 << new_bit;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::SecretKey;

    fn hash(value: u8) -> Hash {
        Hash::from_bytes([value; 32])
    }

    fn peer(value: u8) -> PublicKey {
        SecretKey::from_bytes(&[value; 32]).public()
    }

    #[test]
    fn ranks_fresh_successful_providers_before_unknown_peers() {
        let blob = hash(1);
        let successful = peer(1);
        let failed = peer(2);
        let unknown = peer(3);
        let mut tracker = PeerTracker::with_expiry(8, EvictionStrategy::Lru, Duration::from_mins(1));

        tracker.record_observation(blob, successful, ObservationSource::Transfer, 100);
        tracker.record_transfer(successful, true, 1_000, Duration::from_secs(1));
        tracker.record_observation(blob, failed, ObservationSource::Lease, 75);
        tracker.record_transfer(failed, false, 0, Duration::ZERO);

        assert_eq!(
            tracker.rank_peers(&blob, &[unknown, failed, successful]),
            vec![successful, failed, unknown]
        );
    }
}
