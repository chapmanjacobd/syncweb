# Offline-First Queue and Conflict Resolution

## Offline-First Queue and Conflict Resolution UX

### Offline Editing Workflow
iroh-docs handles offline edits via CRDTs - changes made offline are automatically merged when the node reconnects. However, we need to provide clear UX for this:

```rust
/// Offline queue for pending changes
struct OfflineQueue {
    /// Changes made while offline
    pending: Vec<PendingChange>,
    /// When we went offline
    offline_since: Option<Instant>,
    /// Last sync timestamp
    last_sync: Option<Instant>,
}

struct PendingChange {
    /// File path that was modified
    path: PathBuf,
    /// Type of change (create, modify, delete)
    change_type: ChangeType,
    /// When the change was made
    timestamp: Timestamp,
    /// Local blob hash (if file was modified)
    blob_hash: Option<Hash>,
}

impl OfflineQueue {
    /// Get summary of pending changes
    fn summary(&self) -> OfflineSummary {
        OfflineSummary {
            files_created: self.pending.iter().filter(|c| c.change_type == ChangeType::Create).count(),
            files_modified: self.pending.iter().filter(|c| c.change_type == ChangeType::Modify).count(),
            files_deleted: self.pending.iter().filter(|c| c.change_type == ChangeType::Delete).count(),
            offline_duration: self.offline_since.map(|t| t.elapsed()),
        }
    }
}
```

CLI:
```bash
# Show pending offline changes
syncweb pending

# Output:
# Offline since: 2 hours ago
# Pending changes:
#   Created: 3 files (12.5 MiB)
#   Modified: 5 files (45.2 MiB)
#   Deleted: 1 file
# Total: 9 changes (57.7 MiB)
```

### Conflict Resolution UX
When two devices edit the same file offline, iroh-docs detects the conflict (two entries for the same key from different authors). We provide automatic resolution with clear UX:

Resolution strategy (default):
- Best-effort: at decode time, attempt to read both versions as text (UTF-8)
- If both versions are decodable as text and the generated diff is smaller than the latest LWW winner, save a diff file instead of the full file
- Otherwise, save the full file (both versions kept)
- The winning version always stays at the original path (LWW by timestamp)

File naming for conflicts:
```text
# Diff saved (decodable text, diff smaller than winner)
report.md            # Winner (newer timestamp)
report.md.diff       # Diff between versions (smaller than full file)

# Full file saved (binary or diff not smaller)
photo.jpg                        # Winner (newer timestamp)
photo.jpg.conflict.a1b2c3d4.jpg  # Loser (hash suffix for uniqueness)
```

```rust
/// Conflict between two versions of a file
struct Conflict {
    /// The file path
    path: PathBuf,
    /// Our version (local)
    local: ConflictVersion,
    /// Their version (remote)
    remote: ConflictVersion,
    /// When the conflict was detected
    detected_at: Timestamp,
}

struct ConflictVersion {
    /// Device that made the change
    device: NodeId,
    /// When the change was made
    timestamp: Timestamp,
    /// Blob hash of this version
    hash: Hash,
    /// File size
    size: u64,
}

/// How to resolve a conflict
enum ConflictResolution {
    /// Best-effort text diff: save diff if smaller than winner
    DiffFile,
    /// Full file: both versions kept as complete files
    FullFile,
    /// User manually resolves
    Manual,
}

impl Conflict {
    /// Auto-resolve: best-effort text decode, diff if smaller
    fn auto_resolve(&self) -> ConflictResolution {
        if let Some(diff_size) = self.estimate_diff_size() {
            if diff_size < self.winner().size as usize {
                return ConflictResolution::DiffFile;
            }
        }
        ConflictResolution::FullFile
    }

    /// Try to compute diff size (returns None if not decodable as text)
    fn estimate_diff_size(&self) -> Option<usize> {
        let local_text = std::str::from_utf8(&self.local.content).ok()?;
        let remote_text = std::str::from_utf8(&self.remote.content).ok()?;
        Some(diff::unified_size(local_text, remote_text))
    }

    /// Rename the losing version (FullFile resolution)
    fn rename_loser(&self, winner: &ConflictVersion) -> Result<PathBuf> {
        let loser = if winner == &self.local { &self.remote } else { &self.local };
        let new_name = format!("{}.conflict.{}.{}",
            self.path.file_stem().unwrap().to_string_lossy(),
            loser.timestamp.as_unix(),
            self.path.extension().unwrap().to_string_lossy(),
        );
        let new_path = self.path.parent().unwrap().join(new_name);
        std::fs::rename(&self.path, &new_path)?;
        Ok(new_path)
    }

    /// Path for the diff file: <stem>.diff (if enough filename characters)
    fn diff_path(&self) -> Result<PathBuf> {
        let stem = self.path.file_stem().unwrap().to_string_lossy();
        let diff_name = format!("{}.diff", stem);
        let diff_path = self.path.parent().unwrap().join(&diff_name);
        if diff_path.to_string_lossy().len() <= 255 {
            Ok(diff_path)
        } else {
            Err(Error::FilenameTooLong)
        }
    }
}
```

