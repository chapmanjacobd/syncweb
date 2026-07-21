# Implementation Phases

### Phase 1: Foundation
Goal: IrohNode + basic identity + storage + logging

- [x] `Cargo.toml` with correct iroh 1.0.2 dependencies + distributed-topic-tracker 0.3.5
- [x] `IrohNode` - Endpoint + Router + protocol setup
- [x] `IdentityManager` - SecretKey persistence, NodeId
- [x] `BlobStore` - iroh-blobs persistent store
- [x] `DocsEngine` - iroh-docs setup
- [x] `GossipService` - iroh-gossip setup
- [x] `TopicTracker` - distributed-topic-tracker integration (DHT-based peer discovery)
- [x] Basic CLI with `clap`
- [x] `tracing` structured logging setup
- [x] `syncweb version`, `syncweb repl` commands

### Phase 2: Folder Core + Syncthing Relay Piggyback
Goal: Create/join folders, basic sync, Syncthing relay fallback for CGNAT traversal

- [x] `SyncwebFolder` - NamespaceId, entries, blob refs
- [x] `FolderManager` - create, join, list, accept, drop
- [x] `SyncMode` implementations (SendReceive, SendOnly, ReceiveOnly)
- [x] `syncweb create`, `syncweb join`, `syncweb accept`, `syncweb drop`
- [x] `syncweb folders`, `syncweb devices`
- [x] `DeviceId` bidirectional conversion (Syncthing ↔ Iroh Ed25519)
- [x] `SyncthingRelayTransport` - bounded framed TCP tunnel
- [x] `TransportFallback` - ordered configured Syncthing relay attempts
- [x] Syncthing relay protocol message codec (JoinRelayRequest, SessionInvitation, JoinSessionRequest)
- [x] Datagram-over-TCP tunnel framing
- [x] `--relay-fallback` flag on relevant commands
- [x] `syncweb network test-relay` command
- [x] Config: `[bep]` section for relay URLs, timeout, auto_fallback

### Phase 3: File Operations + Search/Sort/Stat
Goal: ls, find, sort, stat, download, selective sync, init/config

- [x] `FsWatcher` - notify-rs
- [x] `Scanner` - walk dir, BLAKE3 hash
- [x] `ParallelScanner` - parallel directory scanning
- [x] `Importer` - add to blob store, update doc
- [x] `ParallelImporter` - parallel import pipeline
- [x] `Exporter` - export blobs to local filesystem
- [x] `ParallelExporter` - parallel export pipeline
- [x] `LazyFetch` - on-demand blob download
- [x] `Actor` - dedicated storage actor
- [x] `SessionMode` - ReconcileOnce vs Continuous
- [x] `IntentHandle` - Stream + Sink for sync operations
- [x] `FindEngine` - regex/glob/exact search with depth/size/time filters
- [x] `Sorter` - niche, frecency, peers, random, folder-aggregate sorting
- [x] `StatOutput` - detailed file metadata and availability
- [x] `InitResult` - folder creation and shareable URL output
- [x] `syncweb ls`, `syncweb find`, `syncweb sort`, `syncweb stat`, `syncweb download`
- [x] `syncweb init`, `syncweb config`
- [x] Streaming output with optional collected sorting

### Phase 4: Advanced Sync + Networks
Goal: Sync engine, automatic daemon, networks abstraction

- [x] `SyncEngine` - orchestration
- [x] Progress tracking, transfer stats
- [x] `PeerTracker` - cached peer availability from natural iroh flow
- [x] `PeerTracker` - age-based cache eviction ((standard CS pattern: age-based cache eviction))
- [x] `EfficientPeerCache` - memory-efficient bitmask cache ((standard CS pattern: memory-efficient bitmask presence))
- [x] `FilterEngine` - rules-based automatic daemon
- [x] `SubscribeParams` - subscription filtering (from iroh-willow)
- [x] `DeletedTracker` - track deleted-but-previously-seen files (from iroh-willow)
- [x] `AreaOfInterest` with limits (max_size, max_count) (from iroh-willow)
- [x] `Network` struct + `NetworkManager` - create, join, leave, invite, kick
- [x] Network gossip topics (`syncweb/net/<id>`)
- [x] `syncweb automatic` with filter engine
- [x] `syncweb subscribe` with SubscribeParams
- [x] `syncweb network create`, `syncweb network ls`, `syncweb network join`
- [x] `syncweb network leave`, `syncweb network invite`, `syncweb network kick`
- [x] `syncweb create --network <name>`, `syncweb join --network <name>`

### Phase 5: Public Folders + Living Folders
Goal: Public sharing + data package versioning

- [x] `SyncMode::PublicReadOnly`
- [x] Blob ticket generation
- [x] Content pinning (prevent GC for shared blobs)
- [x] `syncweb publish`, `syncweb unpublish`, `syncweb subscribe`
- [x] `CollectionManifest` struct + iroh-docs storage
- [x] `CollectionState` local tracking (installed collections, versions)
- [x] `syncweb collection init` (with package profile) -- initialize folder as data package
- [x] `syncweb collection add` -- scan + hash files, update manifest
- [x] `syncweb collection versions` -- create new version with changelog
- [x] `syncweb collection publish` -- blob ticket + gossip announcement
- [x] `syncweb package search` -- discover packages via gossip
- [x] `syncweb package info` -- detailed package metadata
- [x] `syncweb package install` -- fetch + verify + stage + atomic swap
- [x] `syncweb package upgrade` -- update to latest version
- [x] `syncweb package remove` -- clean up installed package
- [x] `syncweb package verify` -- integrity check against manifest
- [x] `syncweb package list` -- list locally installed packages
- [x] `syncweb package versions` -- list installed versions
- [x] `syncweb package switch` -- change active version
- [x] Multi-version coexistence (versioned dirs + `current` symlink)
- [x] Atomic upgrade (stage → verify → symlink swap → cleanup)

### Phase 6: Backup/Snapshot + Partial Fetch
Goal: Content-addressed snapshots + robustness fetch

- [x] `syncweb backup` - create content-addressed snapshot
- [x] `syncweb restore` - restore from snapshot
- [x] `syncweb snapshots` - list available snapshots
- [x] `FetchStrategy::Filter` with `min_peers`/`max_peers` - fetch by seeder count
- [x] `FetchStrategy::Filter` with `min_count`/`max_count` - fetch by file count
- [x] `syncweb download --max-peers N` - improve folder network health
- [x] `syncweb health` - show seeding status per blob

### Phase 7: Polish + Integrations
Goal: Full CLI parity + UX + advanced features

- [ ] All commands implemented
- [ ] Rich output (tables, progress bars)
- [ ] Config file support (TOML)
- [ ] Shell completions
- [ ] Integration tests
- [ ] Documentation
- [ ] `syncweb watch` -- file watcher for real-time sync (lowest priority)
- [ ] `syncweb stats` -- bandwidth accounting per folder/peer
- [ ] `syncweb verify` -- integrity verification (re-check all local blobs)
- [ ] Sync schedules (global + per-folder overrides)
- [ ] Platform settings files (suggested configs for laptop/server/phone)
