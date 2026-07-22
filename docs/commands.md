# Command Designs

## `find` Command Design

Full-text search across folder entries with regex, glob, or exact substring matching,
plus depth, size, time, extension, and type filters. Uses doc metadata only -- no blob download needed.

### Search Parameters

```rust
/// Search parameters for find command
struct FindParams {
    /// Search paths (folder roots or subpaths)
    search_paths: Vec<PathBuf>,
    /// Pattern type: regex, glob, or exact substring
    pattern_type: PatternType,
    /// User-provided pattern(s)
    patterns: Vec<String>,
    /// Case sensitivity
    ignore_case: bool,
    /// File type filter: "f" (file) or "d" (directory)
    file_type: Option<FileType>,
    /// Depth constraints
    min_depth: Option<usize>,
    max_depth: Option<usize>,
    /// Size constraints (>1GB, <500MB, =100MB, etc.)
    sizes: Option<Vec<SizeConstraint>>,
    /// Time modified constraints (<7d, >30d, etc.)
    time_modified: Option<Vec<TimeConstraint>>,
    /// File extension filter
    ext: Option<Vec<String>>,
    /// Show hidden files
    hidden: bool,
    /// Search full path vs. just filename
    full_path: bool,
    /// Output format
    output: OutputFormat,
}

enum PatternType {
    Regex,
    Glob,
    Exact,
}

enum SizeConstraint {
    GreaterThan(u64),
    LessThan(u64),
    EqualTo(u64),
    /// Within percentage range of a value
    Within(u64, f64),
}

enum TimeConstraint {
    OlderThan(Duration),
    NewerThan(Duration),
}
```

### Search Engine

```rust
/// Dispatches to the appropriate pattern engine
fn search_entries(params: &FindParams, entries: &[DocEntry]) -> Vec<DocEntry> {
    entries.iter()
        .filter(|e| matches_all_constraints(params, e))
        .collect()
}

fn matches_all_constraints(p: &FindParams, entry: &DocEntry) -> bool {
    let target = if p.full_path { &entry.path } else { &entry.filename };

    let pattern_ok = match p.pattern_type {
        PatternType::Regex => regex_match(target, &p.patterns, p.ignore_case),
        PatternType::Glob => glob_match(target, &p.patterns, p.ignore_case),
        PatternType::Exact => exact_match(target, &p.patterns, p.ignore_case),
    };

    let type_ok = p.file_type.map_or(true, |ft| ft == entry.file_type);
    let ext_ok = p.ext.as_ref().map_or(true, |exts| {
        exts.iter().any(|ext| entry.filename.ends_with(ext))
    });
    let size_ok = p.sizes.as_ref().map_or(true, |sizes| {
        sizes.iter().all(|s| s.matches(entry.size))
    });
    let time_ok = p.time_modified.as_ref().map_or(true, |times| {
        times.iter().all(|t| t.matches(entry.modified))
    });
    let depth_ok = p.min_depth.map_or(true, |d| entry.depth >= d)
        && p.max_depth.map_or(true, |d| entry.depth <= d);

    pattern_ok && type_ok && ext_ok && size_ok && time_ok && depth_ok
}
```

### CLI

```bash
# Regex search (default)
syncweb find '.*\.mp3$' music/

# Glob search
syncweb find --glob '/*.mp3' music/

# Fixed/exact search (substring)
syncweb find --fixed-string 'beethoven' music/

# Combined filters
syncweb find --type f --ext mp3 --min-size 10MB --max-size 500MB music/

# Time filters
syncweb find 'report.*\.md$' --modified-within 7d
syncweb find 'old-log.*' --modified-before 30d

# Depth constraints
syncweb find --depth +2 --depth -5 'config.*'

# Pipe to download
syncweb find '*.iso' linux/ | syncweb download -

# Pipe from stdin
syncweb find --print '*.mp3' audio/ | xargs -I{} syncweb download {}
```

Output (simple, pipe-friendly):
```
syncweb find '*.pdf' docs/
docs/manual.pdf
docs/reference.pdf
docs/getting-started.pdf
```

