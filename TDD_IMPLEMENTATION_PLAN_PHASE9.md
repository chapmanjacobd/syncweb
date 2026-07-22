# TDD Implementation Plan for Phase 9: Provider Reputation & Smart Ban

Following the strict TDD workflow from TDD_IMPLEMENTATION_PLAN.md:
1. Write failing test first → Watch it fail (Red)
2. Write minimal implementation → Make test pass (Green)
3. Refactor while keeping tests green (Refactor)
4. Run `cargo test --all-targets --all-features` after each module
5. Run `cargo clippy --all-targets --all-features -- -D warnings` after each phase
6. Run `cargo fmt --all` before commit

---

## Motivation

Phase 8.7 introduces `ProviderLease` tracking and `ensure_replication`, but a signed lease is only a claim — it does not verify that the provider actually has the data. If a provider loses data (or never had it), its lease remains in the tracker until expiry, counts toward the verified count, and blocks `needs_replication()` from triggering a repair fetch.

Phase 9 closes this gap using techniques inspired by the smart ban pattern from libtorrent: correlate fetch failures to specific providers, invalidate stale leases on failure, and retroactively clean up after a successful fetch. It also introduces provider reputation scoring and integrates with the Web of Trust layer to enable both automated and manual provider trust streams.

## Identified Gaps (Phase 8 → Phase 9)

| Gap | Current State | Phase 9 Fix |
|-----|---------------|-------------|
| No fetch error categorization | All fetch errors collapse into `SyncwebError::Operation` | `FetchFailureKind` enum classifies failures |
| Fetch failures don't affect leases | `ensure_replication` logs errors and moves on | Failure tracking + lease invalidation |
| No provider reputation | Binary trust only (trusted/untrusted) | `ProviderReputation` scoring with decay |
| No provider-level denylist | Phase 8.10 covers content denylists only | Provider ban records (manual + automated) |
| Moderation doesn't affect sync | `WotService` only filters search results | Provider trust records hook into fetch pipeline |
| No retroactive cleanup | Stale leases survive until expiry | After success, invalidate providers that failed definitively |
| `ReplicationResult` omits failures | Only `fetched_from` on success | `failed_from` + `invalidated_leases` fields |

---

## Phase 9A: Fetch Failure Intelligence — Weeks 21-22

### 9.1 Fetch Error Taxonomy (TDD)

**New types** in `syncweb-core/src/indexing/resilience.rs`:

```rust
pub enum FetchFailureKind {
    NotFound,           // provider explicitly lacks the blob
    ConnectionRefused,  // provider unreachable
    Timeout,            // connection timed out
    Corruption,         // data received but hash mismatch
    Unknown,            // uncategorized error
}

pub struct FetchFailure {
    pub kind: FetchFailureKind,
    pub provider: PublicKey,
    pub hash: Hash,
    pub timestamp: u64,
    pub error_detail: String,
}
```

**Unit Tests** (`tests/unit/indexing/resilience_test.rs`):
- [ ] `test_fetch_failure_kind_classification` - errors map to correct kinds
- [ ] `test_fetch_failure_from_syncweb_error` - `SyncwebError::Operation` string parsing
- [ ] `test_fetch_failure_definitive_vs_transient` - `NotFound`/`Corruption` are definitive; others transient
- [ ] `test_fetch_failure_serialization` - round-trip serialize/deserialize
- [ ] `test_fetch_failure_timestamp` - defaults to current epoch
- [ ] `test_fetch_rejects_data_above_expected_size` - oversized provider streams are stopped before hashing/buffering
- [ ] `test_fetch_rejects_truncated_data` - short streams fail without being classified as valid content
- [ ] `test_fetch_memory_usage_is_bounded` - streaming validation does not retain an unbounded provider response

### 9.2 Failure Tracking (TDD)

**New type** in `resilience.rs`:

```rust
pub struct FailureRecord {
    pub failures: Vec<FetchFailure>,
    pub consecutive_failures: u32,
    pub last_failure_at: u64,
    pub first_failure_at: u64,
}
```

