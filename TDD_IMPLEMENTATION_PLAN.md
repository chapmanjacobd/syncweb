# TDD Implementation Plan for syncweb (Rust + Iroh 1.0+)

Based on CONVERSION_PLAN.md phases and docs/testing.md strategy.

---

## TDD Workflow Principles

1. **Write failing test first** → Watch it fail (Red)
2. **Write minimal implementation** → Make test pass (Green)
3. **Refactor** while keeping tests green (Refactor)
4. **Run `cargo test --all-targets --all-features`** after each module
5. **Run `cargo clippy --all-targets --all-features -- -D warnings`** after each phase
6. **Run `cargo fmt --all`** before commit

---

## Phase 1: Foundation (Weeks 1-2)

### 1.1 Project Setup & Dependencies
**Tests First:**
- [ ] `tests/unit/dependencies_test.rs` - Verify Cargo.toml dependencies compile
- [ ] `tests/unit/cargo_test.rs` - `cargo check --all-targets --all-features` passes

**Implementation:**
- [ ] `Cargo.toml` with iroh 1.0.2, iroh-blobs 0.103.0, iroh-docs 0.101.0, iroh-gossip 0.101.0, distributed-topic-tracker 0.3.5
- [ ] Workspace setup with `syncweb-core`, `syncweb-cli` crates

### 1.2 IdentityManager (TDD)
**Unit Tests (`tests/unit/identity_test.rs`):**
- [ ] `test_generate_node_id` - Generate Ed25519 keypair, derive NodeId
- [ ] `test_persist_secret_key` - Save/load SecretKey to disk
- [ ] `test_load_existing_identity` - Load existing identity from disk
- [ ] `test_node_id_derivation` - NodeId = Ed25519 public key (52-char base32)
- [ ] `test_device_id_conversion` - Syncthing DeviceId (56-char base32) ↔ Iroh NodeId (52-char base32) bidirectional

**Integration Tests (`tests/integration/identity_test.rs`):**
- [ ] `test_persistent_identity_across_restarts` - Start node, restart, verify same NodeId

**Implementation (`src/node/identity.rs`):**
- [ ] `IdentityManager::new(path)` - Load or create identity
- [ ] `IdentityManager::node_id()` -> NodeId
- [ ] `IdentityManager::secret_key()` -> SecretKey
- [ ] `DeviceId::from_syncthing()` / `to_syncthing()`

**Verify:** `cargo test identity --all-features && cargo clippy --all-targets --all-features -- -D warnings`

### 1.3 IrohNode (TDD)
**Unit Tests (`tests/unit/iroh_node_test.rs`):**
- [ ] `test_endpoint_creation` - Endpoint binds to port
- [ ] `test_router_setup` - Router with blobs, docs, gossip protocols
- [ ] `test_protocol_registration` - All 4 protocols registered
- [ ] `test_shutdown` - Clean shutdown closes all protocols

**Integration Tests (`tests/integration/iroh_node_test.rs`):**
- [ ] `test_two_nodes_connect` - Two nodes connect via direct QUIC
- [ ] `test_node_discovery` - Nodes discover each other via gossip

**Implementation (`src/node/iroh_node.rs`):**
- [ ] `IrohNode::new(IdentityManager)` - Build Endpoint + Router
- [ ] `IrohNode::start()` - Start listening
- [ ] `IrohNode::stop()` - Graceful shutdown
- [ ] Accessors: `blobs()`, `docs()`, `gossip()`, `endpoint()`

**Verify:** `cargo test iroh_node --all-features && cargo clippy --all-targets --all-features -- -D warnings`

### 1.4 BlobStore (TDD)
**Unit Tests (`tests/unit/blob_store_test.rs`):**
- [ ] `test_add_bytes` - Add bytes, get hash, verify retrieval
- [ ] `test_add_file` - Add file via streaming, verify BLAKE3
- [ ] `test_has_blob` - Check local existence
- [ ] `test_get_blob` - Retrieve blob by hash
- [ ] `test_blob_ticket` - Generate blob ticket for sharing

**Integration Tests (`tests/integration/blob_store_test.rs`):**
- [ ] `test_two_nodes_sync_blob` - Add blob on node A, fetch on node B via ticket

**Implementation (`src/node/blob_store.rs`):**
- [ ] `BlobStore::new(blobs_proto)` - Wrap iroh-blobs
- [ ] `BlobStore::add_bytes()`, `add_file()`, `has()`, `get()`, `ticket()`

