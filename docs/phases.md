# Implementation Phases

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
- [ ] Network gossip topics (`syncweb/net/<id>`)
- [ ] `syncweb automatic` with filter engine
- [ ] `syncweb subscribe` with SubscribeParams
- [ ] `syncweb network create`, `syncweb network ls`, `syncweb network join`
- [ ] `syncweb network leave`, `syncweb network invite`, `syncweb network kick`
- [ ] `syncweb create --network <name>`, `syncweb join --network <name>`

### Phase 5: Public Folders + Living Folders
**Goal**: Public sharing + data package versioning

- [ ] `SyncMode::PublicReadOnly`
- [ ] Blob ticket generation
- [ ] Content pinning (prevent GC for shared blobs)
- [ ] `syncweb publish`, `syncweb unpublish`, `syncweb subscribe`
- [ ] `CollectionManifest` struct + iroh-docs storage
- [ ] `CollectionState` local tracking (installed collections, versions)
- [ ] `syncweb collection init` (with package profile) — initialize folder as data package
- [ ] `syncweb collection add` — scan + hash files, update manifest
- [ ] `syncweb collection versions` — create new version with changelog
- [ ] `syncweb collection publish` — blob ticket + gossip announcement
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
