# TDD Implementation Plan for syncweb (Rust + Iroh 1.0+)

Based on the implementation phases and [testing strategy](docs/testing.md).

---

## TDD Workflow Principles

1. Write failing test first → Watch it fail (Red)
2. Write minimal implementation → Make test pass (Green)
3. Refactor while keeping tests green (Refactor)
4. Run `cargo test --all-targets --all-features` after each module
5. Run `cargo clippy --all-targets --all-features -- -D warnings` after each phase
6. Run `cargo fmt --all` before commit

---

## Phase 6: Backup/Snapshot + Partial Fetch (Weeks 11-12)

### 6.1 Snapshot System (TDD)
Unit Tests (`tests/unit/snapshot_test.rs`):
- [ ] `test_create_snapshot` - Instant, references blobs
- [ ] `test_restore_snapshot` - Instant restore
- [ ] `test_snapshot_diff` - Added/removed/modified
- [ ] `test_snapshot_pin_gc` - Pinned blobs not GC'd
- [ ] `test_snapshot_sharing` - Share via ticket

Integration Tests:
- [ ] `test_backup_restore_cycle` - Backup -> modify -> restore -> verify

### 6.2 Partial Fetch / Health (TDD)
Unit Tests (`tests/unit/partial_fetch_test.rs`):
- [ ] `test_fetch_filter_min_peers` - Fetch blobs with <= N peers
- [ ] `test_fetch_filter_max_count` - Limit blob count
- [ ] `test_health_command` - Well/under/unseeded counts

Integration Tests:
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
Tests:
- [ ] `test_config_load_save` - Load TOML, merge with CLI args
- [ ] `test_per_folder_overrides` - Folder-specific config

### 7.4 Shell Completions
- [ ] `syncweb completions <shell>` generates completions

### 7.5 Integration Tests (Full Suite)
Tests (`tests/integration/full_suite_test.rs`):
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

Tests (`tests/interop/syncthing_test.rs`):
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