**Verify:** `cargo test blob_store --all-features && cargo clippy --all-targets --all-features -- -D warnings`

### 1.5 DocsEngine (TDD)
**Unit Tests (`tests/unit/docs_engine_test.rs`):**
- [ ] `test_create_namespace` - Create namespace, get NamespaceId
- [ ] `test_author_from_secret` - Create Author from NamespaceSecret
- [ ] `test_set_get_entry` - Set entry, get entry by key
- [ ] `test_watch_entries` - Watch for entry changes (Stream)

**Implementation (`src/node/docs_engine.rs`):**
- [ ] `DocsEngine::new(docs_proto)` - Wrap iroh-docs
- [ ] `create_namespace()`, `author()`, `set()`, `get()`, `watch()`

**Verify:** `cargo test docs_engine --all-features && cargo clippy --all-targets --all-features -- -D warnings`

### 1.6 GossipService (TDD)
**Unit Tests (`tests/unit/gossip_test.rs`):**
- [ ] `test_subscribe_publish` - Subscribe to topic, publish, receive
- [ ] `test_multiple_subscribers` - Multiple subscribers receive same message

**Implementation (`src/node/gossip_service.rs`):**
- [ ] `GossipService::new(gossip_proto)` - Wrap iroh-gossip
- [ ] `subscribe(topic)`, `publish(topic, msg)`, `event_stream()`

**Verify:** `cargo test gossip --all-features && cargo clippy --all-targets --all-features -- -D warnings`

### 1.7 TopicTracker (distributed-topic-tracker) (TDD)
**Unit Tests (`tests/unit/topic_tracker_test.rs`):**
- [ ] `test_announce_topic` - Announce topic to DHT
- [ ] `test_find_peers` - Find peers for topic via DHT
- [ ] `test_bubble_detection` - Detect gossip mesh partitions

**Implementation (`src/node/discovery.rs`):**
- [ ] `TopicTracker::new(iroh_node)` - Initialize distributed-topic-tracker
- [ ] `announce(namespace_id)`, `find_peers(namespace_id)`

**Verify:** `cargo test topic_tracker --all-features && cargo clippy --all-targets --all-features -- -D warnings`

### 1.8 CLI Foundation (TDD)
**Unit Tests (`tests/unit/cli_test.rs`):**
- [ ] `test_version_command` - `syncweb version` outputs version
- [ ] `test_repl_command_starts` - `syncweb repl` starts REPL (smoke test)

**Integration Tests (`tests/integration/cli_test.rs`):**
- [ ] `test_help_output` - `syncweb --help` shows all commands

**Implementation (`src/cli/`):**
- [ ] `src/cli/commands.rs` - Clap command definitions
- [ ] `src/cli/args.rs` - Arg parsing/validation
- [ ] `src/cli/output.rs` - Table/JSON output formatting
- [ ] `src/main.rs` - Entry point, tracing init

**Verify:** `cargo test cli --all-features && cargo clippy --all-targets --all-features -- -D warnings`

### 1.9 Tracing/Logging Setup
**Tests:**
- [ ] `test_tracing_output` - Structured JSON logs to stdout
- [ ] `test_log_levels` - Filter by RUST_LOG

**Implementation:**
- [ ] `tracing-subscriber` with JSON formatting, env filter

---

## Phase 2: Folder Core + Syncthing Relay (Weeks 3-4)

### 2.1 SyncwebFolder (TDD)
**Unit Tests (`tests/unit/folder_test.rs`):**
- [ ] `test_create_folder` - Create folder, get NamespaceId
- [ ] `test_join_folder` - Join via ticket
- [ ] `test_sync_modes` - SendReceive, SendOnly, ReceiveOnly, ReceiveEncrypted, PublicReadOnly
- [ ] `test_capability_map` - NodeId -> Capability mapping
- [ ] `test_namespace_key_derivation` - Per-folder author key from master

**Implementation (`src/folder/syncweb_folder.rs`):**
- [ ] `SyncwebFolder::new(namespace_id, author, blob_store, docs_engine)`
- [ ] `SyncwebFolder::create()`, `join(ticket)`, `accept()`, `drop()`
- [ ] `SyncMode` enum with capability logic

