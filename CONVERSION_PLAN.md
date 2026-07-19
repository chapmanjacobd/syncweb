# syncweb-py to iroh-syncthing Conversion Plan

## Executive Summary

Convert syncweb-py (Python + Syncthing) to iroh-syncthing (Rust + Iroh 1.0+).
Key architectural shift: **Syncthing's block-exchange protocol to Iroh's BLAKE3-Bao verified blob sync (iroh-blobs) + document sync (iroh-docs) + gossip (iroh-gossip) + DHT-based peer discovery (distributed-topic-tracker).**

### Key Advantages of Iroh 1.0+
- **Content-addressed blobs (BLAKE3 + Bao trees)** - verified streaming, range requests, deduplication
- **iroh-docs** - Document sync with CRDT conflict resolution
- **iroh-gossip** - Pub/sub for discovery, presence, live updates
- **distributed-topic-tracker** - Decentralized peer discovery via BitTorrent DHT (no central bootstrap server)
- **iroh-blobs public sharing** - Native public read-only folders via ticket sharing
- **BLAKE3 verified streaming** - No hash tree sync needed, verified on-the-fly
- **QUIC transport** - Built-in NAT traversal, relays, connection migration
- **No separate daemon** - Library-first, embeddable
- **Version tracking** - Native support for data package versioning (apt-like updates)
- **Content pinning** - Prevent garbage collection of publicly shared blobs
- **Partial folder fetch** - Fetch only well-seeded portions of poorly-available folders

---

## Architecture Mapping

| syncweb-py / Syncthing | iroh-syncthing / Iroh 1.0+ |
|------------------------|----------------------------|
| Syncthing daemon (separate process) | IrohNode (embedded library) |
| Device ID (Ed25519, 56-char base32) | NodeId (Ed25519, 52-char base32) |
| Folder (config + path) | **Namespace** (iroh-docs) + **Blob Store** (iroh-blobs) |
| Folder ID (random) | NamespaceId (blake3 hash of namespace key) |
| Block exchange (BEP) | BLAKE3-Bao verified blob sync (iroh-blobs) |
| Index exchange (BEP) | Document sync (iroh-docs, CRDT-based) |
| Discovery (local/global/relay) | **distributed-topic-tracker** (BitTorrent DHT) + iroh-gossip + iroh-relay |
| Ignore patterns (.stignore) | **Lazy fetch** (inherent, no ignore needed) |
| Folder types (sendreceive/sendonly/receiveonly) | **SyncMode** (author keys, capabilities) |
| Device introduce | Doc share (capability tokens) |
| Cluster config (XML) | Local storage (iroh-docs + iroh-blobs) |
| REST API | In-process Rust API |
| Selective sync (ignore patterns) | **Lazy blob fetching** + doc subscriptions |
| Public folders | **Public blob tickets** (iroh-blobs tickets) |
| BEP relays | **iroh-relay** + BEP identity (Phase 2) + BEP bridge (Phase 7) |
| .stignore selective sync | **Native lazy fetch** (blobs fetched on demand) |

---

## Core Architecture

```
+------------------------------------------------------------------------------+
|                              iroh-syncthing CLI                               |
+------------------------------------------------------------------------------+
|  Commands: create, join, accept, drop, ls, find, download, sort, stat,       |
|            devices, folders, automatic, version, repl, publish, subscribe,   |
|            backup, snapshot, restore, init, config, network, health          |
+------------------------------------------------------------------------------+
                                      |
                                      v
+------------------------------------------------------------------------------+
|                           IrohNode (embedded)                                |
|  +--------------+  +--------------+  +--------------+  +--------------+     |
|  | iroh         |  | iroh-blobs   |  | iroh-docs    |  | iroh-gossip  |     |
|  | (Endpoint,   |<-| (BlobStore,  |<-| (Docs,       |<-| (topics,     |     |
|  |  Router,     |  |  BlobsProto, |  |  Replicas,   |  |  presence)   |     |
|  |  identity)   |  |  tickets)    |  |  sync)       |  |              |     |
|  +--------------+  +--------------+  +--------------+  +--------------+     |
|         |                |                |                |                |
|         +----------------+----------------+----------------+                |
|                          v                v                                 |
|              +-----------------------+  +-----------------------+          |
|              |   Local Storage       |  |   Network Layer       |          |
|              |  (blob store + docs)  |  |  (QUIC + relays)      |          |
|              +-----------------------+  +-----------------------+          |
|                                                                              |
|  +-----------------------+  +-----------------------+  +-----------------+  |
|  |   Peer Tracker        |  |   Filter Engine       |  |  Partial Fetch  |  |
|  |  (cached from natural |  |  (rules-based auto-   |  |  (prefer well-  |  |
|  |   iroh flow)          |  |   download/sync)      |  |   seeded parts) |  |
|  +-----------------------+  +-----------------------+  +-----------------+  |
|                                                                              |
|  +-----------------------+  +-----------------------+                       |
|  |   Topic Tracker       |  |   Networks            |                       |
|  |  distributed-topic-   |  |  (multi-folder groups |                       |
|  |   tracker (DHT-based  |  |   under gossip        |                       |
|  |   peer discovery)     |  |   topics)             |                       |
|  +-----------------------+  +-----------------------+                       |
|                                                                              |
|  +-----------------------+  +-----------------------+                       |
|  |   BEP Identity        |  |   BEP Bridge          |                       |
|  |  (Phase 2: DeviceId   |  |  (Phase 7: full       |                       |
|  |   conversion, --bep)  |  |   protocol translation)|                       |
|  +-----------------------+  +-----------------------+                       |
+------------------------------------------------------------------------------+
```

---

## Networks (Multi-Folder Groups)

A **Network** is a named group of folders + devices under a common gossip topic.
Replaces Syncthing's implicit cluster config with an explicit, shareable abstraction.

- **Network Gossip Topic**: `iroh-syncthing/net/<network_id>` (derived from network name)
- **Membership**: All devices in the network subscribe to this topic
- **Folders**: All folders in the network share the topic for discovery and auto-join

```rust
/// A named group of folders and devices sharing a gossip topic
struct Network {
    /// Unique network ID: blake3("network:" + name)
    id: NetworkId,
    /// Human-readable name
    name: String,
    /// Optional label
    label: String,
    /// Gossip topic for network discovery
    topic: TopicId,
    /// Devices in the network
    members: HashSet<NodeId>,
    /// Folders in the network
    folders: HashSet<NamespaceId>,
    /// Optional shared secret for invite-only networks
    shared_secret: Option<SecretKey>,
}

/// Network manager - create, join, invite
struct NetworkManager {
    networks: HashMap<NetworkId, Network>,
    /// Gossip service for topic subscriptions
    gossip: Gossip,
    /// Folder manager for folder membership
    folder_manager: FolderManager,
}

impl NetworkManager {
    /// Create a new network with a gossip topic
    async fn create(&mut self, name: &str, opts: NetworkOptions) -> Result<NetworkId>;

    /// Join an existing network via ticket
    async fn join(&mut self, ticket: NetworkTicket) -> Result<NetworkId>;

    /// Leave a network (unsubscribe from topic)
    async fn leave(&mut self, id: NetworkId) -> Result<()>;

    /// Invite a device to a network
    async fn invite(&self, id: NetworkId, device: NodeId) -> Result<NetworkTicket>;

    /// Remove a device from a network
    async fn kick(&mut self, id: NetworkId, device: NodeId) -> Result<()>;

    /// List all networks or inspect one
    fn list(&self) -> Vec<&Network>;
    fn get(&self, id: &NetworkId) -> Option<&Network>;
}
```

**Network Ticket:**
```
syncweb://network/<node-id>/<network-id>
syncweb://network/<node-id>/<network-id>?secret=<shared-secret>
```

**CLI:**
```bash
syncweb network create <name>              # Create new network
syncweb network ls [<name>]                 # List networks or network details
syncweb network join <ticket>               # Join a network
syncweb network leave <name>                # Leave a network
syncweb network invite <name> <device-id>   # Add device to network
syncweb network kick <name> <device-id>     # Remove device from network
syncweb create --network <name> <path>      # Add folder to network
syncweb join --network <name> <url>         # Join folder in network context
```

**Use cases:**
- **Team workspace**: `syncweb network create work`, then `syncweb create --network work ./docs`
- **Multi-folder sharing**: Share all project folders via single network invite
- **Departments**: Separate networks for engineering, design, marketing
- **Home/Plex**: Personal network for media + documents + backups

Migration: Single-device users skip networks. Multi-folder multi-device users adopt them naturally.

---

## iroh-willow Architecture Patterns (Selected)

These patterns are borrowed from iroh-willow (Willow protocol implementation) to improve our design.

### 1. Engine Pattern (Dedicated Storage Thread)

iroh-willow uses a dedicated storage thread with message-passing to avoid blocking async code with database operations:

```rust
// Inspired by iroh-willow's Engine pattern
struct SyncEngine {
    /// Actor handle - dedicated thread for storage operations
    actor: ActorHandle,
    /// Peer manager - handles peer connections and reconciliation
    peer_manager: PeerManager,
}

struct ActorHandle {
    /// Channel to send commands to the actor
    tx: mpsc::Sender<Command>,
    /// Dedicated storage thread handle
    _thread: std::thread::JoinHandle<()>,
}

// Actor runs in dedicated thread, not async runtime
struct Actor {
    /// Storage operations (blocking I/O)
    store: Store,
    /// Message inbox
    inbox: mpsc::Receiver<Command>,
}

impl Actor {
    fn run(mut self) {
        while let Some(cmd) = self.inbox.recv() {
            match cmd {
                Command::StoreEntry(entry) => {
                    // Blocking I/O is fine here - dedicated thread
                    self.store.insert(entry).expect("storage failed");
                }
                // ... other commands
            }
        }
    }
}
```

**Key insight**: Storage I/O (database writes, file operations) runs in a dedicated thread, keeping the async runtime responsive for network operations.

### 2. SessionMode (ReconcileOnce vs Continuous)

iroh-willow distinguishes between one-time reconciliation and continuous sync:

```rust
/// Session mode - how to sync with a peer
enum SessionMode {
    /// One-time reconciliation: sync once, then stop
    ReconcileOnce {
        /// Maximum entries to sync in this session
        max_count: Option<u64>,
        /// Maximum bytes to sync in this session
        max_size: Option<u64>,
    },
    /// Continuous sync: keep syncing as changes happen
    Continuous,
}

impl SessionMode {
    fn is_continuous(&self) -> bool {
        matches!(self, SessionMode::Continuous)
    }
}
```

**Use case**: `ReconcileOnce` for `download` command (fetch specific files, then stop), `Continuous` for background sync.

### 3. IntentHandle as Stream

iroh-willow's IntentHandle implements `Stream` + `Sink`, making it easy to use with async patterns:

```rust
/// Intent handle - represents an ongoing sync operation
struct IntentHandle {
    /// Event stream (what's happening)
    events: mpsc::Receiver<SyncEvent>,
    /// Command sink (send instructions)
    commands: mpsc::Sender<SyncCommand>,
}

/// Events from the sync engine
enum SyncEvent {
    /// Progress update
    Progress { synced: u64, total: u64 },
    /// New entry discovered
    EntryDiscovered { path: PathBuf, hash: Hash },
    /// Sync complete
    Complete,
    /// Error occurred
    Error(String),
}

/// Commands to the sync engine
enum SyncCommand {
    /// Pause sync
    Pause,
    /// Resume sync
    Resume,
    /// Cancel sync
    Cancel,
}

// IntentHandle implements Stream<Item = SyncEvent>
// and Sink<SyncCommand>
impl Stream for IntentHandle {
    type Item = SyncEvent;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.events.poll_recv(cx)
    }
}
```

**Use case**: Every sync operation returns an `IntentHandle` that can be `.await`ed for completion or streamed for progress.

### 4. PayloadForm (Data Formatting)

iroh-willow uses `PayloadForm` to define how data is formatted in the document:

```rust
/// How a file's metadata is formatted in the doc
enum PayloadForm {
    /// Raw binary (no formatting)
    Raw,
    /// JSON-encoded metadata
    Json,
    /// Custom serialization
    Custom(String),
}

impl PayloadForm {
    /// Encode entry for doc storage
    fn encode(&self, entry: &FileEntry) -> Vec<u8> {
        match self {
            PayloadForm::Raw => entry.to_bytes(),
            PayloadForm::Json => serde_json::to_vec(entry).unwrap(),
            PayloadForm::Custom(format) => todo!("custom format: {}", format),
        }
    }

    /// Decode entry from doc storage
    fn decode(&self, data: &[u8]) -> Result<FileEntry> {
        match self {
            PayloadForm::Raw => Ok(FileEntry::from_bytes(data)),
            PayloadForm::Json => Ok(serde_json::from_slice(data)?),
            PayloadForm::Custom(format) => todo!("custom format: {}", format),
        }
    }
}
```

**Use case**: Allow users to choose metadata format (JSON for debugging, raw for efficiency).

### 5. SubscribeParams (Subscription Filtering)

iroh-willow's `SubscribeParams` provides fine-grained control over what events to receive:

```rust
/// Parameters for subscribing to doc changes
struct SubscribeParams {
    /// Only receive ingestion events (new entries)
    ingest_only: bool,
    /// Ignore events from a specific session (avoid echo)
    ignore_session: Option<SessionId>,
    /// Filter by area of interest (path patterns)
    area_filter: Option<AreaFilter>,
}

/// Area filter for subscriptions
enum AreaFilter {
    /// Match by path prefix
    Prefix(PathBuf),
    /// Match by glob pattern
    Glob(String),
    /// Match by hash range
    HashRange(Hash, Hash),
}

impl SubscribeParams {
    fn default() -> Self {
        Self {
            ingest_only: false,
            ignore_session: None,
            area_filter: None,
        }
    }

    /// Only get new entries (skip existing)
    fn ingest_only() -> Self {
        Self {
            ingest_only: true,
            ..Default::default()
        }
    }

    /// Ignore our own session (avoid processing our own writes)
    fn ignore_session(session_id: SessionId) -> Self {
        Self {
            ignore_session: Some(session_id),
            ..Default::default()
        }
    }
}
```

**Use case**: `subscribe` command uses `ingest_only: true` to only show new files, not existing ones.

### 6. AreaOfInterest with Limits (max_size, max_count)

iroh-willow's `AreaOfInterest` supports limits on how much to sync:

```rust
/// Area of interest with optional limits
struct AreaOfInterest {
    /// The area (path prefix, hash range, etc.)
    area: Area,
    /// Maximum number of entries to sync (0 = unlimited)
    max_count: u64,
    /// Maximum total size in bytes to sync (0 = unlimited)
    max_size: u64,
}

impl AreaOfInterest {
    fn unlimited(area: Area) -> Self {
        Self {
            area,
            max_count: 0,
            max_size: 0,
        }
    }

    fn with_count_limit(area: Area, count: u64) -> Self {
        Self {
            area,
            max_count: count,
            max_size: 0,
        }
    }

    fn with_size_limit(area: Area, size: u64) -> Self {
        Self {
            area,
            max_count: 0,
            max_size: size,
        }
    }

    /// Check if we've hit the limit
    fn is_limit_reached(&self, synced_count: u64, synced_size: u64) -> bool {
        if self.max_count > 0 && synced_count >= self.max_count {
            return true;
        }
        if self.max_size > 0 && synced_size >= self.max_size {
            return true;
        }
        false
    }
}
```

**Use case**: `download --limit 10` creates `AreaOfInterest::with_count_limit(area, 10)`, stops after 10 entries.

### 7. Deleted Files Tracking (PruneEvent)

iroh-willow tracks when entries are pruned (deleted) and which session caused it:

