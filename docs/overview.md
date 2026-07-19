# Overview

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
- **Version tracking & Collections** - Generalized collection manifests with immutable versions and mutable heads, supporting data packages, datasets, media libraries, and virtual collections.
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
| Package manifests | **Collection manifests** (generalized datasets + versions) |

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

### 21. Living Folders
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