---

## `stat` Command Design

File metadata from doc entries + blob store, showing detailed info similar to `stat(1)`.
Shows local vs. global diffs (for conflict detection), availability (peer count), version vectors.

### Stat Output Structure

```rust
struct StatOutput {
    /// Basic info
    path: PathBuf,
    size: u64,
    num_blocks: u64,
    file_type: FileType,
    permissions: String,

    /// Timing
    modified: Timestamp,
    inode_change: Timestamp,
    modified_by: NodeId,

    /// Version info
    version: Vec<Version>,

    /// Availability
    available_on: Vec<NodeId>,

    /// Local vs Global differences (for conflict detection)
    diffs: Vec<DiffEntry>,
}

struct DiffEntry {
    key: String,
    local_value: String,
    global_value: String,
}

impl StatOutput {
    fn display(&self, format: StatFormat);
    fn diff_with_global(&self) -> Vec<DiffEntry>;
}

enum StatFormat {
    /// Human-readable (default)
    Human,
    /// Terse, pipe-separated (for scripting)
    Terse,
    /// Custom template string
    Custom(String),
}
```

### CLI

```bash
# Default human-readable stat
syncweb stat docs/report.md
```

Output:
```
  Path: docs/report.md
  Size: 245760             Blocks: 12             regular file
  Device: alice, bob       Version: v1, v2, v3
  Access: (0644/---------)
  Modify: 2024-01-15 10:30:00.000000 +0000 (alice)
  Change: 2024-01-15 10:25:00.000000 +0000
```

With local/global differences:
```
  Key        Local              Global
  size       245760             250000
  modified   1705317000         1705320000
```

```bash
# Terse format (for scripting)
syncweb stat --terse docs/report.md
# Output: report.md|245760|12|0644|regular file|1705317000|3|0

# Custom format
syncweb stat --format '%n %s %y' docs/report.md

# Multiple files
syncweb stat docs/*.md
```

---

## `sort` Command Design

Sort files by multiple criteria: niche (how close to N seeders), frecency (popular + recent),
peers/seeds, size, date, random. Supports folder-level aggregates for sorting by folder stats.
Uses the PeerTracker for seed/availability data.

### Sort Engine

```rust
/// Sort criteria
enum SortCriterion {
    /// Number of peers/blobs
    Peers { reverse: bool },
    /// Niche: |num_peers - target| -- find blobs with ~N seeders
    Niche { target: usize, reverse: bool },
    /// Frecency: peers - (days_since_modified / weight) -- popular + recent
    Frecency { weight: f64, reverse: bool },
    /// File size
    Size { reverse: bool },
    /// Modified time
    Time { reverse: bool },
    /// Random (stable per session)
    Random,
    /// Folder-level aggregate
    FolderAggregate { field: AggregateField, agg: AggregateFunc, reverse: bool },
}

enum AggregateField { Size, Modified }
enum AggregateFunc { Sum, Mean, Median, Min, Max, Count }

/// Sort engine -- uses PeerTracker for availability data
struct Sorter {
    criteria: Vec<SortCriterion>,
    filters: Vec<SortFilter>,
    peer_tracker: PeerTracker,
}

impl Sorter {
    /// Compute sort key for an entry
    fn sort_key(&self, entry: &DocEntry, folder_aggregates: &FolderAggregates) -> Vec<OrderedFloat<f64>>;

    /// Sort entries in place
    fn sort(&self, entries: &mut [DocEntry]);

    /// Apply size/count limit after sorting
    fn limit(&self, entries: &[DocEntry]) -> Vec<DocEntry>;
}

/// Folder-level aggregates (computed from doc entries)
struct FolderAggregates {
    aggregates: HashMap<PathBuf, FolderAggregate>,
}

struct FolderAggregate {
    size_sum: u64,
    size_median: f64,
    size_mean: f64,
    modified_median: i64,
    file_count: usize,
}
```

### CLI