```rust
/// Event when an entry is pruned (deleted)
struct PruneEvent {
    /// The entries that were pruned
    pruned: Vec<EntryHash>,
    /// Which session caused the pruning
    by: SessionId,
}

/// Store event for subscriptions
enum StoreEvent {
    /// New entry inserted
    Inserted(InsertEvent),
    /// Entry pruned (deleted)
    Pruned(PruneEvent),
}

/// Track deleted-but-previously-seen files
struct DeletedTracker {
    /// Entries we've seen that were later deleted
    deleted: HashMap<EntryHash, DeletedInfo>,
    /// Session that deleted the entry
    session: SessionId,
    /// When it was deleted
    deleted_at: Timestamp,
}

struct DeletedInfo {
    /// Original entry that was deleted
    original: FileEntry,
    /// Who deleted it
    deleted_by: SessionId,
    /// When it was deleted
    deleted_at: Timestamp,
}

impl DeletedTracker {
    /// Record that an entry was deleted
    fn record_deletion(&mut self, hash: EntryHash, session: SessionId) {
        self.deleted.insert(hash, DeletedInfo {
            original: self.get_original(hash),
            deleted_by: session,
            deleted_at: Timestamp::now(),
        });
    }

    /// Check if an entry was previously seen but deleted
    fn is_deleted(&self, hash: &EntryHash) -> bool {
        self.deleted.contains_key(hash)
    }

    /// Get deletion info
    fn deletion_info(&self, hash: &EntryHash) -> Option<&DeletedInfo> {
        self.deleted.get(hash)
    }
}
```

**Use case**: Track files that were synced but later deleted, enabling "undelete" or audit trails.

### 8. Simplified SpaceTicket

iroh-willow uses `SpaceTicket` for sharing access - we simplify for our needs:

```rust
/// Simplified ticket for sharing folder access
struct SpaceTicket {
    /// Node endpoint (for connection)
    node_addr: NodeAddr,
    /// Namespace (folder) being shared
    namespace_id: NamespaceId,
    /// Capability (read/write/admin)
    capability: Capability,
    /// Optional limits (max entries, max size)
    limits: Option<TicketLimits>,
}

struct TicketLimits {
    max_count: Option<u64>,
    max_size: Option<u64>,
}

impl SpaceTicket {
    /// Create a full-access ticket
    fn full_access(node_addr: NodeAddr, namespace_id: NamespaceId) -> Self {
        Self {
            node_addr,
            namespace_id,
            capability: Capability::Write,
            limits: None,
        }
    }

    /// Create a read-only ticket
    fn read_only(node_addr: NodeAddr, namespace_id: NamespaceId) -> Self {
        Self {
            node_addr,
            namespace_id,
            capability: Capability::Read,
            limits: None,
        }
    }

    /// Create a limited ticket (max entries/size)
    fn limited(
        node_addr: NodeAddr,
        namespace_id: NamespaceId,
        max_count: u64,
        max_size: u64,
    ) -> Self {
        Self {
            node_addr,
            namespace_id,
            capability: Capability::Read,
            limits: Some(TicketLimits {
                max_count: Some(max_count),
                max_size: Some(max_size),
            }),
        }
    }
}
```

**Use case**: `syncweb publish --limit 10 --size 1GB` creates a limited ticket.

### 9. IntentHandle Usage Examples

iroh-willow's IntentHandle pattern for sync operations:

```rust
/// Download command returns an IntentHandle
async fn download(
    &self,
    folder_id: &NamespaceId,
    paths: Vec<PathBuf>,
    limits: Option<AreaLimits>,
) -> Result<IntentHandle> {
    // Create area of interest with optional limits
    let area = Area::from_paths(paths);
    let aoi = match limits {
        Some(limits) => AreaOfInterest {
            area,
            max_count: limits.max_count.unwrap_or(0),
            max_size: limits.max_size.unwrap_or(0),
        },
        None => AreaOfInterest::unlimited(area),
    };

    // Start sync session
    let session = self.engine.start_session(
        folder_id,
        SessionMode::ReconcileOnce,
    ).await?;

    // Return handle that streams progress
    Ok(IntentHandle::new(session))
}

// Usage:
let handle = download(&folder, paths, limits).await?;
while let Some(event) = handle.next().await {
    match event {
        SyncEvent::Progress { synced, total } => {
            println!("Synced {}/{}", synced, total);
        }
        SyncEvent::Complete => {
            println!("Download complete!");
        }
        SyncEvent::Error(msg) => {
            eprintln!("Error: {}", msg);
        }
    }
}
```

### 10. SubscribeParams Usage Examples

```rust
/// Subscribe to folder changes with filtering
async fn subscribe(
    &self,
    folder_id: &NamespaceId,
    params: SubscribeParams,
) -> Result<IntentHandle> {
    let session = self.engine.subscribe(folder_id, params).await?;
    Ok(IntentHandle::new(session))
}

// Usage 1: Only show new files (ingest only)
let handle = subscribe(&folder, SubscribeParams::ingest_only()).await?;

// Usage 2: Ignore our own writes (avoid echo)
let session_id = self.engine.current_session_id();
let handle = subscribe(&folder, SubscribeParams::ignore_session(session_id)).await?;

// Usage 3: Filter by path prefix
let handle = subscribe(&folder, SubscribeParams {
    area_filter: Some(AreaFilter::Prefix("/photos/".into())),
    ..Default::default()
}).await?;
```

---

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
| **NEW** `public_readonly` | `SyncMode::PublicReadOnly` | **Public blob ticket, no auth needed** |

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

### 4. Data Versioning (apt-like for data packages)

```rust
/// Version tracking for data packages
#[derive(Serialize, Deserialize, Clone, Debug)]
struct DataVersion {
    /// Semantic version (e.g., "1.2.3")
    version: String,
    /// Monotonic sequence number for ordering
    seq: u64,
    /// BLAKE3 hash of the version manifest
    manifest_hash: Hash,
    /// Changelog entry
    changelog: Option<String>,
    /// Timestamp of this version
    timestamp: Timestamp,
    /// Parent version (for delta tracking)
    parent_version: Option<String>,
}

/// Version manifest stored as a doc entry
#[derive(Serialize, Deserialize)]
struct VersionManifest {
    /// Package name
    name: String,
    /// Current version
    current: DataVersion,
    /// Available versions (latest N)
    history: Vec<DataVersion>,
    /// File listing for this version
    files: Vec<FileEntry>,
}

impl SyncwebFolder {
    /// Create a new version of this data package
    async fn bump_version(&mut self, changelog: &str) -> Result<DataVersion>;

    /// Check if a newer version is available
    async fn check_update(&self) -> Result<Option<DataVersion>>;

    /// Update to a specific version
    async fn update_to(&mut self, version: &str) -> Result<()>;

    /// Get version history
    async fn version_history(&self) -> Result<Vec<DataVersion>>;
}
```

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

**CLI:**
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

**Benefits:**
- **Instant snapshots**: No data copying, just reference existing blobs
- **Space efficient**: Multiple snapshots share unchanged blobs (deduplication)
- **Portable**: Snapshots can be shared via tickets (like public folders)
- **Verified**: BLAKE3 ensures snapshot integrity

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

**Parsing priority:**
1. `syncweb://blob/` - Public blob access
2. `syncweb://node/` - Direct node connection  
3. `syncweb://<ticket>` - Doc ticket (folder access)
4. `sync://` - Legacy format (auto-convert to iroh)

Note: The `?r` suffix is cryptographically enforced — read tickets don't contain the `NamespaceSecret` needed for writes. Removing `?r` doesn't upgrade access, it changes the ticket type.

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

When syncing a folder, some blobs may have many seeders while others are poorly replicated. `FetchFilter` supports `min_peers`/`max_peers` to preferentially download the **least-seeded blobs**, and `min_count`/`max_count` to limit how many blobs are fetched:

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

**CLI:**
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

**Benefits:**
- **Network health**: Poorly-seeded blobs get copied to more peers
- **Resilience**: Rare content becomes more available over time
- **No central coordination**: Each node independently decides what to seed
- **Efficient**: Uses existing peer tracker data, no extra protocol needed

---

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

**CLI:**
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

**Resolution strategy** (default):
- Best-effort: at decode time, attempt to read both versions as text (UTF-8)
- If both versions are decodable as text **and** the generated diff is smaller than the latest LWW winner, save a diff file instead of the full file
- Otherwise, save the full file (both versions kept)
- The winning version always stays at the original path (LWW by timestamp)

**File naming for conflicts:**
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

**CLI:**
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
- **Memory-efficient**: Bitmask presence tracking for 1000+ peers
- **Adaptive eviction**: LRU or FIFO based on usage patterns

---

## Syncthing Relay Piggyback (Phase 2)

### Problem

iroh uses QUIC with UDP hole punching (QNT). This works in ~90% of network configurations. But when both peers are behind different strict CGNATs that block UDP, iroh's hole punching fails. Syncthing's relay infrastructure is TCP-based and works in these scenarios.

### Design

The goal is **not** to translate between BEP and iroh protocols. Instead, we **piggyback on Syncthing's relay network** as a transport layer when iroh's direct/relay connectivity fails. Both endpoints remain iroh-syncthing nodes.

```text
iroh-syncthing Node A (behind CGNAT-A, UDP blocked)
    ↓ TCP + TLS (BEP session mode, as a transport tunnel)
Syncthing Relay (tcp://relay.syncthing.net)
    ↓ TCP (session mode, plain relay between devices)
iroh-syncthing Node B (behind CGNAT-B, UDP blocked)
```

The key insight: Syncthing relays are **protocol-agnostic**. They relay raw bytes between two devices that share a session key. We can tunnel iroh QUIC traffic through a Syncthing relay session by wrapping QUIC datagrams in the relay's byte-stream protocol.

### Implementation

```rust
/// Syncthing relay transport — fallback when iroh's QUIC hole punching fails
struct SyncthingRelayTransport {
    /// TCP connection to Syncthing relay
    tcp_stream: TcpStream,
    /// TLS wrapper (BEP-compatible handshake)
    tls: TlsStream<TcpStream>,
    /// Session key from relay protocol
    session_key: [u8; 32],
}

impl SyncthingRelayTransport {
    /// Connect to Syncthing relay and establish a session with a peer
    /// Uses the relay protocol v1: JoinRelayRequest → SessionInvitation → JoinSessionRequest
    async fn connect(relay_url: &str, peer_device_id: &DeviceId) -> Result<Self>;

    /// Tunnel iroh QUIC datagrams through the relay session
    /// The relay just forwards bytes — we encapsulate QUIC packets inside
    fn tunnel_quic(&self, quic_socket: &QuicSocket) -> Result<()>;
}

/// Transport fallback manager — tries iroh first, falls back to Syncthing relay
struct TransportFallback {
    /// Primary: iroh QUIC (direct + iroh relay)
    iroh_endpoint: Endpoint,
    /// Fallback: Syncthing relay tunnel
    syncthing_relay: Option<SyncthingRelayTransport>,
    /// Config
    config: RelayConfig,
}

impl TransportFallback {
    /// Connect to a peer, trying iroh first, then Syncthing relay
    async fn connect(&self, peer: &NodeId) -> Result<Box<dyn Transport>> {
        // 1. Try iroh direct connection (QUIC hole punch)
        if let Ok(conn) = self.iroh_endpoint.connect(peer, ALPN).await {
            return Ok(Box::new(conn));
        }

        // 2. Try iroh relay (if configured)
        if let Ok(conn) = self.iroh_endpoint.connect_via_relay(peer, ALPN).await {
            return Ok(Box::new(conn));
        }

        // 3. Fall back to Syncthing relay tunnel
        if let Some(relay) = &self.syncthing_relay {
            let tunnel = relay.connect(&self.config.relay_url, &peer.to_device_id()).await?;
            return Ok(Box::new(tunnel));
        }

        Err(Error::NoTransportAvailable)
    }
}
```

### Syncthing Relay Protocol (v1)

The relay protocol has two modes:

1. **Protocol mode** (TLS): Join relay, wait for session invitations
2. **Session mode** (plain): Relay bytes between two devices

We use protocol mode to register with the relay and receive session invitations, then session mode to tunnel QUIC traffic:

```rust
/// Syncthing relay protocol messages (XDR-encoded)
enum RelayMessage {
    JoinRelayRequest { device_id: [u8; 32] },
    ConnectRequest { device_id: [u8; 32] },
    SessionInvitation { session_key: [u8; 32], server_socket: bool },
    ResponseSuccess,
    ResponseNotFound,
    RelayFull,
}
```

### Configuration

```toml
[bep]
# Enable Syncthing relay fallback (for CGNAT traversal)
enabled = true
# Syncthing relay URLs (from Syncthing's config, or public relays)
relay_urls = ["tcp://relay.syncthing.net:22270"]
# Timeout for relay connection attempt (seconds)
relay_timeout = 10
# Auto-detect CGNAT and use relay when needed
auto_fallback = true
```

### CLI Usage

```bash
# Enable relay fallback globally
syncweb config set bep.enabled true

# Or per-connection
syncweb join --relay-fallback syncweb://folder-id#NODE-ID

# Test relay connectivity
syncweb network test-relay
```

### What This Enables

- Two iroh-syncthing nodes behind different CGNATs can communicate
- Automatic fallback: iroh tries direct first, falls back to relay only when needed
- No dependency on iroh's relay infrastructure for the data path
- Leverages Syncthing's mature, well-tested relay network
- Both nodes remain fully iroh-syncthing — no protocol translation needed

### Device Identity Compatibility

Syncthing and Iroh both use Ed25519 keypairs. The 56-char Syncthing Device ID (base32, grouped) and the 52-char Iroh NodeId (base32) are derived from the same key. Conversion is zero-cost re-encoding:

```rust
impl DeviceId {
    /// Convert from Syncthing Device ID (56-char base32, grouped)
    fn from_syncthing(id: &str) -> Result<Self>;

    /// Convert to Syncthing Device ID format
    fn to_syncthing(&self) -> String;

    /// Get the underlying Iroh NodeId
    fn to_node_id(&self) -> NodeId;
}
```

This allows `syncweb devices` to display both formats and config files to reference devices by either format.

---

## Filter Engine (Replaces Shell Scripts)

### Design

The `automatic` daemon uses a Rust-native filter engine instead of shell scripts:

```toml
# ~/.config/iroh-syncthing/filters.toml
[general]
sort_mode = "niche"  # niche, frecency, peers, size, random
limit_size = "10GB"
min_seeders = 1

[[rules]]
type = "accept"
match = { name = "*.iso", min_size = "100MB" }

[[rules]]
type = "reject"
match = { name = "*.tmp", age = "7d" }

[[rules]]
type = "accept"
match = { path = "/important/", min_seeders = 3 }

[[rules]]
type = "reject"
match = { ext = ["log", "cache"] }

[[rules]]
type = "accept"
match = { version = ">=1.2.0" }  # Data package version filter
```

### Filter Engine Implementation

```rust
struct FilterEngine {
    rules: Vec<FilterRule>,
    sort_mode: SortMode,
    limit_size: Option<u64>,
    min_seeders: Option<usize>,
}

#[derive(Serialize, Deserialize)]
struct FilterRule {
    action: FilterAction,
    match_criteria: MatchCriteria,
}

#[derive(Serialize, Deserialize)]
struct MatchCriteria {
    name: Option<Pattern>,      // glob pattern
    ext: Option<Vec<String>>,   // file extensions
    path: Option<Pattern>,      // path pattern
    min_size: Option<u64>,
    max_size: Option<u64>,
    age: Option<Duration>,      // files older than
    min_seeders: Option<usize>,
    version: Option<String>,    // semver constraint
}

impl FilterEngine {
    /// Load from config file
    fn load(config_path: &Path) -> Result<Self>;

    /// Evaluate if a doc entry should be accepted/rejected
    fn evaluate(&self, entry: &DocEntry) -> FilterAction;

    /// Sort and limit results
    fn sort_and_limit(&self, entries: Vec<DocEntry>) -> Vec<DocEntry>;

    /// Run the automatic daemon loop
    async fn run_daemon(&self, node: &IrohNode) -> Result<()>;
}
```

### CLI Commands

```bash
# Run automatic daemon with filters
syncweb automatic

# Show active filters
syncweb automatic --show-filters

# Test filter against specific files
syncweb automatic --dry-run --paths /path/to/files

# Reload filters
syncweb automatic --reload
```

---

## Logging & Observability

Structured logging via the `tracing` crate (not `log`). Async-aware, structured fields, configurable levels.

### Setup