CLI:
```bash
# Show conflicts
syncweb conflicts

# Output:
# Conflict: docs/report.md
#   Local (you): 2024-01-15 10:30 (modified)
#   Remote (alice): 2024-01-15 11:45 (modified)
#   Auto-resolve: DiffFile (text, diff smaller than winner)

# Resolve all conflicts (auto-resolve: diff if smaller, else full file)
syncweb conflicts --auto-resolve

# Resolve specific conflict
syncweb conflicts resolve <conflict-id> --keep-local
syncweb conflicts resolve <conflict-id> --keep-remote
```

### Sync Status Display
Enhanced folder status showing offline state:

```bash
$ syncweb folders
Folder      Status      Offline     Pending     Last Sync
----------- ----------- ----------- ----------- -----------
documents   OK          -           -           2 min ago
photos      Modified    3 hours     5 changes   -
work        Conflict    1 hour      2 conflicts 5 min ago
```

---

## Peer Availability Tracking (Cached from Natural Flow + DHT Discovery)

Instead of building a custom gossip overlay, we use distributed-topic-tracker for DHT-based topic discovery, then cache peer availability from iroh's natural blob request/response flow.

### Cache Eviction Strategy ((standard CS pattern: age-based cache eviction))

Age-based cache management with LRU/FIFO eviction for memory-efficient peer tracking:

```rust
/// Age-based cache eviction for PeerTracker
/// Inspired by standard CS pattern: age-based cache eviction
struct PeerTracker {
    /// Blob hash -> set of peers that responded to requests
    availability: HashMap<Hash, HashSet<NodeId>>,
    /// Age of each entry (steps since last access)
    age: HashMap<Hash, u32>,
    /// Last update timestamp per blob
    last_seen: HashMap<Hash, Instant>,
    /// Cache expiry (default: 5 minutes)
    expiry: Duration,
    /// Use LRU (true) or FIFO (false) eviction
    use_lru: bool,
    /// Maximum cache size before eviction
    max_cache_size: usize,
}

impl PeerTracker {
    /// Update from natural iroh flow (called when blob is fetched)
    fn on_blob_fetched(&mut self, hash: Hash, peer: NodeId) {
        self.availability
            .entry(hash)
            .or_default()
            .insert(peer);
        self.last_seen.insert(hash, Instant::now());
        // Reset age on access (LRU) or leave unchanged (FIFO)
        if self.use_lru {
            self.age.insert(hash, 0);
        }
    }

    /// Update from connection events
    fn on_peer_connected(&mut self, peer: NodeId);
    fn on_peer_disconnected(&mut self, peer: NodeId);

    /// Get peer count for a blob (for sort: niche/frecency)
    fn peer_count(&self, hash: &Hash) -> usize {
        self.availability.get(hash).map_or(0, |peers| peers.len())
    }

    /// Get all peers for a blob
    fn peers(&self, hash: &Hash) -> &HashSet<NodeId> {
        self.availability.get(hash).unwrap_or(&HashSet::new())
    }

    /// Increment age of all entries and evict oldest if over capacity
    fn tick_and_maybe_evict(&mut self) {
        // Increment age for all entries
        for age in self.age.values_mut() {
            *age += 1;
        }
        
        // Evict if over capacity
        if self.availability.len() > self.max_cache_size {
            self.evict_oldest();
        }
    }

    /// Evict the oldest entry (highest age)
    fn evict_oldest(&mut self) {
        if let Some((&oldest_hash, _)) = self.age.iter().max_by_key(|(_, &age)| age) {
            self.availability.remove(&oldest_hash);
            self.age.remove(&oldest_hash);
            self.last_seen.remove(&oldest_hash);
        }
    }

    /// Prune expired entries
    fn prune(&mut self) {
        let now = Instant::now();
        let expired: Vec<Hash> = self.last_seen.iter()
            .filter(|(_, &time)| now.duration(time) > self.expiry)
            .map(|(&hash, _)| hash)
            .collect();
        
        for hash in expired {
            self.availability.remove(&hash);
            self.age.remove(&hash);
            self.last_seen.remove(&hash);
        }
    }

    /// Get freshness status
    fn is_fresh(&self, hash: &Hash) -> bool {
        self.last_seen.get(hash)
            .map(|&time| time.elapsed() < self.expiry)
            .unwrap_or(false)
    }
}
```

