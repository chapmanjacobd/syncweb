# TDD Plan: `sort` command

## Divergence
Name implies generic sorting. Actual: "Sort local files by discovery criteria" — but **niche, frecency, and peers criteria always score 0** because the CLI never populates those fields from network data. Criterias like `time`, `date`, `size` work fine since they use filesystem metadata.

## Current scope
- Pure local filesystem scan + in-memory sort
- No gossip channels, no network queries
- `niche`, `frequency`, `peers` fields are always `0.0`, `0`, `0`

## Proposed scope
1. Keep local-only filesystem sorting as-is (it works for `time`, `date`, `size`, etc.)
2. Add `--enrich` flag that queries the indexing/resilience layer for real peer counts, access frequencies, and niche scores
3. When `--enrich` is used with `--by niche|frecency|peers`, populate entries from `ProviderLeaseTracker` and local observation data

---

## Tests

### Phase 1 — Verify current behavior is correct (no regressions)

```rust
// syncweb-core/tests/sort_test.rs — already exists, verify:
// test_niche_sort, test_frecency_sort, test_peers_sort, test_random_sort, test_folder_aggregate
```

```rust
// syncweb-cli/tests/cli_test.rs — already exists, verify:
// test_sort_algorithms (line 400) — runs each --by mode, expects 2 files listed
```

### Phase 2 — New unit tests for enrichment

```rust
// syncweb-core/tests/sort_test.rs

#[test]
fn test_sort_with_peer_enrichment() {
    // Given entries with no peers set
    let mut entries = vec![
        SortEntry::new("a.txt").with_folder("f").with_size(100),
        SortEntry::new("b.txt").with_folder("f").with_size(200),
    ];
    // When enriched with peer data
    let enrichment = [("a.txt", 5), ("b.txt", 1)].iter().cloned().collect();
    Sorter::new(SortConfig::default()).enrich_peers(&mut entries, &enrichment);
    // Then peers should be populated
    assert_eq!(entries[0].peers, 5);
    assert_eq!(entries[1].peers, 1);
}

#[test]
fn test_sort_by_peers_with_enrichment() {
    let mut entries = vec![
        SortEntry::new("a.txt").with_folder("f"),
        SortEntry::new("b.txt").with_folder("f"),
    ];
    let enrichment = [("a.txt", 1), ("b.txt", 10)].iter().cloned().collect();
    let mut config = SortConfig::default();
    config.criteria = vec![(SortCriterion::Peers, true)];
    let sorter = Sorter::new(config);
    sorter.enrich_peers(&mut entries, &enrichment);
    sorter.sort(&mut entries);
    assert_eq!(entries[0].path, std::path::PathBuf::from("b.txt"));
    assert_eq!(entries[1].path, std::path::PathBuf::from("a.txt"));
}

#[test]
fn test_sort_by_niche_with_enrichment() {
    let mut entries = vec![
        SortEntry::new("popular.txt").with_folder("f"),
        SortEntry::new("rare.txt").with_folder("f"),
    ];
    // Niche = f64 from peer count: lower peers → higher niche
    let enrichment = [("popular.txt", 100), ("rare.txt", 2)].iter().cloned().collect();
    let mut config = SortConfig::default();
    config.criteria = vec![(SortCriterion::Niche, true)];
    config.niche = 3; // ideal peer count for "niche"
    let sorter = Sorter::new(config);
    sorter.enrich_peers(&mut entries, &enrichment);
    sorter.enrich_niche(&mut entries);
    sorter.sort(&mut entries);
    assert_eq!(entries[0].path, std::path::PathBuf::from("rare.txt"));
    assert_eq!(entries[1].path, std::path::PathBuf::from("popular.txt"));
}
```

### Phase 3 — CLI integration test

```rust
// syncweb-cli/tests/cli_test.rs or full_suite_test.rs

#[test]
fn test_sort_with_enrich_flag() -> anyhow::Result<()> {
    let source = cli_test_dir("sort-enrich");
    fs::write(source.join("a.txt"), b"content_a")?;
    fs::write(source.join("b.txt"), b"content_b")?;

    // --enrich with no daemon should warn and fall back to unsorted local
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["sort", source.to_str().unwrap(), "--by", "peers", "--enrich"])
        .output()?;
    // Should still succeed but output files (without real peer data)
    assert!(output.status.success());

    fs::remove_dir_all(&source)?;
    Ok(())
}
```

### Phase 4 — Integration test with running daemon + indexing

```rust
// syncweb-cli/tests/daemon_integration_test.rs (sketch)
// 1. Start daemon with indexing enabled
// 2. Create folder, add files
// 3. Run `syncweb sort --by peers --enrich` via IPC
// 4. Verify output includes peer-enriched sorting
```

---

## Implementation

### `syncweb-core/src/sort.rs`

1. Add `enrich_peers(&mut self, entries: &mut [SortEntry], peer_map: &HashMap<String, usize>)` — populates `entry.peers` from a path→count map
2. Add `enrich_niche(&mut self, entries: &mut [SortEntry])` — computes `entry.niche` from `entry.peers` using `config.niche` as ideal count (formula: `1.0 / (1.0 + |peers - ideal| as f64)`)
3. Add `enrich_frequency(&mut self, entries: &mut [SortEntry], freq_map: &HashMap<String, u64>)` — populates `entry.frequency` from access logs

### `syncweb-cli/src/cli/commands.rs`

1. Add `--enrich` flag to `SortArgs`

### `syncweb-cli/src/main.rs`

1. In `handle_sort`, when `--enrich` is set:
   - If daemon is available, send IPC to fetch peer/frequency data
   - If no daemon, log warning and skip enrichment
   - Call `sorter.enrich_peers()` and/or `sorter.enrich_niche()` before `sorter.sort()`

### `syncweb-core/src/daemon/ipc.rs`

1. Add `IpcCommand::EnrichSort { path: PathBuf }` variant
2. Handler queries `ProviderLeaseTracker` for peer counts on each blob in the folder
3. Returns `HashMap<String, usize>` (relative path → peer count)

---

## Gossip/network integration note

Currently the `ProviderLeaseTracker` learns about providers via:
- Gossiped `ProviderLease` messages (signed leases from peers)
- Local observations from successful fetches

The `sort --enrich` command will use whatever data the tracker already has. To get **live** peer counts, the tracker must be subscribed to gossip for the relevant folders. This is already happening when the daemon runs with indexing enabled. No new gossip channels are needed — just wiring the existing data to the sort command.

## Files to modify

| File | Changes |
|------|---------|
| `syncweb-core/src/sort.rs` | Add `enrich_peers()`, `enrich_niche()`, `enrich_frequency()` methods |
| `syncweb-core/src/sort.rs` | Add `SortConfig::enrich` field |
| `syncweb-cli/src/cli/commands.rs` | Add `--enrich` flag to `SortArgs` |
| `syncweb-cli/src/main.rs` | Enrichment logic in `handle_sort` |
| `syncweb-core/src/daemon/ipc.rs` | `IpcCommand::EnrichSort` + handler |
| `syncweb-core/tests/sort_test.rs` | Tests for enrichment methods |
| `syncweb-cli/tests/cli_test.rs` | Test `--enrich` flag |