```rust
use tracing_subscriber::{fmt, EnvFilter};

fn setup_logging(verbose: bool, trace: bool, log_file: Option<&Path>) {
    let filter = if trace {
        EnvFilter::new("trace")
    } else if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_file(true)
        .with_line_number(true)
        .with_target(false);

    if let Some(path) = log_file {
        let file_appender = tracing_appender::rolling::daily(path, "syncweb.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        subscriber.with_writer(non_blocking).init();
    } else {
        subscriber.init();
    }
}
```

### Usage

```rust
#[instrument(skip_all, fields(folder = %folder_id))]
async fn sync_folder(&self, folder_id: &NamespaceId) -> Result<()> {
    let peers = self.get_peers(folder_id).await?;
    info!(peer_count = peers.len(), "starting sync");
    
    for peer in &peers {
        debug!(peer = %peer, "connecting");
        // ...
    }
    
    info!(files_synced = 42, bytes = 1_048_576, "sync complete");
    Ok(())
}
```

### CLI Flags

| Flag | Level | Use Case |
|------|-------|----------|
| (default) | `info` | Normal operation |
| `--verbose` / `-v` | `debug` | Debugging sync issues |
| `--trace` | `trace` | Maximum detail (protocol-level) |
| `--log-file <path>` | (as above) | Write to rotating log file |

Log files rotate daily, 7-day retention (via `tracing-appender`).

---

## Sync Schedules

Time-based sync rules for bandwidth management. Global settings with per-folder overrides.

### Configuration

```toml
# ~/.config/iroh-syncthing/schedules.toml (or inline in config.toml)

[schedule]
# Global active hours (empty = always active)
active_hours = ""

# Bandwidth limits by time of day
[[schedule.bandwidth]]
hours = "08:00-18:00"
max_upload = "1MB/s"
max_download = "5MB/s"

[[schedule.bandwidth]]
hours = "18:00-08:00"
max_upload = "0"   # unlimited
max_download = "0"

# Per-folder overrides (inherites global, overrides specific fields)
[schedule.folders.media]
active_hours = "01:00-05:00"
max_download = "50MB/s"

[schedule.folders.backups]
# Always sync backups, no bandwidth limit
active_hours = ""
max_upload = "0"
max_download = "0"
```

### Implementation

```rust
struct ScheduleManager {
    global: Schedule,
    folder_overrides: HashMap<NamespaceId, Schedule>,
}

struct Schedule {
    active_hours: Option<(u8, u8)>,  // (start_hour, end_hour) in 24h
    bandwidth_limits: Vec<BandwidthWindow>,
}

struct BandwidthWindow {
    hours: (u8, u8),
    max_upload: Option<u64>,    // bytes/sec, None = unlimited
    max_download: Option<u64>,
}

impl ScheduleManager {
    /// Check if sync is currently allowed (within active hours)
    fn is_active(&self, folder: Option<&NamespaceId>) -> bool;

    /// Get current bandwidth limits (considering time of day)
    fn current_limits(&self, folder: Option<&NamespaceId>) -> BandwidthLimits;
}
```

### CLI

```bash
syncweb schedule                     # Show current schedule
syncweb schedule set --active "22:00-06:00"
syncweb schedule set --bandwidth "1MB/s" --period "08:00-18:00"
syncweb schedule folder media --active "01:00-05:00"
```

---

## Platform Settings Files

Suggested global configuration files for different use cases. These are not a "template" subcommand — just example configs users can copy.

### Profiles

```toml
# ~/.config/iroh-syncthing/config-laptop.toml
# Optimized for battery life and mobile networks

[bandwidth]
max_upload = "500KB/s"
max_download = "2MB/s"

[schedule]
active_hours = "08:00-22:00"
[[schedule.bandwidth]]
hours = "08:00-18:00"
max_upload = "250KB/s"
max_download = "1MB/s"

[parallel]
threads = 2  # Limit CPU usage

[cache]
max_cache_size = 5000  # Smaller cache for limited RAM
```

```toml
# ~/.config/iroh-syncthing/config-server.toml
# Optimized for throughput and availability

[bandwidth]
max_upload = "0"   # unlimited
max_download = "0"

[schedule]
active_hours = ""  # always active

[parallel]
threads = 0  # auto-detect (use all cores)

[cache]
max_cache_size = 50000  # Large cache for plenty of RAM
```

```toml
# ~/.config/iroh-syncthing/config-phone.toml
# Optimized for storage and battery

[bandwidth]
max_upload = "100KB/s"
max_download = "500KB/s"

[parallel]
threads = 1  # Single-threaded for battery

[advanced]
blob_cache_size_gb = 2  # Limited storage
```

Users copy the relevant file to `~/.config/iroh-syncthing/config.toml` and customize as needed.

---

## Integrity Verification

General integrity checking beyond package verification. iroh-blobs verifies every blob on fetch (BLAKE3), but this re-checks local blobs against doc entries.

### Implementation

```rust
struct IntegrityChecker {
    blob_store: BlobStore,
    docs: Docs,
}

struct VerifyResult {
    total: u64,
    verified: u64,
    corrupted: Vec<CorruptionInfo>,
    missing: Vec<PathBuf>,
}

struct CorruptionInfo {
    path: PathBuf,
    expected_hash: Hash,
    actual_hash: Hash,
}

impl IntegrityChecker {
    /// Verify all blobs in a folder match doc entries
    async fn verify_folder(&self, folder_id: &NamespaceId) -> Result<VerifyResult>;

    /// Verify a single file
    async fn verify_file(&self, path: &Path) -> Result<bool>;

    /// Background verification (configurable period)
    async fn periodic_verify(&self, interval: Duration) -> Result<()>;
}
```

### CLI

```bash
syncweb verify ./documents              # Verify all blobs
syncweb verify ./documents/photo.jpg    # Verify single file
```

---

## Bandwidth Accounting

Track upload/download per folder and per peer. Persisted across restarts.

### Implementation

```rust
struct BandwidthStats {
    total_upload: u64,
    total_download: u64,
    per_folder: HashMap<NamespaceId, FolderStats>,
    per_peer: HashMap<NodeId, PeerStats>,
    period_start: Instant,
}

struct FolderStats {
    upload: u64,
    download: u64,
    files_transferred: u64,
}

struct PeerStats {
    upload: u64,
    download: u64,
    connection_count: u32,
}
```

### CLI

```bash
syncweb stats                          # Show all stats
syncweb stats --period 24h             # Last 24 hours
syncweb stats --folder ./documents     # Per-folder breakdown
syncweb stats --peer <node-id>         # Per-peer breakdown
```

---

## Watch Mode (Lowest Priority)

File system watcher for real-time sync. Monitor local changes and sync automatically.

### Implementation

Uses the `notify` crate (Rust). On file change:
1. Detect change type (create/modify/delete)
2. Import modified files to blob store
3. Update doc entries
4. Debounce rapid changes (default 500ms)

```rust
struct Watcher {
    watcher: RecommendedWatcher,
    debounce: Duration,
    exclude_patterns: Vec<glob::Pattern>,
}

impl Watcher {
    async fn watch(&self, path: &Path, folder_id: &NamespaceId) -> Result<()> {
        // Watch for changes, debounce, import, update doc
    }
}
```

### CLI

```bash
syncweb watch ./documents                           # Monitor and sync
syncweb watch --debounce 500ms ./documents          # Custom debounce
syncweb watch --exclude ".git/" --exclude "node_modules/" ./documents
```

**Priority**: Lowest — implement after all core features are stable.

---

## Module Structure

```
iroh-syncthing/
+-- Cargo.toml
+-- src/
|   +-- main.rs                 # CLI entry point
|   +-- cli/
|   |   +-- mod.rs
|   |   +-- commands.rs         # Command definitions (clap)
|   |   +-- args.rs             # Arg parsing, validation
|   |   +-- output.rs           # Table formatting, JSON output
|   |
|   +-- node/
|   |   +-- mod.rs
|   |   +-- iroh_node.rs        # IrohNode: Endpoint + Router + protocols
|   |   +-- identity.rs         # Key management, device IDs
|   |   +-- relay.rs            # iroh-relay config (NAT traversal fallback)
|   |   +-- discovery.rs        # Gossip + DHT + local + topic tracker setup
|   |
|   +-- folder/
|   |   +-- mod.rs
|   |   +-- manager.rs          # FolderManager (create, join, list)
|   |   +-- syncweb_folder.rs   # SyncwebFolder struct + methods
|   |   +-- sync_mode.rs        # SyncMode enum + behavior
|   |   +-- ignore.rs           # Ignore patterns
|   |   +-- capabilities.rs     # Capability management
|   |   +-- public.rs           # Public read-only folder support
|   |   +-- versioning.rs       # Data version tracking
|   |
|   +-- sync/
|   |   +-- mod.rs
|   |   +-- engine.rs           # SyncEngine (orchestrates blob + doc sync)
|   |   +-- actor.rs            # Actor (dedicated storage thread) - from iroh-willow
|   |   +-- session.rs          # SessionMode (ReconcileOnce, Continuous) - from iroh-willow
|   |   +-- intents.rs          # IntentHandle (Stream + Sink) - from iroh-willow
|   |   +-- blob_sync.rs        # iroh-blobs integration
|   |   +-- doc_sync.rs         # iroh-docs integration
|   |   +-- lazy_fetch.rs       # Selective sync (on-demand blob fetch)
|   |   +-- progress.rs         # Progress tracking, stats
|   |   +-- peer_tracker.rs     # Cached peer availability from natural flow
|   |   +-- subscribe.rs        # SubscribeParams, subscription filtering - from iroh-willow
|   |   +-- deleted.rs          # DeletedTracker, PruneEvent - from iroh-willow
|   |
|   +-- fs/
|   |   +-- mod.rs
|   |   +-- watcher.rs          # notify-rs file watcher
|   |   +-- scanner.rs          # Directory scanner, hashing (parallel - (standard CS pattern: parallel directory traversal))
|   |   +-- importer.rs         # Import local files to blob store
|   |   +-- exporter.rs         # Export blobs to local filesystem
|   |   +-- ignore_filter.rs    # Apply ignore patterns
|   |
|   +-- net/
|   |   +-- mod.rs
|   |   +-- gossip.rs           # iroh-gossip topics
|   |   +-- discovery.rs        # Peer discovery (gossip + DHT + local)
|   |   +-- topic_tracker.rs    # distributed-topic-tracker integration
|   |   +-- network.rs          # Network struct + management
|   |   +-- network_manager.rs  # NetworkManager (create, join, leave, invite, kick)
|   |   +-- bep_bridge.rs       # BEP relay bridge (opt-in with --bep)
|   |   +-- bep_identity.rs     # BEP DeviceId ↔ Iroh NodeId conversion (Phase 2)
|   |   +-- tickets.rs          # Ticket parsing/generation
|   |
|   +-- filter/
|   |   +-- mod.rs              # Filter engine
|   |   +-- rules.rs            # Filter rule definitions
|   |   +-- evaluator.rs        # Rule evaluation
|   |   +-- config.rs           # Filter config parsing
|   |
|   +-- cli_commands/
|   |   +-- mod.rs
|   |   +-- create.rs           # syncweb create
|   |   +-- join.rs             # syncweb join
|   |   +-- accept.rs           # syncweb accept
|   |   +-- drop.rs             # syncweb drop
|   |   +-- ls.rs               # syncweb ls
|   |   +-- find.rs             # syncweb find
|   |   +-- download.rs         # syncweb download
|   |   +-- sort.rs             # syncweb sort
|   |   +-- stat.rs             # syncweb stat
|   |   +-- devices.rs          # syncweb devices
|   |   +-- folders.rs          # syncweb folders
|   |   +-- automatic.rs        # syncweb automatic (with filter engine)
|   |   +-- init.rs             # syncweb init (create folder + URL)
|   |   +-- config.rs           # syncweb config (show/set settings)
|   |   +-- network.rs          # syncweb network (create/ls/join/leave/invite/kick)
|   |   +-- repl.rs             # syncweb repl
|   |   +-- publish.rs          # syncweb publish
|   |   +-- subscribe.rs        # syncweb subscribe
|   |   +-- version.rs          # syncweb version (data packages)
|   |
|   +-- package/
|   |   +-- mod.rs
|   |   +-- manifest.rs       # PackageManifest, PackageFileEntry, PackageDependency
|   |   +-- publish.rs        # Publish workflow (pin + announce on gossip)
|   |   +-- catalog.rs        # Gossip-based package discovery + search
|   |   +-- install.rs        # Install/upgrade/remove + atomic symlink swap
|   |   +-- state.rs          # Local PackageState tracking
|   |   +-- verify.rs         # Integrity verification against manifest
|   |
|   +-- storage/
|   |   +-- mod.rs
|   |   +-- config.rs           # Persistent config (TOML)
|   |   +-- migrations.rs       # Schema migrations
|   |
|   +-- util/
|       +-- mod.rs
|       +-- path.rs             # Path utilities
|       +-- format.rs           # Human formatting
|       +-- error.rs            # Error types
```

---

## Parallel Scanning ((standard CS pattern: parallel directory traversal))

Shared-memory parallel primitives for fast directory scanning and file operations.

### Parallel Directory Scanner

```rust
use rayon::prelude::*;

/// Parallel directory scanner using work-stealing
/// Inspired by standard CS pattern: parallel directory traversal
struct ParallelScanner {
    /// Number of parallel threads (default: num_cpus)
    num_threads: usize,
    /// Maximum files per batch before yielding
    batch_size: usize,
}

impl ParallelScanner {
    /// Scan directory tree in parallel
    fn scan_parallel(&self, root: &Path) -> Vec<FileEntry> {
        let dirs = self.collect_dirs(root);
        
        dirs.par_iter()
            .flat_map(|dir| self.scan_directory(dir))
            .collect()
    }

    /// Collect all directories first (parallel)
    fn collect_dirs(&self, root: &Path) -> Vec<PathBuf> {
        let mut dirs = vec![root.to_path_buf()];
        let mut i = 0;
        
        while i < dirs.len() {
            let current = &dirs[i];
            if let Ok(entries) = std::fs::read_dir(current) {
                let new_dirs: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .map(|e| e.path())
                    .collect();
                dirs.extend(new_dirs);
            }
            i += 1;
        }
        
        dirs
    }

    /// Scan single directory (called in parallel)
    fn scan_directory(&self, dir: &Path) -> Vec<FileEntry> {
        std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .filter_map(|e| {
                let metadata = e.metadata().ok()?;
                let hash = blake3::hash(&std::fs::read(e.path()).ok()?);
                Some(FileEntry {
                    path: e.path(),
                    size: metadata.len(),
                    hash: hash.into(),
                    modified: metadata.modified().ok(),
                })
            })
            .collect()
    }

    /// Parallel hash computation for large files
    fn hash_file_parallel(&self, path: &Path) -> Result<Hash> {
        let data = std::fs::read(path)?;
        let hash = blake3::Hasher::new().update(&data).finalize();
        Ok(hash.into())
    }
}
```

### Parallel Import Pipeline

```rust
/// Parallel import pipeline for adding files to blob store
struct ParallelImporter {
    scanner: ParallelScanner,
    blob_store: BlobStore,
    /// Channel for sending entries to blob store
    import_tx: mpsc::Sender<ImportCommand>,
}

impl ParallelImporter {
    /// Import directory in parallel
    async fn import_parallel(&self, root: &Path) -> Result<ImportStats> {
        let entries = self.scanner.scan_parallel(root);
        let stats = Arc::new(Mutex::new(ImportStats::default()));
        
        // Process entries in parallel batches
        entries.par_iter()
            .for_each(|entry| {
                let stats = stats.clone();
                let tx = self.import_tx.clone();
                
                // Hash and send to blob store
                if let Ok(hash) = self.scanner.hash_file_parallel(&entry.path) {
                    let blob_entry = BlobEntry {
                        hash,
                        size: entry.size,
                        path: entry.path.clone(),
                    };
                    
                    tx.send(ImportCommand::Add(blob_entry)).ok();
                    
                    let mut s = stats.lock().unwrap();
                    s.files_imported += 1;
                    s.bytes_imported += entry.size;
                }
            });
        
        Ok(Arc::try_unwrap(stats).unwrap().into_inner().unwrap())
    }
}

/// Statistics for parallel import
struct ImportStats {
    files_imported: u64,
    bytes_imported: u64,
    errors: u64,
}
```

### Parallel Export Pipeline