```bash
# Default: niche + frecency
syncweb sort music/

# Sort by peers (most seeded first)
syncweb sort --sort peers music/

# Sort by niche (files with ~3 seeders)
syncweb sort --sort niche music/
syncweb sort --sort +niche music/      # most niche
syncweb sort --sort -niche music/      # least niche

# Sort by frecency (popular + recent)
syncweb sort --sort frecency music/

# Sort by folder size (largest folder first)
syncweb sort --sort folder-size music/

# Combined: peers primary, time secondary
syncweb sort --sort peers --sort time music/

# With limits
syncweb sort --limit-size 10GB music/
syncweb sort --min-seeders 2 music/

# Pipe to download
syncweb sort --sort niche music/ | syncweb download -
```

---

## `init`/`config` Command Design

The `init` command creates a folder and outputs a shareable URL. The `config` command
manages local configuration (data dir, default paths, sync modes, filters).

### Init Command

```rust
struct InitResult {
    folder_id: String,
    share_url: String,
    path: PathBuf,
    namespace_id: NamespaceId,
}

impl IrohNode {
    /// Initialize a folder: create dir, set up namespace, output URL
    async fn init_folder(&self, path: &Path, opts: InitOptions) -> Result<InitResult>;
}
```

CLI:
```bash
# Create folder + output URL
syncweb init ./documents
# Output: sync://documents#<device-id>

# Init with sync mode
syncweb init --sync-mode sendreceive ./documents

# Init with network membership
syncweb init --network work ./documents

# Init with description
syncweb init --label "Work Documents" ./documents
```

### Config Command

```rust
impl IrohNode {
    async fn get_config(&self) -> Result<Config>;
    async fn set_config(&self, patch: ConfigPatch) -> Result<()>;
}
```

CLI:
```bash
# Show config
syncweb config
# Shows: data_dir, default_path, relay, discovery, networks, etc.

# Modify config
syncweb config set data_dir ~/.syncweb
syncweb config set default_path ~/Syncweb
syncweb config set default_sync_mode SendReceive

# Config sections
syncweb config show networks
syncweb config show bep
syncweb config show filter
```

---

## CLI Command Mapping

| syncweb-py | syncweb | Notes |
|------------|----------------|-------|
| `create` | `create` | Create folder + doc + blob store |
| `join` | `join` | Import doc via ticket/capability |
| `accept` | `accept` | Grant capability to peer |
| `drop` | `drop` | Revoke capability, remove peer |
| `folders` | `folders` | List local docs + status |
| `devices` | `devices` | List known peers + connection status |
| `ls` | `ls` | List doc entries (lazy) |
| `find` | `find` | Search doc entries (with filters) |
| `download` | `download` | Trigger lazy fetch for paths |
| `sort` | `sort` | Sort results (uses peer tracker) |
| `stat` | `stat` | File metadata from doc + blob store |
| `automatic` | `automatic` | Auto-accept/join with filter engine |
| `shutdown` | `shutdown` | Gracefully stop the node |
| `init` | `init` | Create folder + output shareable sync:// URL |
| `config` | `config` | Show/modify local configuration |
| `start` | `start` | Start the node (or log that it started) |
| `version` | `version` | Show versions |
| `repl` | `repl` | Interactive REPL |
| (implicit) | `import` | Import local files to blob store + doc entries |
| NEW | `policy` | Manage deployment policy levers (access, encryption, searchable, pinning) at various scopes (`show`, `set`, `explain`) |
| NEW | `subscribe` | Join public folder via ticket |
| NEW | `public list` | List announced public folders |
| NEW | `collection init` | Initialize folder as data package |
| NEW | `collection add` | Scan + hash files, update manifest |
| NEW | `collection versions` | Create new version with changelog |
| NEW | `collection publish` | Store manifest, pin content, and announce a blob ticket |
| NEW | `package search` | Discover packages via gossip |
| NEW | `package info` | Detailed package metadata |
| NEW | `package export` | Export package versions as compressed `.car.zst` drops |
| NEW | `package import` | Import and install a compressed `.car.zst` drop |
| NEW | `package install` | Fetch + verify + install package |
| NEW | `package upgrade` | Update to latest version |
| NEW | `package remove` | Remove installed package |
| NEW | `package verify` | Integrity check against manifest |
| NEW | `package list` | List locally installed packages |
| NEW | `package versions` | List installed versions |
| NEW | `package switch` | Change active version |
| NEW | `health` | Show seeding status per blob (well/under/unseeded) |
| NEW | `backup` | Create content-addressed snapshot of folder |
| NEW | `restore` | Restore folder from snapshot |
| NEW | `snapshots` | List available snapshots |
| NEW | `network create` | Create named network group |
| NEW | `network ls` | List networks or network details |
| NEW | `network join` | Join a network via ticket |
| NEW | `network leave` | Leave a network |
| NEW | `network invite` | Invite device to a network |
| NEW | `network kick` | Remove device from a network |
| NEW | `stats` | Bandwidth accounting per folder/peer |
| NEW | `verify` | Integrity verification (re-check local blobs) |
| NEW | `schedule` | Show/modify sync schedule |
| NEW | `conflicts` | List/resolve file conflicts |
| NEW | `watch` | File watcher for real-time sync (lowest priority) |
| NEW | `network test-relay` | Test Syncthing relay connectivity |

