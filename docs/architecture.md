# Architecture

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
