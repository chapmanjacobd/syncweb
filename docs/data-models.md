# Data Models

## Data Models

### 1. Syncweb Folder = Iroh Namespace + Blob Store

```rust
// Each syncweb folder = 1 Namespace + 1 BlobStore + entries in docs
struct SyncwebFolder {
    // Identity
    namespace_id: NamespaceId,        // blake3(namespace_pubkey)
    namespace_secret: NamespaceSecret, // write capability (if we have it)
    author: Author,                    // Ed25519 keypair for writes

    // Blob storage
    blob_store: BlobStore,            // iroh-blobs persistent store

    // Sync state
    sync_mode: SyncMode,              // SendReceive | SendOnly | ReceiveOnly | PublicReadOnly

    // Limits (inspired by iroh-willow)
    max_entries: Option<u64>,         // Maximum entries to sync (0 = unlimited)
    max_size: Option<u64>,            // Maximum bytes to sync (0 = unlimited)

    // Bandwidth limiting (per-folder and per-peer)
    max_upload_speed: Option<u64>,    // Max upload bytes/sec (0 = unlimited)
    max_download_speed: Option<u64>,  // Max download bytes/sec (0 = unlimited)
    peer_limits: HashMap<NodeId, PeerLimits>, // Per-peer bandwidth limits

    // Deleted files tracking (inspired by iroh-willow)
    deleted_tracker: DeletedTracker,  // Track deleted-but-previously-seen files

    // Local filesystem
    local_path: PathBuf,              // Local mount point

    // Peers (cached from natural iroh flow, not custom gossip)
    known_peers: HashSet<NodeId>,     // Devices with doc access
    capabilities: CapabilityMap,      // NodeId -> Capability (Read/Write/Admin)

    // Version tracking (for data packages)
    version: Option<DataVersion>,     // Package version info

    // Snapshots
    snapshots: Vec<Snapshot>,         // Content-addressed snapshots
}

/// Per-peer bandwidth limits
struct PeerLimits {
    max_upload: Option<u64>,          // Max upload to this peer (bytes/sec)
    max_download: Option<u64>,        // Max download from this peer (bytes/sec)
    priority: u8,                     // Sync priority (0-255, higher = more important)
}

/// Track deleted-but-previously-seen files (from iroh-willow)
struct DeletedTracker {
    /// Entries we've seen that were later deleted
    deleted: HashMap<EntryHash, DeletedInfo>,
}

struct DeletedInfo {
    /// Original entry that was deleted
    original: FileEntry,
    /// Who deleted it
    deleted_by: SessionId,
    /// When it was deleted
    deleted_at: Timestamp,
}

impl SyncwebFolder {
    /// Create folder with limits
    fn with_limits(
        namespace_id: NamespaceId,
        max_entries: u64,
        max_size: u64,
    ) -> Self {
        Self {
            namespace_id,
            max_entries: Some(max_entries),
            max_size: Some(max_size),
            ..Default::default()
        }
    }

    /// Check if we've hit the sync limit
    fn is_limit_reached(&self, synced_count: u64, synced_size: u64) -> bool {
        if let Some(max) = self.max_entries {
            if synced_count >= max {
                return true;
            }
        }
        if let Some(max) = self.max_size {
            if synced_size >= max {
                return true;
            }
        }
        false
    }

    /// Record a file deletion
    fn record_deletion(&mut self, hash: EntryHash, session: SessionId) {
        self.deleted_tracker.record_deletion(hash, session);
    }

    /// Check if a file was previously seen but deleted
    fn is_deleted(&self, hash: &EntryHash) -> bool {
        self.deleted_tracker.is_deleted(hash)
    }
}
```

### 2. Sync Modes (replaces Syncthing folder types)

| Syncweb Type | Iroh Equivalent | Implementation |
|--------------|-----------------|----------------|
| `sendreceive` | `SyncMode::SendReceive` | Namespace key shared, full doc write |
| `sendonly` | `SyncMode::SendOnly` | Namespace key local only, share read cap |
| `receiveonly` | `SyncMode::ReceiveOnly` | Import doc with read cap only |
| `receiveencrypted` | `SyncMode::ReceiveEncrypted` | Encrypted blob store, no namespace key |
| NEW `public_readonly` | `SyncMode::PublicReadOnly` | Public blob ticket, no auth needed |

