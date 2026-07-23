# TDD Plan: `stats` command family

## Divergence
Name "stats" implies general statistics. Actual: only bandwidth accounting (`BandwidthStats` with upload/download bytes per folder/peer).

## Decision
- **Keep** `stats` as bandwidth accounting (name is reasonable for its actual purpose once scoped)
- **Add** `filestats` subcommand for file-level statistics (counts, sizes, types, ages, distributions)
- **Add** `syncstats` subcommand for sync-specific metrics (sync rounds, conflicts resolved, events processed)

---

## Tests

### Phase 1 — Existing bandwidth tests (no regressions)

```rust
// syncweb-core/tests/schedule_stats_test.rs — bandwidth_stats_persist_and_reset (line 31)
// syncweb-cli/tests/cli_test.rs — schedule_and_stats_commands_persist_state (line 279)
// syncweb-cli/tests/full_suite_test.rs — json_stats_output (line 99), schedule_and_stats_persist (line 293)
```

### Phase 2 — `filestats` unit tests (syncweb-core)

```rust
// syncweb-core/tests/file_stats_test.rs  (new file)

use syncweb_core::stats::FileStatsCollector;

#[test]
fn test_file_stats_empty_directory() {
    let dir = tempfile::tempdir()?;
    let collector = FileStatsCollector::new(dir.path());
    let report = collector.collect()?;
    assert_eq!(report.total_files, 0);
    assert_eq!(report.total_size, 0);
    assert!(report.by_extension.is_empty());
    Ok(())
}

#[test]
fn test_file_stats_counts_by_extension() {
    let dir = tempfile::tempdir()?;
    fs::write(dir.path().join("a.txt"), b"hello")?;
    fs::write(dir.path().join("b.txt"), b"world")?;
    fs::write(dir.path().join("img.png"), vec![0; 100])?;
    let collector = FileStatsCollector::new(dir.path());
    let report = collector.collect()?;
    assert_eq!(report.total_files, 3);
    assert_eq!(report.by_extension["txt"].count, 2);
    assert_eq!(report.by_extension["png"].count, 1);
    assert_eq!(report.by_extension["txt"].total_size, 10);
    Ok(())
}

#[test]
fn test_file_stats_size_distribution() {
    let dir = tempfile::tempdir()?;
    fs::write(dir.path().join("tiny.txt"), b"a")?;
    fs::write(dir.path().join("medium.txt"), vec![0; 50_000])?;
    fs::write(dir.path().join("large.txt"), vec![0; 5_000_000])?;
    let collector = FileStatsCollector::new(dir.path());
    let report = collector.collect()?;
    assert_eq!(report.size_buckets["<1KB"], 1);
    assert!(report.size_buckets.get("1KB-1MB").copied().unwrap_or(0) >= 1);
    assert!(report.size_buckets.get("1MB-100MB").copied().unwrap_or(0) >= 1);
    Ok(())
}

#[test]
fn test_file_stats_age_distribution() {
    let dir = tempfile::tempdir()?;
    fs::write(dir.path().join("old.txt"), b"old")?;
    // Set mtime to 30 days ago
    let old_time = SystemTime::now() - Duration::from_secs(30 * 86400);
    filetime::set_file_mtime(dir.path().join("old.txt"), old_time.into())?;
    fs::write(dir.path().join("new.txt"), b"new")?;
    let collector = FileStatsCollector::new(dir.path());
    let report = collector.collect()?;
    assert_eq!(report.age_buckets[">30 days"], 1);
    assert_eq!(report.age_buckets["<7 days"], 1);
    Ok(())
}
```

### Phase 3 — `syncstats` unit tests

```rust
// syncweb-core/tests/sync_stats_test.rs  (new file)

use syncweb_core::stats::SyncStatsCollector;

#[test]
fn test_sync_stats_records_rounds() {
    let collector = SyncStatsCollector::new();
    collector.record_round("folder_a", 5, 2); // 5 new, 2 conflicts
    collector.record_round("folder_a", 3, 0);
    let report = collector.report();
    assert_eq!(report.per_folder["folder_a"].rounds, 2);
    assert_eq!(report.per_folder["folder_a"].files_synced, 8);
    assert_eq!(report.per_folder["folder_a"].conflicts_resolved, 2);
}

#[test]
fn test_sync_stats_empty() {
    let collector = SyncStatsCollector::new();
    let report = collector.report();
    assert_eq!(report.total_rounds, 0);
}
```

### Phase 4 — CLI integration tests