### 2.2 FolderManager (TDD)
**Unit Tests (`tests/unit/folder_manager_test.rs`):**
- [ ] `test_list_folders` - List all managed folders
- [ ] `test_create_folder` - Create with path, mode, network
- [ ] `test_join_folder` - Join via ticket/URL
- [ ] `test_accept_invite` - Accept pending invite
- [ ] `test_drop_folder` - Remove local folder

**Integration Tests (`tests/integration/folder_manager_test.rs`):**
- [ ] `test_two_nodes_create_join_sync` - Node A creates, Node B joins, sync files
- [ ] `test_sendonly_receiveonly` - SendOnly -> SendReceive sync

**Implementation (`src/folder/manager.rs`):**
- [ ] `FolderManager::new(iroh_node)`
- [ ] `create()`, `join()`, `accept()`, `drop()`, `list()`

### 2.3 DeviceId Conversion (TDD)
**Unit Tests (`tests/unit/device_id_test.rs`):**
- [ ] `test_syncthing_to_iroh` - 56-char base32 -> 52-char base32
- [ ] `test_iroh_to_syncthing` - Round-trip conversion
- [ ] `test_invalid_format` - Reject invalid DeviceIds

**Implementation (`src/net/bep_identity.rs`):**
- [ ] `DeviceId::from_syncthing()`, `to_syncthing()`

### 2.4 SyncthingRelayTransport (TDD)
**Unit Tests (`tests/unit/relay_test.rs`):**
- [ ] `test_join_relay_request` - Encode/decode JoinRelayRequest
- [ ] `test_session_invitation` - Encode/decode SessionInvitation
- [ ] `test_join_session_request` - Encode/decode JoinSessionRequest
- [ ] `test_quic_over_tcp` - QUIC handshake over TCP tunnel

**Integration Tests (`tests/integration/relay_test.rs`):**
- [ ] `test_relay_connection` - Two nodes behind CGNAT connect via Syncthing relay
- [ ] `test_relay_fallback` - Direct fails -> iroh relay fails -> Syncthing relay succeeds

**Implementation (`src/net/relay.rs`):**
- [ ] `SyncthingRelayTransport::new(relay_url)`
- [ ] `connect(node_id)` - Establish QUIC-over-TCP tunnel
- [ ] `TransportFallback` - Direct -> Iroh Relay -> Syncthing Relay chain

### 2.5 CLI Commands (TDD)
**Commands:** `create`, `join`, `accept`, `drop`, `folders`, `devices`, `network test-relay`

**Integration Tests per command:**
- [ ] `test_create_command` - Creates folder, outputs ticket/URL
- [ ] `test_join_command` - Joins folder from ticket
- [ ] `test_folders_command` - Lists folders with status
- [ ] `test_devices_command` - Lists known devices
- [ ] `test_test_relay_command` - Tests relay connectivity

---

## Phase 3: File Operations + Search/Sort/Stat (Weeks 5-6)

### 3.1 ParallelScanner (TDD)
**Unit Tests (`tests/unit/scanner_test.rs`):**
- [ ] `test_scan_empty_dir` - Returns empty
- [ ] `test_scan_single_file` - Returns file with BLAKE3 hash
- [ ] `test_scan_nested_dirs` - Recursive traversal
- [ ] `test_parallel_vs_sequential` - Parallel matches sequential results
- [ ] `test_ignore_patterns` - Respects .gitignore-style patterns
- [ ] `test_large_directory_perf` - 10k files < 500ms (perf target)

**Implementation (`src/fs/scanner.rs`):**
- [ ] `Scanner::new(root, ignore_filter)` - Sequential
- [ ] `ParallelScanner::new(root, ignore_filter, threads)` - Rayon-based

### 3.2 Importer / ParallelImporter (TDD)
**Unit Tests (`tests/unit/importer_test.rs`):**
- [ ] `test_import_single_file` - Hash -> blob store -> doc entry
- [ ] `test_import_directory` - Recursive import
- [ ] `test_parallel_import` - Parallel matches sequential
- [ ] `test_import_idempotent` - Re-import same file = no duplicate blobs

**Integration Tests (`tests/integration/importer_test.rs`):**
- [ ] `test_import_then_sync` - Import on Node A, sync to Node B, verify

**Implementation (`src/fs/importer.rs`):**
- [ ] `Importer::new(blob_store, docs_engine, namespace)`
- [ ] `import_path(path)` -> Vec<Entry>
- [ ] `ParallelImporter` - Rayon pipeline

