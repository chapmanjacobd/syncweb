# TDD Implementation Plan for Phase 8: Indexing Service + Drop Format

Following the strict TDD workflow from TDD_IMPLEMENTATION_PLAN.md:
1. Write failing test first → Watch it fail (Red)
2. Write minimal implementation → Make test pass (Green)
3. Refactor while keeping tests green (Refactor)
4. Run `cargo test --all-targets --all-features` after each module
5. Run `cargo clippy --all-targets --all-features -- -D warnings` after each phase
6. Run `cargo fmt --all` before commit

---

## Phase 8A: Package Drop Format (CAR/ZSTD) - Weeks 15-16

### 8.1 Core Manifest Updates (TDD)
**Unit Tests** (`tests/unit/package_manifest_test.rs`):
- [ ] `test_manifest_signature_serialization` - Signature field serializes correctly
- [ ] `test_manifest_hash_excludes_signature` - Hash computed over unsigned manifest
- [ ] `test_manifest_sign_verify` - Ed25519 sign/verify roundtrip
- [ ] `test_manifest_dependencies` - Dependency validation logic
- [ ] `test_manifest_version_ordering` - Semver comparison for upgrades

**Integration Tests**:
- [ ] `test_package_init_with_signature` - Create package with maintainer key

### 8.2 Export Pipeline (TDD)
**Unit Tests** (`tests/unit/drop_export_test.rs`):
- [ ] `test_export_drop_basic` - Full package exports to .car.zst
- [ ] `test_export_drop_partial_filter` - Filter engine excludes files
- [ ] `test_export_drop_version_selection` - Specific version exported
- [ ] `test_export_drop_multi_package` - Multiple packages to output dir
- [ ] `test_export_drop_concurrency_safety` - Snapshot/lock prevents corruption
- [ ] `test_export_drop_empty_package` - Empty package creates valid CAR
- [ ] `test_export_drop_large_package` - Streaming handles large files (memory test)

**Integration Tests**:
- [ ] `test_export_drop_cli` - `syncweb package drop export` command works
- [ ] `test_export_drop_roundtrip` - Export → Import → Verify identical

### 8.3 Import Pipeline (TDD)
**Unit Tests** (`tests/unit/drop_import_test.rs`):
- [ ] `test_import_drop_basic` - Valid .car.zst imports successfully
- [ ] `test_import_drop_corrupted` - Corrupted archive fails with clear error
- [ ] `test_import_drop_missing_deps` - Missing dependencies abort import
- [ ] `test_import_drop_invalid_signature` - Bad signature rejected
- [ ] `test_import_drop_filter_stream` - Filter engine skips blobs during import
- [ ] `test_import_drop_partial` - Partial drop imports correctly
- [ ] `test_import_drop_version_conflict` - Existing version handled correctly

**Integration Tests**:
- [ ] `test_import_drop_cli` - `syncweb package drop import` command works
- [ ] `test_import_drop_materialize` - Auto-checkout after import

### 8.4 Drop Format Verification (TDD)
**Unit Tests** (`tests/unit/drop_verify_test.rs`):
- [ ] `test_drop_tamper_detection` - Modified blob fails verification
- [ ] `test_drop_manifest_mismatch` - Manifest hash mismatch detected
- [ ] `test_drop_dos_protection` - Large varints don't OOM (streaming safety)
- [ ] `test_drop_streaming_integrity` - No full archive loaded in memory

---

## Phase 8B: Opt-In Indexing Service - Weeks 17-20

### 8.5 Indexing Core Infrastructure (TDD)
**Unit Tests** (`tests/unit/indexing/core_test.rs`):
- [ ] `test_indexing_service_init` - Service starts with SQLite FTS5
- [ ] `test_indexing_enable_folder` - Folder opts into indexing
- [ ] `test_indexing_event_subscription` - Receives core engine events
- [ ] `test_indexing_database_schema` - FTS tables + metadata tables created
- [ ] `test_indexing_concurrent_access` - Thread-safe database access