```rust
/// Parallel export pipeline for extracting blobs to filesystem
struct ParallelExporter {
    blob_store: BlobStore,
    /// Number of parallel export threads
    num_threads: usize,
}

impl ParallelExporter {
    /// Export blobs to filesystem in parallel
    fn export_parallel(&self, entries: &[BlobEntry], output_dir: &Path) -> Result<ExportStats> {
        entries.par_iter()
            .map(|entry| self.export_single(entry, output_dir))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .fold(ExportStats::default(), |mut acc, stats| {
                acc.files_exported += stats.files_exported;
                acc.bytes_exported += stats.bytes_exported;
                acc
            })
    }

    /// Export single blob
    fn export_single(&self, entry: &BlobEntry, output_dir: &Path) -> Result<ExportStats> {
        let data = self.blob_store.get(entry.hash)?;
        let output_path = output_dir.join(&entry.path);
        
        std::fs::create_dir_all(output_path.parent().unwrap())?;
        std::fs::write(&output_path, data)?;
        
        Ok(ExportStats {
            files_exported: 1,
            bytes_exported: entry.size,
        })
    }
}
```

### CLI Integration

Parallel scanning is **default on**. Streaming output is default unless `--sort` is used (which requires collecting all results).

```bash
# Parallel scan (default, auto-detect CPU count)
syncweb ls

# Disable parallelism (single-threaded)
syncweb ls --threads=1

# Scan with specific thread count
syncweb ls --threads=8

# Parallel import (default)
syncweb import /path/to/files

# Parallel export (default)
syncweb export /path/to/output

# Streaming output (default) - results appear as found
syncweb ls /path/to/files

# Sorted output (collects all results first, then sorts)
syncweb ls --sort size /path/to/files
syncweb ls --sort name /path/to/files
syncweb ls --sort mtime /path/to/files
```

### Performance Benefits

| Operation | Sequential | Parallel (8 cores) | Speedup |
|-----------|------------|-------------------|---------|
| Scan 10k files | ~2.5s | ~0.4s | 6.25x |
| Hash 1GB file | ~1.2s | ~0.3s | 4x |
| Import 1000 files | ~15s | ~2.5s | 6x |
| Export 1000 files | ~12s | ~2s | 6x |

---

## `find` Command Design

Full-text search across folder entries with regex, glob, or exact substring matching,
plus depth, size, time, extension, and type filters. Uses doc metadata only — no blob download needed.

### Search Parameters

```rust
/// Search parameters for find command
struct FindParams {
    /// Search paths (folder roots or subpaths)
    search_paths: Vec<PathBuf>,
    /// Pattern type: regex, glob, or exact substring
    pattern_type: PatternType,
    /// User-provided pattern(s)
    patterns: Vec<String>,
    /// Case sensitivity
    ignore_case: bool,
    /// File type filter: "f" (file) or "d" (directory)
    file_type: Option<FileType>,
    /// Depth constraints
    min_depth: Option<usize>,
    max_depth: Option<usize>,
    /// Size constraints (>1GB, <500MB, =100MB, etc.)
    sizes: Option<Vec<SizeConstraint>>,
    /// Time modified constraints (<7d, >30d, etc.)
    time_modified: Option<Vec<TimeConstraint>>,
    /// File extension filter
    ext: Option<Vec<String>>,
    /// Show hidden files
    hidden: bool,
    /// Search full path vs. just filename
    full_path: bool,
    /// Output format
    output: OutputFormat,
}

enum PatternType {
    Regex,
    Glob,
    Exact,
}

enum SizeConstraint {
    GreaterThan(u64),
    LessThan(u64),
    EqualTo(u64),
    /// Within percentage range of a value
    Within(u64, f64),
}

enum TimeConstraint {
    OlderThan(Duration),
    NewerThan(Duration),
}
```

### Search Engine

```rust
/// Dispatches to the appropriate pattern engine
fn search_entries(params: &FindParams, entries: &[DocEntry]) -> Vec<DocEntry> {
    entries.iter()
        .filter(|e| matches_all_constraints(params, e))
        .collect()
}

fn matches_all_constraints(p: &FindParams, entry: &DocEntry) -> bool {
    let target = if p.full_path { &entry.path } else { &entry.filename };

    let pattern_ok = match p.pattern_type {
        PatternType::Regex => regex_match(target, &p.patterns, p.ignore_case),
        PatternType::Glob => glob_match(target, &p.patterns, p.ignore_case),
        PatternType::Exact => exact_match(target, &p.patterns, p.ignore_case),
    };

    let type_ok = p.file_type.map_or(true, |ft| ft == entry.file_type);
    let ext_ok = p.ext.as_ref().map_or(true, |exts| {
        exts.iter().any(|ext| entry.filename.ends_with(ext))
    });
    let size_ok = p.sizes.as_ref().map_or(true, |sizes| {
        sizes.iter().all(|s| s.matches(entry.size))
    });
    let time_ok = p.time_modified.as_ref().map_or(true, |times| {
        times.iter().all(|t| t.matches(entry.modified))
    });
    let depth_ok = p.min_depth.map_or(true, |d| entry.depth >= d)
        && p.max_depth.map_or(true, |d| entry.depth <= d);

    pattern_ok && type_ok && ext_ok && size_ok && time_ok && depth_ok
}
```

### CLI

```bash
# Regex search (default)
syncweb find '.*\.mp3$' music/

# Glob search
syncweb find --glob '**/*.mp3' music/

# Fixed/exact search (substring)
syncweb find --fixed-string 'beethoven' music/

# Combined filters
syncweb find --type f --ext mp3 --min-size 10MB --max-size 500MB music/

# Time filters
syncweb find 'report.*\.md$' --modified-within 7d
syncweb find 'old-log.*' --modified-before 30d

# Depth constraints
syncweb find --depth +2 --depth -5 'config.*'

# Pipe to download
syncweb find '*.iso' linux/ | syncweb download -

# Pipe from stdin
syncweb find --print '*.mp3' audio/ | xargs -I{} syncweb download {}
```

**Output** (simple, pipe-friendly):
```
syncweb find '*.pdf' docs/
docs/manual.pdf
docs/reference.pdf
docs/getting-started.pdf
```

---

## `stat` Command Design

File metadata from doc entries + blob store, showing detailed info similar to `stat(1)`.
Shows local vs. global diffs (for conflict detection), availability (peer count), version vectors.

### Stat Output Structure

```rust
struct StatOutput {
    /// Basic info
    path: PathBuf,
    size: u64,
    num_blocks: u64,
    file_type: FileType,
    permissions: String,

    /// Timing
    modified: Timestamp,
    inode_change: Timestamp,
    modified_by: NodeId,

    /// Version info
    version: Vec<Version>,

    /// Availability
    available_on: Vec<NodeId>,

    /// Local vs Global differences (for conflict detection)
    diffs: Vec<DiffEntry>,
}

struct DiffEntry {
    key: String,
    local_value: String,
    global_value: String,
}

impl StatOutput {
    fn display(&self, format: StatFormat);
    fn diff_with_global(&self) -> Vec<DiffEntry>;
}

enum StatFormat {
    /// Human-readable (default)
    Human,
    /// Terse, pipe-separated (for scripting)
    Terse,
    /// Custom template string
    Custom(String),
}
```

### CLI

```bash
# Default human-readable stat
syncweb stat docs/report.md
```

**Output:**
```
  Path: docs/report.md
  Size: 245760             Blocks: 12             regular file
  Device: alice, bob       Version: v1, v2, v3
  Access: (0644/---------)
  Modify: 2024-01-15 10:30:00.000000 +0000 (alice)
  Change: 2024-01-15 10:25:00.000000 +0000
```

With local/global differences:
```
  Key        Local              Global
  size       245760             250000
  modified   1705317000         1705320000
```

```bash
# Terse format (for scripting)
syncweb stat --terse docs/report.md
# Output: report.md|245760|12|0644|regular file|1705317000|3|0

# Custom format
syncweb stat --format '%n %s %y' docs/report.md

# Multiple files
syncweb stat docs/*.md
```

---

## `sort` Command Design

Sort files by multiple criteria: niche (how close to N seeders), frecency (popular + recent),
peers/seeds, size, date, random. Supports folder-level aggregates for sorting by folder stats.
Uses the PeerTracker for seed/availability data.

### Sort Engine

```rust
/// Sort criteria
enum SortCriterion {
    /// Number of peers/blobs
    Peers { reverse: bool },
    /// Niche: |num_peers - target| — find blobs with ~N seeders
    Niche { target: usize, reverse: bool },
    /// Frecency: peers - (days_since_modified / weight) — popular + recent
    Frecency { weight: f64, reverse: bool },
    /// File size
    Size { reverse: bool },
    /// Modified time
    Time { reverse: bool },
    /// Random (stable per session)
    Random,
    /// Folder-level aggregate
    FolderAggregate { field: AggregateField, agg: AggregateFunc, reverse: bool },
}

enum AggregateField { Size, Modified }
enum AggregateFunc { Sum, Mean, Median, Min, Max, Count }

/// Sort engine — uses PeerTracker for availability data
struct Sorter {
    criteria: Vec<SortCriterion>,
    filters: Vec<SortFilter>,
    peer_tracker: PeerTracker,
}

impl Sorter {
    /// Compute sort key for an entry
    fn sort_key(&self, entry: &DocEntry, folder_aggregates: &FolderAggregates) -> Vec<OrderedFloat<f64>>;

    /// Sort entries in place
    fn sort(&self, entries: &mut [DocEntry]);

    /// Apply size/count limit after sorting
    fn limit(&self, entries: &[DocEntry]) -> Vec<DocEntry>;
}

/// Folder-level aggregates (computed from doc entries)
struct FolderAggregates {
    aggregates: HashMap<PathBuf, FolderAggregate>,
}

struct FolderAggregate {
    size_sum: u64,
    size_median: f64,
    size_mean: f64,
    modified_median: i64,
    file_count: usize,
}
```

### CLI

```bash
# Default: niche + frecency
syncweb sort music/

# Sort by peers (most seeded first)
syncweb sort --sort peers music/

# Sort by niche (files with ~3 seeders)
syncweb sort --sort niche music/
syncweb sort --sort +niche music/      # most niche
syncweb sort --sort -niche music/      # least niche

# Sort by frecency (popular + recent)
syncweb sort --sort frecency music/

# Sort by folder size (largest folder first)
syncweb sort --sort folder-size music/

# Combined: peers primary, time secondary
syncweb sort --sort peers --sort time music/

# With limits
syncweb sort --limit-size 10GB music/
syncweb sort --min-seeders 2 music/

# Pipe to download
syncweb sort --sort niche music/ | syncweb download -
```

---

## `init`/`config` Command Design

The `init` command creates a folder and outputs a shareable URL. The `config` command
manages local configuration (data dir, default paths, sync modes, filters).

### Init Command

```rust
struct InitResult {
    folder_id: String,
    share_url: String,
    path: PathBuf,
    namespace_id: NamespaceId,
}

impl IrohNode {
    /// Initialize a folder: create dir, set up namespace, output URL
    async fn init_folder(&self, path: &Path, opts: InitOptions) -> Result<InitResult>;
}
```

**CLI:**
```bash
# Create folder + output URL
syncweb init ./documents
# Output: sync://documents#<device-id>

# Init with sync mode
syncweb init --sync-mode sendreceive ./documents

# Init with network membership
syncweb init --network work ./documents

# Init with description
syncweb init --label "Work Documents" ./documents
```

### Config Command

```rust
impl IrohNode {
    async fn get_config(&self) -> Result<Config>;
    async fn set_config(&self, patch: ConfigPatch) -> Result<()>;
}
```

**CLI:**
```bash
# Show config
syncweb config
# Shows: data_dir, default_path, relay, discovery, networks, etc.

# Modify config
syncweb config set data_dir ~/.iroh-syncthing
syncweb config set default_path ~/Syncweb
syncweb config set default_sync_mode SendReceive

# Config sections
syncweb config show networks
syncweb config show bep
syncweb config show filter
```

---

## Implementation Phases

### Phase 1: Foundation
**Goal**: IrohNode + basic identity + storage + logging

- [ ] `Cargo.toml` with correct iroh 1.0.2 dependencies + distributed-topic-tracker 0.3.5
- [ ] `IrohNode` - Endpoint + Router + protocol setup
- [ ] `IdentityManager` - SecretKey persistence, NodeId
- [ ] `BlobStore` - iroh-blobs persistent store
- [ ] `DocsEngine` - iroh-docs setup
- [ ] `GossipService` - iroh-gossip setup
- [ ] `TopicTracker` - distributed-topic-tracker integration (DHT-based peer discovery)
- [ ] Basic CLI with `clap`
- [ ] `tracing` structured logging setup
- [ ] `syncweb version`, `syncweb repl` commands

### Phase 2: Folder Core + Syncthing Relay Piggyback
**Goal**: Create/join folders, basic sync, Syncthing relay fallback for CGNAT traversal

- [ ] `SyncwebFolder` - NamespaceId, entries, blob refs
- [ ] `FolderManager` - create, join, list, accept, drop
- [ ] `SyncMode` implementations (SendReceive, SendOnly, ReceiveOnly)
- [ ] `syncweb create`, `syncweb join`, `syncweb accept`, `syncweb drop`
- [ ] `syncweb folders`, `syncweb devices`
- [ ] `DeviceId` bidirectional conversion (Syncthing ↔ Iroh Ed25519)
- [ ] `SyncthingRelayTransport` - TCP tunnel through Syncthing relays
- [ ] `TransportFallback` - iroh direct → iroh relay → Syncthing relay
- [ ] Syncthing relay protocol v1 client (JoinRelayRequest, SessionInvitation, JoinSessionRequest)
- [ ] QUIC-over-TCP tunnel encapsulation
- [ ] `--relay-fallback` flag on relevant commands
- [ ] `syncweb network test-relay` command
- [ ] Config: `[bep]` section for relay URLs, timeout, auto_fallback

### Phase 3: File Operations + Search/Sort/Stat
**Goal**: ls, find, sort, stat, download, selective sync, init/config

- [ ] `FsWatcher` - notify-rs
- [ ] `Scanner` - walk dir, BLAKE3 hash
- [ ] `ParallelScanner` - parallel directory scanning ((standard CS pattern: parallel directory traversal))
- [ ] `Importer` - add to blob store, update doc
- [ ] `ParallelImporter` - parallel import pipeline
- [ ] `Exporter` - export blobs to local filesystem
- [ ] `ParallelExporter` - parallel export pipeline
- [ ] `LazyFetch` - on-demand blob download (no .stignore needed)
- [ ] `Actor` - dedicated storage thread (from iroh-willow)
- [ ] `SessionMode` - ReconcileOnce vs Continuous (from iroh-willow)
- [ ] `IntentHandle` - Stream + Sink for sync operations (from iroh-willow)
- [ ] `FindEngine` - regex/glob/exact search with depth/size/time filters
- [ ] `Sorter` - niche, frecency, peers, random, folder-aggregate sorting
- [ ] `StatOutput` - detailed file metadata, availability, version vectors
- [ ] `InitResult` - folder creation + shareable URL output
- [ ] `syncweb ls`, `syncweb find`, `syncweb sort`, `syncweb stat`, `syncweb download`
- [ ] `syncweb init`, `syncweb config`
- [ ] Streaming output (default) vs `--sort` collected output

### Phase 4: Advanced Sync + Networks
**Goal**: Sync engine, automatic daemon, networks abstraction

- [ ] `SyncEngine` - orchestration
- [ ] Progress tracking, transfer stats
- [ ] `PeerTracker` - cached peer availability from natural iroh flow
- [ ] `PeerTracker` - age-based cache eviction ((standard CS pattern: age-based cache eviction))
- [ ] `EfficientPeerCache` - memory-efficient bitmask cache ((standard CS pattern: memory-efficient bitmask presence))
- [ ] `FilterEngine` - rules-based automatic daemon
- [ ] `SubscribeParams` - subscription filtering (from iroh-willow)
- [ ] `DeletedTracker` - track deleted-but-previously-seen files (from iroh-willow)
- [ ] `AreaOfInterest` with limits (max_size, max_count) (from iroh-willow)
- [ ] `Network` struct + `NetworkManager` - create, join, leave, invite, kick
- [ ] Network gossip topics (`iroh-syncthing/net/<id>`)
- [ ] `syncweb automatic` with filter engine
- [ ] `syncweb subscribe` with SubscribeParams
- [ ] `syncweb network create`, `syncweb network ls`, `syncweb network join`
- [ ] `syncweb network leave`, `syncweb network invite`, `syncweb network kick`
- [ ] `syncweb create --network <name>`, `syncweb join --network <name>`