### 3.3 Exporter / ParallelExporter (TDD)
**Unit Tests (`tests/unit/exporter_test.rs`):**
- [ ] `test_export_single_blob` - Blob -> file
- [ ] `test_export_directory` - Recursive export preserving structure
- [ ] `test_parallel_export` - Matches sequential
- [ ] `test_export_verify_hash` - Exported file matches BLAKE3

### 3.4 LazyFetch (Selective Sync) (TDD)
**Unit Tests (`tests/unit/lazy_fetch_test.rs`):**
- [ ] `test_ls_without_download` - `ls` shows metadata, no blob fetch
- [ ] `test_download_triggers_fetch` - `download` fetches blobs on demand
- [ ] `test_find_without_download` - `find` searches metadata only

**Integration Tests (`tests/integration/lazy_fetch_test.rs`):**
- [ ] `test_selective_sync` - Node A has 100 files, Node B ls -> downloads 1 file

**Implementation (`src/sync/lazy_fetch.rs`):**
- [ ] `LazyFetch::new(blob_store, docs_engine)`
- [ ] `fetch(path)` -> IntentHandle (Stream + Sink)

### 3.5 Actor Pattern (Storage Thread) (TDD)
**Unit Tests (`tests/unit/actor_test.rs`):**
- [ ] `test_actor_handles_messages` - Send messages to actor thread
- [ ] `test_actor_panic_isolation` - Panic in actor doesn't crash main

**Implementation (`src/sync/actor.rs`):**
- [ ] `Actor::spawn(storage)` - Dedicated thread with mpsc channel
- [ ] `ActorHandle` - Send commands, receive responses

### 3.6 SessionMode & IntentHandle (TDD)
**Unit Tests (`tests/unit/session_test.rs`):**
- [ ] `test_reconcile_once` - Single sync, then done
- [ ] `test_continuous` - Ongoing sync subscription
- [ ] `test_intent_handle_stream` - Progress events as Stream
- [ ] `test_intent_handle_sink` - Pause/resume/cancel via Sink

**Implementation (`src/sync/session.rs`, `src/sync/intents.rs`):**
- [ ] `SessionMode::ReconcileOnce`, `Continuous`
- [ ] `IntentHandle` - Stream<Item=SyncEvent> + Sink<SyncCommand>

### 3.7 FindEngine (TDD)
**Unit Tests (`tests/unit/find_test.rs`):**
- [ ] `test_exact_match` - Exact filename match
- [ ] `test_glob_match` - `*.rs` pattern
- [ ] `test_regex_match` - `report-\d+\.pdf`
- [ ] `test_depth_filter` - Max depth
- [ ] `test_size_filter` - Min/max size
- [ ] `test_time_filter` - Modified after/before

**Implementation (`src/cli_commands/find.rs`):**
- [ ] `FindEngine::new(folder)` with filters

### 3.8 Sorter (TDD)
**Unit Tests (`tests/unit/sort_test.rs`):**
- [ ] `test_niche_sort` - Rare files first
- [ ] `test_frecency_sort` - Frequency + recency
- [ ] `test_peers_sort` - Most seeders first
- [ ] `test_random_sort` - Random order
- [ ] `test_folder_aggregate` - Aggregate across folders

**Implementation (`src/cli_commands/sort.rs`):**
- [ ] `Sorter::new(peer_tracker)` with algorithms

### 3.9 StatOutput (TDD)
**Unit Tests (`tests/unit/stat_test.rs`):**
- [ ] `test_stat_file` - Metadata, hash, availability, version vector
- [ ] `test_stat_folder` - Aggregate stats

**Implementation (`src/cli_commands/stat.rs`):**
- [ ] `StatOutput::from_entry(entry, peer_tracker)`

### 3.10 CLI Commands (TDD)
**Commands:** `ls`, `find`, `sort`, `stat`, `download`, `init`, `config`

**Integration Tests per command:**
- [ ] `test_ls_streaming` - Default streaming output
- [ ] `test_ls_sort` - `--sort` collects then sorts
- [ ] `test_find_regex_glob_exact` - All match types
- [ ] `test_sort_algorithms` - All sort modes
- [ ] `test_stat_detailed` - Full metadata output
- [ ] `test_download_selective` - Download specific paths
- [ ] `test_init_outputs_url` - Creates folder + prints share URL
- [ ] `test_config_toml` - Read/write config file