**Extend `ProviderLeaseTracker`** with:
- `failures: HashMap<Hash, HashMap<PublicKey, FailureRecord>>`

**New methods on `ProviderLeaseTracker`**:
- `record_failure_at(hash, provider, failure, now)` → records failure, increments consecutive count
- `failure_record(hash, provider)` → returns `Option<&FailureRecord>`
- `failure_count(hash, provider)` → total failure count
- `consecutive_failures(hash, provider)` → consecutive failure count
- `is_definitively_failed(hash, provider)` → consecutive >= threshold (default 3)
- `clear_failures_for_provider(hash, provider)` → reset on success
- `purge_stale_failures(now, ttl)` → aggressively remove old failure records and empty hash/provider buckets

Failure history must be bounded per `(hash, provider)`. Add a configurable
`max_failures_per_provider` cap (default 128); recording a new failure evicts the
oldest detail entries after applying the cap, while preserving the aggregate
counts needed for reputation and ban decisions. Run stale-failure cleanup on the
recording path and at the start of retroactive invalidation.

**Unit Tests** (`tests/unit/indexing/resilience_test.rs`):
- [ ] `test_record_failure_increments_count` - consecutive count goes up
- [ ] `test_record_failure_definitive_threshold` - becomes definitive after N failures
- [ ] `test_clear_failures_on_success` - resets consecutive count to 0
- [ ] `test_purge_stale_failures` - old records removed by TTL
- [ ] `test_failure_per_hash_isolation` - failures for hash A don't affect hash B
- [ ] `test_failure_per_provider_isolation` - failures for provider A don't affect provider B
- [ ] `test_definitive_failure_requires_consecutive` - non-consecutive failures don't trigger definitive
- [ ] `test_transient_failure_not_definitive` - timeout alone is never definitive
- [ ] `test_failure_history_is_capped_per_provider` - oldest records are evicted at the configured cap
- [ ] `test_purge_stale_failures_removes_empty_buckets` - stale cleanup leaves no empty hash/provider entries

### 9.3 ReplicationResult Failure Reporting (TDD)

**Extend `ReplicationResult`** with:
```rust
pub failed_from: Vec<(PublicKey, FetchFailureKind)>,
pub invalidated_leases: Vec<PublicKey>,
```

**Unit Tests** (`tests/unit/indexing/resilience_test.rs`):
- [ ] `test_replication_result_reports_failures` - failed providers listed with kinds
- [ ] `test_replication_result_reports_invalidations` - invalidated providers listed
- [ ] `test_replication_result_no_failures_on_success` - clean success has empty lists

---

## Phase 9B: Smart Ban & Lease Invalidation — Weeks 23-24

### 9.4 Lease Invalidation (TDD)

**New type** in `resilience.rs`:

```rust
pub enum BanSource {
    Manual,
    Automated,
    WoT,
}

pub struct BanRecord {
    pub provider: PublicKey,
    pub hash: Option<Hash>,        // None = global, Some = per-hash
    pub banned_at: u64,
    pub expires_at: Option<u64>,   // None = permanent, Some = temporary
    pub reason: String,
    pub source: BanSource,
}
```

**Extend `ProviderLeaseTracker`** with:
- `bans: HashMap<PublicKey, BanRecord>` (global bans)
- `hash_bans: HashMap<Hash, HashMap<PublicKey, BanRecord>>` (per-hash bans)

**New methods on `ProviderLeaseTracker`**:
- `invalidate_lease(hash, provider)` → removes lease + records automated ban
- `is_banned(provider, hash, now)` → checks global + per-hash bans
- `ban_provider(provider, hash, reason, source, duration, now)` → manual/wot ban
- `unban_provider(provider, hash)` → remove ban
- `banned_providers(hash, now)` → list all bans for a hash (global + per-hash)

**New methods on `ResilienceService`**:
- `invalidate_provider(hash, provider)` → thread-safe wrapper
- `ban_provider(provider, reason, hash, duration)` → manual ban
- `unban_provider(provider, hash)` → manual unban