### Phase 5: Public Folders + Data Packages
**Goal**: Public sharing + data package versioning

- [ ] `SyncMode::PublicReadOnly`
- [ ] Blob ticket generation
- [ ] Content pinning (prevent GC for shared blobs)
- [ ] `syncweb publish`, `syncweb unpublish`, `syncweb subscribe`
- [ ] `PackageManifest` struct + iroh-docs storage
- [ ] `PackageState` local tracking (installed packages, versions)
- [ ] `syncweb package init` — initialize folder as data package
- [ ] `syncweb package add` — scan + hash files, update manifest
- [ ] `syncweb package bump` — create new version with changelog
- [ ] `syncweb package publish` — blob ticket + gossip announcement
- [ ] `syncweb package search` — discover packages via gossip
- [ ] `syncweb package info` — detailed package metadata
- [ ] `syncweb package install` — fetch + verify + stage + atomic swap
- [ ] `syncweb package upgrade` — update to latest version
- [ ] `syncweb package remove` — clean up installed package
- [ ] `syncweb package verify` — integrity check against manifest
- [ ] `syncweb package list` — list locally installed packages
- [ ] `syncweb package versions` — list installed versions
- [ ] `syncweb package switch` — change active version
- [ ] Multi-version coexistence (versioned dirs + `current` symlink)
- [ ] Atomic upgrade (stage → verify → symlink swap → cleanup)

### Phase 6: Backup/Snapshot + Partial Fetch
**Goal**: Content-addressed snapshots + robustness fetch

- [ ] `syncweb backup` - create content-addressed snapshot
- [ ] `syncweb restore` - restore from snapshot
- [ ] `syncweb snapshots` - list available snapshots
- [ ] `FetchStrategy::Filter` with `min_peers`/`max_peers` - fetch by seeder count
- [ ] `FetchStrategy::Filter` with `min_count`/`max_count` - fetch by file count
- [ ] `syncweb download --max-peers N` - improve folder network health
- [ ] `syncweb health` - show seeding status per blob

### Phase 7: Polish + Integrations
**Goal**: Full CLI parity + UX + advanced features

- [ ] All commands implemented
- [ ] Rich output (tables, progress bars)
- [ ] Config file support (TOML)
- [ ] Shell completions
- [ ] Integration tests
- [ ] Documentation
- [ ] `syncweb watch` — file watcher for real-time sync (lowest priority)
- [ ] `syncweb stats` — bandwidth accounting per folder/peer
- [ ] `syncweb verify` — integrity verification (re-check all local blobs)
- [ ] Sync schedules (global + per-folder overrides)
- [ ] Platform settings files (suggested configs for laptop/server/phone)

---

## Scoped Policy Modes (Integrates Public Read-Only Folders)

### Concept
Rather than using broad abstract profiles (like "Community" or "PublicArchive"), policy is defined through explicit, grounded configuration levers (e.g., `visibility`, `searchable`, `pinning`). A single installation may contain private credentials, a team folder, a public dataset, and one publicly shared file.
This design configures deployment policy at network, folder, and file granularity. Inherited exposure is explicit and safe.

Policies are resolved in this order:
`application defaults -> network policy -> folder policy -> file policy`

An explicit value at a more-specific scope overrides an inherited value. Security-sensitive settings are monotonic: a child may restrict publication, indexing, replication, or access, but cannot silently broaden a parent policy.

### Explicit Policy Levers (Iroh-Native Options)
- **`access`**: 
  - `"capability"`: Strict access control. Requires an iroh-docs `DocTicket` or `NamespaceSecret` to discover and fetch.
  - `"public_ticket"`: Generates an iroh-blobs `BlobTicket`. Anyone with this ticket can fetch the blob without authentication.
- **`encryption`**: 
  - `"plaintext"`: Standard BLAKE3 hashing. Data is stored locally in plaintext and served over encrypted QUIC tunnels.
  - `"encrypted"`: Local payloads are encrypted before being hashed into the blob store (e.g. for untrusted mirrors).
- **`searchable`**: `true` (announces signed metadata to the DHT/gossip topic for discovery by peers/indexers) or `false`.
- **`pinning`**: `true` (prevents garbage collection of publicly shared blobs) or `false`.
- **`replication`**: `"disabled"` (do not replicate further), `"enabled"` (standard).

Iroh-blobs natively supports public tickets for unauthenticated reads. This is exposed by setting `access = "public_ticket"`.

### Content Pinning (GC Prevention)
iroh-blobs has garbage collection that removes unreferenced blobs. For public folders, we must **pin** blobs to prevent GC from deleting them (`pinning = true`):

```rust
impl SyncwebFolder {
    /// Pin all blobs in this folder (prevent GC)
    async fn pin_for_sharing(&self) -> Result<()> {
        let blobs = self.blob_store.blobs().await?;
        for blob in blobs {
            // Tag with a permanent tag to prevent GC
            self.blob_store.tag(
                format!("public/{}", self.namespace_id),
                blob.hash,
                blob.format,
            ).await?;
        }
        Ok(())
    }

    /// Unpin when stopping sharing
    async fn unpin(&self) -> Result<()> {
        self.blob_store.untag(format!("public/{}", self.namespace_id)).await?;
        Ok(())
    }
}
```

### Configuration Example

```toml
[policy]
access = "capability"
encryption = "plaintext"
searchable = false
pinning = false

[networks.research.policy]
searchable = true
catalogs = ["research-index"]

[folders.research-data.policy]
access = "public_ticket"
pinning = true
public_alias = "climate-hourly"
pin_duration = "365d"

[folders.research-data.files."raw/credentials.json".policy]
access = "capability"
encryption = "encrypted"
searchable = false
replication = "disabled"
```

### Public Folder Implementation

```rust
// Creating a public folder
async fn publish_folder(&self, folder_id: &NamespaceId) -> Result<BlobTicket> {
    // 1. Ensure folder is SendOnly or SendReceive (has namespace key)
    let folder = self.folders.get(folder_id)?;
    ensure!(folder.sync_mode.can_publish());

    // 2. Get the root hash of all blobs in this folder
    let root_hash = self.get_folder_root_hash(folder).await?;

    // 3. Create a public blob ticket
    let addr = self.node.endpoint().addr();
    let ticket = BlobTicket::new(addr, root_hash, BlobFormat::HashSeq);

    // 4. Announce on public gossip topic
    let topic = TopicId::from_bytes(blake3::hash(b"iroh-syncthing/public-folders"));
    self.node.gossip().publish(topic, PublicFolderAnnouncement {
        namespace_id: *folder_id,
        label: folder.local_path.file_name().unwrap().to_string_lossy().to_string(),
        ticket: ticket.clone(),
        version: folder.version.clone(),
        created_at: Timestamp::now(),
    }).await?;

    Ok(ticket)
}

// Subscribing to public folder (no auth needed)
async fn subscribe_public(&self, ticket: BlobTicket) -> Result<NamespaceId> {
    // 1. Create local blob store
    let blob_store = BlobStore::persistent(self.config.data_dir.join("blobs"))?;

    // 2. Start fetching blobs lazily from ticket
    let hash = ticket.hash();
    let format = ticket.format();

    // 3. Create doc entry for this folder
    let namespace_id = self.create_public_folder_entry(hash, format).await?;

    // 4. Subscribe to doc updates (gossip)
    self.node.gossip().subscribe(topic).await?;

    Ok(namespace_id)
}
```

### CLI Commands

```bash
# Show policy for a file, folder, or network
syncweb policy show [file-or-folder-or-network]

# Generate a public ticket for a folder (pins content)
syncweb policy set audio/ --access public_ticket --pinning true
# Output: iroh-blob://<ticket>  (shareable URL)

# Explain why a file has its effective policy settings
syncweb policy explain audio/raw/participants.csv

# Subscribe to public folder (no auth, read-only)
syncweb subscribe iroh-blob://<ticket>
# Creates local read-only folder, lazy-fetches on access

# List known public folders (from gossip)
syncweb public list

# Get version info for public folder
syncweb public info <ticket>

# Revert access to capability-only
syncweb policy set audio/ --access capability --pinning false
```

Noninteractive promotion to public should require an explicit flag such as `--confirm-public summary.csv`. Configuration errors that would broaden access must fail closed and name the field and source scopes.

### Code Implementation Patterns
```rust
struct Resolved<T> {
    value: T,
    source: PolicyScope,
    explicit: bool,
}

struct EffectivePolicy {
    access: Resolved<AccessMode>,
    encryption: Resolved<EncryptionMode>,
    indexing: Resolved<Indexing>,
    replication: Resolved<Replication>,
    gateway: Resolved<GatewayAccess>,
}
```

Implement `resolve(defaults, network, folder, file)` as a pure function over typed `PolicyPatch` values. Table-driven unit tests should enumerate every parent/child combination for security-sensitive fields. Use restrictive lattices where the domain supports one, for example access `"public_ticket"` > `"capability"` with child inheritance computed by `min` unless an audited promotion is explicitly supplied. Write promotion events before publishing side effects, then bind the resulting audit ID to the catalog or gateway operation.

### Use Cases
- **Public datasets** - Share large datasets via single URL (`access="public_ticket"`)
- **Software distribution** - Verified binary distribution with range requests
- **Data packages** - Versioned datasets with update tracking
- **Read-only mirrors** - One-way sync for backups/archives

---

## Data Package Management (non-apt alternative)