---

## Phase 4: Advanced Sync + Networks (Weeks 7-8)

### 4.1 SyncEngine (TDD)
**Unit Tests (`tests/unit/sync_engine_test.rs`):**
- [ ] `test_orchestrates_blob_doc_sync` - Coordinates blob + doc sync
- [ ] `test_progress_tracking` - Emits progress events
- [ ] `test_transfer_stats` - Bytes/sec, peer count, ETA

**Integration Tests (`tests/integration/sync_engine_test.rs`):**
- [ ] `test_full_sync_cycle` - Create -> modify -> sync -> verify

**Implementation (`src/sync/engine.rs`):**
- [ ] `SyncEngine::new(folder_manager, blob_store, docs_engine, gossip)`
- [ ] `sync(folder_id, mode)` -> IntentHandle

### 4.2 PeerTracker with Cache Eviction (TDD)
**Unit Tests (`tests/unit/peer_tracker_test.rs`):**
- [ ] `test_track_peer_availability` - Record peer seen for blob
- [ ] `test_age_based_eviction_lru` - LRU eviction under memory pressure
- [ ] `test_age_based_eviction_fifo` - FIFO eviction
- [ ] `test_max_cache_size` - Respects configured limit
- [ ] `test_peer_cache_lookup_perf` - < 1ms lookup (perf target)

**Implementation (`src/sync/peer_tracker.rs`):**
- [ ] `PeerTracker::new(max_cache_size, eviction_strategy)`
- [ ] `record_peer(blob_hash, node_id)`, `get_peers(blob_hash)`
- [ ] `tick_and_maybe_evict()` - Periodic cleanup

### 4.3 EfficientPeerCache (Bitmask) (TDD)
**Unit Tests (`tests/unit/efficient_cache_test.rs`):**
- [ ] `test_bitmask_operations` - Set/clear/check bits
- [ ] `test_memory_efficiency` - 1000 peers < 1MB (perf target)
- [ ] `test_fallback_hashmap` - Small networks use HashMap
- [ ] `test_compressed_indices` - Active peers per blob

**Implementation (`src/sync/peer_tracker.rs`):**
- [ ] `EfficientPeerCache` - BitVec for presence, compressed indices

### 4.4 FilterEngine (TDD)
**Unit Tests (`tests/unit/filter_test.rs`):**
- [ ] `test_rule_evaluation` - Match rules (include/exclude patterns)
- [ ] `test_version_constraints` - Semver constraints
- [ ] `test_per_folder_overrides` - Global + per-folder rules
- [ ] `test_eval_perf` - < 10ms per entry (perf target)

**Integration Tests (`tests/integration/filter_test.rs`):**
- [ ] `test_automatic_daemon` - Filter engine runs continuous sync

**Implementation (`src/filter/`):**
- [ ] `FilterEngine`, `Rules`, `Evaluator`, `Config`

### 4.5 SubscribeParams / DeletedTracker / AreaOfInterest (TDD)
**Unit Tests:**
- [ ] `test_subscribe_params` - Filter subscriptions by path/size/time
- [ ] `test_deleted_tracker` - Track deleted entries, enable undelete
- [ ] `test_area_of_interest_limits` - Max size/count enforcement

### 4.6 Network Management (TDD)
**Unit Tests (`tests/unit/network_test.rs`):**
- [ ] `test_network_create` - Creates gossip topic
- [ ] `test_network_join` - Joins via invite
- [ ] `test_network_invite` - Generates invite ticket
- [ ] `test_network_kick` - Removes member
- [ ] `test_folder_network_membership` - Folder joins network gossip topic

**Integration Tests (`tests/integration/network_test.rs`):**
- [ ] `test_two_node_network` - Create, invite, join, folder sync
- [ ] `test_three_node_mixed_roles` - Admin + members

**Implementation (`src/net/network.rs`, `src/net/network_manager.rs`):**
- [ ] `Network`, `NetworkManager`
- [ ] Gossip topic: `syncweb/net/<id>`

### 4.7 CLI Commands (TDD)
**Commands:** `automatic`, `subscribe`, `network create/ls/join/leave/invite/kick`, `create --network`, `join --network`

---

## Phase 5: Public Folders + Living Folders (Weeks 9-10)