### 8.6 Discovery & Catalogs (TDD)
**Unit Tests** (`tests/unit/indexing/catalog_test.rs`):
- [ ] `test_catalog_publish` - Publish folder metadata to catalog namespace
- [ ] `test_catalog_search_fts` - FTS5 search across catalogs
- [ ] `test_catalog_subscribe` - Subscribe to remote catalog
- [ ] `test_catalog_sync` - Catalog syncs via iroh-docs
- [ ] `test_catalog_local_vs_global` - `find` (local) vs `indexing search` (global)

**Integration Tests**:
- [ ] `test_catalog_publish_search` - End-to-end publish and search

### 8.7 Resilience & Availability (TDD)
**Unit Tests** (`tests/unit/indexing/resilience_test.rs`):
- [ ] `test_provider_lease_tracking` - Track signed ProviderLeases
- [ ] `test_replication_budget_enforcement` - Fetch/pin when below threshold
- [ ] `test_consistent_hashing_selection` - Only closest peers fetch
- [ ] `test_jitter_staggering` - Randomized delays prevent thundering herd
- [ ] `test_gossip_short_circuit` - New lease cancels pending fetch
- [ ] `test_health_verified_vs_local` - Verified leases vs local observation

**Integration Tests**:
- [ ] `test_resilience_fetch_triggered` - Low availability triggers fetch

### 8.8 Web of Trust Metadata (TDD)
**Unit Tests** (`tests/unit/indexing/wot_test.rs`):
- [ ] `test_wot_metadata_append` - Trusted authors append metadata
- [ ] `test_wot_metadata_index` - Metadata indexed and searchable
- [ ] `test_wot_trust_evaluation` - Local trust policy evaluation
- [ ] `test_wot_delegation` - Cryptographic trust delegation
- [ ] `test_wot_self_revocation` - Publisher revokes own content
- [ ] `test_wot_moderation_hide` - ModerationRecord hides content
- [ ] `test_wot_attestation_verify` - License/provenance attestations verified

### 8.9 Stable Links, Resolvers, Mirrors (TDD)
**Unit Tests** (`tests/unit/indexing/links_test.rs`):
- [ ] `test_immutable_link_create` - `syncweb://content/<hash>` links
- [ ] `test_mutable_link_create` - `syncweb://name/<publisher>/<alias>`
- [ ] `test_link_resolve` - Resolves to manifest + providers
- [ ] `test_link_version_pin` - Version pinning works
- [ ] `test_link_sequence_monotonic` - Sequence numbers prevent rollback
- [ ] `test_private_link_capability` - Capability-based with expiration
- [ ] `test_link_revoke` - Revoked private links block new fetches
- [ ] `test_mirror_registration` - Alternate providers registered
- [ ] `test_mirror_fallback` - Falls back to mirrors on failure

### 8.10 Denylists & Filtering (TDD)
**Unit Tests** (`tests/unit/indexing/denylist_test.rs`):
- [ ] `test_denylist_device` - Device-level blocking
- [ ] `test_denylist_file` - File-level blocking
- [ ] `test_denylist_hash` - Hash-level blocking
- [ ] `test_denylist_federated_subscribe` - Subscribe to filter list namespace
- [ ] `test_denylist_federated_sync` - Filter list auto-updates
- [ ] `test_denylist_hook_fetch` - Fetch pipeline validates against denylist
- [ ] `test_denylist_hook_discovery` - Discovery pipeline validates

### 8.11 Trust, Governance, Moderation (TDD)
**Unit Tests** (`tests/unit/indexing/moderation_test.rs`):
- [ ] `test_moderation_record_create` - Signed ModerationRecord
- [ ] `test_moderation_decision` - Show/Warn/Hide/Quarantine decisions
- [ ] `test_moderation_scope` - Scoped by network/folder/file
- [ ] `test_trust_policy_evaluate` - Local policy evaluation
- [ ] `test_moderation_hide_record` - Hide based on policy
- [ ] `test_moderation_list` - List records and decisions