**Unit Tests** (`tests/unit/indexing/resilience_test.rs`):
- [ ] `test_invalidate_lease_removes_from_tracker` - lease removed, verified count drops
- [ ] `test_invalidate_lease_affects_verified_count` - health reflects invalidation
- [ ] `test_invalidate_lease_does_not_affect_other_hashes` - isolated per hash
- [ ] `test_automated_ban_expires` - temporary ban has TTL
- [ ] `test_permanent_ban_persists` - no expiry means permanent
- [ ] `test_manual_ban_overrides_automated` - manual ban takes precedence
- [ ] `test_unban_provider_restores_lease_eligibility` - unbanned provider can be tracked again
- [ ] `test_banned_provider_excluded_from_selection` - `responsible_providers` skips bans
- [ ] `test_banned_provider_excluded_from_health` - health count excludes bans
- [ ] `test_global_ban_applies_to_all_hashes` - None scope = universal

### 9.5 Smart Ban Integration with ensure_replication (TDD)

**Modify `ensure_replication`** in `ResilienceService`:

On fetch failure:
1. Classify the error into `FetchFailureKind`
2. Call `tracker.record_failure_at(...)`
3. If definitive (`NotFound` or `Corruption`): call `tracker.invalidate_lease(...)` + `bump_generation`
4. Record in `ReplicationResult::failed_from`

On fetch success:
1. Call `tracker.clear_failures_for_provider(hash, provider)`
2. Call `retroactive_invalidate(hash, provider)` (see 9.6)

Before fetch loop:
1. Skip providers where `tracker.is_banned(provider, hash, now)` is true

**Unit Tests** (`tests/unit/indexing/resilience_test.rs`):
- [ ] `test_ensure_replication_invalidates_on_not_found` - lease removed after not-found
- [ ] `test_ensure_replication_does_not_invalidate_on_timeout` - timeout is transient
- [ ] `test_ensure_replication_clears_failures_on_success` - consecutive reset after success
- [ ] `test_ensure_replication_reports_all_failures` - all failed providers in result
- [ ] `test_ensure_replication_skips_banned_providers` - banned provider not attempted
- [ ] `test_ensure_replication_consecutive_failures_trigger_ban` - 3+ failures → definitive
- [ ] `test_ensure_replication_generation_bump_on_invalidated` - pending fetches cancelled
- [ ] `test_ensure_replication_mixed_definitive_and_transient` - only definitive invalidates

### 9.6 Retroactive Invalidation (TDD)

**New method on `ProviderLeaseTracker`**:
- `retroactive_invalidate(hash, successful_provider, now)` → after success from B, look at all definitive failures for this hash and invalidate those providers' leases

**New method on `ResilienceService`**:
- `retroactive_invalidate(hash, successful_provider)` → thread-safe wrapper

**Unit Tests** (`tests/unit/indexing/resilience_test.rs`):
- [ ] `test_retroactive_invalidate_after_success` - providers with definitive failures lose leases
- [ ] `test_retroactive_invalidate_only_definitive` - transient failures not retroactively invalidated
- [ ] `test_retroactive_invalidate_no_failures` - no-op when no failure records exist
- [ ] `test_retroactive_invalidate_preserves_successful_provider` - winner not invalidated
- [ ] `test_retroactive_invalidate_affects_verified_count` - health drops after retroactive cleanup
- [ ] `test_retroactive_invalidate_bumps_generation` - pending fetches for other providers cancelled

---

## Phase 9C: Provider Reputation & WoT Integration — Weeks 25-27

### 9.7 Provider Reputation Scoring (TDD)

**New file** `syncweb-core/src/indexing/reputation.rs`:

