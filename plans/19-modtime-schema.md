# TDD Plan: Expose entry insertion time in FileStats and other commands

## Rationale

Every entry in iroh-docs already carries a `timestamp()` — microseconds since Unix epoch, set automatically when the entry is created or updated. This field is already stored and synchronized across all peers. Today `sywcweb` ignores it entirely — there are zero callers of `entry.timestamp()` in the codebase.

Surfacing this field in `FileStats` and other commands gives users insight into:
- How recently files were added/modified by any peer (not just locally)
- Age distribution of the entire synchronized tree
- Staleness detection — which files haven't been touched in months

## Approach

No new storage. No schema migration. No companion entries. Just read `entry.timestamp()` where entries are already enumerated.

- `FileStats` gains a `--by time` mode showing insertion-time distribution buckets
- `snapshot` listing gains an optional `--show-time` column
- Future consumers (`sort --by insertion`, `filter --older-than`, etc.) can use the same field

## Scope

| Command | Current | Proposed |
|---------|---------|-----------|
| `filestats` | `--by extension\|size\|all` | + `--by time` |
| `snapshot` / `list` | shows path, hash, size | + optional `inserted` column |
| `sort` | `--by time` reads filesystem mtime | future: `--by insertion` reads doc timestamp |

---

## Tests

### Phase 1 — Unit: insertion time on `Entry`

```rust
// syncweb-core/tests/insertion_time_test.rs

#[test]
fn test_entry_timestamp_is_microseconds() {
    // Given: an entry in a test doc created via set_hash
    // When: reading entry.timestamp()
    // Then: value is > 1_700_000_000_000_000 (microseconds since epoch, year >= 2023)
}

#[test]
fn test_entry_timestamp_advances_on_update() {
    // Given: two set_hash calls on the same key, 1 second apart
    let t1 = entry1.timestamp();
    let t2 = entry2.timestamp();
    assert!(t2 > t1);
}
```

### Phase 2 — Unit: FileStats time buckets

```rust
// syncweb-core/tests/stats_test.rs (extend existing)

#[test]
fn test_filestats_time_distribution() {
    let mut collector = FileStatsCollector::new();
    let now = std::time::SystemTime::now();
    let ts = |offset_secs: i64| -> u64 {
        let t = now + std::time::Duration::from_secs(offset_secs.max(0) as u64);
        // convert to micros
        t.duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64
    };
    collector.add_entry_with_time("recent.txt", 100, Some(ts(0)));
    collector.add_entry_with_time("old.txt", 200,
        Some(ts(-365 * 24 * 3600)));
    collector.add_entry_with_time("no_time.txt", 50, None);
    let report = collector.report();
    assert_eq!(report.time_buckets.get("<24h").copied().unwrap_or(0), 1);
    assert_eq!(report.time_buckets.get(">1y").copied().unwrap_or(0), 1);
    assert_eq!(report.time_buckets.get("unknown").copied().unwrap_or(0), 1);
}
```

### Phase 3 — CLI integration

```rust
// syncweb-cli/tests/cli_filestats_test.rs

#[test]
fn test_filestats_by_time() -> anyhow::Result<()> {
    // 1. Import files into a managed folder
    // 2. Run `syncweb filestats <folder> --by time`
    // 3. Verify output contains time-distribution buckets
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["filestats", &folder.to_string_lossy(), "--by", "time"])
        .output()?;
    assert!(output.status.success());
}

#[test]
fn test_filestats_by_all_includes_time() {
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["filestats", &folder.to_string_lossy(), "--by", "all"])
        .output()?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("insertion time") || stdout.contains("time distribution"));
}
```

---

## Implementation

### 1. `FileStatsCollector` — add time tracking (`syncweb-core/src/stats.rs`)

Add `timestamps: Vec<Option<u64>>` to the collector (microseconds since epoch, None for unknown).

```rust
pub fn add_entry_with_time(&mut self, path: &str, size: u64, timestamp_micros: Option<u64>) {
    self.add_entry(path, size);
    self.timestamps.push(timestamp_micros);
}
```

Add `time_buckets: BTreeMap<String, u64>` to `FileStatsReport`. Bucket labels:

| Label     | Range (age from now) |
|-----------|----------------------|
| `<24h`    | 0--24h                |
| `1d-7d`   | 24h--7d               |
| `7d-30d`  | 7d--30d               |
| `1m-1y`   | 30d--1y               |
| `>1y`     | older than 1y        |
| `unknown` | no timestamp         |

Populate in `report()` by computing `now_micros - timestamp` for each entry and binning.

### 2. CLI: `FileStatsArgs` — add `--by time` (`syncweb-cli/src/cli/commands.rs`)

```rust
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum StatsBy {
    Extension,
    Size,
    All,
    Time,  // NEW
}
```

### 3. CLI: `handle_filestats` — feed timestamp into collector (`syncweb-cli/src/main.rs`)

In the existing loop over `docs_engine().list_latest()` results, pass `entry.timestamp()` through to `add_entry_with_time()`.

```rust
// Current (simplified):
collector.add_entry_bytes(entry.key(), entry.content_len());

// New:
collector.add_entry_with_time(entry.key(), entry.content_len(), Some(entry.timestamp()));
```

### 4. CLI: display time buckets (`syncweb-cli/src/main.rs`)

When `--by time` (or `--by all`), print the `time_buckets` map with human-readable labels:

```
insertion time distribution:
  <24h       1,234 files
  1d-7d        567 files
  7d-30d       890 files
  1m-1y      3,210 files
  >1y        5,432 files
  unknown        0 files
```

### 5. (Optional) `snapshot` — add `--show-time` flag

Add a `--show-time` flag to the existing `snapshot` / `list` output format that appends an ISO-8601 formatted insertion timestamp column.

---

## Files to modify

| File | Changes |
|------|---------|
| `syncweb-core/src/stats.rs` | Add `timestamps` vec, `add_entry_with_time()`, `time_buckets` to report |
| `syncweb-cli/src/cli/commands.rs` | Add `Time` variant to `StatsBy` enum |
| `syncweb-cli/src/main.rs` | Feed `entry.timestamp()` into collector; display time buckets |
| `syncweb-core/tests/stats_test.rs` | Add time distribution tests |
| `syncweb-cli/tests/cli_filestats_test.rs` | Add `--by time` CLI test |

## Notes

- `entry.timestamp()` is `u64` (microseconds), while `SystemTime` uses seconds/nanos. Convert via `Duration::from_micros(timestamp)` and `SystemTime::UNIX_EPOCH + duration` when human-readable formatting is needed.
- This is insertion time, not filesystem modification time. For most use cases (age of synced content, recency, staleness) this is actually more useful than filesystem mtime.
- No backward-compatibility concerns — all existing entries in any doc already have `timestamp()` populated by iroh-docs.
- Future work: `sort --by insertion`, `filter --inserted-before/after`, `find --newer-than <duration>` can all use the same field.
