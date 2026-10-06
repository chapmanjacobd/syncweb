# Filter Engine, Logging, and Schedules

## Filter Engine

### Design

The `watch` command uses a Rust-native filter engine:

```toml
# ~/.config/syncweb/filters.toml
[general]
sort_mode = "niche"  # niche, frecency, peers, size, random
limit_size = "10GB"
min_seeders = 1

[[rules]]
type = "accept"
match = { name = "*.iso", min_size = "100MB" }

[[rules]]
type = "reject"
match = { name = "*.tmp", age = "7d" }

[[rules]]
type = "accept"
match = { path = "/important/", min_seeders = 3 }

[[rules]]
type = "reject"
match = { ext = ["log", "cache"] }

[[rules]]
type = "accept"
match = { version = ">=1.2.0" }  # Data package version filter
```

### Filter Engine Implementation

```rust
struct FilterEngine {
    rules: Vec<FilterRule>,
    sort_mode: SortMode,
    limit_size: Option<u64>,
    min_seeders: Option<usize>,
}

#[derive(Serialize, Deserialize)]
struct FilterRule {
    action: FilterAction,
    match_criteria: MatchCriteria,
}

#[derive(Serialize, Deserialize)]
struct MatchCriteria {
    name: Option<Pattern>,      // glob pattern
    ext: Option<Vec<String>>,   // file extensions
    path: Option<Pattern>,      // path pattern
    min_size: Option<u64>,
    max_size: Option<u64>,
    age: Option<Duration>,      // files older than
    min_seeders: Option<usize>,
    version: Option<String>,    // semver constraint
}

impl FilterEngine {
    /// Load from config file
    fn load(config_path: &Path) -> Result<Self>;

    /// Evaluate if a doc entry should be accepted/rejected
    fn evaluate(&self, entry: &DocEntry) -> FilterAction;

    /// Sort and limit results
    fn sort_and_limit(&self, entries: Vec<DocEntry>) -> Vec<DocEntry>;

    /// Run the automatic daemon loop
    async fn run_daemon(&self, node: &IrohNode) -> Result<()>;
}
```

### CLI Commands

```bash
# Watch a folder, honoring filter rules
syncweb watch ./folder

# Watch with a custom filter configuration
syncweb watch --filters filters.toml ./folder

# Show active filters
syncweb watch --show-filters

# Test filter against specific files
syncweb watch --dry-run --paths /path/to/files
```

---

## Logging & Observability

Structured logging via the `tracing` crate (not `log`). Async-aware, structured fields, configurable levels.

### Setup

```rust
use tracing_subscriber::{fmt, EnvFilter};

fn setup_logging(verbose: bool, trace: bool, log_file: Option<&Path>) {
    let filter = if trace {
        EnvFilter::new("trace")
    } else if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_file(true)
        .with_line_number(true)
        .with_target(false);

    if let Some(path) = log_file {
        let file_appender = tracing_appender::rolling::daily(path, "syncweb.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        subscriber.with_writer(non_blocking).init();
    } else {
        subscriber.init();
    }
}
```

### Usage

```rust
#[instrument(skip_all, fields(folder = %folder_id))]
async fn sync_folder(&self, folder_id: &NamespaceId) -> Result<()> {
    let peers = self.get_peers(folder_id).await?;
    info!(peer_count = peers.len(), "starting sync");
    
    for peer in &peers {
        debug!(peer = %peer, "connecting");
        // ...
    }
    
    info!(files_synced = 42, bytes = 1_048_576, "sync complete");
    Ok(())
}
```

### CLI Flags

| Flag | Level | Use Case |
|------|-------|----------|
| (default) | `info` | Normal operation |
| `--verbose` / `-v` | `debug` | Debugging sync issues |
| `--trace` | `trace` | Maximum detail (protocol-level) |
| `--log-file <path>` | (as above) | Write to rotating log file |

Log files rotate daily, 7-day retention (via `tracing-appender`).

---

## Transfer Limits

qBittorrent-style rolling byte quotas. `syncweb-core/src/transfer_limits.rs`
tracks transferred bytes per direction over the last hour, day (24 h), and
month (30 days). A queued blob is fetched only when its full size fits the
remaining allowance in every configured window; otherwise it stays queued.

```toml
[transfer_limits]
# Optional byte counts; omit a key (or set it to "0") for unlimited.
max_download_per_hour = "2G"
max_download_per_day  = "20G"
max_download_per_month = "400G"
max_upload_per_hour   = "1G"
max_upload_per_day    = "10G"
max_upload_per_month  = "200G"
```

```bash
syncweb config set transfer_limits.max_download_per_day 20G
syncweb config show transfer_limits
```

When a download quota is configured the daemon switches managed folders to the
metadata-only iroh-docs download policy and fetches queued blobs explicitly
within budget. When an upload quota is exhausted it pauses live sessions so the
node stops seeding until the window frees budget. Limits are re-read each sync
cycle, so `config set` takes effect without a restart.

---

## Sync Schedules