```rust
pub struct ProviderReputation {
    pub provider: PublicKey,
    pub total_fetches: u64,
    pub successful_fetches: u64,
    pub failed_fetches: u64,
    pub consecutive_failures: u32,
    pub last_success_at: Option<u64>,
    pub last_failure_at: Option<u64>,
}

pub struct ProviderReputationStore {
    reputations: HashMap<PublicKey, ProviderReputation>,
    config: ReputationConfig,
}

pub struct ReputationConfig {
    pub min_samples: usize,           // min fetches before scoring (default 5)
    pub decay_half_life: Duration,    // score decay over time (default 24h)
    pub failure_weight: f64,          // weight of failures vs successes (default 2.0)
    pub temporary_ban_duration: Duration, // auto-ban after consecutive failures (default 1h)
    pub auto_ban_backoff_factor: f64, // repeated bans for the same key (default 2.0)
    pub max_auto_ban_duration: Duration, // cap repeated-ban backoff
}
```

**Key methods on `ProviderReputation`**:
- `record_success(now)` → increments success, resets consecutive, sets last_success
- `record_failure(kind, now)` → increments failure, increments consecutive, sets last_failure
- `reliability_score(now)` → 0.0–1.0, decays toward 0.5 with time
- `is_reliable(threshold)` → score above threshold
- `should_auto_ban(consecutive_threshold)` → consecutive >= threshold

**Key methods on `ProviderReputationStore`**:
- `record_fetch_result(provider, success, kind, now)` → update reputation
- `reputation(provider)` → get or default
- `score(provider, now)` → get reliability score
- `rank_providers(now, hash)` → sort providers by score
- `should_skip_provider(provider, now, threshold)` → below threshold
- `purge_stale(now, ttl)` → remove old entries

**Unit Tests** (`tests/unit/indexing/reputation_test.rs`):
- [ ] `test_reputation_initial_state` - defaults to 0 fetches, neutral score
- [ ] `test_reputation_score_perfect` - 100% success → score near 1.0
- [ ] `test_reputation_score_zero` - 100% failure → score near 0.0
- [ ] `test_reputation_score_mixed` - 70/30 split → proportional score
- [ ] `test_reputation_score_decays` - old successes pull score toward 0.5
- [ ] `test_reputation_consecutive_failures` - consecutive count tracks streak
- [ ] `test_reputation_success_resets_consecutive` - success breaks failure streak
- [ ] `test_reputation_auto_ban_threshold` - triggers auto-ban at threshold
- [ ] `test_reputation_auto_ban_backoff_increases_on_rejoin` - repeated bans for one public key use exponential durations
- [ ] `test_reputation_auto_ban_backoff_is_capped` - repeated bans never exceed the configured maximum
- [ ] `test_reputation_new_key_is_not_assumed_to_be_same_provider` - WoT remains the mitigation for identity rotation/Sybil keys
- [ ] `test_reputation_min_samples` - low sample count returns neutral score
- [ ] `test_reputation_store_ranking` - providers sorted by score
- [ ] `test_reputation_store_skip_unreliable` - below threshold filtered out
- [ ] `test_reputation_store_purge` - old entries cleaned up

### 9.8 Automated Trust Streams (TDD)

**New type** in `reputation.rs` (or new `trust_stream.rs`):

```rust
pub enum TrustSignalKind {
    ObservedSuccess,
    ObservedFailure,
    ObservedCorruption,
}

pub struct ProviderTrustSignal {
    pub provider: PublicKey,
    pub signal: TrustSignalKind,
    pub hash: Option<Hash>,
    pub reporter: PublicKey,
    pub timestamp: u64,
    pub sequence: u64,
    pub signature: Option<String>,
}
```

Trust signals are published to an iroh-docs namespace (one per reporter or per community). Other nodes subscribe to trust streams from reporters they trust (via the WoT delegation chain from Phase 8.8). Do not publish one gossip record for every fetch result by default: buffer/coalesce observations and publish on a configurable batch interval or when a provider crosses a reputation threshold (for example, Reliable → Unreliable).

**Methods**:
- `ProviderTrustSignal::new(...)` → create unsigned signal
- `ProviderTrustSignal::sign(secret_key)` → sign with reporter key
- `ProviderTrustSignal::verify()` → verify signature + structure
- `ProviderTrustSignal::to_bytes()` / `from_bytes()` → serialization