### New CLI Options (from iroh-willow)

```bash
# Global flags (all commands)
syncweb --home /path/to/data ls    # Custom data directory
syncweb --verbose find .            # Verbose output
syncweb --json folders              # JSON output (for scripting)
syncweb --no-color devices          # Disable color output

syncweb import ./documents
syncweb watch --once ./documents
syncweb stats --period 24h
syncweb verify ./documents
syncweb schedule
syncweb schedule set --active "22:00-06:00"
syncweb schedule set --bandwidth "5MB/s" --period "08:00-18:00"
syncweb schedule folder media --active "01:00-05:00"

# Download with limits (max entries)
syncweb download --limit 10 /path/to/files

# Download with size limit
syncweb download --size 1GB /path/to/files

# Subscribe with filtering (only new files)
syncweb subscribe --ingest-only /path/to/folder

# Subscribe ignoring our own writes
syncweb subscribe --ignore-self /path/to/folder

# Publish with limits
syncweb publish --limit 100 --size 10GB /path/to/folder

# Show deleted files
syncweb deleted /path/to/folder

# Restore deleted file
syncweb undelete <entry-hash>

# Scan with all available CPUs (the default)
syncweb ls

# Scan with a specific thread count; use 1 to disable parallelism
syncweb ls --threads 8
syncweb ls --threads 1

# Parallel import (the default)
syncweb import /path/to/files

# Parallel export (the default)
syncweb export /path/to/output

# Health check (show seeding status)
syncweb health audio/

# Download poorly-seeded blobs to improve network health
syncweb download --max-peers 2 audio/

# Download a local tree in parallel (the default); use 1 for sequential copying
syncweb download --threads 1 /path/to/source /path/to/destination

# Bandwidth limiting
syncweb folders --limit-upload 1MB/s --limit-download 5MB/s
syncweb devices --peer-limit NODE-ID --upload 500KB/s --download 2MB/s

# Backup/snapshot commands
syncweb backup documents/ --description "before edit"
syncweb snapshots documents/
syncweb restore documents/ a1b2c3d4
syncweb snapshots diff documents/ a1b2c3d4 e5f6g7h8

# Network commands
syncweb network create work
syncweb network ls
syncweb network ls work
syncweb network invite work <device-id>

# Find with filters
syncweb find --glob '/*.mp3' music/
syncweb find --type f --ext mp3 --min-size 10MB music/
syncweb find 'report.*' --modified-within 7d

# Stat with format
syncweb stat docs/report.md
syncweb stat --terse docs/report.md
syncweb stat --format '%n %s %y' docs/report.md

# Sort with criteria
syncweb sort --sort niche music/
syncweb sort --sort peers --sort time music/
syncweb sort --limit-size 10GB --min-seeders 2 music/

# Init/config
syncweb init ./documents
syncweb init --network work ./documents
syncweb config set default_path ~/Syncweb

# BEP-compatible device ID display
syncweb devices --bep

# Conflict resolution
syncweb conflicts
syncweb conflicts --resolve
syncweb conflicts --auto-resolve
syncweb conflicts resolve <id> --keep-local

# Offline queue
syncweb pending
```

