use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use iroh::PublicKey;
use iroh_blobs::Hash;

/// Policy used when the peer availability cache reaches its size limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum EvictionStrategy {
    Lru,
    Fifo,
}

#[derive(Clone, Debug)]
struct Availability {
    peers: HashSet<PublicKey>,
    inserted_at: u64,
    accessed_at: u64,
    last_seen: Instant,
}

/// Bounded cache of peers observed serving individual blobs.
#[derive(Clone, Debug)]
pub struct PeerTracker {
    entries: HashMap<Hash, Availability>,
    max_cache_size: usize,
    strategy: EvictionStrategy,
    expiry: Duration,
    clock: u64,
}

impl PeerTracker {
    #[must_use]
    pub fn new(max_cache_size: usize, strategy: EvictionStrategy) -> Self {
        Self::with_expiry(max_cache_size, strategy, Duration::from_mins(5))
    }

    #[must_use]
    pub fn with_expiry(max_cache_size: usize, strategy: EvictionStrategy, expiry: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            max_cache_size,
            strategy,
            expiry,
            clock: 0,
        }
    }

    pub fn record_peer(&mut self, blob_hash: Hash, node_id: PublicKey) {
        self.clock = self.clock.saturating_add(1);
        let now = Instant::now();
        let entry = self.entries.entry(blob_hash).or_insert_with(|| Availability {
            peers: HashSet::new(),
            inserted_at: self.clock,
            accessed_at: self.clock,
            last_seen: now,
        });
        entry.peers.insert(node_id);
        entry.accessed_at = self.clock;
        entry.last_seen = now;
        self.evict_to_limit();
    }

    /// Record a peer observed during a blob transfer.
    pub fn on_blob_fetched(&mut self, blob_hash: Hash, node_id: PublicKey) {
        self.record_peer(blob_hash, node_id);
    }

    #[must_use]
    pub fn get_peers(&mut self, blob_hash: &Hash) -> Vec<PublicKey> {
        self.clock = self.clock.saturating_add(1);
        self.entries.get_mut(blob_hash).map_or_else(Vec::new, |entry| {
            if self.strategy == EvictionStrategy::Lru {
                entry.accessed_at = self.clock;
            }
            entry.peers.iter().copied().collect()
        })
    }

    #[must_use]
    pub fn peers(&self, blob_hash: &Hash) -> Vec<PublicKey> {
        self.entries
            .get(blob_hash)
            .map_or_else(Vec::new, |entry| entry.peers.iter().copied().collect())
    }

    #[must_use]
    pub fn peer_count(&self, blob_hash: &Hash) -> usize {
        self.entries.get(blob_hash).map_or(0, |entry| entry.peers.len())
    }

    #[must_use]
    pub fn contains(&self, blob_hash: &Hash, node_id: &PublicKey) -> bool {
        self.entries
            .get(blob_hash)
            .is_some_and(|entry| entry.peers.contains(node_id))
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
        self.evict_to_limit();
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
}

/// Compact peer presence cache using stable integer indices.
#[derive(Clone, Debug, Default)]
pub struct EfficientPeerCache {
    active_indices: HashMap<Hash, Vec<u16>>,
    presence: HashMap<Hash, Vec<u64>>,
    peer_index: HashMap<PublicKey, u16>,
    index_peer: Vec<PublicKey>,
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
        let index = if let Some(index) = self.peer_index.get(&peer) {
            *index
        } else {
            let index = u16::try_from(self.index_peer.len())
                .map_err(|error| crate::SyncwebError::operation("peer cache capacity exceeded", error))?;
            self.index_peer.push(peer);
            self.peer_index.insert(peer, index);
            index
        };
        let indices = self.active_indices.entry(hash).or_default();
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
        let Some(index) = self.peer_index.get(peer) else {
            return false;
        };
        let Some(indices) = self.active_indices.get_mut(hash) else {
            return false;
        };
        let removed = indices.binary_search(index).is_ok_and(|position| {
            indices.remove(position);
            true
        });
        if indices.is_empty() {
            self.active_indices.remove(hash);
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
        self.peer_index.get(peer).is_some_and(|index| {
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
        self.active_indices.get(hash).map_or_else(Vec::new, |indices| {
            indices
                .iter()
                .filter_map(|index| self.index_peer.get(usize::from(*index)).copied())
                .collect()
        })
    }

    #[must_use]
    pub const fn peer_count(&self) -> usize {
        self.index_peer.len()
    }

    #[must_use]
    pub fn active_peer_count(&self, hash: &Hash) -> usize {
        self.active_indices.get(hash).map_or(0, Vec::len)
    }

    #[must_use]
    pub fn memory_usage(&self) -> usize {
        let word_bytes = self
            .presence
            .values()
            .map(|words| words.len().saturating_mul(8))
            .sum::<usize>();
        let index_bytes = self
            .active_indices
            .values()
            .map(|indices| indices.len().saturating_mul(2))
            .sum::<usize>();
        word_bytes
            .saturating_add(index_bytes)
            .saturating_add(
                self.peer_index
                    .len()
                    .saturating_mul(std::mem::size_of::<PublicKey>().saturating_add(2)),
            )
            .saturating_add(self.index_peer.len().saturating_mul(std::mem::size_of::<PublicKey>()))
    }

    /// Remove a peer from every blob while retaining stable indices for peers
    /// that may be observed again later.
    pub fn remove_peer(&mut self, peer: &PublicKey) {
        if !self.peer_index.contains_key(peer) {
            return;
        }
        let hashes = self.active_indices.keys().copied().collect::<Vec<_>>();
        for hash in hashes {
            let _removed = self.remove_presence(&hash, peer);
        }
    }
}