```rust
// syncweb-cli/tests/cli_test.rs

#[test]
fn test_filestats_extension_summary() -> anyhow::Result<()> {
    let source = cli_test_dir("filestats-ext");
    fs::write(source.join("a.txt"), b"a")?;
    fs::write(source.join("b.txt"), b"bb")?;
    fs::write(source.join("c.rs"), b"fn main() {}")?;

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["filestats", source.to_str().unwrap(), "--by", "extension"])
        .output()?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("txt")); // should list .txt extension
    assert!(stdout.contains("rs")); // should list .rs extension

    fs::remove_dir_all(&source)?;
    Ok(())
}

#[test]
fn test_filestats_json_output() -> anyhow::Result<()> {
    let source = cli_test_dir("filestats-json");
    fs::write(source.join("d.txt"), b"data")?;

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["filestats", "--json", source.to_str().unwrap()])
        .output()?;
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["total_files"], 1);

    fs::remove_dir_all(&source)?;
    Ok(())
}

#[test]
fn test_syncstats_persists_across_rounds() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("syncstats-persist");
    // Run syncstats after a sync
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir.to_str().unwrap(), "syncstats"])
        .output()?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("total_rounds"));

    fs::remove_dir_all(&data_dir)?;
    Ok(())
}
```

### Phase 5 — Full suite

```rust
// syncweb-cli/tests/full_suite_test.rs
// Add "filestats" and "syncstats" to the full_help_lists_all_commands test
```

---

## Implementation

### `syncweb-core/src/stats.rs` — Add `FileStatsCollector` and `SyncStatsCollector`

```rust
// New types

pub struct FileStatsCollector {
    root: PathBuf,
}

pub struct FileStatsReport {
    pub total_files: u64,
    pub total_size: u64,
    pub by_extension: BTreeMap<String, ExtensionGroup>,
    pub size_buckets: BTreeMap<String, u64>,
    pub age_buckets: BTreeMap<String, u64>,
    pub largest_files: Vec<FileEntry>,
    pub newest_files: Vec<FileEntry>,
}

pub struct ExtensionGroup {
    pub count: u64,
    pub total_size: u64,
}

pub struct SyncStatsCollector { ... }

pub struct SyncStatsReport {
    pub total_rounds: u64,
    pub total_files_synced: u64,
    pub total_conflicts: u64,
    pub per_folder: BTreeMap<String, SyncFolderStats>,
}
```

### `syncweb-core/src/stats.rs` — Add `collect()` method

```rust
impl FileStatsCollector {
    pub fn new(root: PathBuf) -> Self;
    pub fn collect(&self) -> Result<FileStatsReport>;
    pub fn with_extensions_filter(extensions: Vec<String>) -> Self;
}
```

### `syncweb-cli/src/cli/commands.rs`

- Keep `Stats(StatsArgs)` as-is for bandwidth
- Add `FileStats(FileStatsArgs)` variant
- Add `SyncStats(SyncStatsArgs)` variant

```rust
#[command(about = "Show file-level statistics for a directory")]
FileStats(FileStatsArgs),

#[command(about = "Show sync engine metrics")]
SyncStats,
```

### `syncweb-cli/src/cli/commands.rs` — `FileStatsArgs`

```rust
pub struct FileStatsArgs {
    pub path: PathBuf,
    #[arg(long, help = "Group by extension|size|age|all")]
    pub by: String,
    #[arg(long, help = "Top N largest files")]
    pub top_largest: Option<usize>,
    #[arg(long, help = "Top N newest files")]
    pub top_newest: Option<usize>,
    #[arg(long, help = "Filter by extensions (comma-separated)")]
    pub ext: Option<String>,
}
```

### `syncweb-cli/src/main.rs`

- `handle_filestats` — scans path, collects `FileStatsReport`, prints summary table
- `handle_syncstats` — reads from daemon or persisted sync log

---

## Gossip/network integration note

- `filestats` is purely local filesystem — no gossip needed
- `syncstats` can optionally subscribe to gossip to collect sync metrics from peers, but initial implementation is local-only (daemon persists round stats to disk)
- `stats` (bandwidth) already has `per_peer` tracking — could be extended to receive bandwidth announcements over gossip in future

## Files to modify/plan

| File | Changes |
|------|---------|
| `syncweb-core/src/stats.rs` | Add `FileStatsCollector`, `FileStatsReport`, `SyncStatsCollector`, `SyncStatsReport` |
| `syncweb-core/src/lib.rs` | No changes needed (already `pub mod stats`) |
| `syncweb-cli/src/cli/commands.rs` | Add `FileStats`, `SyncStats` enum variants + args structs |
| `syncweb-cli/src/main.rs` | Add `handle_filestats`, `handle_syncstats` |
| `syncweb-core/tests/file_stats_test.rs` | New file — extension, size, age distribution tests |
| `syncweb-core/tests/sync_stats_test.rs` | New file — round recording tests |
| `syncweb-cli/tests/cli_test.rs` | CLI integration tests for `filestats` and `syncstats` |
| `syncweb-cli/tests/full_suite_test.rs` | Add to help listing test |
