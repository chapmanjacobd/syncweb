use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use iroh::PublicKey;
use tokio::time::{Instant, sleep};

/// Default maximum number of active gossip neighbors retained per node.
pub const DEFAULT_MAX_NEIGHBORS: usize = 64;

/// Information about a known peer in the neighbor map.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PeerInfo {
    pub node_id: PublicKey,
    pub last_seen: Instant,
    pub name: Option<String>,
    pub connected: bool,
}

impl PeerInfo {
    #[must_use]
    pub fn new(node_id: PublicKey) -> Self {
        Self {
            node_id,
            last_seen: Instant::now(),
            name: None,
            connected: true,
        }
    }

    pub fn touch(&mut self) {
        self.last_seen = Instant::now();
    }
}

/// Thread-safe map of discovered gossip peers with automatic staleness expiry.
///
/// Tracks which peers are actively participating in a gossip topic. A background
/// cleanup task removes peers whose `last_seen` exceeds the expiration timeout.
/// This handles the case where a peer crashes silently without sending
/// `NeighborDown`.
#[derive(Debug, Clone)]
pub struct NeighborMap {
    peers: Arc<DashMap<PublicKey, PeerInfo>>,
    expiration_timeout: Duration,
    max_neighbors: usize,
}

impl NeighborMap {
    /// Create a new neighbor map and spawn a background cleanup task.
    ///
    /// The cleanup task runs every `expiration_timeout / 3` and removes
    /// peers not seen within the timeout.
    #[must_use]
    pub fn new(expiration_timeout: Duration) -> Self {
        Self::with_capacity(expiration_timeout, DEFAULT_MAX_NEIGHBORS)
    }

    /// Create a neighbor map with a bounded active-neighbor set.
    ///
    /// A zero capacity is treated as one so a caller cannot accidentally
    /// disable discovery completely.
    #[must_use]
    pub fn with_capacity(expiration_timeout: Duration, max_neighbors: usize) -> Self {
        let map = Self::new_without_cleanup_with_capacity(expiration_timeout, max_neighbors);
        map.spawn_cleanup_task();
        map
    }

    /// Create a neighbor map without automatic cleanup (for testing).
    #[must_use]
    pub fn new_without_cleanup(expiration_timeout: Duration) -> Self {
        Self::new_without_cleanup_with_capacity(expiration_timeout, DEFAULT_MAX_NEIGHBORS)
    }

    /// Create a bounded neighbor map without automatic cleanup.
    #[must_use]
    pub fn new_without_cleanup_with_capacity(expiration_timeout: Duration, max_neighbors: usize) -> Self {
        Self {
            peers: Arc::new(DashMap::new()),
            expiration_timeout,
            max_neighbors: max_neighbors.max(1),
        }
    }

    /// Record a peer as seen, resetting its staleness timer.
    pub fn touch(&self, node_id: PublicKey, name: Option<&str>) {
        {
            let mut entry = self.peers.entry(node_id).or_insert_with(|| PeerInfo::new(node_id));
            entry.touch();
            if let Some(n) = name {
                entry.name = Some(n.to_string());
            }
            entry.connected = true;
        }
        self.trim_to_capacity();
    }

    /// Mark a peer as disconnected (without removing it).
    pub fn mark_disconnected(&self, node_id: &PublicKey) {
        if let Some(mut entry) = self.peers.get_mut(node_id) {
            entry.connected = false;
        }
    }

    /// Remove a peer (e.g. on explicit `NeighborDown`).
    #[must_use]
    pub fn remove(&self, node_id: &PublicKey) -> Option<PeerInfo> {
        self.peers.remove(node_id).map(|(_, info)| info)
    }

