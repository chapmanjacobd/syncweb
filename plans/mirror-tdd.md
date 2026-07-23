# TDD Plan: `mirror` command (provider/network replication)

## Rationale

The old `mirror` command only handled single-blob replication. That was merged into `download` (`syncweb download <ticket>`). A new `mirror` command should operate at a higher level: replicate **all blobs** owned by a provider, or all blobs in a network.

## Scope

- Discover all hashes advertised by a given provider (via gossip or direct query)
- Fetch and pin each blob locally
- Track replication progress and health per-provider
- `--network` flag to mirror all blobs from all providers in a network

## CLI sketch

```
syncweb mirror <provider-id>                   # mirror all blobs from a single provider
syncweb mirror --network <network-id>           # mirror all blobs across an entire network
syncweb mirror <provider-id> --network <net>    # provider-scoped within a network
```

### Flags

| Flag | Description |
|------|-------------|
| `--network` | Network or namespace scope to mirror within |
| `--min-providers` | Replication budget per blob (default 3) |
| `--no-sharing` / `--no-seeding` | Skip lease announcements |
| `--dry-run` | Report what would be mirrored without fetching |

## Tests

### Phase 1 — Provider blob discovery

```rust
// syncweb-core/tests/mirror_discovery_test.rs

#[test]
fn test_discover_provider_blobs_returns_all_hashes() {
    // Given: a provider with N advertised blobs
    // When: discover_provider_blobs(provider_id) is called
    // Then: all N hashes are returned
}

#[test]
fn test_discover_provider_blobs_empty_when_provider_unknown() {
    // Given: no known provider
    // When: discover_provider_blobs(unknown_id) is called
    // Then: empty list returned, no error
}
```

### Phase 2 — Single-provider mirror

```rust
#[test]
fn test_mirror_provider_fetches_and_pins_all_blobs() {
    // Given: a provider with blobs A, B, C
    // When: mirror_provider(provider_id) completes
    // Then: A, B, C are all pinned locally
    // And: provider lease is recorded for each
}

#[test]
fn test_mirror_provider_dry_run_lists_blobs_without_fetch() {
    // Given: a provider with blobs A, B, C
    // When: mirror_provider(provider_id, dry_run=true)
    // Then: output lists A, B, C
    // And: no blobs are fetched or pinned
}

#[test]
fn test_mirror_provider_no_sharing_skips_leases() {
    // Given: a provider with blobs
    // When: mirror_provider(provider_id, no_sharing=true)
    // Then: blobs are fetched and pinned
    // And: no leases are recorded or gossiped
}
```

### Phase 3 — Network-wide mirror

```rust
#[test]
fn test_mirror_network_discovers_all_providers() {
    // Given: a network with providers P1, P2, P3
    // When: mirror_network(network_id) is called
    // Then: blobs from P1, P2, P3 are all discovered
}

#[test]
fn test_mirror_network_dry_run_reports_per_provider() {
    // Given: a network with providers P1 (blobs A,B) and P2 (blob C)
    // When: mirror_network(network_id, dry_run=true)
    // Then: output groups by provider:
    //   P1: A, B
    //   P2: C
}
```

### Phase 4 — Progress and resumption

```rust
#[test]
fn test_mirror_reports_progress_per_blob() {
    // Given: 10 blobs to mirror
    // When: mirror process runs
    // Then: progress events are emitted per-blob (started/fetched/pinned/failed)
}

#[test]
fn test_mirror_resumes_from_checkpoint() {
    // Given: 10 blobs, first 4 already pinned
    // When: mirror starts again
    // Then: only blobs 5-10 are fetched
}
```

### Phase 5 — CLI integration

```rust
// syncweb-cli/tests/cli_mirror_test.rs

#[test]
fn test_cli_mirror_provider_subcommand_exists() {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["mirror", "--help"])
        .output()?;
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout)?;
    assert!(help.contains("provider"));
    assert!(help.contains("--network"));
}

#[test]
fn test_cli_mirror_missing_args_fails_gracefully() {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["mirror"])
        .output()?;
    assert!(!output.status.success());
}
```

## Implementation notes

- Provider discovery depends on gossip infrastructure — `ProviderLeaseTracker` may need a `list_providers()` or `blobs_for_provider()` query
- Network discovery depends on the doc/namespace sync layer — enumerate providers via document members
- Progress can reuse `indicatif::ProgressBar` (similar to `handle_download`)
- Checkpoint state lives in a JSON file (e.g. `mirror-state.json` alongside `indexing-state.json`)
- Each blob uses the existing `ensure_replication` or `download_blob` path under the hood
