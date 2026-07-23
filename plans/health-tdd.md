# TDD Plan: `health` command

## Divergence
Name "health" implies general system/network health. Actual: "Show seeding status per folder blob" — but **peer_count is hardcoded to 0**, making the report always show "all blobs unseeded". The concept is correct; the data pipeline is broken.

## Current scope
- Enumerates blobs in a synchronized folder
- Classifies them as well-seeded (>=4), under-seeded (1-3), unseeded (0)
- **Bug:** `peer_count` is always `0` in both daemon and direct paths — `FetchCandidate::new(path, hash, size, 0, local)` is called with literal `0`
- No live peer-count tracking integrated
- No gossip channels queried

## Decision
- **Keep** the name `health` (it accurately describes the intent)
- **Fix** the `peer_count = 0` bug by wiring to `ProviderLeaseTracker` for real peer counts
- **Connect** to the indexing/resilience layer to get live seeding data
- Add a separate `network health` subcommand for general connectivity checks

---

## Tests

### Phase 1 — Reproduce the zero-count bug

```rust
// syncweb-core/tests/partial_fetch_test.rs

#[test]
fn test_health_report_peer_count_not_zero() {
    // When candidates have explicit peer counts, report must use them
    let candidates = vec![
        FetchCandidate::new("a", Hash::from_bytes([1u8; 32]), 100, 3, true),
        FetchCandidate::new("b", Hash::from_bytes([2u8; 32]), 200, 0, true),
    ];
    let report = HealthReport::from_candidates(&candidates, 4);
    assert_eq!(report.under_seeded, 1);
    assert_eq!(report.unseeded, 1);
    assert_eq!(report.well_seeded, 0);
    // "a" has 3 peers → under-seeded, not unseeded
    assert!(report.least_seeded[0].peer_count == 0); // "b" is least seeded
}
```

### Phase 2 — `peer_count` population from `ProviderLeaseTracker`

```rust
// syncweb-core/tests/health_live_test.rs  (new file)

use syncweb_core::indexing::{resilience::{ResilienceService, ResilienceConfig, ReplicationBudget}, ProviderLease};

#[tokio::test]
async fn test_health_populates_peer_count_from_leases() -> anyhow::Result<()> {
    let indexing = IndexingService::in_memory()?;
    let resilience = indexing.resilience_service(
        ResilienceConfig::new(ReplicationBudget::default())
    );

    let hash = Hash::from_bytes([1u8; 32]);

    // Register 3 provider leases for the same hash
    for i in 0..3 {
        let mut lease = ProviderLease::new_with_times(hash, format!("ticket_{i}"), i, 0, u64::MAX)?;
        lease.sign_with_secret_key(&secret_key)?;
        resilience.record_lease(lease)?;
    }

    // Now query health — should see 3 verified providers
    let health = resilience.health(&hash)?;
    assert_eq!(health.verified, 3);
    assert_eq!(health.verified_providers.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_health_shows_zero_when_no_leases() -> anyhow::Result<()> {
    let indexing = IndexingService::in_memory()?;
    let resilience = indexing.resilience_service(...);
    let hash = Hash::from_bytes([2u8; 32]);
    let health = resilience.health(&hash)?;
    assert_eq!(health.verified, 0);

    // But local if we have the blob
    // (would need a blob store check — Future: add local flag to health)
    Ok(())
}
```

### Phase 3 — Folder health with real peer counts (daemon path)

```rust
// syncweb-cli/tests/daemon_integration_test.rs

#[tokio::test]
async fn test_daemon_health_returns_real_peer_counts() -> anyhow::Result<()> {
    // 1. Start two daemons: publisher + subscriber
    // 2. Publisher creates folder, adds a blob, shares ticket
    // 3. Subscriber joins folder
    // 4. Topics sync → provider leases are gossiped
    // 5. Run `syncweb health <path>` on subscriber
    // 6. Assert: peer_count > 0 for the blob (subscriber knows publisher is seeding)

    let (relay, relay_url, _server) = iroh::test_utils::run_relay_server().await?;

    let dir_a = cli_test_dir("health-publisher");
    let dir_b = cli_test_dir("health-subscriber");

    // ... start daemon A, create folder, add file, get ticket ...
    // ... start daemon B, join folder, wait for sync ...

    // Run health on subscriber
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", dir_b.to_str().unwrap(),
               "health", folder_path.to_str().unwrap()])
        .output()?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;

    // At minimum, not all blobs should be unseeded
    assert!(!stdout.contains("Unseeded: 1"));
    // "Unseeded: 0" indicates the publisher was detected
    assert!(stdout.contains("Unseeded: 0"));

    fs::remove_dir_all(&dir_a)?;
    fs::remove_dir_all(&dir_b)?;
    Ok(())
}
```