**Integration with `ProviderReputationStore`**:
- `ingest_trust_signal(signal)` → if reporter is trusted, update provider reputation
- `publish_trust_signal(kind, provider, hash, gossip)` → publish observation to stream

**Unit Tests** (`tests/unit/indexing/reputation_test.rs`):
- [ ] `test_trust_signal_create_and_sign` - round-trip create + sign
- [ ] `test_trust_signal_verify_valid` - valid signature passes
- [ ] `test_trust_signal_verify_invalid` - bad signature fails
- [ ] `test_trust_signal_verify_expired` - old signal rejected
- [ ] `test_trust_signal_serialization` - JSON round-trip
- [ ] `test_trust_signal_monotonic_sequence` - older sequences ignored
- [ ] `test_reputation_store_ingest_trusted_signal` - trusted signal updates score
- [ ] `test_reputation_store_ingest_untrusted_signal` - untrusted signal ignored
- [ ] `test_reputation_store_ingest_delegated_signal` - delegated trust chains work
- [ ] `test_trust_signals_batch_and_coalesce` - repeated observations produce one bounded batch
- [ ] `test_trust_signal_emitted_on_reputation_transition` - threshold crossing emits a signal
- [ ] `test_trust_signal_not_emitted_for_ordinary_success` - steady-state successes do not flood gossip

### 9.9 Manual Provider Trust Records (TDD)

**New type** in `wot.rs` (extending existing WoT module):

```rust
pub enum ProviderTrustAction {
    Trust,      // vouch for provider reliability
    Distrust,   // mark provider as unreliable
    Vouch,      // one-time endorsement (lighter than Trust)
    Warn,       // caution, unverified reports
}

pub struct ProviderTrustRecord {
    pub provider: PublicKey,
    pub action: ProviderTrustAction,
    pub scope: Option<Hash>,        // None = global, Some = per-content
    pub issuer: String,             // trusted author identity
    pub sequence: u64,
    pub issued_at: u64,
    pub expires_at: Option<u64>,
    pub reason: String,
    pub signature: Option<String>,
}
```

**Extend `WotService`** with:
- `records: HashMap<PublicKey, Vec<ProviderTrustRecord>>`
- `apply_provider_trust(record)` → verify + store if issuer trusted
- `evaluate_provider_trust(provider, hash, now)` → aggregate all records into decision
- `provider_trust_records(provider)` → list all records

**Extend `TrustDecision`** (or add new enum):
```rust
pub enum ProviderTrustDecision {
    Trusted,
    Distrusted,
    Unknown,
    Conflicting,  // mixed trust/distrust records
}
```

**Unit Tests** (`tests/unit/indexing/wot_test.rs`):
- [ ] `test_provider_trust_record_create` - signed record created
- [ ] `test_provider_trust_record_verify` - signature valid
- [ ] `test_provider_trust_record_scope` - per-hash scope limits evaluation
- [ ] `test_provider_trust_record_expires` - expired records ignored
- [ ] `test_provider_trust_evaluate_trusted` - single Trust record → Trusted
- [ ] `test_provider_trust_evaluate_distrusted` - single Distrust record → Distrusted
- [ ] `test_provider_trust_evaluate_conflicting` - mixed records → Conflicting
- [ ] `test_provider_trust_evaluate_untrusted_issuer` - untrusted issuer ignored
- [ ] `test_provider_trust_evaluate_delegation` - delegated trust chains work
- [ ] `test_provider_trust_evaluate_sequence` - newer record supersedes older
- [ ] `test_provider_trust_self_revocation` - provider can distrust themselves

### 9.10 WoT × Reputation Integration (TDD)

**Extend `ResilienceService`** to accept a `WotService` reference (optional):