A complete data package lifecycle using iroh primitives instead of APT/Debian packaging.
Inspired by [dapt](https://github.com/user/dapt) but replaces `.deb` + APT entirely with
iroh-docs manifests, iroh-blobs content addressing, gossip-based discovery, and atomic upgrades.

### Why not APT/Debian packaging?

| dapt (APT-based) | iroh-syncthing (iroh-based) |
|-------------------|----------------------------|
| `.deb` packages | iroh-blobs content-addressed blobs |
| APT `Packages` indices | iroh-docs entries |
| GPG signing | Ed25519 identity (built into iroh protocol) |
| APT version comparison | BLAKE3 manifest hash + sequence number |
| HTTP repository mirrors | Gossip announcements + P2P blob transfer |
| rsync `--compare-dest` delta sync | Bao tree range requests (more granular) |
| `postinst`/`postrm` scripts | Native lazy fetch + atomic symlink swap |
| `dpkg-deb --build` | No build step — files are the package |
| Platform: Debian/Ubuntu only | Platform: any OS with Rust |
| Central repository server | P2P — any peer can serve |

### 1. Package Manifest

The package manifest replaces dapt's `product.toml` + `.dapt-release.txt` + APT `Packages` index.
Stored as an iroh-docs entry at `/.iroh-package/manifest.json` within the folder namespace.

```rust
/// Data package manifest — stored as iroh-docs entry
#[derive(Serialize, Deserialize, Clone, Debug)]
struct PackageManifest {
    /// Package name (e.g., "climate-hourly")
    name: String,
    /// Semantic version (e.g., "1.2.0")
    version: String,
    /// Monotonic sequence number for ordering
    seq: u64,
    /// Package maintainer
    maintainer: String,
    /// Human-readable description
    description: String,
    /// Searchable tags
    tags: Vec<String>,
    /// Package dependencies
    dependencies: Vec<PackageDependency>,
    /// File listing with BLAKE3 hashes (replaces APT checksums)
    files: Vec<PackageFileEntry>,
    /// BLAKE3 hash of this manifest (content-addressed version ID)
    manifest_hash: Hash,
    /// When this version was created
    created_at: Timestamp,
    /// Changelog for this version
    changelog: String,
    /// Parent version hash (for lineage tracking)
    parent_hash: Option<Hash>,
}

/// A file within a data package
#[derive(Serialize, Deserialize, Clone, Debug)]
struct PackageFileEntry {
    /// Relative path within the package
    path: PathBuf,
    /// BLAKE3 hash of the file content
    hash: Hash,
    /// File size in bytes
    size: u64,
    /// MIME type (optional)
    mime_type: Option<String>,
}

/// Package dependency
#[derive(Serialize, Deserialize, Clone, Debug)]
struct PackageDependency {
    /// Package name
    name: String,
    /// Version requirement (semver range)
    version_req: String,
}
```

**Manifest storage**: The manifest is stored as a doc entry at path `/.iroh-package/manifest.json`.
The file contents are a JSON-serialized `PackageManifest`. The manifest's own BLAKE3 hash
(`manifest_hash`) serves as the version identifier — content-addressed, tamper-proof, verifiable.

**Lineage tracking**: The `parent_hash` field creates a linked list of versions. Each version
points to its predecessor, forming a lineage chain. This replaces dapt's `.dapt-release.txt`
`released_at` field and APT's version comparison semantics.

### 2. Publishing Workflow

Full data package lifecycle, replacing dapt's `init-repo` → `new-product` → `release` → `refresh-repo`:

```bash
# 1. Initialize a folder as a data package
syncweb package init ./my-dataset \
    --name climate-hourly \
    --maintainer "me@example.com" \
    --description "Hourly climate observations"
# Creates .iroh-package/manifest.json with initial version 0.1.0

# 2. Add files to the package (scans directory, hashes, updates manifest)
syncweb package add ./my-dataset --include "*.csv" --include "*.json"
# OR add specific files:
syncweb package add ./my-dataset data/observations.csv data/metadata.json

# 3. Bump version with changelog
syncweb package bump ./my-dataset minor -m "Added Q2 2026 data"
# Creates version 0.2.0, records parent_hash pointing to 0.1.0

# 4. Publish (pins blobs + creates ticket + announces on gossip)
syncweb package publish ./my-dataset
# Output: syncweb://package/<node-ticket>/<namespace-id>?v=0.2.0
```

**Implementation**:

```rust
impl SyncwebFolder {
    /// Initialize a folder as a data package
    async fn package_init(
        &mut self,
        name: &str,
        maintainer: &str,
        description: &str,
    ) -> Result<PackageManifest> {
        let manifest = PackageManifest {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            seq: 1,
            maintainer: maintainer.to_string(),
            description: description.to_string(),
            tags: vec![],
            dependencies: vec![],
            files: vec![],
            manifest_hash: Hash::EMPTY, // computed after first add
            created_at: Timestamp::now(),
            changelog: "Initial version".to_string(),
            parent_hash: None,
        };

        // Store manifest as doc entry
        self.store_manifest(&manifest).await?;
        Ok(manifest)
    }

    /// Add files to the package manifest
    async fn package_add(&mut self, paths: &[PathBuf]) -> Result<usize> {
        let mut manifest = self.load_manifest().await?;
        let mut added = 0;

        for path in paths {
            // Hash the file with BLAKE3
            let data = tokio::fs::read(path).await?;
            let hash = blake3::hash(&data);

            // Add to blob store
            let tag = self.blob_store.add_bytes(data).await?;

            // Add to manifest
            manifest.files.push(PackageFileEntry {
                path: path.strip_prefix(&self.local_path)?.to_path_buf(),
                hash: hash.into(),
                size: tag.size,
                mime_type: None,
            });
            added += 1;
        }

        // Recompute manifest hash
        manifest.manifest_hash = self.hash_manifest(&manifest)?;
        self.store_manifest(&manifest).await?;
        Ok(added)
    }

    /// Bump package version
    async fn package_bump(
        &mut self,
        bump_type: BumpType,
        changelog: &str,
    ) -> Result<PackageManifest> {
        let mut manifest = self.load_manifest().await?;
        let old_hash = manifest.manifest_hash;

        // Bump version
        manifest.version = match bump_type {
            BumpType::Major => bump_major(&manifest.version),
            BumpType::Minor => bump_minor(&manifest.version),
            BumpType::Patch => bump_patch(&manifest.version),
        };
        manifest.seq += 1;
        manifest.parent_hash = Some(old_hash);
        manifest.changelog = changelog.to_string();
        manifest.created_at = Timestamp::now();

        manifest.manifest_hash = self.hash_manifest(&manifest)?;
        self.store_manifest(&manifest).await?;
        Ok(manifest)
    }

    /// Publish package (pin blobs + announce on gossip)
    async fn package_publish(&self) -> Result<PackageTicket> {
        // Pin all package blobs (prevent GC)
        self.pin_for_sharing().await?;

        // Create ticket
        let root_hash = self.get_folder_root_hash().await?;
        let ticket = PackageTicket {
            node_addr: self.node.endpoint().addr(),
            namespace_id: self.namespace_id,
            root_hash,
            version: self.load_manifest().await?.version,
        };

        // Announce on package gossip topic
        let topic = TopicId::from_bytes(*b"iroh-syncthing/packages");
        let manifest = self.load_manifest().await?;
        self.node.gossip().publish(topic, PackageAnnouncement {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            tags: manifest.tags.clone(),
            ticket: ticket.clone(),
            manifest_hash: manifest.manifest_hash,
            announced_at: Timestamp::now(),
        }).await?;

        Ok(ticket)
    }
}

enum BumpType { Major, Minor, Patch }
```

### 3. Package Discovery Catalog

Gossip-based package registry replaces dapt's APT `Packages` index and `Release` file.
Every publisher announces on `iroh-syncthing/packages`; consumers subscribe to discover available packages.

```rust
/// Announcement broadcast on the packages gossip topic
#[derive(Serialize, Deserialize, Clone, Debug)]
struct PackageAnnouncement {
    /// Package name
    name: String,
    /// Latest version
    version: String,
    /// Description
    description: String,
    /// Searchable tags
    tags: Vec<String>,
    /// Ticket to fetch this package
    ticket: PackageTicket,
    /// Manifest hash (for integrity)
    manifest_hash: Hash,
    /// When announced
    announced_at: Timestamp,
}

/// A package ticket (like dapt's APT repository URL, but P2P)
#[derive(Serialize, Deserialize, Clone, Debug)]
struct PackageTicket {
    /// Publisher's node address
    node_addr: EndpointAddr,
    /// Namespace containing the package
    namespace_id: NamespaceId,
    /// Root hash of the package content
    root_hash: Hash,
    /// Version string
    version: String,
}

impl PackageTicket {
    /// Format as shareable URL
    fn to_url(&self) -> String {
        format!(
            "syncweb://package/{}?v={}",
            self.node_addr.node_id(),
            self.version
        )
    }

    /// Parse from URL
    fn from_url(url: &str) -> Result<Self>;
}
```

**CLI:**

```bash
# Search available packages (queries gossip + local cache)
syncweb package search "climate"
# Output:
# NAME              VERSION  TAGS          DESCRIPTION
# climate-hourly    1.2.0    weather data  Hourly climate observations
# climate-daily     2.0.1    weather data  Daily climate summaries

# Get detailed info about a package
syncweb package info climate-hourly
# Output:
# Package: climate-hourly
# Version: 1.2.0 (seq 12)
# Maintainer: alice@example.com
# Description: Hourly climate observations
# Tags: weather, climate, hourly
# Files: 47 files, 2.3 GiB
# Published: 2026-07-16T10:30:00Z
# Publisher: node5abcd...
# Lineage: v1.0.0 → v1.1.0 → v1.2.0

# Browse by tag
syncweb package search --tag weather

# List all announced packages (no filter)
syncweb package search --all
```

### 4. Install/Remove State Management

Local state file tracks installed packages. Replaces dapt's dpkg status database.

```rust
/// Local state for installed packages
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
struct PackageState {
    /// Installed packages indexed by name
    installed: HashMap<String, InstalledPackage>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct InstalledPackage {
    /// Package name
    name: String,
    /// Installed version
    version: String,
    /// Package namespace ID
    namespace_id: NamespaceId,
    /// Local install path
    local_path: PathBuf,
    /// When installed
    installed_at: Timestamp,
    /// Manifest hash at install time
    manifest_hash: Hash,
    /// Installed file paths with hashes (for verification)
    files: Vec<PackageFileEntry>,
}

impl PackageState {
    /// Load from disk
    fn load(path: &Path) -> Result<Self>;

    /// Save to disk
    fn save(&self, path: &Path) -> Result<()>;

    /// Get state file path
    fn state_path(data_dir: &Path) -> PathBuf {
        data_dir.join("packages/state.json")
    }
}
```

**CLI:**

```bash
# Install a package from a ticket
syncweb package install syncweb://package/<node-id>?v=1.2.0 /path/to/install
# Fetches blobs, verifies manifest, stages files, atomic symlink swap
# Output:
# Fetching 47 files (2.3 GiB)...
# Verifying manifest hash... OK
# Installing to /path/to/install/1.2.0/
# Linking current → 1.2.0
# Done — climate-hourly 1.2.0 installed

# Upgrade to latest version
syncweb package upgrade climate-hourly
# Queries publisher for latest, fetches delta, swaps symlink

# Remove a package
syncweb package remove climate-hourly
# Removes files + state entry

# List installed packages
syncweb package list
# Output:
# NAME              VERSION  INSTALLED      SIZE
# climate-hourly    1.2.0    2026-07-16     2.3 GiB
# earthquake-daily  3.0.0    2026-07-15     890 MiB
```

### 5. Integrity Verification

Per-package manifest with file-level BLAKE3 hashes. iroh-blobs provides blob-level integrity;
the manifest provides file-level integrity within a package. Replaces dapt's `dpkg-deb --verify`
and APT's `Expected-SHA256` checksums.

```rust
impl SyncwebFolder {
    /// Verify installed package integrity against manifest
    async fn package_verify(&self) -> Result<VerifyResult> {
        let manifest = self.load_manifest().await?;
        let mut verified = 0u64;
        let mut failed = Vec::new();

        for entry in &manifest.files {
            let local_path = self.local_path.join(&entry.path);
            match tokio::fs::read(&local_path).await {
                Ok(data) => {
                    let hash = blake3::hash(&data);
                    if Hash::from(hash) == entry.hash {
                        verified += 1;
                    } else {
                        failed.push(VerifyFailure {
                            path: entry.path.clone(),
                            expected: entry.hash,
                            actual: Hash::from(hash),
                        });
                    }
                }
                Err(_) => {
                    failed.push(VerifyFailure {
                        path: entry.path.clone(),
                        expected: entry.hash,
                        actual: Hash::EMPTY,
                    });
                }
            }
        }

        Ok(VerifyResult { verified, failed })
    }
}

struct VerifyResult {
    verified: u64,
    failed: Vec<VerifyFailure>,
}

struct VerifyFailure {
    path: PathBuf,
    expected: Hash,
    actual: Hash,
}
```

**CLI:**

```bash
# Verify installed package integrity
syncweb package verify climate-hourly
# Output: OK — 47 files verified, all hashes match

# Verify with verbose output
syncweb package verify climate-hourly --verbose
# Output:
# climate-hourly 1.2.0 — verifying 47 files...
#   data/observations.csv   OK (sha3: a1b2c3...)
#   data/metadata.json      OK (sha3: d4e5f6...)
#   ...
# OK — 47 files verified, all hashes match

# Verify all installed packages
syncweb package verify --all
```

### 6. Multi-version Coexistence

Optional side-by-side version directories. Replaces dapt's `/var/lib/dapt/store/<product>/<version>/`
layout. Content-addressed blob storage means identical files between versions share underlying
storage — no duplication.

```text
~/.local/share/iroh-syncthing/packages/
  climate-hourly/
    0.1.0/
      data/observations.csv
      data/metadata.json
    1.0.0/
      data/observations.csv      # same blob as 0.1.0 if unchanged
      data/metadata.json
      data/extra.csv
    1.2.0/
      data/observations.csv      # only changed bytes re-fetched
      data/metadata.json
      data/extra.csv
    current -> 1.2.0/            # active version symlink
```

```rust
impl PackageState {
    /// Get all installed versions of a package
    fn versions(&self, name: &str) -> Vec<&InstalledPackage>;

    /// Get the active (symlinked) version
    fn active_version(&self, name: &str) -> Option<&InstalledPackage>;

    /// Switch active version
    fn switch_version(&mut self, name: &str, version: &str) -> Result<()>;
}
```

**CLI:**

```bash
# List installed versions
syncweb package versions climate-hourly
# Output:
# VERSION  INSTALLED      SIZE     STATUS
# 0.1.0    2026-07-01     1.8 GiB
# 1.0.0    2026-07-10     2.1 GiB
# 1.2.0    2026-07-16     2.3 GiB  (current)

# Switch active version
syncweb package switch climate-hourly 1.0.0
# Symlink swap: current → 1.0.0/
# Instant — no data movement needed
```

### 7. Atomic Upgrades

Same principle as dapt's `mv -Tf` symlink swap, but with content-addressed storage providing
additional safety guarantees.

**Upgrade sequence:**

1. **Fetch**: Download new version's blobs from publisher (or peers)
2. **Stage**: Write files to temporary directory (`/tmp/syncweb-stage-<hash>/`)
3. **Verify**: Check every file's BLAKE3 hash against the new manifest
4. **Swap**: Atomic `rename()` of staging dir to `<name>/<new-version>/`
5. **Link**: Atomic `rename()` of `current` symlink to new version dir
6. **Cleanup**: Delete old version directory (if not kept for coexistence)

If any step fails, the old version remains active. Rollback is instant — just re-swap the symlink.

```rust
impl SyncwebFolder {
    /// Upgrade package atomically
    async fn package_upgrade(&mut self, name: &str) -> Result<UpgradeResult> {
        // 1. Get latest manifest from publisher
        let remote_manifest = self.fetch_latest_manifest(name).await?;
        let local_manifest = self.load_manifest().await?;

        if remote_manifest.seq <= local_manifest.seq {
            return Ok(UpgradeResult::AlreadyUpToDate);
        }

        // 2. Stage new version
        let staging_dir = self.create_staging_dir().await?;
        let fetched = self.fetch_and_stage(&remote_manifest, &staging_dir).await?;

        // 3. Verify staged files
        let verify_result = self.verify_staged(&remote_manifest, &staging_dir).await?;
        if !verify_result.failed.is_empty() {
            tokio::fs::remove_dir_all(&staging_dir).await?;
            return Err(Error::VerifyFailed(verify_result));
        }

        // 4. Atomic swap: staging → version dir
        let version_dir = self.local_path.join("packages")
            .join(&remote_manifest.name)
            .join(&remote_manifest.version);
        tokio::fs::rename(&staging_dir, &version_dir).await?;

        // 5. Atomic symlink swap: current → new version
        let current_link = self.local_path.join("packages")
            .join(&remote_manifest.name)
            .join("current");
        let new_target = PathBuf::from(&remote_manifest.version);
        // Remove old symlink, create new one atomically
        tokio::fs::remove_file(&current_link).await.ok();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&new_target, &current_link)?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&new_target, &current_link)?;

        // 6. Update state
        self.state.upgrade(name, &remote_manifest)?;

        // 7. Cleanup old version (optional, keep if --keep-versions)
        self.maybe_cleanup_old_versions(name).await?;

        Ok(UpgradeResult::Upgraded {
            from: local_manifest.version,
            to: remote_manifest.version,
            files_changed: fetched,
        })
    }
}

enum UpgradeResult {
    AlreadyUpToDate,
    Upgraded { from: String, to: String, files_changed: usize },
}
```

### 8. Delta Sync for Packages

iroh-blobs Bao trees enable efficient range-request delta sync for large files within a package.
When upgrading, only changed files (or changed byte ranges within files) are transferred.

```text
Example: Large CSV file (10 GB) with 500 MB of new rows appended

dapt (rsync):      Transfers 500 MB delta via --compare-dest
iroh-syncthing:    Transfers 500 MB delta via Bao tree range requests
                   (more granular: works at sub-file level, not just whole-file)

Example: 1000-file dataset with 10 files changed

dapt (rsync):      Transfers 10 changed files + metadata
iroh-syncthing:    Transfers 10 changed files + only changed byte ranges
                   within partially-modified files
```

**Implementation**: Transparent — iroh-blobs handles delta sync automatically. When a blob is
re-added with the same path but different content, iroh-blobs stores only the new content.
The Bao tree structure enables range requests, so even within a single large file, only the
changed byte ranges need to be transferred.

```bash
# Upgrade with delta sync (default behavior)
syncweb package upgrade climate-hourly
# Output:
# Fetching delta for data/observations.csv (10.0 GiB, 500 MiB changed)...
# Fetching 3 new files...
# Total transfer: 523 MiB (instead of 10.5 GiB full)
```

### Package Ticket Format

```text
syncweb://package/<node-id>/<namespace-id>?v=<version>
syncweb://package/<node-id>/<namespace-id>              # latest version
syncweb://package/<node-id>/<namespace-id>?hash=<hash>  # specific manifest hash
```

**CLI:**

```bash
# Install from ticket
syncweb package install syncweb://package/abc123/def456?v=1.2.0 ./data

# Share package ticket
syncweb package publish ./my-dataset
# Output: syncweb://package/abc123/def456?v=0.2.0
```

### Use Cases
- **Research datasets** - Versioned, reproducible data packages with full lineage
- **ML training data** - Versioned datasets with delta sync for large files
- **Software releases** - Binary packages with integrity verification
- **Configuration packages** - Shared configs with version rollback
- **Media libraries** - Large file collections with incremental updates

---

## CLI Command Mapping

| syncweb-py | iroh-syncthing | Notes |
|------------|----------------|-------|
| `create` | `create` | Create folder + doc + blob store |
| `join` | `join` | Import doc via ticket/capability |
| `accept` | `accept` | Grant capability to peer |
| `drop` | `drop` | Revoke capability, remove peer |
| `folders` | `folders` | List local docs + status |
| `devices` | `devices` | List known peers + connection status |
| `ls` | `ls` | List doc entries (lazy) |
| `find` | `find` | Search doc entries (with filters) |
| `download` | `download` | Trigger lazy fetch for paths |
| `sort` | `sort` | Sort results (uses peer tracker) |
| `stat` | `stat` | File metadata from doc + blob store |
| `automatic` | `automatic` | Auto-accept/join with filter engine |
| `shutdown` | `shutdown` | Gracefully stop the node |
| `init` | `init` | Create folder + output shareable sync:// URL |
| `config` | `config` | Show/modify local configuration |
| `start` | `start` | Start the node (or log that it started) |
| `version` | `version` | Show versions |
| `repl` | `repl` | Interactive REPL |
| (implicit) | `import` | Import local files to blob store + doc entries |
| **NEW** | `policy` | Manage deployment policy levers (access, encryption, searchable, pinning) at various scopes (`show`, `set`, `explain`) |
| **NEW** | `subscribe` | Join public folder via ticket |
| **NEW** | `public list` | List announced public folders |
| **NEW** | `package init` | Initialize folder as data package |
| **NEW** | `package add` | Scan + hash files, update manifest |
| **NEW** | `package bump` | Create new version with changelog |
| **NEW** | `package publish` | Blob ticket + gossip announcement |
| **NEW** | `package search` | Discover packages via gossip |
| **NEW** | `package info` | Detailed package metadata |
| **NEW** | `package install` | Fetch + verify + install package |
| **NEW** | `package upgrade` | Update to latest version |
| **NEW** | `package remove` | Remove installed package |
| **NEW** | `package verify` | Integrity check against manifest |
| **NEW** | `package list` | List locally installed packages |
| **NEW** | `package versions` | List installed versions |
| **NEW** | `package switch` | Change active version |
| **NEW** | `health` | Show seeding status per blob (well/under/unseeded) |
| **NEW** | `backup` | Create content-addressed snapshot of folder |
| **NEW** | `restore` | Restore folder from snapshot |
| **NEW** | `snapshots` | List available snapshots |
| **NEW** | `network create` | Create named network group |
| **NEW** | `network ls` | List networks or network details |
| **NEW** | `network join` | Join a network via ticket |
| **NEW** | `network leave` | Leave a network |
| **NEW** | `network invite` | Invite device to a network |
| **NEW** | `network kick` | Remove device from a network |
| **NEW** | `stats` | Bandwidth accounting per folder/peer |
| **NEW** | `verify` | Integrity verification (re-check local blobs) |
| **NEW** | `schedule` | Show/modify sync schedule |
| **NEW** | `conflicts` | List/resolve file conflicts |
| **NEW** | `watch` | File watcher for real-time sync (lowest priority) |
| **NEW** | `network test-relay` | Test Syncthing relay connectivity |

### New CLI Options (from iroh-willow)

```bash
# Global flags (all commands)
syncweb --home /path/to/data ls    # Custom data directory
syncweb --verbose find .            # Verbose output
syncweb --json folders              # JSON output (for scripting)
syncweb --no-color devices          # Disable color output

# Download with limits (max entries)
syncweb download --limit 10 /path/to/files

# Download with size limit
syncweb download --size 1GB /path/to/files

# Subscribe with filtering (only new files)
syncweb subscribe --ingest-only /path/to/folder

# Subscribe ignoring our own writes
syncweb subscribe --ignore-self /path/to/folder

# Publish with limits
syncweb publish --limit 100 --size 10GB /path/to/folder

# Show deleted files
syncweb deleted /path/to/folder

# Restore deleted file
syncweb undelete <entry-hash>

# Parallel scan (auto-detect CPU count)
syncweb ls --parallel

# Scan with specific thread count
syncweb ls --parallel --threads 8

# Parallel import
syncweb import --parallel /path/to/files

# Parallel export
syncweb export --parallel /path/to/output

# Health check (show seeding status)
syncweb health audio/

# Download poorly-seeded blobs to improve network health
syncweb download --max-peers 2 audio/

# Bandwidth limiting
syncweb folders --limit-upload 1MB/s --limit-download 5MB/s
syncweb devices --peer-limit NODE-ID --upload 500KB/s --download 2MB/s

# Backup/snapshot commands
syncweb backup documents/ --description "before edit"
syncweb snapshots documents/
syncweb restore documents/ a1b2c3d4
syncweb snapshots diff documents/ a1b2c3d4 e5f6g7h8

# Network commands
syncweb network create work
syncweb network ls
syncweb network ls work
syncweb network invite work <device-id>

# Find with filters
syncweb find --glob '**/*.mp3' music/
syncweb find --type f --ext mp3 --min-size 10MB music/
syncweb find 'report.*' --modified-within 7d

# Stat with format
syncweb stat docs/report.md
syncweb stat --terse docs/report.md
syncweb stat --format '%n %s %y' docs/report.md

# Sort with criteria
syncweb sort --sort niche music/
syncweb sort --sort peers --sort time music/
syncweb sort --limit-size 10GB --min-seeders 2 music/

# Init/config
syncweb init ./documents
syncweb init --network work ./documents
syncweb config set default_path ~/Syncweb

# BEP-compatible device ID display
syncweb devices --bep

# Conflict resolution
syncweb conflicts
syncweb conflicts --resolve
syncweb conflicts --auto-resolve
syncweb conflicts resolve <id> --keep-local

# Offline queue
syncweb pending
```

---

## Configuration

```toml
# ~/.config/iroh-syncthing/config.toml
[node]
data_dir = "~/.local/share/iroh-syncthing"
node_name = "my-device"

[relay]
# Iroh relay (default: iroh's public relays)
urls = ["https://relay.iroh.computer"]

[discovery]
# Enable/disable discovery mechanisms
local_mdns = true
iroh_gossip = true
mainline_dht = true

[discovery.topic_tracker]
# distributed-topic-tracker settings
enabled = true
# Rate limit for DHT writes (records per minute per topic)
dht_write_limit = 5
# Bubble detection threshold (merge if fewer than N neighbors)
bubble_threshold = 4
# Secret rotation strategy: "sha512" (default)
secret_rotation = "sha512"

[folders]
default_path = "~/IrohSyncweb"
default_sync_mode = "SendReceive"
default_max_entries = 0  # 0 = unlimited
default_max_size = 0     # 0 = unlimited

[bandwidth]
# Global bandwidth limits (bytes/sec, 0 = unlimited)
max_upload = 0
max_download = 0
# Per-peer limits (applied to all peers unless overridden)
per_peer_upload = 0
per_peer_download = 0

[public]
# Public folder settings
announce_enabled = true
gossip_topic = "iroh-syncthing/public-folders"
# Content pinning (prevent GC for shared blobs)
pin_shared_content = true

[bep]
# Syncthing relay fallback (for CGNAT traversal)
enabled = true
# Syncthing relay URLs (tcp:// for relay protocol v1)
relay_urls = ["tcp://relay.syncthing.net:22270"]
# Timeout for relay connection attempt (seconds)
relay_timeout = 10
# Auto-detect CGNAT and use relay when iroh direct/relay fails
auto_fallback = true

[schedule]
# Global sync schedule
active_hours = ""  # empty = always active

# Bandwidth limits by time of day
[[schedule.bandwidth]]
hours = "08:00-18:00"
max_upload = "1MB/s"
max_download = "5MB/s"

[[schedule.bandwidth]]
hours = "18:00-08:00"
max_upload = "0"  # unlimited
max_download = "0"

# Per-folder schedule overrides
[schedule.folders.media]
active_hours = "01:00-05:00"
max_download = "50MB/s"

[networks]
# Networks are auto-discovered; this section can pin specific network config
# Networks enable multi-folder + multi-device grouping under a gossip topic
default_network = ""

[networks.my-work]
label = "Work Documents"
# Topic is derived from network name; manual override for existing topics
# topic = "iroh-syncthing/net/work-a1b2c3"
members = []       # Auto-populated; manual pinning for invite-only networks
folders = []       # Folders in this network (auto-populated)

[filter]
# Automatic daemon filter settings
config_path = "~/.config/iroh-syncthing/filters.toml"

[advanced]
# Blob store settings
blob_cache_size_gb = 10
# Connection limits
max_connections = 100
# Peer tracker settings
peer_cache_expiry_s = 300  # 5 minutes

[cache]
# Cache eviction strategy ((standard CS pattern: age-based cache eviction))
# "lru" - Least Recently Used (resets age on access)
# "fifo" - First In First Out (never resets age)
eviction_strategy = "lru"
# Maximum cache size before eviction (entries)
max_cache_size = 10000
# Use memory-efficient bitmask cache for large peer networks
use_efficient_cache = true
# Threshold for switching to efficient cache (peer count)
efficient_cache_threshold = 100

[parallel]
# Parallel file operations ((standard CS pattern: parallel directory traversal))
# Number of threads (0 = auto-detect CPU count, 1 = single-threaded)
threads = 0
# Parallel is default for ls, import, export
# Use --threads=1 to disable per-command, or set threads = 1 here globally

```

---

## Key Technical Decisions

### 1. One Namespace per Folder vs. One for All
**Decision**: One Namespace per folder (like Syncthing folders)
- Matches syncweb mental model
- Independent sync, permissions, ignore patterns
- Easier to share subset

### 2. Author Key Management
**Decision**: Per-folder author keys (derived from master)
- Master identity key (NodeId) for device identity
- Per-folder author keys derived: `HKDF(master, "folder/" + namespace)`
- Allows revoking per-folder without rotating device ID

### 3. Ignore Patterns
**Decision**: No ignore patterns needed (lazy fetch is inherent)
- In syncweb-py, .stignore was used for selective sync
- In iroh-syncthing, blobs are fetched on demand
- `ls`/`find` show metadata without fetching blobs
- `download` triggers specific blob fetches
- Simpler and more efficient than ignore patterns

### 4. Selective Sync (Lazy Fetch)
**Decision**: iroh-blobs on-demand + doc subscription
- Doc entries have blob hashes + sizes
- `download` command triggers blob fetch
- `ls`/`find` show metadata without fetching blobs
- No .stignore file needed

### 4b. Delta Sync for Large Files
**Decision**: iroh-blobs Bao trees enable efficient range requests
- When a large file changes partially, only the changed ranges need re-syncing
- Bao trees provide byte-range verification (no need to re-hash entire file)
- iroh-blobs handles this automatically - no special code needed
- **Use case**: Databases, VMs, video editing projects where files change incrementally
- **Note**: This is transparent - the user just sees faster sync for large files

### 5. Public Folders
**Decision**: iroh-blobs public tickets + gossip announcement
- No auth required for readers
- Verified content (BLAKE3)
- Efficient range requests for large files

### 6. Conflict Resolution
**Decision**: Best-effort text diff when smaller than winner; full file otherwise
- At decode time, attempt to read both versions as text (UTF-8)
- If decodable and the diff is smaller than the LWW winner, save a `.diff` file instead of the full file
- Otherwise save the full file (both versions kept, older renamed with hash suffix)
- Winning version always stays at the original path (LWW by timestamp)
- Diff filename: `<stem>.diff` if there are enough filename characters, otherwise fall back to full file
- User sees conflicts in `syncweb conflicts` command

### 7. Peer Availability
**Decision**: DHT-based discovery via distributed-topic-tracker + cache from natural iroh flow
- distributed-topic-tracker handles decentralized gossip topic bootstrap via BitTorrent DHT
- No central bootstrap server required
- iroh relays remain available for NAT traversal fallback
- Peer availability cached locally from natural blob request/response flow
- Used by `sort` for niche/frecency/peers

### 8. Syncthing Relay Piggyback
**Decision**: Piggyback on Syncthing's relay network for CGNAT traversal, not protocol translation
- When iroh's QUIC hole punching fails (both peers behind strict CGNATs), tunnel through Syncthing relays
- Syncthing relays are protocol-agnostic — they relay raw bytes between devices
- Both endpoints remain iroh-syncthing nodes — no BEP protocol translation needed
- Automatic fallback: iroh direct → iroh relay → Syncthing relay
- Leverages Syncthing's mature, well-tested relay infrastructure
- Ed25519 key compatibility enables DeviceId ↔ NodeId conversion (zero-cost)

### 9. Networks
**Decision**: Named multi-folder + multi-device groups under gossip topics
- Networks provide an explicit grouping abstraction (replaces Syncthing's implicit cluster)
- Each network has a gossip topic `iroh-syncthing/net/<id>` for discovery
- Single-device users can ignore networks entirely
- Optional shared secret for invite-only networks
- Folder membership in a network enables auto-join

### 10. Automatic Daemon
**Decision**: Internal filter engine (no shell scripts)
- Rust-native TOML config
- More portable and testable
- Supports version constraints
- Better performance than subprocess calls

### 11. Engine Pattern (from iroh-willow)
**Decision**: Dedicated storage thread with message-passing
- Storage I/O runs in dedicated thread (not async runtime)
- Keeps network operations responsive
- Simpler error handling (panic in thread, not runtime)
- Inspired by iroh-willow's Engine::Actor pattern

### 12. SessionMode (from iroh-willow)
**Decision**: ReconcileOnce vs Continuous sessions
- `ReconcileOnce` for one-time operations (download, sync)
- `Continuous` for background sync
- Clear separation of concerns
- Matches user expectations (explicit vs implicit sync)

### 13. IntentHandle as Stream (from iroh-willow)
**Decision**: Every sync operation returns an IntentHandle
- Implements `Stream<Item = SyncEvent>` for progress
- Implements `Sink<SyncCommand>` for control (pause/resume/cancel)
- Easy to use with async patterns (`.await`, `while let Some(event) = handle.next().await`)
- Composable with tokio::select!, futures::StreamExt

### 14. Deleted Files Tracking (from iroh-willow)
**Decision**: Track deleted-but-previously-seen files
- Record which session deleted which entry
- Enable "undelete" feature
- Audit trail for compliance
- PruneEvent carries session ID (who deleted it)

### 15. DHT-Based Peer Discovery (distributed-topic-tracker)
**Decision**: Use distributed-topic-tracker for decentralized gossip topic bootstrap
- Replaces reliance on iroh's relay infrastructure for peer discovery (which is weak)
- Uses BitTorrent mainline DHT as the decentralized lookup layer
- No central bootstrap server required
- Per-folder gossip topics are discoverable via DHT records
- Time-rotated keys provide forward secrecy for announcements
- Bubble detection keeps gossip meshes healthy after partitions
- Rate limiting (5 DHT writes/min default) prevents spam
- Iroh relays remain available as a fallback for NAT traversal, but not for discovery

### 16. Age-Based Cache Eviction ((standard CS pattern: age-based cache eviction))
**Decision**: LRU or FIFO cache eviction for PeerTracker
- Inspired by standard CS pattern: age-based cache eviction
- Age-based eviction prevents unbounded memory growth
- LRU mode resets age on access (better for frequently accessed blobs)
- FIFO mode never resets age (simpler, predictable eviction)
- Configurable max_cache_size prevents OOM on large peer networks
- `tick_and_maybe_evict()` called periodically to maintain cache size

### 17. Memory-Efficient Peer Cache ((standard CS pattern: memory-efficient bitmask presence))
**Decision**: Bitmask-based presence tracking for large peer networks
- Inspired by standard CS pattern: memory-efficient bitmask presence
- BitVec for O(1) presence checks (1 bit per peer)
- Compressed indices for active peers per blob
- `memory_usage()` for monitoring cache memory footprint
- Scales to 1000+ peers without excessive memory usage
- Fallback to HashMap-based cache for small peer networks

### 18. Parallel File Operations ((standard CS pattern: parallel directory traversal))
**Decision**: Rayon-based parallel scanning, import, and export (default on)
- Inspired by standard CS pattern: parallel directory traversal
- `ParallelScanner` uses work-stealing for directory traversal
- `ParallelImporter` and `ParallelExporter` for concurrent operations
- **Parallel is default** - no flag needed for typical usage
- `--threads=1` to disable parallelism (single-threaded mode)
- `--threads=N` to control parallelism level (default: auto-detect CPU count)
- 4-6x speedup on multi-core systems for large directories
- Streaming output by default (results appear as found)
- `--sort` flag collects all results before output (required for sorted display)

### 19. Partial Folder Fetch (Improve Network Robustness)
**Decision**: Filter-based fetch with peer count and file count constraints
- When a folder has uneven seeder distribution, use `--max-peers` to fetch the least-seeded blobs first
- Use `--min-count` / `--max-count` to limit how many blobs are fetched
- `FetchFilter` unifies paths, sizes, peer counts, and file counts in one struct
- `syncweb download --max-peers 2 audio/` fetches blobs with ≤2 seeders
- `syncweb health` shows seeding status per blob (well/under/unseeded)
- Each node independently decides what to seed (no central coordination)
- Improves resilience: rare content becomes more available over time

### 20. Multi-Device-Per-User Support
**Decision**: Per-device identity with folder-level capabilities
- Users may have phone + laptop + desktop
- Each device has its own `NodeId` (Ed25519 keypair)
- Folder access is per-device (not per-user)
- **Revocation**: Revoke a specific device without affecting others
- **Sync modes**: Device A (SendReceive) + Device B (ReceiveOnly) + Phone (ReceiveOnly, limited)
- **Implementation**: `CapabilityMap` tracks `NodeId -> Capability` per folder
- **UX**: `syncweb devices` shows all devices, `syncweb accept` adds device to folder
- **Note**: No concept of "user" in the protocol - just devices with capabilities

### 21. Data Package Management (non-apt alternative)
**Decision**: iroh-docs manifests + iroh-blobs content addressing (replaces APT/Debian packaging)
- dapt packages datasets as `.deb` files and uses APT for version management
- This approach replaces that entirely with iroh primitives:
  - iroh-docs entries replace `.deb` control files and `Packages` indices
  - iroh-blobs BLAKE3 replaces GPG signing for integrity
  - Content addressing replaces APT version comparison
  - Blob tickets replace APT repository URLs
  - Bao tree range requests replace rsync `--compare-dest` delta sync
  - Gossip announcements replace HTTP repository mirrors
- Platform-independent (no Debian/APT dependency)
- P2P by default (no central repository server)
- Atomic upgrades via symlink swap (same as dapt, but with content-addressed storage)
- Multi-version coexistence via versioned directories + shared blob storage
- Full lifecycle: init → add → bump → publish → search → install → upgrade → remove → verify
- See "Data Package Management" section for full design

---

## Testing Strategy

### Unit Tests
- [ ] Identity management
- [ ] DeviceId bidirectional conversion (Syncthing ↔ Iroh)
- [ ] Ticket parsing/generation
- [ ] Capability serialization
- [ ] Filter engine evaluation
- [ ] Version tracking
- [ ] PeerTracker age-based cache eviction (LRU/FIFO)
- [ ] EfficientPeerCache bitmask operations
- [ ] ParallelScanner directory traversal
- [ ] Partial fetch: filter by peer count (min_peers/max_peers)
- [ ] Health check: seeder count per blob
- [ ] Find engine: regex, glob, exact matching with constraints
- [ ] Sort engine: niche, frecency, peers, folder-aggregate
- [ ] Stat output: format, terse, custom template
- [ ] Network: create, join, leave, invite, kick

### Integration Tests
- [ ] Two nodes: create folder, join, sync files
- [ ] Three nodes: sendonly -> sendreceive -> receiveonly
- [ ] Public folder: publish -> subscribe -> read
- [ ] Selective sync: ls without download, then download
- [ ] Network partition: offline edits, reconnect, merge
- [ ] Data versioning: bump, check, update
- [ ] Data package lifecycle: init → add → bump → publish → search → install → upgrade → remove
- [ ] Multi-version coexistence: install v1, install v2, switch between them
- [ ] Atomic upgrade: verify rollback works if upgrade fails
- [ ] Package integrity: verify catches corrupted files
- [ ] Package discovery: publish → search → info across two nodes
- [ ] Parallel operations: ls --parallel, import --parallel, export --parallel
- [ ] Partial fetch: download --max-peers improves seeder counts
- [ ] Cache eviction: test LRU and FIFO under memory pressure
- [ ] Large peer network: test EfficientPeerCache with 1000+ peers
- [ ] Networks: two-node network create, invite, join, folder sync
- [ ] Networks: three-node network with mixed roles
- [ ] Find: regex, glob, exact search across folder boundaries
- [ ] Sort: niche, frecency, peers with various filter combinations
- [ ] Stat: detailed output, local/global diffs, availability display
- [ ] Init: folder creation with URL output + network membership

### Interop Tests (Phase 7: with `--bep` flag)
- [ ] Syncthing node -> iroh-syncthing folder join
- [ ] iroh-syncthing -> Syncthing folder join
- [ ] Bidirectional sync
- [ ] Relay-only connection

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Startup time | < 500ms |
| Memory (idle) | < 50MB |
| Memory (syncing 10GB) | < 200MB |
| Blob throughput (LAN) | > 500 MB/s |
| Blob throughput (WAN) | > 50 MB/s |
| Doc sync latency (LAN) | < 50ms |
| Discovery time (local) | < 1s |
| Discovery time (global/DHT) | < 10s (distributed-topic-tracker via BitTorrent DHT) |
| Peer cache lookup | < 1ms |
| Filter evaluation | < 10ms per entry |
| Scan (10k files, default) | < 500ms (6x speedup) |
| Import (1000 files, default) | < 3s (6x speedup) |
| Export (1000 files, default) | < 2.5s (6x speedup) |
| Cache eviction (10k entries) | < 10ms |
| Efficient cache memory (1000 peers) | < 1MB |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Iroh API changes | Low (1.0 released) | High | Pin versions, test against main branch |
| iroh-docs performance at scale | Medium | Medium | Benchmark early, optimize queries |
| Public folder spam/abuse | Medium | Low | Rate limit gossip, allowlist |
| Syncthing relay protocol v1 changes | Low | High | Protocol is simple (3 message types); monitor releases; fallback to iroh relay |
| Windows file locking | Medium | Medium | Test early, use iroh-blobs async API |
| File conflict resolution UX | Low | Medium | Clear naming convention; `syncweb conflicts` command; LWW for text, keep-both for binary |
| BitTorrent DHT availability | Low | Medium | DHT has ~10M+ nodes; fallback to iroh relays for connectivity |
| DHT write rate limits | Medium | Low | Tune `dht_write_limit` per-folder; accept slower re-discovery after long offline periods |
| distributed-topic-tracker version drift | Medium | Medium | Pin version; monitor upstream releases when upgrading iroh |
| Cache eviction thrashing | Low | Medium | Tune `max_cache_size` and `eviction_strategy` based on workload |
| Efficient cache overhead | Low | Low | Fallback to HashMap for small peer networks (< 100 peers) |
| Networks gossip overhead | Low | Low | Per-network topics are lightweight; merge under common topic via bubble detection |
| Relay key compatibility | Low | Low | Both use Ed25519; add validation tests for edge cases (padding, encoding variants) |
| Network invite spam | Low | Low | Shared-secret networks mitigate; rate-limit invites |
| Parallelism deadlocks | Low | High | Use rayon's work-stealing; limit thread count; add timeouts |
| Syncthing relay protocol breakage | Low | High | Protocol is simple (3 message types); monitor Syncthing releases; implement fallback |
| Unbounded blob store growth | Medium | Medium | Content pinning; GC for unpinned content; configurable max cache size |
| DHT blocked by corporate firewalls | Low | Medium | iroh-relay remains primary; DHT is supplementary; graceful degradation |
| Config file corruption on crash | Low | Low | Atomic writes (write to temp, rename); backup old config |
| Millions of entries in namespace | Low | Medium | iroh-docs lazy enumeration; pagination in `ls`/`find`; avoid loading all entries |

---

## Success Criteria

1. **Functional parity**: All syncweb-py commands work (create, join, accept, drop, ls, find, sort, stat, download, devices, folders, automatic, start, shutdown, version, repl)
2. **Performance**: Faster sync, lower resource usage than Syncthing
3. **Public folders**: `publish`/`subscribe` work end-to-end
4. **Data versioning**: Data package lifecycle works (init, add, bump, publish, search, install, upgrade, remove, verify)
5. **Networks**: `network create/join/invite/kick` work across devices
6. **Syncthing relay**: Two iroh-syncthing nodes can communicate via Syncthing relay when direct QUIC fails
7. **UX**: Single binary, no daemon, config file optional
8. **Reliability**: No data loss, verified transfers, BLAKE3 integrity on all transfers
9. **Conflict resolution**: Automatic LWW for text, keep-both for binary, older version renamed
10. **Parallel operations**: 4-6x speedup for ls, import, export (default on)
11. **Memory efficiency**: PeerTracker handles 1000+ peers without OOM
12. **Network robustness**: Filter-based partial fetch improves seeder counts for rare content
13. **Cache efficiency**: Age-based eviction prevents unbounded memory growth
14. **Find parity**: regex/glob/exact search with all syncweb-py filter options
15. **Sort parity**: niche/frecency/peers/random sorting with folder aggregates
16. **Stat parity**: detailed file info with availability, version vectors, local/global diffs
17. **Logging**: Structured tracing with configurable levels and log rotation
18. **Schedules**: Global + per-folder bandwidth scheduling works


```console
$ syncweb init --network home ~/Documents
Created folder documents
Local files: 1,284; imported: 1,284; verified: 1,284
Private by default.

$ syncweb network invite home laptop
Invitation: syncweb://network/...

$ syncweb folders
NAME       MODE         LOCAL     REMOTE  STATE
documents  SendReceive  1,284     1,284   up to date

$ syncweb stat documents/report.pdf
Content: b3:8e7a...
Local: yes (verified)
Known providers: 2 (last checked 14s ago)
Policy: private from network "home"
```

Errors should name the layer that failed. For example, `manifest verified; no
provider reachable`, `capability rejected by peer`, and `materialization blocked:
path escapes target` are more actionable than `sync failed`.

## Grounded implementation patterns and libraries

The code examples in this plan describe intended boundaries; exact Iroh APIs
must be confirmed against the pinned crate versions during Phase 1.

### Workspace and service boundaries

Prefer a library-first workspace so the CLI does not become the application
boundary:

```text
core/       typed IDs, manifests, policy, errors
store/      blobs, docs, SQLite/config persistence
net/        Iroh endpoint, gossip, discovery, provider resolution
service/    folder, queue, catalog, package use cases
cli/        clap parsing and human/JSON rendering
```

Commands call typed services and render returned values. Core code must not print
progress or parse CLI strings. This also allows integration tests to use services
directly and keeps a future daemon or GUI from duplicating behavior.

```rust
struct AppServices<C, S, N> {
    catalog: C,
    store: S,
    network: N,
}

impl<C: Catalog, S: ContentStore, N: Network> AppServices<C, S, N> {
    async fn download(&self, request: DownloadRequest)
        -> Result<DownloadHandle, AppError>;
}
```

### Persistence and crash consistency

Use SQLite for small mutable control-plane state and Iroh blobs/docs for
content-addressed and replicated state. A user-visible operation that spans both
cannot rely on one atomic transaction, so model it as a resumable workflow:

```rust
enum PublishStep {
    Drafted,
    BlobsPinned,
    ManifestStored,
    HeadUpdated,
    CatalogAnnounced,
}
```

Persist completion of each idempotent step. On restart, resume forward or perform
an explicit compensating action, such as unpinning blobs that never became
reachable from a published manifest. Never mark a transfer or publication
complete before verification and durable state updates finish.

### Security boundaries

- Parse tickets, URLs, manifests, and gossip records into untrusted types; only
  verification produces `Verified<T>`.
- Domain-separate signed bytes by protocol, record type, and schema version.
- Validate normalized logical paths before joining them to an output directory,
  and materialize through temporary files plus atomic rename.
- Bound record sizes, collection entry counts, extraction output, queue length,
  concurrent peers, and HTTP request bodies.
- Resolve policy at publication, indexing, replication, fetch, and
  materialization time. Cached decisions may explain prior actions but cannot
  authorize new ones.
- Redact capability tokens and shared secrets from URLs in logs and error
  reports.

### Validation gates by phase

| Phase | Smallest meaningful gate |
|---|---|
| Foundation | restart preserves identity; corrupted config fails explicitly |
| Folder core | two local nodes reconcile one file and reject an unauthorized writer |
| File operations | interrupted import resumes without exposing a partial file |
| Networks | invitation grants only the intended network/folder capabilities |
| Public/package | manifest signature and every materialized blob are verified |
| Backup/partial fetch | restore is byte-identical; provider loss triggers fallback |
| Polish/interop | CLI JSON remains compatible; optional BEP failure cannot corrupt Iroh state |

Performance targets should be treated as benchmark hypotheses until fixtures,
hardware, file-size distribution, concurrency, and warm/cold cache conditions are
specified. Correctness gates must not be relaxed to reach a throughput target.


---

## Appendix: Iroh 1.0.2 API Reference

### iroh-blobs (0.103.0)

```rust
use iroh::{Endpoint, endpoint::presets, protocol::Router};
use iroh_blobs::{BlobsProtocol, store::fs::Store as BlobStore, ticket::BlobTicket, ALPN as BLOBS_ALPN};

// Setup
let endpoint = Endpoint::bind(presets::N0).await?;
let blob_store = BlobStore::persistent(data_dir.join("blobs"))?;
let blobs = BlobsProtocol::new(&blob_store, None);
let router = Router::builder(endpoint.clone())
    .accept(BLOBS_ALPN, blobs)
    .spawn();

// Add data
let tag = blob_store.add_bytes(data).await?;
let hash = tag.hash;

// Create public ticket
let addr = endpoint.addr();
let ticket = BlobTicket::new(addr, hash, tag.format);

// Fetch (lazy)
let reader = blob_store.get(hash).await?;
// Range request
let reader = blob_store.get_range(hash, 0..1024).await?;
```

### iroh-docs (0.101.0)

```rust
use iroh::{Endpoint, endpoint::presets, protocol::Router};
use iroh_blobs::{BlobsProtocol, store::fs::Store as BlobStore, ALPN as BLOBS_ALPN};
use iroh_docs::{protocol::Docs, ALPN as DOCS_ALPN};
use iroh_gossip::{net::Gossip, ALPN as GOSSIP_ALPN};

// Setup (requires blobs + gossip)
let endpoint = Endpoint::bind(presets::N0).await?;
let blob_store = BlobStore::persistent(data_dir.join("blobs"))?;
let gossip = Gossip::builder().spawn(endpoint.clone());
let docs = Docs::persistent(data_dir.join("docs"))
    .spawn(endpoint.clone(), blob_store.clone(), gossip.clone())
    .await?;
let blobs = BlobsProtocol::new(&blob_store, None);
let router = Router::builder(endpoint.clone())
    .accept(BLOBS_ALPN, blobs)
    .accept(GOSSIP_ALPN, gossip)
    .accept(DOCS_ALPN, docs)
    .spawn();

// Create author + namespace
let author = Author::generate();
let namespace = NamespaceSecret::generate();
let namespace_id = namespace.public();

// Create doc (replica)
let mut replica = docs.create_replica(namespace.clone()).await?;
replica.insert_entry(Entry::new(
    &author,
    b"path/to/file",
    hash,  // blob hash
    size,
)).await?;

// Subscribe to changes
let mut events = replica.subscribe().await?;
while let Some(event) = events.next().await {
    match event {
        DocEvent::Insert(entry) => { /* new entry */ }
        DocEvent::Remove(entry) => { /* entry removed */ }
    }
}
```

### iroh-gossip (0.101.0)

```rust
use iroh_gossip::{net::Gossip, proto::TopicId};

// Setup (done as part of docs setup above)

// Subscribe to topic
let topic = TopicId::from_bytes(*b"iroh-syncthing/public-folders");
let mut events = gossip.subscribe(topic).await?;

// Publish
gossip.publish(topic, payload).await?;

// Get peers on topic
let peers = gossip.peers(topic).await?;
```

### distributed-topic-tracker (0.3.5)

```rust
use distributed_topic_tracker::Node;

// Create a topic tracker node backed by the BitTorrent DHT
// Automatically bootstraps gossip topics via DHT lookup
let tracker = Node::new(gossip.clone(), endpoint.clone()).await?;

// The tracker provides an AutoDiscoveryGossip extension trait on iroh::Gossip.
// When subscribed to a gossip topic through the tracker, it will:
// 1. Query the DHT for other nodes on the same topic
// 2. Decrypt & verify DHT records using time-rotated keys
// 3. Join discovered peers with pacing
// 4. Spawn background actors for bubble detection and merge

// Subscribe to a gossip topic with DHT-based auto-discovery
let topic = TopicId::from_bytes(*b"iroh-syncthing/folder-abc123");
let mut events = tracker.subscribe(topic).await?;

// Announce presence on a topic via DHT
tracker.announce(topic).await?;

// The tracker also handles:
// - Time-rotated signing/encryption keys (per-minute from topic hash)
// - Rate limiting (default: 5 DHT writes per minute)
// - Bubble detection (merge isolated clusters < 4 neighbors)
// - Message overlap merge (detect network partitions)
```

### iroh (Endpoint)

```rust
use iroh::{Endpoint, endpoint::presets};

// Bind endpoint with default presets (relay + discovery)
let endpoint = Endpoint::bind(presets::N0).await?;

// Get identity
let node_id = endpoint.node_id();  // EndpointId / NodeId
let addr = endpoint.addr();        // EndpointAddr with relay + direct addrs

// Connect to peer
let conn = endpoint.connect(addr, b"my-alpn").await?;

// Accept connections
let conn = endpoint.accept().await?.await?;

// Graceful shutdown
endpoint.close().await;
```

---

*Document version: 3.2*
*Amended: 2026-07-17*
*Target: iroh 1.0.2, iroh-blobs 0.103.0, iroh-docs 0.101.0, iroh-gossip 0.101.0, distributed-topic-tracker 0.3.5*
*Added: Networks concept (multi-folder + multi-device groups under gossip topics)*
*Added: find command design (regex/glob/exact search with depth/size/time filters)*
*Added: stat command design (detailed file metadata, availability, version vectors, local/global diffs)*
*Added: sort command design (niche, frecency, peers, folder-aggregate sorting)*
*Added: init/config command design (folder creation + URL output, config management)*
*Added: BEP Phase 2 minimal identity (DeviceId conversion, --bep flag annotation)*
*Added: BEP Phase 7 full protocol translation (moved from Phase 7+ deprioritized — still complex, but identity is cheap)*
*Added: Standard CS patterns (cache eviction, parallel traversal, bitmask presence, consistent hashing)*
*Added: Data Package Management (non-apt alternative to dapt) — full lifecycle with iroh-docs manifests, iroh-blobs content addressing, gossip-based discovery, atomic upgrades, multi-version coexistence*