### 5.1 PublicReadOnly SyncMode (TDD)
**Unit Tests:**
- [ ] `test_public_readonly_mode` - No auth required for readers
- [ ] `test_blob_ticket_generation` - Public blob tickets
- [ ] `test_content_pinning` - Prevents GC of shared blobs

**Integration Tests:**
- [ ] `test_publish_subscribe` - Node A publishes, Node B subscribes, reads

### 5.2 CollectionManifest / CollectionState (TDD)
**Unit Tests (`tests/unit/collection_test.rs`):**
- [ ] `test_manifest_serialization` - Serialize/deserialize
- [ ] `test_collection_head` - Mutable head tracking
- [ ] `test_virtual_collections` - Assemble from content-addressed entries

### 5.3 Data Package Lifecycle (TDD)
**Commands:** `collection init/add/versions/publish`, `package search/info/install/upgrade/remove/verify/list/versions/switch`

**Integration Tests (`tests/integration/package_test.rs`):**
- [ ] `test_package_lifecycle` - init -> add -> bump -> publish -> search -> install -> upgrade -> remove
- [ ] `test_multi_version_coexistence` - Install v1, install v2, switch between
- [ ] `test_atomic_upgrade` - Stage -> verify -> symlink swap -> cleanup
- [ ] `test_package_integrity` - Verify catches corruption
- [ ] `test_package_discovery` - Publish -> search -> info across nodes

**Implementation (`src/folder/versioning.rs`, `src/folder/public.rs`):**
- [ ] Collection manifests in iroh-docs
- [ ] Package profiles (dependency + atomic install)
- [ ] Versioned directories + `current` symlink

---

## Phase 6: Backup/Snapshot + Partial Fetch (Weeks 11-12)

### 6.1 Snapshot System (TDD)
**Unit Tests (`tests/unit/snapshot_test.rs`):**
- [ ] `test_create_snapshot` - Instant, references blobs
- [ ] `test_restore_snapshot` - Instant restore
- [ ] `test_snapshot_diff` - Added/removed/modified
- [ ] `test_snapshot_pin_gc` - Pinned blobs not GC'd
- [ ] `test_snapshot_sharing` - Share via ticket

**Integration Tests:**
- [ ] `test_backup_restore_cycle` - Backup -> modify -> restore -> verify

### 6.2 Partial Fetch / Health (TDD)
**Unit Tests (`tests/unit/partial_fetch_test.rs`):**
- [ ] `test_fetch_filter_min_peers` - Fetch blobs with <= N peers
- [ ] `test_fetch_filter_max_count` - Limit blob count
- [ ] `test_health_command` - Well/under/unseeded counts

**Integration Tests:**
- [ ] `test_download_max_peers` - Improves seeder distribution

---

## Phase 7: Polish + Integrations (Weeks 13-14)

### 7.1 All Commands Complete
- [ ] Verify all commands from `docs/commands.md` implemented

### 7.2 Rich Output
- [ ] Tables with `comfy-table` or `tabled`
- [ ] Progress bars with `indicatif`
- [ ] JSON output flag (`--json`)

### 7.3 Config File (TOML)
**Tests:**
- [ ] `test_config_load_save` - Load TOML, merge with CLI args
- [ ] `test_per_folder_overrides` - Folder-specific config

### 7.4 Shell Completions
- [ ] `syncweb completions <shell>` generates completions

### 7.5 Integration Tests (Full Suite)
**Tests (`tests/integration/full_suite_test.rs`):**
- [ ] All scenarios from `docs/testing.md` Integration Tests section

### 7.6 Documentation
- [ ] Man pages via `clap_mangen`
- [ ] README with quickstart
- [ ] Command reference

### 7.7 Advanced Features (Lower Priority)
- [ ] `syncweb watch` - File watcher (notify-rs)
- [ ] `syncweb stats` - Bandwidth accounting
- [ ] `syncweb verify` - Integrity re-check
- [ ] Sync schedules (cron-like)
- [ ] Platform configs (laptop/server/phone presets)

---

## Performance Benchmark Tests (Continuous)