When evaluating providers for `ensure_replication`:
1. Get providers from leases (existing)
2. Filter out globally banned providers (Phase 9B)
3. Filter out providers that `evaluate_provider_trust()` returns `Distrusted` for
4. Rank remaining providers by `ProviderReputationStore::score()`
5. Select top-N by consistent hashing (existing) but weighted by reputation

**Unit Tests** (`tests/unit/indexing/resilience_test.rs`):
- [ ] `test_resilience_respects_provider_trust_distrust` - distrusted provider skipped
- [ ] `test_resilience_respects_provider_trust_trust` - trusted provider prioritized
- [ ] `test_resilience_reputation_weighted_selection` - higher-scored providers tried first
- [ ] `test_resilience_no_wot_service` - graceful degradation without WoT
- [ ] `test_resilience_combined_ban_and_trust` - ban + distrust both applied
- [ ] `test_resilience_reputation_on_fetch_success` - score improves after success
- [ ] `test_resilience_reputation_on_fetch_failure` - score drops after failure

---

## Phase 9D: CLI & Integration — Week 28

### 9.11 CLI Commands (TDD)

**Unit Tests** (`tests/unit/indexing/cli_test.rs`):
- [ ] `test_trust_provider_show` - `syncweb trust provider show <pubkey>`
- [ ] `test_trust_provider_list` - `syncweb trust provider list`
- [ ] `test_trust_provider_ban` - `syncweb trust provider ban <pubkey> [--hash <hash>] [--reason <reason>]`
- [ ] `test_trust_provider_unban` - `syncweb trust provider unban <pubkey>`
- [ ] `test_trust_provider_vouch` - `syncweb trust provider vouch <pubkey> [--scope <hash>]`
- [ ] `test_trust_provider_distrust` - `syncweb trust provider distrust <pubkey>`
- [ ] `test_trust_stream_subscribe` - `syncweb trust stream subscribe <ticket>`
- [ ] `test_trust_stream_publish` - `syncweb trust stream publish --provider <pubkey> --signal <kind>`

### 9.12 Integration Tests (TDD)

**Integration Tests** (`tests/integration/smart_ban_test.rs`):
- [ ] `test_full_smart_ban_workflow` - Lease → fetch fail → invalidation → health reflects
- [ ] `test_retroactive_invalidation_workflow` - Multiple fails → one success → cleanup
- [ ] `test_provider_reputation_across_fetches` - Score tracks history over time
- [ ] `test_trust_stream_aggregation` - Signal publish → subscribe → reputation update
- [ ] `test_manual_provider_trust_workflow` - Vouch → distrust → evaluate
- [ ] `test_ban_and_trust_interplay` - Ban overrides trust, unban restores
- [ ] `test_consecutive_failure_auto_ban` - 3 failures → auto-ban → timeout → unban
- [ ] `test_wot_delegation_provider_trust` - Delegated trust affects provider evaluation
- [ ] `test_full_replication_with_smart_ban` - End-to-end: lease → fail → ban → retry other → success

### 9.13 Resilience Hardening Integration Tests

These tests are required before the Phase 9 gates are considered complete:
- [ ] `test_oversized_corruption_response_is_bounded` - a malicious provider cannot exhaust memory or CPU with an infinite/random byte stream
- [ ] `test_failure_record_retention_under_sustained_failures` - repeated failures remain within the per-provider cap
- [ ] `test_rejoined_provider_receives_exponential_ban` - repeated bans for one public key back off instead of using a flat duration
- [ ] `test_trust_gossip_is_batched` - high-volume fetch outcomes do not create one network message per outcome

---

## Phase 9E: Performance Benchmarks (Continuous)

Add to `benches/` with `criterion`:
- [ ] `bench_failure_tracking_1000_hashes` - Target < 10ms
- [ ] `bench_reputation_score_calculation` - Target < 1μs per provider
- [ ] `bench_provider_ban_lookup` - Target < 100μs
- [ ] `bench_retroactive_invalidation_100_providers` - Target < 5ms
- [ ] `bench_trust_signal_verification` - Target < 1ms
- [ ] `bench_reputation_ranking_1000_providers` - Target < 5ms