### Phase 4 — CLI health with `--json` shows real peer counts

```rust
// syncweb-cli/tests/cli_test.rs

#[test]
fn test_health_json_includes_peer_counts() -> anyhow::Result<()> {
    // Uses --embedded node with an injected lease for testing
    let dir = cli_test_dir("health-json");
    // ... setup node with known lease ...

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", dir.to_str().unwrap(),
               "--json", "--embedded",
               "health", some_folder.to_str().unwrap()])
        .output()?;
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    // peer_count should be > 0 for entries with known providers
    for blob in value["least_seeded"].as_array().unwrap() {
        // At minimum, some blobs must have peer_count > 0 if providers exist
    }
    fs::remove_dir_all(&dir)?;
    Ok(())
}
```

### Phase 5 — Health after `mirror` (replication) improves seeding

```rust
// syncweb-core/tests/mirror_test.rs

#[tokio::test]
async fn test_health_improves_after_replication() -> anyhow::Result<()> {
    // 1. Blob has 0 providers → health shows unseeded
    // 2. Run mirror/replicate from a provider
    // 3. Health now shows well-seeded or under-seeded
    Ok(())
}
```

---

## Implementation

### `syncweb-core/src/daemon/ipc.rs` — Fix `handle_health_check`

**Current (broken):**
```rust
candidates.push(FetchCandidate::new(path_str, hash, size, 0, local));
//                                                   ^ hardcoded 0
```

**Fixed:**
```rust
let provider_count = match context.indexing {
    Some(ref indexing) => {
        let resilience = indexing.resilience_service(...);
        let health = resilience.health(&entry.content_hash())?;
        health.verified
    }
    None => 0,
};
candidates.push(FetchCandidate::new(path_str, hash, size, provider_count, local));
```

### `syncweb-cli/src/main.rs` — Fix `handle_health` direct path

**Current:**
```rust
candidates.push(FetchCandidate::new(path, hash, size, 0, local));
//                                                ^ hardcoded 0
```

**Fixed:** Same pattern — query `ResilienceService::health()` when indexing is available.

### `syncweb-core/src/indexing/resilience.rs` — Add `ResilienceService::health_batch()`

For efficiency when health-checking an entire folder (many hashes):

```rust
/// Query health for many hashes at once.
pub fn health_batch(&self, hashes: &[Hash]) -> Result<HashMap<Hash, AvailabilityHealth>> {
    let tracker = self.tracker.lock()?;
    let ttl = self.config.budget.observation_ttl;
    Ok(hashes.iter().map(|hash| (*hash, tracker.health(hash, ttl))).collect())
}
```

### `syncweb-core/src/sync/partial_fetch.rs` — Update `HealthReport`

Add a method that integrates with `AvailabilityHealth`:

```rust
impl HealthReport {
    pub fn from_candidates_with_health(
        candidates: &[FetchCandidate],
        health_map: &HashMap<Hash, AvailabilityHealth>,
        well_seeded_threshold: usize,
    ) -> Self {
        // Override each candidate's peer_count from health_map
        let enriched: Vec<FetchCandidate> = candidates.iter().map(|c| {
            let peers = health_map
                .get(&c.hash)
                .map(|h| h.verified)
                .unwrap_or(c.peer_count);
            FetchCandidate::new(&c.path, c.hash, c.size, peers, c.local)
        }).collect();
        Self::from_candidates(&enriched, well_seeded_threshold)
    }
}
```

### `syncweb-cli/src/cli/commands.rs` — No changes needed

The `HealthArgs` struct is fine. (Optionally add `--min-providers` flag to customize threshold.)

---

## Gossip/network integration note

- `ProviderLeaseTracker` already receives provider information via gossip (signed `ProviderLease` messages)
- The health command doesn't need new gossip channels — it reads from the `ProviderLeaseTracker` that gossip already populates
- **Prerequisite:** The daemon must have indexing enabled for the tracked folder (which enables `GossipService` subscription to provider lease topics)
- If indexing is not enabled, `health` falls back to local-only (peer_count = 0, as today)

## Files to modify/plan

| File | Changes |
|------|---------|
| `syncweb-core/src/daemon/ipc.rs` | Fix `handle_health_check` to query `resilience.health()` |
| `syncweb-cli/src/main.rs` | Fix `handle_health` direct path to query resilience |
| `syncweb-core/src/sync/partial_fetch.rs` | Add `from_candidates_with_health` constructor |
| `syncweb-core/src/indexing/resilience.rs` | Add `health_batch()` for bulk queries |
| `syncweb-core/tests/health_live_test.rs` | New file — integration test for peer count population |
| `syncweb-cli/tests/daemon_integration_test.rs` | Test daemon health returns real peer counts |
| `syncweb-cli/tests/cli_test.rs` | Test `--json` health with peer counts |