Add to `benches/` with `criterion`:
- [ ] `bench_startup_time` - Target < 500ms
- [ ] `bench_memory_idle` - Target < 50MB
- [ ] `bench_memory_sync_10gb` - Target < 200MB
- [ ] `bench_blob_throughput_lan` - Target > 500 MB/s
- [ ] `bench_blob_throughput_wan` - Target > 50 MB/s
- [ ] `bench_doc_sync_latency` - Target < 50ms
- [ ] `bench_discovery_local` - Target < 1s
- [ ] `bench_discovery_dht` - Target < 10s
- [ ] `bench_peer_cache_lookup` - Target < 1ms
- [ ] `bench_filter_eval` - Target < 10ms/entry
- [ ] `bench_scan_10k` - Target < 500ms
- [ ] `bench_import_1k` - Target < 3s
- [ ] `bench_export_1k` - Target < 2.5s
- [ ] `bench_cache_eviction_10k` - Target < 10ms
- [ ] `bench_efficient_cache_1k_peers` - Target < 1MB

Run with: `cargo bench --all-features`

---

## Interop Tests (Phase 7 - with `--bep` flag)

**Tests (`tests/interop/syncthing_test.rs`):**
- [ ] `test_syncthing_to_syncweb` - Syncthing node joins syncweb folder
- [ ] `test_syncweb_to_syncthing` - Syncweb node joins Syncthing folder
- [ ] `test_bidirectional_sync` - Both directions work
- [ ] `test_relay_only_connection` - Direct fails, relay works

Requires: Syncthing test container in CI

---

## CI/CD Pipeline

`.github/workflows/ci.yml`:
```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - cargo test --all-targets --all-features
      - cargo test --doc
      - cargo clippy --all-targets --all-features -- -D warnings
      - cargo fmt --all -- --check
      - cargo bench --all-features -- --no-run  # Compile check

  test-windows:
    runs-on: windows-latest
    steps:
      - cargo test --all-targets --all-features

  test-macos:
    runs-on: macos-latest
    steps:
      - cargo test --all-targets --all-features

  interop:
    runs-on: ubuntu-latest
    services:
      syncthing: # container
    steps:
      - cargo test --test interop --features bep
```

---

## Phase Gates (Must Pass Before Next Phase)

| Phase | Gate |
|-------|------|
| 1 | All unit tests pass, clippy clean, 2-node integration test passes |
| 2 | Folder create/join/sync works, relay fallback works |
| 3 | ls/find/sort/stat/download work, parallel ops 4-6x speedup |
| 4 | SyncEngine orchestrate, FilterEngine auto-sync, Networks work |
| 5 | Public folders + full package lifecycle work |
| 6 | Snapshots instant, partial fetch improves seeder health |
| 7 | All commands, rich output, config, completions, docs, CI green |

---

## TDD Checklist Per Module

For EVERY module/file created:
- [ ] Write failing unit tests first
- [ ] Run tests -> confirm RED
- [ ] Write minimal implementation
- [ ] Run tests -> confirm GREEN
- [ ] Refactor with tests passing
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Run `cargo fmt --all`
- [ ] Add integration test if involves multiple components
- [ ] Add benchmarks for performance-critical paths

---


## Risk Mitigation (from docs/testing.md)

| Risk | TDD Mitigation |
|------|----------------|
| Iroh API changes | Pin versions, integration tests against pinned versions |
| iroh-docs performance | Benchmark early in Phase 1, regression tests in CI |
| Syncthing relay protocol | Interop tests in Phase 7, protocol is simple (3 msg types) |
| Windows file locking | Test on Windows CI from Phase 1 |
| DHT availability | Integration tests use local DHT bootstrap; fallback to relays |
| Cache eviction thrashing | Unit tests for LRU/FIFO under load |
| Parallelism deadlocks | Stress tests with rayon, timeout guards |

---

## Getting Started (First Commands)

```bash
# 1. Initialize Cargo workspace
cargo new --lib syncweb-core
cargo new --bin syncweb-cli

# 2. Add dependencies to Cargo.toml (see phases.md Phase 1)

# 3. Write first failing test
# tests/unit/identity_test.rs
#[test]
fn test_generate_node_id() {
    let mgr = IdentityManager::new(temp_dir());
    let node_id = mgr.node_id();
    assert_eq!(node_id.to_string().len(), 52); // base32 Ed25519
}

# 4. Run test -> fails (RED)
cargo test test_generate_node_id --all-features

# 5. Implement IdentityManager
# 6. Run test -> passes (GREEN)
# 7. Run clippy + fmt
# 8. Commit
```

---

*This plan follows strict TDD: no implementation code before a failing test exists. Each module follows Red-Green-Refactor cycle with clippy/fmt gates.*