Run with: `cargo bench --all-features`

---

## New Files

| File | Purpose |
|------|---------|
| `syncweb-core/src/indexing/reputation.rs` | `ProviderReputation`, `ProviderReputationStore`, `ProviderTrustSignal` |
| `tests/unit/indexing/reputation_test.rs` | Unit tests for reputation scoring and trust signals |
| `tests/integration/smart_ban_test.rs` | Integration tests for end-to-end smart ban workflows |

## Modified Files

| File | Changes |
|------|---------|
| `syncweb-core/src/indexing/resilience.rs` | `FetchFailureKind`, `FetchFailure`, `FailureRecord`, `BanRecord`, `BanSource`, failure tracking + ban tracking on `ProviderLeaseTracker`, smart ban integration in `ensure_replication`, retroactive invalidation |
| `syncweb-core/src/indexing/wot.rs` | `ProviderTrustAction`, `ProviderTrustRecord`, `ProviderTrustDecision`, provider trust evaluation in `WotService` |
| `syncweb-core/src/indexing.rs` | Re-export new types from `reputation` module |
| `syncweb-core/src/error.rs` | (Optional) `FetchFailure` variant if string-parsing approach is insufficient |

---

## Phase Gates for Phase 9

| Sub-Phase | Gate |
|-----------|------|
| 9.1-9.3 | `FetchFailureKind` classifies errors, bounded fetch validation prevents tarpit streams, failure tracking records/counts/clears, `ReplicationResult` reports failures |
| 9.4-9.6 | Lease invalidation removes leases, smart ban integration in `ensure_replication`, retroactive cleanup works |
| 9.7-9.10 | Reputation scoring tracks history with rejoin backoff, trust signals batch/threshold-gossip, provider trust records evaluate correctly |
| 9.11-9.12 | All CLI commands work, integration tests pass, CI green |

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
# No new external dependencies required
# All Phase 9 features build on existing deps:
# - ed25519-dalek (already present)
# - blake3 (already present)
# - serde/serde_json (already present)
# - iroh-docs (already present, for trust stream namespaces)
# - tokio (already present)
```

Phase 9 is intentionally dependency-free. It extends existing types and modules without introducing new crates.

---

## Design Decisions

### Why separate `FetchFailureKind` from `SyncwebError`?

The current `SyncwebError::Operation` swallows error context into a display string. Rather than modifying the error enum (which would require changes across the entire codebase), `FetchFailureKind` classifies errors at the resilience layer boundary — where they matter for lease decisions. The string-parsing approach (`contains("not found")`, etc.) is pragmatic for v1; a future phase could add structured error variants to `SyncwebError` if finer-grained classification is needed.

### Why consecutive failures for definitive banning?

A single timeout could be transient network issues. Three consecutive `NotFound` or `Corruption` failures are strong evidence of data loss. The threshold is configurable via `ReputationConfig::consecutive_failures_threshold`.

### Why retroactive invalidation?

The smart ban insight: you don't ban on first failure — you keep trying, succeed, then look back. This avoids false positives from transient network issues. Only after a successful fetch proves the data is obtainable do we retroactively invalidate providers that definitively failed.

### Why both automated and manual trust streams?

Automated streams (fetch success/failure observations) are objective but limited — they only reflect the observer's direct experience. Manual trust records (vouches, distrust) capture community knowledge — "this provider is known to be unreliable" — that no single node may have observed yet. Both feed into the same reputation store.

### Why provider bans are separate from content denylists?

Phase 8.10 denylists block *content* (hashes, files, devices). Phase 9 bans block *providers* (who serve content). A provider may serve good content for hash A but bad content for hash B. Per-hash bans capture this; global bans are for providers that are universally unreliable. The two systems compose: a provider can be banned while the content denylist separately blocks unwanted content.

---

*This plan follows strict TDD: no implementation code before a failing test exists. Each module follows Red-Green-Refactor cycle with clippy/fmt gates.*
