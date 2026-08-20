# Feature Areas

### Foundation
IrohNode with basic identity, storage, and logging:
- `Cargo.toml` with iroh 1.0.2 dependencies and distributed-topic-tracker 0.3.5
- `IrohNode` with Endpoint, Router, and protocol setup
- `IdentityManager` for SecretKey persistence and NodeId
- `BlobStore` via iroh-blobs persistent store
- `DocsEngine` via iroh-docs
- `GossipService` via iroh-gossip
- `TopicTracker` for distributed-topic-tracker integration (DHT-based peer discovery)
- Basic CLI with `clap`
- `tracing` structured logging
- `syncweb version`, `syncweb repl` commands

### Folder Core and Syncthing Relay Piggyback
Folder creation, joining, basic sync, and Syncthing relay fallback for CGNAT traversal:
- `SyncwebFolder` with NamespaceId, entries, and blob refs
- `FolderManager` for create, join, list, accept, drop
- `SyncMode` implementations (SendReceive, SendOnly, ReceiveOnly)
- `syncweb create`, `syncweb join`, `syncweb accept`, `syncweb drop`
- `syncweb folders`, `syncweb devices`
- `DeviceId` bidirectional conversion (Syncthing ↔ Iroh Ed25519)
- `SyncthingRelayTransport` with bounded framed TCP tunnel
- `TransportFallback` for ordered configured Syncthing relay attempts
- Syncthing relay protocol message codec (JoinRelayRequest, SessionInvitation, JoinSessionRequest)
- Datagram-over-TCP tunnel framing
- `--relay-fallback` flag on relevant commands
- `syncweb network test-relay` command
- Config: `[bep]` section for relay URLs, timeout, auto_fallback

### File Operations and Search/Sort/Stat
Commands for ls, find, sort, stat, download, selective sync, create/config:
- `FsWatcher` via notify-rs
- `Scanner` for directory walking and BLAKE3 hashing
- `ParallelScanner` for parallel directory scanning
- `Importer` for adding to blob store and updating docs
- `ParallelImporter` for parallel import pipeline
- `Exporter` for exporting blobs to local filesystem
- `ParallelExporter` for parallel export pipeline
- `LazyFetch` for on-demand blob download
- `Actor` as dedicated storage actor
- `SessionMode` for ReconcileOnce and Continuous modes
- `IntentHandle` as Stream + Sink for sync operations
- `FindEngine` with regex/glob/exact search and depth/size/time filters
- `Sorter` with niche, frecency, peers, random, and folder-aggregate sorting
- `StatOutput` with detailed file metadata and availability
- `InitResult` for folder creation and shareable URL output
- `syncweb ls`, `syncweb find`, `syncweb sort`, `syncweb stat`, `syncweb download`
- `syncweb create`, `syncweb config`
- Streaming output with optional collected sorting

### Advanced Sync and Networks
Sync engine, rules-based watch, and networks abstraction:
- `SyncEngine` for orchestration
- Progress tracking and transfer stats
- `PeerTracker` with cached peer availability from natural iroh flow
- `PeerTracker` with age-based cache eviction
- `EfficientPeerCache` with memory-efficient bitmask cache
- `FilterEngine` for rules-based watch/import filtering
- `SubscribeParams` for subscription filtering
- `DeletedTracker` for tracking deleted-but-previously-seen files
- `AreaOfInterest` with limits (max_size, max_count)
- `Network` struct and `NetworkManager` for create, join, leave, invite, kick
- Network gossip topics (`syncweb/net/<id>`)
- `syncweb watch` with filter engine (`--filters`, `--dry-run`, `--show-filters`)
- `syncweb join --subscribe` with SubscribeParams
- `syncweb network create`, `syncweb network ls`, `syncweb network join`
- `syncweb network leave`, `syncweb network invite`, `syncweb network kick`
- `syncweb create --network <name>`, `syncweb join --network <name>`

### Public Folders and Living Folders
Public sharing and data package versioning:
- Networks are always private; default daemon (no `--network`) is fully open
- Blob ticket generation
- Content pinning (prevent GC for shared blobs)
- `syncweb publish`, `syncweb unpublish`, `syncweb join --subscribe`
- `CollectionManifest` struct and iroh-docs storage
- `CollectionState` local tracking (installed collections, versions)
- `syncweb package init` (with package profile) for initializing paths as a versioned package
- `syncweb package add` for scanning, hashing files, and updating manifest
- `syncweb package bump` for creating a new version with changelog
- `syncweb package publish` for blob ticket and gossip announcement
- `syncweb package search` for discovering packages via gossip
- `syncweb package info` for detailed package metadata
- `syncweb package install` for fetch, verify, stage, and atomic swap
- `syncweb package upgrade` for updating to latest version
- `syncweb package remove` for cleaning up installed packages
- `syncweb package verify` for integrity check against manifest
- `syncweb package list` for listing locally installed packages
- `syncweb package versions` for listing installed versions
- `syncweb package switch` for changing active version
- Multi-version coexistence via versioned dirs and `current` symlink
- Atomic upgrade (stage, verify, symlink swap, cleanup)

### Backup/Snapshot and Partial Fetch
Content-addressed snapshots and robustness fetch:
- `syncweb snapshot create` for creating content-addressed snapshots
- `syncweb snapshot restore` for restoring from snapshot
- `syncweb snapshot list` for listing available snapshots
- `FetchStrategy::Filter` with `min_peers`/`max_peers` for fetch by seeder count
- `FetchStrategy::Filter` with `min_count`/`max_count` for fetch by file count
- `syncweb download --max-peers N` for improving folder network health
- `syncweb stats seeding` for showing seeding status per blob

### Polish and Integrations
Full CLI parity, UX, and advanced features:
- All commands implemented
- Rich output (tables, progress bars, and JSON output for scripting)
- Config file support (TOML)
- Shell completions
- Integration tests
- Documentation
- `syncweb watch` for file watcher real-time sync
- `syncweb stats network` for bandwidth accounting per folder/peer
- `syncweb verify` for integrity verification (re-check all local blobs)
- Sync schedules (global and per-folder overrides)
- Platform settings files for laptop/server/phone configurations