---

## Configuration

```toml
# ~/.config/syncweb/config.toml
[node]
data_dir = "~/.local/share/syncweb"
node_name = "my-device"

[relay]
# Iroh relay (default: iroh's public relays)
urls = ["https://relay.iroh.computer"]

[discovery]
# Enable/disable discovery mechanisms
local_mdns = true
iroh_gossip = true
mainline_dht = true

[discovery.topic_tracker]
# distributed-topic-tracker settings
enabled = true
# Rate limit for DHT writes (records per minute per topic)
dht_write_limit = 5
# Bubble detection threshold (merge if fewer than N neighbors)
bubble_threshold = 4
# Secret rotation strategy: "sha512" (default)
secret_rotation = "sha512"

[folders]
default_path = "~/IrohSyncweb"
default_sync_mode = "SendReceive"
default_max_entries = 0  # 0 = unlimited
default_max_size = 0     # 0 = unlimited

[bandwidth]
# Global bandwidth limits (bytes/sec, 0 = unlimited)
max_upload = 0
max_download = 0
# Per-peer limits (applied to all peers unless overridden)
per_peer_upload = 0
per_peer_download = 0

[public]
# Public folder settings
announce_enabled = true
gossip_topic = "syncweb/public-folders"
# Content pinning (prevent GC for shared blobs)
pin_shared_content = true

[bep]
# Syncthing relay fallback (for CGNAT traversal)
enabled = true
# Syncthing relay URLs (tcp:// for relay protocol v1)
relay_urls = ["tcp://relay.syncthing.net:22270"]
# Timeout for relay connection attempt (seconds)
relay_timeout = 10
# Auto-detect CGNAT and use relay when iroh direct/relay fails
auto_fallback = true

[schedule]
# Global sync schedule
active_hours = ""  # empty = always active

# Bandwidth limits by time of day
[[schedule.bandwidth]]
hours = "08:00-18:00"
max_upload = "1MB/s"
max_download = "5MB/s"

[[schedule.bandwidth]]
hours = "18:00-08:00"
max_upload = "0"  # unlimited
max_download = "0"

# Per-folder schedule overrides
[schedule.folders.media]
active_hours = "01:00-05:00"
max_download = "50MB/s"

[networks]
# Networks are auto-discovered; this section can pin specific network config
# Networks enable multi-folder + multi-device grouping under a gossip topic
default_network = ""

[networks.my-work]
label = "Work Documents"
# Topic is derived from network name; manual override for existing topics
# topic = "syncweb/net/work-a1b2c3"
members = []       # Auto-populated; manual pinning for invite-only networks
folders = []       # Folders in this network (auto-populated)

[filter]
# Automatic daemon filter settings
config_path = "~/.config/syncweb/filters.toml"

[advanced]
# Blob store settings
blob_cache_size_gb = 10
# Connection limits
max_connections = 100
# Peer tracker settings
peer_cache_expiry_s = 300  # 5 minutes

[cache]
# Cache eviction strategy ((standard CS pattern: age-based cache eviction))
# "lru" - Least Recently Used (resets age on access)
# "fifo" - First In First Out (never resets age)
eviction_strategy = "lru"
# Maximum cache size before eviction (entries)
max_cache_size = 10000
# Use memory-efficient bitmask cache for large peer networks
use_efficient_cache = true
# Threshold for switching to efficient cache (peer count)
efficient_cache_threshold = 100

[parallel]
# Parallel file operations ((standard CS pattern: parallel directory traversal))
# Number of threads (0 = auto-detect CPU count, 1 = single-threaded)
threads = 0
# Parallel is default for ls, import, export
# Use --threads=1 to disable per-command, or set threads = 1 here globally

```