### Memory-Efficient Cache ((standard CS pattern: memory-efficient bitmask presence))

Bitmask-based presence tracking for large peer networks:

```rust
/// Memory-efficient peer availability cache using bitmasks
/// Inspired by standard CS pattern: memory-efficient bitmask presence
struct EfficientPeerCache {
    /// Bitmask for quick presence checks (1 bit per peer)
    presence: BitVec,
    /// Compressed indices for active peers per blob
    active_indices: HashMap<Hash, Vec<u16>>,
    /// Peer ID -> index mapping
    peer_index: HashMap<NodeId, u16>,
    /// Reverse mapping for cleanup
    index_peer: HashMap<u16, NodeId>,
    /// Next available index
    next_index: u16,
}

impl EfficientPeerCache {
    /// Check if a peer has a blob (O(1) bitmask check)
    fn has_peer(&self, hash: &Hash, peer: &NodeId) -> bool {
        if let Some(&idx) = self.peer_index.get(peer) {
            if let Some(indices) = self.active_indices.get(hash) {
                return indices.contains(&idx);
            }
        }
        false
    }

    /// Get all peers for a blob (decompress indices)
    fn peers(&self, hash: &Hash) -> Vec<NodeId> {
        self.active_indices.get(hash)
            .map(|indices| indices.iter()
                .filter_map(|&idx| self.index_peer.get(&idx).copied())
                .collect())
            .unwrap_or_default()
    }

    /// Add peer presence for a blob
    fn add_presence(&mut self, hash: Hash, peer: NodeId) {
        let idx = self.peer_index.get(&peer).copied().unwrap_or_else(|| {
            let idx = self.next_index;
            self.next_index += 1;
            self.peer_index.insert(peer, idx);
            self.index_peer.insert(idx, peer);
            idx
        });
        
        self.active_indices.entry(hash)
            .or_default()
            .push(idx);
    }

    /// Memory usage estimate (bits for presence + bytes for indices)
    fn memory_usage(&self) -> usize {
        let bitmask_bytes = (self.peer_index.len() + 7) / 8;
        let indices_bytes: usize = self.active_indices.values()
            .map(|v| v.len() * 2)
            .sum();
        bitmask_bytes + indices_bytes
    }
}
```

This approach:
- Leverages distributed-topic-tracker for decentralized peer discovery (no central server)
- Uses iroh's natural peer flow for availability caching
- No custom gossip protocol needed
- Data is cached locally for fast access
- Automatically updates as peers connect/disconnect
- Used by `sort` command for niche/frecency/peers sorting
- Memory-efficient: Bitmask presence tracking for 1000+ peers
- Adaptive eviction: LRU or FIFO based on usage patterns