### 3. Capability System (replaces device + folder config)

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
enum Capability {
    /// Full admin (has NamespaceSecret)
    Admin(NamespaceSecret),
    /// Read + write (has write ticket / NamespacePublicKey)
    Write(DocTicket),
    /// Read only (has read ticket)
    Read(DocTicket),
    /// Public read (blob ticket, no doc access)
    PublicRead(BlobTicket),
}

struct CapabilityMap(HashMap<NodeId, Capability>);
```

### 4. Collections and Generalized Manifests

Generalizes file sharing into a reusable collection model, replacing rigid package manifests. Preserves immutable content identity while supporting mutable version heads. Represents folders, datasets, media libraries, archives, and knowledge bases.
Permits virtual collections assembled from content-addressed entries without duplicating blobs.

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct CollectionManifestV1 {
    schema: SchemaVersion,
    collection_id: CollectionId,
    version: VersionId,
    parent: Option<ManifestHash>,
    entries: Vec<CollectionEntry>,
    publisher: PublicKey,
    signature: Signature,
}

struct CollectionHead {
    collection_id: CollectionId,
    manifest: ManifestHash,
    sequence: u64,
    signature: Signature,
}

struct CollectionEntry {
    content_id: Hash,
    logical_path: String,
    name: String,
    size: u64,
    media_type: Option<String>,
    role: String, // primary, preview, transcript, metadata, etc.
    relationships: Vec<String>,
}
```

Packages are simply a profile of Collections, adding dependency and atomic-install semantics.

### 5. Backup/Snapshot System (Content-Addressed)

iroh's content-addressed storage makes snapshots extremely efficient - they're just references to immutable blobs. No data copying needed:

```rust
/// A content-addressed snapshot of a folder
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Snapshot {
    /// Unique snapshot ID
    id: SnapshotId,
    /// Folder namespace
    namespace_id: NamespaceId,
    /// Root hash of all blobs at snapshot time
    root_hash: Hash,
    /// When the snapshot was created
    created_at: Timestamp,
    /// Optional description
    description: Option<String>,
    /// Total size of snapshot
    total_size: u64,
    /// Number of files in snapshot
    file_count: u64,
}

impl SyncwebFolder {
    /// Create a snapshot (instant - just references existing blobs)
    async fn create_snapshot(&self, description: Option<String>) -> Result<Snapshot> {
        let entries = self.list_all_entries().await?;
        let total_size: u64 = entries.iter().map(|e| e.size).sum();
        let root_hash = self.compute_folder_hash().await?;

        let snapshot = Snapshot {
            id: SnapshotId::generate(),
            namespace_id: self.namespace_id,
            root_hash,
            created_at: Timestamp::now(),
            description,
            total_size,
            file_count: entries.len() as u64,
        };

        // Pin all blobs at snapshot time (prevent GC)
        self.pin_for_snapshot(&snapshot).await?;

        // Store snapshot metadata
        self.store_snapshot(&snapshot).await?;

        Ok(snapshot)
    }

    /// Restore folder from snapshot (instant - just update doc entries)
    async fn restore_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        // Verify all blobs still exist
        self.verify_snapshot_blobs(snapshot).await?;

        // Update doc entries to match snapshot
        self.restore_entries_from_snapshot(snapshot).await?;

        Ok(())
    }

    /// List all snapshots for this folder
    async fn list_snapshots(&self) -> Result<Vec<Snapshot>>;

    /// Delete a snapshot (unpin its blobs)
    async fn delete_snapshot(&self, snapshot_id: &SnapshotId) -> Result<()>;

    /// Diff between two snapshots
    async fn diff_snapshots(
        &self,
        a: &Snapshot,
        b: &Snapshot,
    ) -> Result<SnapshotDiff>;
}

struct SnapshotDiff {
    added: Vec<FileEntry>,
    removed: Vec<FileEntry>,
    modified: Vec<(FileEntry, FileEntry)>, // (old, new)
}
```

CLI:
```bash
# Create a snapshot
syncweb backup documents/ --description "before major edit"

# List snapshots
syncweb snapshots documents/

# Output:
# ID         Date                Size      Files  Description
# ---------- ------------------- --------- ------ ---------------------------
# a1b2c3d4   2024-01-15 10:30    45.2 GiB  1,234  before major edit
# e5f6g7h8   2024-01-14 09:15    44.8 GiB  1,230  weekly backup

# Restore from snapshot
syncweb restore documents/ a1b2c3d4

# Diff between snapshots
syncweb snapshots diff documents/ a1b2c3d4 e5f6g7h8

# Delete old snapshot
syncweb snapshots delete documents/ e5f6g7h8
```

