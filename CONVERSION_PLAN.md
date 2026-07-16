# syncweb-py to iroh-syncthing Conversion Plan

## Executive Summary

Convert syncweb-py (Python + Syncthing) to iroh-syncthing (Rust + Iroh 1.0+). 
Key architectural shift: **Syncthing's block-exchange protocol to Iroh's BLAKE3-Bao verified blob sync (iroh-blobs) + document sync (iroh-docs) + gossip (iroh-gossip).**

### Key Advantages of Iroh 1.0+
- **Content-addressed blobs (BLAKE3 + Bao trees)** - verified streaming, range requests, deduplication
- **iroh-docs** - CRDT-based document sync (like Syncthing's index exchange but CRDT-based)
- **iroh-gossip** - Pub/sub for discovery, presence, live updates
- **iroh-blobs public sharing** - Native public read-only folders via ticket sharing
- **BLAKE3 verified streaming** - No hash tree sync needed, verified on-the-fly
- **QUIC transport** - Built-in NAT traversal, relays, connection migration
- **No separate daemon** - Library-first, embeddable

---

## Architecture Mapping

| syncweb-py / Syncthing | iroh-syncthing / Iroh 1.0+ |
|------------------------|----------------------------|
| Syncthing daemon (separate process) | iroh Node (embedded library) |
| Device ID (Ed25519) | NodeId (Ed25519, same format!) |
| Folder (config + path) | **Doc** (iroh-docs) + **Blob Store** (iroh-blobs) |
| Folder ID (random) | DocId (blake3 hash of author + namespace) |
| Block exchange (BEP) | BLAKE3-Bao verified blob sync (iroh-blobs) |
| Index exchange (BEP) | CRDT document sync (iroh-docs) |
| Discovery (local/global/relay) | iroh-gossip + Mainline DHT + iroh-relay |
| Ignore patterns (.stignore) | **Doc-based ignore patterns** (CRDT) |
| Folder types (sendreceive/sendonly/receiveonly) | **Doc permissions** (author keys, capabilities) |
| Device introduce | Doc share (capability tokens) |
| Cluster config (XML) | Local SQLite (iroh-docs) + blob store (iroh-blobs) |
| REST API | In-process Rust API (or gRPC/JSON-RPC if needed) |
| Selective sync (ignore patterns) | **Lazy blob fetching** + doc subscriptions |
| Public folders | **Public blob tickets** (iroh-blobs tickets) |
| BEP relays | **iroh-relay** (compatible) + BEP relay bridge (optional) |

---

## Core Architecture

```
+------------------------------------------------------------------------------+
|                              iroh-syncthing CLI                               |
+------------------------------------------------------------------------------+
|  Commands: create, join, accept, drop, ls, find, download, sort, devices,    |
|            folders, automatic, version, repl                                 |
+------------------------------------------------------------------------------+
                                      |
                                      v
+------------------------------------------------------------------------------+
|                           IrohNode (embedded)                                |
|  +--------------+  +--------------+  +--------------+  +--------------+     |
|  | iroh::Node   |  | iroh-blobs   |  | iroh-docs    |  | iroh-gossip  |     |
|  | (endpoint,   |<-| (store,      |<-| (author,     |<-| (topic:      |     |
|  |  identity,   |  |  fetch,      |  |  docs,       |  |  discovery,  |     |
|  |  relay)      |  |  share)      |  |  sync)       |  |  presence)   |     |
|  +--------------+  +--------------+  +--------------+  +--------------+     |
|         |                |                |                |                |
|         +----------------+----------------+----------------+                |
|                          v                v                                 |
|              +-----------------------+  +-----------------------+          |
|              |   Local Storage       |  |   Network Layer       |          |
|              |  (sqlite + blobs)     |  |  (QUIC + relays)      |          |
|              +-----------------------+  +-----------------------+          |
+------------------------------------------------------------------------------+
```

---

## Data Models

### 1. Syncweb Folder = Iroh Doc + Blob Store

```rust
// Each syncweb folder = 1 iroh-doc Author + 1 DocId + 1 BlobStore
struct SyncwebFolder {
    // Identity
    doc_id: DocId,                    // blake3(author_pubkey || namespace)
    author: Author,                   // Ed25519 keypair (per-folder or shared)
    namespace: NamespaceId,           // user-defined label (e.g., "audio")
    
    // Blob storage
    blob_store: BlobStore,            // iroh-blobs store (local path)
    
    // Sync state
    doc: Doc,                         // iroh-docs Doc handle
    sync_mode: SyncMode,              // SendReceive | SendOnly | ReceiveOnly | PublicReadOnly
    
    // Local filesystem
    local_path: PathBuf,              // Local mount point
    ignore_patterns: IgnorePatterns,  // Synced via doc (CRDT)
    
    // Peers
    known_peers: HashSet<NodeId>,     // Devices with doc access
    capabilities: CapabilityMap,      // NodeId -> Capability (Read/Write/Admin)
}
```

### 2. Sync Modes (replaces Syncthing folder types)

| Syncweb Type | Iroh Equivalent | Implementation |
|--------------|-----------------|----------------|
| `sendreceive` | `SyncMode::SendReceive` | Author key shared, full doc write |
| `sendonly` | `SyncMode::SendOnly` | Author key local only, share read cap |
| `receiveonly` | `SyncMode::ReceiveOnly` | Import doc with read cap only |
| `receiveencrypted` | `SyncMode::ReceiveEncrypted` | Encrypted blob store, no author key |
| **NEW** `public_readonly` | `SyncMode::PublicReadOnly` | **Public blob ticket, no auth needed** |

### 3. Capability System (replaces device + folder config)

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
enum Capability {
    /// Full read/write/admin (has author key)
    Admin(Author),
    /// Read + write (has write capability token)
    Write(WriteCap),
    /// Read only (has read capability token)
    Read(ReadCap),
    /// Public read (no auth, via blob ticket)
    PublicRead(PublicTicket),
}

struct CapabilityMap: HashMap<NodeId, Capability>
```

### 4. Syncweb URL Format (backward compatible + extended)

```
# Legacy (backward compatible)
sync://<folder-id>#<device-id>
sync://<folder-id>/sub/path#<device-id>

# New Iroh-native formats
iroh://<doc-id>#<node-id>                    # Join doc via node
iroh://<doc-id>/sub/path#<node-id>           # Join + auto-download subpath
iroh-blob://<blob-ticket>                    # Public blob access (no auth)
iroh-doc://<doc-id>?cap=<read-cap>           # Doc with explicit capability
```

### 5. Device Identity

```rust
// Syncthing Device ID = Ed25519 pubkey (base32, 56 chars)
// Iroh NodeId = Ed25519 pubkey (base32, 52 chars - same format!)
// COMPATIBLE: Can use same IDs!
struct DeviceId(NodeId);  // Type alias, same underlying format

impl DeviceId {
    fn from_syncthing(id: &str) -> Result<Self> { ... }
    fn to_syncthing(&self) -> String { ... }
}
```

---

## Iroh 1.0+ Crate Versions (as of 2025)

```toml
[dependencies]
# Core
iroh = "1.0.2"              # Node, endpoint, identity, relay
iroh-blobs = "0.103.0"      # Blob store, streaming, tickets
iroh-docs = "0.101.0"       # Document CRDT sync
iroh-gossip = "0.101.0"     # Gossip/pubsub, discovery
iroh-relay = "1.0.2"        # Relay client/server

# Utilities
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
clap = { version = "4", features = ["derive", "env"] }
tabulate = "0.4"
humanize = "0.1"
```

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
|   |   +-- iroh_node.rs        # IrohNode wrapper (embedded node)
|   |   +-- identity.rs         # Key management, device IDs
|   |   +-- relay.rs            # Relay config (iroh-relay + BEP bridge)
|   |   +-- discovery.rs        # Gossip + DHT + local discovery
|   |
|   +-- folder/
|   |   +-- mod.rs
|   |   +-- manager.rs          # FolderManager (create, join, list)
|   |   +-- syncweb_folder.rs   # SyncwebFolder struct + methods
|   |   +-- sync_mode.rs        # SyncMode enum + behavior
|   |   +-- ignore.rs           # Ignore patterns (CRDT-synced)
|   |   +-- capabilities.rs     # Capability management
|   |   +-- public.rs           # Public read-only folder support
|   |
|   +-- sync/
|   |   +-- mod.rs
|   |   +-- engine.rs           # SyncEngine (orchestrates blob + doc sync)
|   |   +-- blob_sync.rs        # iroh-blobs integration
|   |   +-- doc_sync.rs         # iroh-docs integration
|   |   +-- lazy_fetch.rs       # Selective sync (on-demand blob fetch)
|   |   +-- progress.rs         # Progress tracking, stats
|   |
|   +-- fs/
|   |   +-- mod.rs
|   |   +-- watcher.rs          # notify-rs file watcher
|   |   +-- scanner.rs          # Directory scanner, hashing
|   |   +-- importer.rs         # Import local files to blob store
|   |   +-- exporter.rs         # Export blobs to local filesystem
|   |   +-- ignore_filter.rs    # Apply ignore patterns
|   |
|   +-- net/
|   |   +-- mod.rs
|   |   +-- gossip.rs           # iroh-gossip topics
|   |   +-- discovery.rs        # Peer discovery (gossip + DHT + local)
|   |   +-- bep_relay.rs        # Optional BEP relay compatibility
|   |   +-- tickets.rs          # Ticket parsing/generation
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
|   |   +-- devices.rs          # syncweb devices
|   |   +-- folders.rs          # syncweb folders
|   |   +-- automatic.rs        # syncweb automatic
|   |   +-- repl.rs             # syncweb repl
|   |   +-- publish.rs          # NEW: syncweb publish
|   |   +-- subscribe.rs        # NEW: syncweb subscribe
|   |
|   +-- storage/
|   |   +-- mod.rs
|   |   +-- config.rs           # Persistent config (SQLite)
|   |   +-- folders.db          # Folder metadata
|   |   +-- migrations.rs       # Schema migrations
|   |
|   +-- util/
|       +-- mod.rs
|       +-- path.rs             # Path utilities
|       +-- format.rs           # Human formatting
|       +-- error.rs            # Error types
```

---

## Implementation Phases

### Phase 1: Foundation (Week 1-2)
**Goal**: Embedded Iroh node + basic identity + storage

- [ ] `Cargo.toml` with all dependencies
- [ ] `IrohNode` - embedded node lifecycle (start/stop, identity, relay)
- [ ] `IdentityManager` - DeviceId (Ed25519), key persistence
- [ ] `ConfigStore` - SQLite for folder metadata, peer caps
- [ ] Basic CLI structure with `clap`
- [ ] `syncweb version`, `syncweb repl` commands

### Phase 2: Folder Core (Week 2-3)
**Goal**: Create/join folders, basic sync

- [ ] `SyncwebFolder` - DocId, Author, BlobStore, SyncMode
- [ ] `FolderManager` - create, join, list, accept, drop
- [ ] `SyncMode` implementations (SendReceive, SendOnly, ReceiveOnly)
- [ ] Ignore patterns via iroh-docs (CRDT map)
- [ ] `syncweb create`, `syncweb join`, `syncweb accept`, `syncweb drop`
- [ ] `syncweb folders`, `syncweb devices`

### Phase 3: File Operations (Week 3-4)
**Goal**: ls, find, download, selective sync

- [ ] `FsWatcher` - notify-rs integration
- [ ] `Scanner` - walk dir, hash files (BLAKE3 via iroh-blobs)
- [ ] `Importer` - add files to blob store, update doc
- [ ] `Exporter` - lazy fetch blobs to filesystem
- [ ] `LazyFetch` - on-demand blob download (selective sync)
- [ ] `syncweb ls`, `syncweb find`, `syncweb download`

### Phase 4: Advanced Sync (Week 4-5)
**Goal**: Sort, automatic, progress, stats

- [ ] `SyncEngine` - orchestrates doc + blob sync
- [ ] Progress tracking, transfer stats
- [ ] `syncweb sort` (niche, frecency, etc.)
- [ ] `syncweb automatic` (auto-accept, auto-join)
- [ ] `syncweb devices --xfer`, bandwidth limits

### Phase 5: Public Read-Only Folders (Week 5-6) ⭐ NEW FEATURE
**Goal**: Native public sharing via iroh-blobs tickets

- [ ] `SyncMode::PublicReadOnly` implementation
- [ ] Public blob ticket generation (`iroh-blobs ticket`)
- [ ] `syncweb publish <folder>` - create public ticket
- [ ] `syncweb subscribe <ticket>` - join public folder (no auth)
- [ ] Public folder listing via gossip topic
- [ ] Read-only filesystem mount (FUSE or virtual fs)

### Phase 6: BEP Relay Compatibility (Week 6-7) ⭐ OPTIONAL
**Goal**: Interop with Syncthing/BEP relays

- [ ] BEP relay protocol implementation
- [ ] `iroh-relay` + BEP relay bridge
- [ ] Config: `--bep-relay <url>` for hybrid mode
- [ ] Discovery: announce on both iroh-gossip + BEP relay
- [ ] Test with real Syncthing nodes

### Phase 7: Polish & CLI Parity (Week 7-8)
**Goal**: Full CLI parity + UX

- [ ] All syncweb commands implemented
- [ ] Rich output (tables, progress bars)
- [ ] Config file support
- [ ] Shell completions
- [ ] Man pages
- [ ] Integration tests

---

## Public Read-Only Folders Design (Iroh-Native Feature) ⭐

### Concept
Iroh-blobs supports **public tickets** - anyone with the ticket can fetch blobs without authentication. This enables:
- Public websites/datasets shared via single URL
- No device pairing needed for readers
- Verified content (BLAKE3)
- Efficient range requests, streaming

### Implementation

```rust
// Creating a public folder
async fn publish_folder(&self, folder_id: &DocId) -> Result<PublicTicket> {
    // 1. Ensure folder is SendOnly or SendReceive (has author key)
    let folder = self.folders.get(folder_id)?;
    ensure!(folder.sync_mode.can_publish());
    
    // 2. Create a public blob ticket for the entire doc's blob set
    let ticket = self.node.blobs().share_all(folder.blob_store).await?;
    
    // 3. Announce on public gossip topic
    let topic = TopicId::from_bytes(blake3::hash(b"iroh-syncthing/public-folders"));
    self.node.gossip().publish(topic, PublicFolderAnnouncement {
        doc_id: *folder_id,
        label: folder.namespace.to_string(),
        ticket: ticket.clone(),
        author: folder.author.public_key(),
        created_at: UnixTimestamp::now(),
    }).await?;
    
    Ok(ticket)
}

// Subscribing to public folder (no auth needed)
async fn subscribe_public(&self, ticket: PublicTicket) -> Result<DocId> {
    // 1. Create local blob store
    let blob_store = self.node.blobs().create_store().await?;
    
    // 2. Import ticket - starts fetching blobs lazily
    let (doc_id, _cap) = self.node.docs().import_ticket(ticket).await?;
    
    // 3. Open doc (read-only)
    let doc = self.node.docs().open(doc_id).await?;
    
    // 4. Subscribe to doc updates (gossip)
    self.node.docs().subscribe(doc_id).await?;
    
    Ok(doc_id)
}
```

### CLI Commands

```bash
# Publish a folder publicly
syncweb publish audio/
# Output: iroh-blob://<ticket>  (shareable URL)

# Subscribe to public folder (no auth, read-only)
syncweb subscribe iroh-blob://<ticket>
# Creates local read-only folder, lazy-fetches on access

# List known public folders (from gossip)
syncweb public list
```

### Use Cases
- **Public datasets** - Share large datasets via single URL
- **Static websites** - Host sites on iroh, access via gateway or direct
- **Software distribution** - Verified binary distribution
- **Read-only mirrors** - One-way sync for backups/archives

---

## BEP Relay Compatibility (Optional)

### Why Optional?
- Iroh 1.0 has excellent built-in relay (iroh-relay) + QUIC NAT traversal
- BEP relay adds complexity for marginal gain
- Can be added later as feature flag

### If Implemented: Hybrid Mode

```rust
struct HybridDiscovery {
    iroh_gossip: GossipDiscovery,
    bep_relay: Option<BepRelayClient>,
    local_mdns: MdnsDiscovery,
}

impl Discovery for HybridDiscovery {
    async fn announce(&self, node_id: NodeId, addrs: Vec<SocketAddr>) {
        self.iroh_gossip.announce(node_id, addrs).await;
        if let Some(bep) = &self.bep_relay {
            bep.announce(node_id.to_syncthing_id(), addrs).await;
        }
        self.local_mdns.announce(node_id, addrs).await;
    }
    
    async fn discover(&self, node_id: NodeId) -> Vec<SocketAddr> {
        let mut addrs = self.iroh_gossip.discover(node_id).await;
        if let Some(bep) = &self.bep_relay {
            addrs.extend(bep.discover(node_id.to_syncthing_id()).await);
        }
        addrs.extend(self.local_mdns.discover(node_id).await);
        addrs
    }
}
```

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
| `sort` | `sort` | Sort results (uses doc metadata) |
| `stat` | `stat` | File metadata from doc + blob store |
| `automatic` | `automatic` | Auto-accept/join daemon |
| `version` | `version` | Show versions |
| `repl` | `repl` | Interactive REPL |
| **NEW** `publish` | `publish` | Create public blob ticket |
| **NEW** `subscribe` | `subscribe` | Join public folder via ticket |
| **NEW** `public list` | `public list` | List announced public folders |

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
# Optional BEP relay for Syncthing compat
bep_relay_url = ""  # e.g., "https://relay.syncthing.net"

[discovery]
# Enable/disable discovery mechanisms
local_mdns = true
iroh_gossip = true
mainline_dht = true
bep_relay = false

[folders]
default_path = "~/IrohSyncweb"
default_sync_mode = "SendReceive"

[public]
# Public folder settings
announce_enabled = true
gossip_topic = "iroh-syncthing/public-folders"

[advanced]
# Blob store settings
blob_cache_size_gb = 10
# Connection limits
max_connections = 100
```

---

## Key Technical Decisions

### 1. One Doc per Folder vs. One Doc for All
**Decision**: One Doc per folder (like Syncthing folders)
- Matches syncweb mental model
- Independent sync, permissions, ignore patterns
- Easier to share subset

### 2. Author Key Management
**Decision**: Per-folder author keys (derived from master)
- Master identity key (NodeId) for device identity
- Per-folder author keys derived: `HKDF(master, "folder/" + namespace)`
- Allows revoking per-folder without rotating device ID

### 3. Ignore Patterns
**Decision**: CRDT map in doc (key=path, value=pattern)
- Synced automatically via iroh-docs
- No separate .stignore file needed
- Can be edited from any device with write cap

### 4. Selective Sync (Lazy Fetch)
**Decision**: iroh-blobs on-demand + doc subscription
- Doc entries have blob hashes + sizes
- `download` command triggers blob fetch
- `ls`/`find` show metadata without fetching blobs

### 5. Public Folders
**Decision**: iroh-blobs public tickets + gossip announcement
- No auth required for readers
- Verified content (BLAKE3)
- Efficient range requests for large files

---

## Migration Path from syncweb-py

### For Users
1. Install `iroh-syncthing` (cargo install or binary)
2. Run `iroh-syncthing migrate --from ~/syncweb-home`
3. Imports: device ID, folders, peers, ignore patterns
4. Continues syncing with existing Syncthing peers (if BEP relay enabled)

### Migration Tool
```rust
async fn migrate_from_syncweb(syncweb_home: PathBuf) -> Result<()> {
    // 1. Read Syncthing config.xml
    // 2. Extract device ID (use as master identity)
    // 3. For each folder:
    //    - Create iroh-doc with same folder ID as namespace
    //    - Import files to blob store
    //    - Write doc entries with metadata
    //    - Convert .stignore to CRDT ignore map
    // 4. For each device:
    //    - Add as known peer
    //    - If folder shared, create capability
    // 5. Save to iroh-syncthing config
}
```

---

## Testing Strategy

### Unit Tests
- [ ] Identity management
- [ ] Ticket parsing/generation
- [ ] Ignore pattern matching
- [ ] Capability serialization

### Integration Tests
- [ ] Two nodes: create folder, join, sync files
- [ ] Three nodes: sendonly -> sendreceive -> receiveonly
- [ ] Public folder: publish -> subscribe -> read
- [ ] Selective sync: ls without download, then download
- [ ] Network partition: offline edits, reconnect, merge

### Interop Tests (if BEP relay enabled)
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
| Discovery time (global) | < 5s |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Iroh API changes (pre-1.0) | Low (1.0 released) | High | Pin versions, test against main branch |
| iroh-docs performance at scale | Medium | Medium | Benchmark early, optimize queries |
| Public folder spam/abuse | Medium | Low | Rate limit gossip, allowlist |
| BEP relay complexity | Low (optional) | High | Keep optional, feature-flagged |
| Windows file locking | Medium | Medium | Test early, use iroh-blobs async API |

---

## Success Criteria

1. **Functional parity**: All syncweb-py commands work
2. **Performance**: Faster sync, lower resource usage than Syncthing
3. **Public folders**: `publish`/`subscribe` work end-to-end
4. **Interop**: Can sync with Syncthing nodes (if BEP relay enabled)
5. **UX**: Single binary, no daemon, config file optional
6. **Reliability**: No data loss, verified transfers, conflict-free merges

---

## Appendix: Useful Iroh APIs

### iroh-blobs
```rust
// Store
let store = node.blobs().create_store().await?;
let hash = store.add_bytes(data).await?;

// Share (public ticket)
let ticket = store.share(hash, ShareMode::Public).await?;

// Fetch (lazy)
let reader = store.get(hash).await?;
// Range request
let reader = store.get_range(hash, 0..1024).await?;
```

### iroh-docs
```rust
// Create author + doc
let author = node.docs().create_author().await?;
let doc = author.create_doc().await?;

// Write entries
doc.set_bytes(author, b"path/to/file", blob_hash).await?;

// Subscribe to changes
let mut events = doc.subscribe().await?;
while let Some(event) = events.next().await { ... }

// Import from ticket
let (doc_id, cap) = node.docs().import_ticket(ticket).await?;
```

### iroh-gossip
```rust
// Subscribe to topic
let topic = TopicId::from_bytes(*b"my-topic");
let mut events = node.gossip().subscribe(topic).await?;

// Publish
node.gossip().publish(topic, payload).await?;

// Discover peers
let peers = node.gossip().peers(topic).await?;
```

### iroh (Node)
```rust
let node = Node::builder()
    .with_data_dir(data_dir)
    .with_relay_mode(RelayMode::Default)
    .spawn()
    .await?;

let node_id = node.node_id();
let endpoint = node.endpoint();
```

---

*Document version: 1.0*
*Created: 2025-07-15*
*Target: iroh 1.0+, iroh-blobs 0.103+, iroh-docs 0.101+, iroh-gossip 0.101+*