    /// List all non-expired peers.
    #[must_use]
    pub fn list(&self) -> Vec<PeerInfo> {
        let now = Instant::now();
        self.peers
            .iter()
            .filter(|entry| now.duration_since(entry.last_seen) <= self.expiration_timeout)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// List all non-expired, currently connected peers.
    #[must_use]
    pub fn list_connected(&self) -> Vec<PeerInfo> {
        let now = Instant::now();
        self.peers
            .iter()
            .filter(|entry| entry.connected && now.duration_since(entry.last_seen) <= self.expiration_timeout)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Return up to `limit` connected peers using a deterministic sample.
    ///
    /// The seed changes the sample without requiring a global peer list at the
    /// caller. Only the bounded, non-expired neighbor set is considered.
    #[must_use]
    pub fn sample_connected(&self, limit: usize, seed: u64) -> Vec<PeerInfo> {
        let mut peers = self.list_connected();
        peers.sort_by(|left, right| {
            sample_rank(left.node_id, seed)
                .cmp(&sample_rank(right.node_id, seed))
                .then(left.node_id.as_bytes().cmp(right.node_id.as_bytes()))
        });
        peers.truncate(limit);
        peers
    }

    /// Return a reference to the underlying peer map.
    #[must_use]
    pub const fn inner(&self) -> &Arc<DashMap<PublicKey, PeerInfo>> {
        &self.peers
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.peers.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    /// Return the maximum number of neighbors retained by this map.
    #[must_use]
    pub const fn max_neighbors(&self) -> usize {
        self.max_neighbors
    }

    /// Manually trigger cleanup of all expired peers.
    ///
    /// Returns the number of removed entries.
    #[must_use]
    pub fn cleanup(&self) -> usize {
        let now = Instant::now();
        let expired: Vec<PublicKey> = self
            .peers
            .iter()
            .filter_map(|entry| (now.duration_since(entry.last_seen) > self.expiration_timeout).then(|| *entry.key()))
            .collect();

        let count = expired.len();
        for key in expired {
            self.peers.remove(&key);
        }
        count
    }

    fn spawn_cleanup_task(&self) {
        let peers = Arc::clone(&self.peers);
        let expiration_timeout = self.expiration_timeout;
        let cleanup_interval = Duration::from_secs(expiration_timeout.as_secs().saturating_div(3));

        tokio::spawn(async move {
            loop {
                sleep(cleanup_interval).await;
                let now = Instant::now();
                let expired: Vec<PublicKey> = peers
                    .iter()
                    .filter_map(|entry| {
                        (now.duration_since(entry.last_seen) > expiration_timeout).then(|| *entry.key())
                    })
                    .collect();

                for key in expired {
                    peers.remove(&key);
                }
            }
        });
    }

    fn trim_to_capacity(&self) {
        while self.peers.len() > self.max_neighbors {
            let candidate_option = self
                .peers
                .iter()
                .min_by(|left, right| {
                    left.connected
                        .cmp(&right.connected)
                        .then(left.last_seen.cmp(&right.last_seen))
                        .then(left.key().as_bytes().cmp(right.key().as_bytes()))
                })
                .map(|entry| *entry.key());
            let Some(candidate_key) = candidate_option else {
                break;
            };
            self.peers.remove(&candidate_key);
        }
    }
}

fn sample_rank(node_id: PublicKey, seed: u64) -> u64 {
    let Some(prefix) = node_id.as_bytes().first_chunk::<8>() else {
        return seed;
    };
    let mut value = u64::from_le_bytes(*prefix).wrapping_add(seed);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn test_key(id: u8) -> PublicKey {
        let mut seed = [0_u8; 32];
        seed[0] = id;
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        PublicKey::from_bytes(&verifying_key.to_bytes()).unwrap()
    }

    #[test]
    fn test_insert_and_list() {
        let map = NeighborMap::new_without_cleanup(Duration::from_mins(1));
        let id = test_key(1);

        map.touch(id, Some("alice"));
        assert_eq!(map.len(), 1);
        assert!(!map.is_empty());

        let peers = map.list();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers.first().unwrap().name.as_deref(), Some("alice"));
    }

    #[test]
    fn test_touch_updates_last_seen() {
        let map = NeighborMap::new_without_cleanup(Duration::from_mins(1));
        let id = test_key(1);

        map.touch(id, None);
        let first_seen = map.list().first().unwrap().last_seen;

        // Wait a tiny bit and touch again
        std::thread::sleep(Duration::from_millis(10));
        map.touch(id, None);
        let second_seen = map.list().first().unwrap().last_seen;

        assert!(second_seen > first_seen);
    }

    #[test]
    fn test_remove_explicit() {
        let map = NeighborMap::new_without_cleanup(Duration::from_mins(1));
        let id = test_key(1);
        map.touch(id, None);

        let info = map.remove(&id);
        assert!(info.is_some());
        assert!(map.is_empty());
    }

    #[test]
    fn test_cleanup_removes_expired() {
        let map = NeighborMap::new_without_cleanup(Duration::from_millis(50));
        let id = test_key(1);

        map.touch(id, None);

        std::thread::sleep(Duration::from_millis(100));

        let removed = map.cleanup();
        assert_eq!(removed, 1);
        assert!(map.is_empty());
    }

    #[test]
    fn test_cleanup_preserves_fresh() {
        let map = NeighborMap::new_without_cleanup(Duration::from_mins(1));
        let alice = test_key(1);
        let bob = test_key(2);

        map.touch(alice, None);
        map.touch(bob, None);

        // Bob is expired, alice is not
        // Actually neither is expired with 60s timeout
        let removed = map.cleanup();
        assert_eq!(removed, 0);
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_mark_disconnected() {
        let map = NeighborMap::new_without_cleanup(Duration::from_mins(1));
        let id = test_key(1);

        map.touch(id, None);
        assert!(map.list_connected().len() == 1);

        map.mark_disconnected(&id);
        assert!(map.list_connected().is_empty());
        assert_eq!(map.len(), 1); // still tracked, just not connected
    }

    #[test]
    fn test_empty_map() {
        let map = NeighborMap::new_without_cleanup(Duration::from_mins(1));
        assert!(map.list().is_empty());
        assert!(map.is_empty());
        assert_eq!(map.cleanup(), 0);
    }

    #[test]
    fn test_double_touch_resets_timer() {
        let map = NeighborMap::new_without_cleanup(Duration::from_millis(500));
        let id = test_key(1);

        map.touch(id, None);

        // Wait almost up to expiry
        std::thread::sleep(Duration::from_millis(250));

        // Touch again, resetting the timer
        map.touch(id, None);

        // Now wait past the original expiry but before the new one
        std::thread::sleep(Duration::from_millis(200));

        // Should NOT be expired because we touched at 250ms, expiry at 750ms
        assert_eq!(map.cleanup(), 0);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_multiple_peers() {
        let map = NeighborMap::new_without_cleanup(Duration::from_mins(1));
        let ids: Vec<PublicKey> = (0..5).map(test_key).collect();

        for id in &ids {
            map.touch(*id, None);
        }

        assert_eq!(map.len(), 5);
        assert_eq!(map.list().len(), 5);

        let _ = map.remove(ids.first().unwrap());
        assert_eq!(map.len(), 4);

        let _ = map.cleanup();
        assert_eq!(map.len(), 4);
    }

    #[test]
    fn capacity_prefers_connected_and_newer_neighbors() {
        let map = NeighborMap::new_without_cleanup_with_capacity(Duration::from_mins(1), 2);
        let stale = test_key(1);
        let retained = test_key(2);
        let newest = test_key(3);

        map.touch(stale, None);
        map.mark_disconnected(&stale);
        map.touch(retained, None);
        map.touch(newest, None);

        assert_eq!(map.max_neighbors(), 2);
        assert_eq!(map.len(), 2);
        assert!(map.list().iter().all(|peer| peer.node_id != stale));
    }

    #[test]
    fn sampled_neighbors_are_bounded_and_seeded() {
        let map = NeighborMap::new_without_cleanup_with_capacity(Duration::from_mins(1), 5);
        for id in 1..=5 {
            map.touch(test_key(id), None);
        }

        let first = map.sample_connected(3, 7);
        let second = map.sample_connected(3, 7);
        let changed = map.sample_connected(3, 8);

        assert_eq!(first.len(), 3);
        assert_eq!(
            first.iter().map(|peer| peer.node_id).collect::<Vec<_>>(),
            second.iter().map(|peer| peer.node_id).collect::<Vec<_>>()
        );
        assert_ne!(
            first.iter().map(|peer| peer.node_id).collect::<Vec<_>>(),
            changed.iter().map(|peer| peer.node_id).collect::<Vec<_>>()
        );
    }
}