Benefits:
- Instant snapshots: No data copying, just reference existing blobs
- Space efficient: Multiple snapshots share unchanged blobs (deduplication)
- Portable: Snapshots can be shared via tickets (like public folders)
- Verified: BLAKE3 ensures snapshot integrity

### 6. Syncweb URL Format (unified scheme)

A single `syncweb://` scheme with type-specific authority:

```
# Folder access (doc ticket)
syncweb://<node-ticket>/<namespace-id>          # Full access
syncweb://<node-ticket>/<namespace-id>?r        # Read-only

# Public blob access (no auth needed)
syncweb://blob/<blob-ticket>                    # Single blob
syncweb://blob/<blob-ticket>/<path>             # Specific file in blob

# Legacy (backward compatible, auto-detected)
sync://<folder-id>#<device-id>
sync://<folder-id>/sub/path#<device-id>

# Direct node connection
syncweb://node/<node-id>                        # Direct connection
```

Parsing priority:
1. `syncweb://blob/` - Public blob access
2. `syncweb://node/` - Direct node connection  
3. `syncweb://<ticket>` - Doc ticket (folder access)
4. `sync://` - Legacy format (auto-convert to iroh)

Note: The `?r` suffix is cryptographically enforced -- read tickets don't contain the `NamespaceSecret` needed for writes. Removing `?r` doesn't upgrade access, it changes the ticket type.

### 7. Device Identity

```rust
// Syncthing Device ID = Ed25519 pubkey (base32, 56 chars)
// Iroh NodeId = Ed25519 pubkey (base32, 52 chars)
// COMPATIBLE: Both use Ed25519, can convert between formats
struct DeviceId(NodeId);

impl DeviceId {
    fn from_syncthing(id: &str) -> Result<Self>;
    fn to_syncthing(&self) -> String;
}
```

### 8. Partial Folder Fetch (Improve Network Robustness)

When syncing a folder, some blobs may have many seeders while others are poorly replicated. `FetchFilter` supports `min_peers`/`max_peers` to preferentially download the least-seeded blobs, and `min_count`/`max_count` to limit how many blobs are fetched:

```rust
/// Fetch strategy for a folder
enum FetchStrategy {
    /// Download everything (default)
    All,
    /// Fetch only blobs matching a filter (paths, sizes, peers, count, etc.)
    Filter(FetchFilter),
}

struct FetchFilter {
    paths: Option<Vec<PathBuf>>,
    min_size: Option<u64>,
    max_size: Option<u64>,
    /// Only fetch blobs with at least this many seeders
    min_peers: Option<usize>,
    /// Only fetch blobs with at most this many seeders (improves network health)
    max_peers: Option<usize>,
    /// Only fetch at least this many blobs
    min_count: Option<usize>,
    /// Only fetch at most this many blobs
    max_count: Option<usize>,
}

impl SyncwebFolder {
    /// Fetch blobs using a given strategy
    async fn fetch(&self, strategy: FetchStrategy) -> Result<IntentHandle> {
        match strategy {
            FetchStrategy::All => self.fetch_all().await,
            FetchStrategy::Filter(filter) => self.fetch_filtered(filter).await,
        }
    }

    /// Check if a blob exists locally
    fn has_local(&self, hash: Hash) -> bool {
        self.blob_store.has(hash)
    }
}
```

CLI:
```bash
# Fetch everything (default)
syncweb download audio/

# Fetch only poorly-seeded blobs to improve network health
syncweb download --max-peers 2 audio/

# Fetch only 10 unseeded blobs
syncweb download --max-peers 0 --max-count 10 audio/

# Check which blobs in a folder are poorly seeded
syncweb health audio/
# Output:
# Total blobs: 1,234
# Well-seeded (>3 peers): 890 (72%)
# Under-seeded (1 peer):  312 (25%)
# Unseeded (0 peers):      32 (3%)
#
# Least-seeded blobs:
#   a1b2c3d4  0 peers  12.5 MiB  audio/track01.flac
#   e5f6g7h8  0 peers   8.2 MiB  audio/track02.flac
#   i9j0k1l2  1 peer   15.1 MiB  audio/track03.flac
```

Benefits:
- Network health: Poorly-seeded blobs get copied to more peers
- Resilience: Rare content becomes more available over time
- No central coordination: Each node independently decides what to seed
- Efficient: Uses existing peer tracker data, no extra protocol needed