Time-based sync rules. Global settings with per-folder overrides. Byte quotas
are separate; see [Transfer Limits](#transfer-limits).

### Configuration

```toml
# ~/.config/syncweb/schedules.toml (or inline in config.toml)

[schedule]
# Global active hours (empty = always active)
active_hours = ""

# Per-folder overrides
[schedule.folders.media]
active_hours = "01:00-05:00"

[schedule.folders.backups]
active_hours = ""
```

### Implementation

```rust
struct ScheduleManager {
    global: Schedule,
    folders: BTreeMap<String, FolderSchedule>,
}

struct Schedule {
    active_hours: Option<TimeWindow>,
}

impl ScheduleManager {
    /// Check if sync is currently allowed (within active hours)
    fn is_active(&self, folder: Option<&str>) -> bool;
    /// Next minute the window opens (None when always active)
    fn next_active_window_start_at(&self, folder: Option<&str>, minute: u16) -> Option<u16>;
}
```

### CLI

```bash
syncweb config schedule               # Show current schedule
syncweb config schedule set --active "22:00-06:00"
syncweb config schedule folder --active "01:00-05:00" media
```

---

## Platform Settings Files

Suggested global configuration files for different use cases. These are not a "template" subcommand -- just example configs users can copy.

### Profiles

```toml
# ~/.config/syncweb/config-laptop.toml
# Optimized for battery life and mobile networks

[schedule]
active_hours = "08:00-22:00"

[transfer_limits]
max_upload_per_day = "1G"
max_download_per_day = "5G"

[parallel]
threads = 2  # Limit CPU usage

[cache]
max_cache_size = 5000  # Smaller cache for limited RAM
```

```toml
# ~/.config/syncweb/config-server.toml
# Optimized for throughput and availability

[schedule]
active_hours = ""  # always active

[parallel]
threads = 0  # auto-detect (use all cores)

[cache]
max_cache_size = 50000  # Large cache for plenty of RAM
```

```toml
# ~/.config/syncweb/config-phone.toml
# Optimized for storage and battery

[transfer_limits]
max_upload_per_day = "250M"
max_download_per_day = "1G"

[parallel]
threads = 1  # Single-threaded for battery

[advanced]
blob_cache_size_gb = 2  # Limited storage
```

Users copy the relevant file to `~/.config/syncweb/config.toml` and customize as needed.

---

## Integrity Verification

General integrity checking beyond package verification. iroh-blobs verifies every blob on fetch (BLAKE3), but this re-checks local blobs against doc entries.

### Implementation

```rust
struct IntegrityChecker {
    blob_store: BlobStore,
    docs: Docs,
}

struct VerifyResult {
    total: u64,
    verified: u64,
    corrupted: Vec<CorruptionInfo>,
    missing: Vec<PathBuf>,
}

struct CorruptionInfo {
    path: PathBuf,
    expected_hash: Hash,
    actual_hash: Hash,
}

impl IntegrityChecker {
    /// Verify all blobs in a folder match doc entries
    async fn verify_folder(&self, folder_id: &NamespaceId) -> Result<VerifyResult>;

    /// Verify a single file
    async fn verify_file(&self, path: &Path) -> Result<bool>;

    /// Background verification (configurable period)
    async fn periodic_verify(&self, interval: Duration) -> Result<()>;
}
```

### CLI

```bash
syncweb verify ./documents              # Verify all blobs
syncweb verify ./documents/photo.jpg    # Verify single file
```

---

## Bandwidth Accounting

Track upload/download per folder and per peer. Persisted across restarts.

### Implementation

```rust
struct BandwidthStats {
    total_upload: u64,
    total_download: u64,
    per_folder: HashMap<NamespaceId, FolderStats>,
    per_peer: HashMap<NodeId, PeerStats>,
    period_start: Instant,
}

struct FolderStats {
    upload: u64,
    download: u64,
    files_transferred: u64,
}

struct PeerStats {
    upload: u64,
    download: u64,
    connection_count: u32,
}
```

### CLI

```bash
syncweb stats network                    # Show all stats
syncweb stats network --period 24h       # Last 24 hours
syncweb stats network --since 24h        # Same window via --since (or a unix timestamp)
syncweb stats network --folder ./documents  # Per-folder breakdown
syncweb stats network --peer <node-id>   # Per-peer breakdown
syncweb stats network --follow           # Stream sync sessions/network events live
syncweb stats network --follow --once    # Print the current snapshot and exit (cron-safe)
```

---

## Watch Mode (Lowest Priority)

File system watcher for real-time sync. Monitor local changes and sync automatically.

### Implementation

Uses the `notify` crate (Rust). On file change:
1. Detect change type (create/modify/delete)
2. Import modified files to blob store
3. Update doc entries
4. Debounce rapid changes (default 500ms)

```rust
struct Watcher {
    watcher: RecommendedWatcher,
    debounce: Duration,
    exclude_patterns: Vec<glob::Pattern>,
}

impl Watcher {
    async fn watch(&self, path: &Path, folder_id: &NamespaceId) -> Result<()> {
        // Watch for changes, debounce, import, update doc
    }
}
```

### CLI

```bash
syncweb watch ./documents                           # Monitor and sync
syncweb watch --debounce 500ms ./documents          # Custom debounce
syncweb watch --exclude ".git/" --exclude "node_modules/" ./documents
```

---