### 8.12 CLI Commands (TDD)
**Unit Tests** (`tests/unit/indexing/cli_test.rs`):
- [ ] `test_indexing_enable_disable` - `syncweb indexing enable/disable`
- [ ] `test_indexing_publish` - `syncweb indexing publish`
- [ ] `test_indexing_search` - `syncweb indexing search "query"`
- [ ] `test_indexing_health` - `syncweb indexing health <hash>`
- [ ] `test_indexing_meta_add` - `syncweb indexing meta add`
- [ ] `test_indexing_filter_add` - `syncweb indexing filter add`
- [ ] `test_indexing_filter_subscribe` - `syncweb indexing filter subscribe`
- [ ] `test_link_create` - `syncweb link create`
- [ ] `test_link_resolve` - `syncweb link resolve`
- [ ] `test_link_revoke` - `syncweb link revoke`
- [ ] `test_mirror_add` - `syncweb mirror add`
- [ ] `test_trust_show` - `syncweb trust show`
- [ ] `test_trust_delegate` - `syncweb trust delegate`
- [ ] `test_attest` - `syncweb attest --license`
- [ ] `test_report` - `syncweb report --reason`
- [ ] `test_moderation_ls` - `syncweb moderation ls`
- [ ] `test_moderation_hide` - `syncweb moderation hide`

**Integration Tests** (`tests/integration/indexing_full_test.rs`):
- [ ] `test_full_indexing_workflow` - Enable → Publish → Search → Health
- [ ] `test_full_link_resolution` - Create → Resolve → Mirror fallback
- [ ] `test_full_moderation_workflow` - Report → Moderate → Hide
- [ ] `test_cross_node_catalog_sync` - Multi-node catalog sync

---

## Phase 8C: Performance Benchmarks (Continuous)

Add to `benches/` with `criterion`:
- [ ] `bench_drop_export_1gb` - Target < 10s
- [ ] `bench_drop_import_1gb` - Target < 10s
- [ ] `bench_indexing_search_10k` - Target < 100ms
- [ ] `bench_indexing_catalog_sync` - Target < 5s
- [ ] `bench_link_resolve` - Target < 50ms
- [ ] `bench_denylist_check` - Target < 1ms
- [ ] `bench_moderation_evaluate` - Target < 5ms

Run with: `cargo bench --all-features`

---

## Phase Gates for Phase 8

| Sub-Phase | Gate |
|-----------|------|
| 8.1-8.4 | Drop export/import roundtrip works, all unit tests pass, clippy clean |
| 8.5-8.6 | Indexing service starts, catalog publish/search works |
| 8.7-8.8 | Resilience fetches trigger, WoT metadata appends/queries |
| 8.9-8.11 | Links resolve, denylists block, moderation hides |
| 8.12 | All CLI commands work, integration tests pass, CI green |

---

## TDD Checklist Per Module (MANDATORY)

For EVERY module/file created:
- [ ] Write failing unit tests first
- [ ] Run tests → confirm RED
- [ ] Write minimal implementation
- [ ] Run tests → confirm GREEN
- [ ] Refactor with tests passing
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Run `cargo fmt --all`
- [ ] Add integration test if involves multiple components
- [ ] Add benchmarks for performance-critical paths

---

## Dependencies to Add

```toml
# For Drop Format
async-compression = { version = "0.4", features = ["tokio", "zstd"] }
# For Indexing Service
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio-rustls", "chrono", "uuid"] }
tantivy = "0.21"  # Alternative FTS if needed
# For crypto
ed25519-dalek = "2.0"
x25519-dalek = "2.0"
# For CLI
clap = { version = "4.5", features = ["derive", "env", "cargo"] }
```

---

## Getting Started (First Commands)

```bash
# 1. Add dependencies to Cargo.toml
# 2. Write first failing test for PackageManifest signature
# tests/unit/package_manifest_test.rs
#[test]
fn test_manifest_signature_serialization() {
    let manifest = PackageManifest::new(...);
    manifest.sign(&signing_key).unwrap();
    let json = serde_json::to_string(&manifest).unwrap();
    assert!(json.contains("signature"));
}

# 3. Run test -> fails (RED)
cargo test test_manifest_signature_serialization --all-features

# 4. Implement signature field in PackageManifest
# 5. Run test -> passes (GREEN)
# 6. Run clippy + fmt
# 7. Commit
```

---

*This plan follows strict TDD: no implementation code before a failing test exists. Each module follows Red-Green-Refactor cycle with clippy/fmt gates.*