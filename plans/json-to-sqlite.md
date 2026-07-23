# Plan: JSON/Toml → SQLite Migration

## Divergence

The codebase uses 6 separate JSON files and 2 TOML files for persistent state that would be better served by SQLite databases. No data migration is needed (zero users). The goal is to consolidate flat-file state into logically grouped SQLite databases with proper schemas, transactions, and foreign keys.

## Current State

### JSON files (all in `data_dir/`)

| File | Data Structure | Size | Read/Write Pattern |
|---|---|---|---|
| `daemon.state` | `DaemonState` {pid, node_id, started_at, status} | ~100 bytes | Save at startup/shutdown; load at startup; remove on clean shutdown |
| `daemon.status` | `DaemonStatusReport` {pid, node_id, uptime, folders[], bandwidth snapshot, schedule} | ~1KB per folder | Write on every status poll (IPC); read by `syncweb status` |
| `networks.json` | `Vec<Network>` with members, folders, optional shared_secret | ~1KB per network | Full read + full write on every mutation (create/join/leave/invite/kick/add_folder/remove_folder) |
| `collections.json` | `CollectionState` {installed: BTreeMap<Uuid, InstalledCollection>} with version history and install paths | ~1KB per collection | Full read + full write on install/switch/remove |
| `stats.json` | `BandwidthStats` {total_upload, total_download, per_folder, per_peer, period_start} | ~1KB per folder | Atomic write on every sync tick (~60s); read on `syncweb stats` |
| `indexing-state.json` | `IndexingState` (12 sub-collections, see below) | Can grow large | Full read + full write on every CLI command that touches indexing state |

### TOML files

| File | Data Structure | Size | Read/Write Pattern |
|---|---|---|---|
| `config.toml` | `AppConfig` {sync_interval, relay, schedule, log_level, log_file, watch_debounce, rayon_threads} | ~200 bytes | Read at daemon startup; write on `config set` |
| `filters.toml` | `FilterConfig` {rules: Vec<FilterEntry>} with glob patterns | ~100 bytes per rule | Read at daemon startup and on reload; write on `automatic` filter update |

### Files to KEEP as-is

- `identity.key` — base32-encoded secret key (plaintext file is correct for key material)
- `daemon.lock` — PID lock file protected by OS-level `flock` (must remain a file for the lock)
- `daemon.sock` — Unix domain socket (transport, not state)

### IndexingState sub-collections

```
IndexingState {
    catalogs: Vec<CatalogState>,           // name + namespace_id
    federated_filters: Vec<FederatedFilterState>, // namespace + sequence
    denylist: Vec<DenylistRule>,           // type + value pairs
    links: LinkState {
        pointers: Vec<MutablePointer>,     // signed mutable pointers
        mirrors: Vec<String>,              // mirror URIs
        revoked: Vec<PrivateLink>,         // revoked link records
    },
    leases: Vec<ProviderLease>,            // signed provider lease records
    delegations: Vec<TrustDelegation>,     // trust delegation records
    moderation: Vec<ModerationRecord>,     // moderation actions
    attestations: Vec<Attestation>,        // content attestations
    reports: Vec<ReportRecord>,            // content reports
    provider_bans: Vec<BanRecord>,         // banned provider records
    provider_trust: Vec<ProviderTrustRecord>, // provider trust records
    trust_signals: Vec<ProviderTrustSignal>, // signed trust observations
    trust_streams: Vec<String>,            // subscribed trust stream namespaces
}
```

### Existing SQLite Schema (indexing.sqlite)

Already has tables (duplicated in indexing-state.json!):
- `index_metadata`, `indexed_folders`, `indexed_entries`, `indexed_entries_fts`
- `indexed_catalogs`, `indexed_catalog_entries`, `indexed_catalog_entries_fts`
- `wot_metadata`, `wot_metadata_fts`
- `stable_links`, `link_mirrors`
- `denylist_rules`, `filter_lists`
- `moderation_records`

The JSON file is the actual source of truth at runtime — the SQLite tables for links, denylist, etc. exist but are NOT used by the CLI code path.

## Decision

Create three SQLite databases organized by domain:

1. **`node.db`** — Daemon lifecycle, networks, collections, config, filters
2. **`stats.db`** — Bandwidth accounting and daemon logging
3. **`indexing.sqlite`** — EXISTING database, expand it with trust/moderation/provider tables (these have natural foreign keys to content, folders, catalogs, and WoT metadata already in this database)

## Database 1: `node.db` — Daemon, Networks, Collections, Config

Path: `$data_dir/node.db`

### Schema

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE schema_version (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- ── Daemon lifecycle ──────────────────────────────

CREATE TABLE daemon_lifecycle (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    pid INTEGER NOT NULL,
    node_id TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('starting','running','stopping','stopped')),
    data_dir TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Replaces daemon.status: status report snapshot
CREATE TABLE daemon_status (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    pid INTEGER NOT NULL,
    node_id TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    uptime_seconds INTEGER NOT NULL,
    upload_total INTEGER NOT NULL DEFAULT 0,
    download_total INTEGER NOT NULL DEFAULT 0,
    upload_rate INTEGER NOT NULL DEFAULT 0,
    download_rate INTEGER NOT NULL DEFAULT 0,
    in_active_window INTEGER NOT NULL DEFAULT 0,
    next_window_start INTEGER,
    rayon_threads INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE folder_status_reports (
    namespace_id TEXT NOT NULL,
    path TEXT NOT NULL,
    session_active INTEGER NOT NULL DEFAULT 0,
    last_sync_at INTEGER,
    entries_synced INTEGER NOT NULL DEFAULT 0,
    errors_json TEXT NOT NULL DEFAULT '[]',
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(namespace_id)
);

-- ── Networks ──────────────────────────────────────

CREATE TABLE networks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    label TEXT NOT NULL DEFAULT '',
    owner TEXT NOT NULL,
    shared_secret BLOB,           -- [u8; 32] or NULL
    created_at INTEGER NOT NULL
);

CREATE TABLE network_members (
    network_id TEXT NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    member TEXT NOT NULL,         -- PublicKey as string
    PRIMARY KEY(network_id, member)
);

CREATE TABLE network_folders (
    network_id TEXT NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    namespace_id TEXT NOT NULL,
    PRIMARY KEY(network_id, namespace_id)
);

-- ── Collections (installed packages) ──────────────

CREATE TABLE installed_collections (
    collection_id TEXT PRIMARY KEY,  -- Uuid as string
    manifest_hash TEXT NOT NULL,
    current_version TEXT NOT NULL,
    installed_at INTEGER NOT NULL
);

CREATE TABLE collection_versions (
    collection_id TEXT NOT NULL REFERENCES installed_collections(collection_id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    install_path TEXT NOT NULL,
    PRIMARY KEY(collection_id, version)
);

-- ── App configuration ─────────────────────────────

CREATE TABLE app_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- ── Filter rules ──────────────────────────────────

CREATE TABLE filter_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    namespace_id TEXT,            -- NULL = global rule
    rule_type TEXT NOT NULL,      -- 'include' | 'exclude'
    pattern TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);

-- Seed initial schema version
INSERT INTO schema_version(key, value) VALUES ('version', '1');
```

### Implementation Plan

#### Step 1: Create `NodeDatabase` abstraction

New file: `syncweb-core/src/storage/node_db.rs`

```rust
pub struct NodeDatabase {
    connection: Arc<Mutex<Connection>>,
}

impl NodeDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;
    pub fn migrate(&self) -> Result<()>;  // apply schema migrations

    // Daemon lifecycle
    pub fn save_lifecycle(&self, state: &DaemonState) -> Result<()>;
    pub fn load_lifecycle(&self) -> Result<Option<DaemonState>>;
    pub fn save_status(&self, report: &DaemonStatusReport) -> Result<()>;
    pub fn load_status(&self) -> Result<Option<DaemonStatusReport>>;

    // Networks
    pub fn create_network(&self, network: &Network) -> Result<()>;
    pub fn delete_network(&self, id: NetworkId) -> Result<()>;
    pub fn add_member(&self, network_id: NetworkId, member: PublicKey) -> Result<()>;
    pub fn remove_member(&self, network_id: NetworkId, member: PublicKey) -> Result<()>;
    pub fn add_folder(&self, network_id: NetworkId, namespace_id: NamespaceId) -> Result<()>;
    pub fn remove_folder(&self, network_id: NetworkId, namespace_id: NamespaceId) -> Result<()>;
    pub fn list_networks(&self) -> Result<Vec<Network>>;

    // Collections
    pub fn get_state(&self) -> Result<CollectionState>;
    pub fn install_collection(&self, collection_id: Uuid, manifest_hash: Hash, version: &str, path: &Path) -> Result<()>;
    pub fn switch_version(&self, collection_id: Uuid, version: &str) -> Result<()>;
    pub fn remove_version(&self, collection_id: Uuid, version: &str) -> Result<()>;

    // Config
    pub fn get_config(&self, key: &str) -> Result<Option<String>>;
    pub fn set_config(&self, key: &str, value: &str) -> Result<()>;
    pub fn load_app_config(&self) -> Result<AppConfig>;
    pub fn save_app_config(&self, config: &AppConfig) -> Result<()>;

    // Filters
    pub fn load_filter_engine(&self) -> Result<Option<FilterEngine>>;
    pub fn save_filter_rules(&self, rules: &[FilterEntry]) -> Result<()>;
}
```

#### Step 2: Migrate `NetworkManager`

File: `syncweb-core/src/net/network_manager.rs`

- Remove `path: PathBuf` field
- Replace with `db: NodeDatabase` reference
- Replace `load_networks()` with `db.list_networks()`
- Replace all JSON save operations with SQL operations
- Remove `NetworkRecord` intermediate serialization struct
- `new()` takes `&NodeDatabase` instead of path and returns `Result<Self>`

#### Step 3: Migrate `DaemonState` / `StateFile`

File: `syncweb-core/src/daemon/state.rs`

- Remove `StateFile` struct (the JSON-based one)
- `Daemon::new()` calls `node_db.save_lifecycle(...)` and `node_db.save_status(...)` directly
- `DaemonHandle.status()` calls `node_db.load_status()`
- Remove `STATE_FILE_NAME`, `STATUS_FILE_NAME` constants
- Keep `PidLock` (it manages the flock-based lock file, which is correct as-is)

#### Step 4: Migrate `PackageManager`

File: `syncweb-core/src/folder/package.rs`

- Remove JSON file read/write methods (`state()` and `save_state()`)
- Add `db: NodeDatabase` field to `PackageManager`
- Replace `state()` with `self.db.get_state()`
- Replace `save_state()` with SQL operations (insert/update/delete on installed_collections and collection_versions tables)

#### Step 5: Migrate Config

File: `syncweb-core/src/storage/config.rs`

- Replace TOML file load/save with `NodeDatabase` methods
- `AppConfig::load(path)` → `NodeDatabase::load_app_config()`
- `AppConfig::save(path)` → `NodeDatabase::save_app_config()`
- Each config key is stored as a row in `app_config`

#### Step 6: Migrate Filters

File: `syncweb-core/src/filter.rs`

- Replace TOML file load with `NodeDatabase::load_filter_engine()`
- `save()` method → `NodeDatabase::save_filter_rules()`
- The `FilterEngine::load_filters(path)` function now reads from node.db

#### Step 7: Wire up in `Daemon::new()`

File: `syncweb-core/src/daemon/daemon.rs:114-241`

```rust
let node_db = NodeDatabase::open(config.data_dir.join("node.db"))?;
node_db.migrate()?;
```

Pass `node_db` (or `Arc<NodeDatabase>`) to:
- `StateFile` replacement
- `NetworkManager`
- `PackageManager`
- Config loading
- Filter loading

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/storage/node_db.rs` | **NEW** — NodeDatabase struct with all SQLite methods |
| `syncweb-core/src/storage/mod.rs` | Add `pub mod node_db` |
| `syncweb-core/src/daemon/state.rs` | Remove `StateFile` (JSON); use `NodeDatabase` |
| `syncweb-core/src/daemon/daemon.rs` | Open `node.db`; pass to subsystems |
| `syncweb-core/src/net/network_manager.rs` | JSON → SQLite via `NodeDatabase` |
| `syncweb-core/src/folder/package.rs` | JSON → SQLite via `NodeDatabase` |
| `syncweb-core/src/storage/config.rs` | TOML → SQLite via `NodeDatabase` |
| `syncweb-core/src/filter.rs` | TOML → SQLite via `NodeDatabase` |
| `syncweb-cli/src/cli/commands.rs` | Update any direct file reads that become DB reads |
| `syncweb-cli/src/main.rs` | Wire `NodeDatabase` to CLI handlers |

---

## Database 2: `stats.db` — Bandwidth & Logging

Path: `$data_dir/stats.db`

### Schema

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE schema_version (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Time-series bandwidth events (append-only)
CREATE TABLE bandwidth_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    direction TEXT NOT NULL CHECK(direction IN ('upload','download')),
    bytes INTEGER NOT NULL CHECK(bytes > 0),
    files INTEGER NOT NULL DEFAULT 1,
    folder_namespace TEXT,
    peer TEXT
);
CREATE INDEX idx_bw_ts ON bandwidth_events(timestamp);
CREATE INDEX idx_bw_folder ON bandwidth_events(folder_namespace);
CREATE INDEX idx_bw_peer ON bandwidth_events(peer);

-- Current period aggregate (materialized, one row per period)
CREATE TABLE bandwidth_period (
    period_start INTEGER PRIMARY KEY,
    period_end INTEGER,
    total_upload INTEGER NOT NULL DEFAULT 0,
    total_download INTEGER NOT NULL DEFAULT 0,
    closed INTEGER NOT NULL DEFAULT 0
);

-- Per-folder aggregates for current period
CREATE TABLE bandwidth_folder (
    period_start INTEGER NOT NULL REFERENCES bandwidth_period(period_start),
    folder_namespace TEXT NOT NULL,
    upload INTEGER NOT NULL DEFAULT 0,
    download INTEGER NOT NULL DEFAULT 0,
    files_transferred INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(period_start, folder_namespace)
);

-- Per-peer aggregates for current period
CREATE TABLE bandwidth_peer (
    period_start INTEGER NOT NULL REFERENCES bandwidth_period(period_start),
    peer TEXT NOT NULL,
    upload INTEGER NOT NULL DEFAULT 0,
    download INTEGER NOT NULL DEFAULT 0,
    connection_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(period_start, peer)
);

-- Daemon log (structured logging)
CREATE TABLE daemon_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    level TEXT NOT NULL CHECK(level IN ('trace','debug','info','warn','error')),
    module TEXT,
    message TEXT NOT NULL
);
CREATE INDEX idx_log_ts ON daemon_log(timestamp);
CREATE INDEX idx_log_level ON daemon_log(level);

INSERT INTO schema_version(key, value) VALUES ('version', '1');
```

### Implementation Plan

#### Step 1: Create `StatsDatabase`

New file: `syncweb-core/src/storage/stats_db.rs`

```rust
pub struct StatsDatabase {
    connection: Arc<Mutex<Connection>>,
}

impl StatsDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;
    pub fn migrate(&self) -> Result<()>;

    // Append bandwidth event (fast, append-only)
    pub fn record_upload(&self, bytes: u64, files: u64, folder: Option<&str>, peer: Option<&str>) -> Result<()>;
    pub fn record_download(&self, bytes: u64, files: u64, folder: Option<&str>, peer: Option<&str>) -> Result<()>;
    pub fn record_connection(&self, peer: &str) -> Result<()>;

    // Query current period stats (replaces BandwidthStats struct for display)
    pub fn current_stats(&self) -> Result<BandwidthStats>;
    pub fn stats_for_period(&self, period_start: u64) -> Result<BandwidthStats>;

    // Period management
    pub fn start_new_period(&self) -> Result<()>;
    pub fn close_period(&self, period_start: u64) -> Result<()>;

    // Logging
    pub fn append_log(&self, level: &str, module: Option<&str>, message: &str) -> Result<()>;
    pub fn query_logs(&self, level: Option<&str>, limit: usize) -> Result<Vec<LogEntry>>;

    // Maintenance
    pub fn purge_old_logs(&self, older_than: Duration) -> Result<usize>;
    pub fn purge_old_bandwidth(&self, older_than: Duration) -> Result<usize>;
}
```

#### Step 2: Migrate `BandwidthStats`

File: `syncweb-core/src/stats.rs`

- Keep `BandwidthStats` as an in-memory transient struct for the `stats` CLI output
- Remove `BandwidthStats::load()` and `BandwidthStats::save()` (JSON methods)
- Replace `record_upload`/`record_download`/`record_connection` in-memory with `StatsDatabase` writes
- The daemon calls `stats_db.record_upload(...)` on each sync event
- The `stats` command calls `stats_db.current_stats()` and formats output

#### Step 3: Integrate structured logging

Daemon logging currently goes to a log file via `tracing`. Add a `StatsDatabase` layer to the tracing subscriber so structured events (level >= warn by default, configurable) are also written to `daemon_log` table:

```rust
// In Daemon::run_inner()
let stats_db = Arc::new(StatsDatabase::open(data_dir.join("stats.db"))?);
// Add a tracing layer that calls stats_db.append_log() for warn+ events
```

#### Step 4: Wire up in daemon

File: `syncweb-core/src/daemon/daemon.rs`

- Open `stats_db` in `Daemon::new()`
- Pass `Arc<StatsDatabase>` to sync engine, bandwidth tracking, and log capture

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/storage/stats_db.rs` | **NEW** — StatsDatabase struct |
| `syncweb-core/src/storage/mod.rs` | Add `pub mod stats_db` |
| `syncweb-core/src/stats.rs` | Replace JSON save/load with StatsDatabase calls; keep BandwidthStats as in-memory/report struct |
| `syncweb-core/src/daemon/daemon.rs` | Open stats.db; wire to subsystems |
| `syncweb-core/src/sync/engine.rs` | Record bandwidth events to StatsDatabase instead of BandwidthStats |

---

## Database 3: `indexing.sqlite` — Expand existing with Trust/Moderation/Provider tables

Path: `$data_dir/indexing.sqlite` (ALREADY EXISTS — add tables to the existing schema)

### Additional Schema

Append these tables to the existing `initialize_schema()` in `syncweb-core/src/indexing.rs:1303-1429`:

```sql
-- ── Provider reputation ───────────────────────────

CREATE TABLE provider_reputation (
    provider TEXT PRIMARY KEY,
    total_fetches INTEGER NOT NULL DEFAULT 0,
    successful_fetches INTEGER NOT NULL DEFAULT 0,
    failed_fetches INTEGER NOT NULL DEFAULT 0,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    last_success_at INTEGER,
    last_failure_at INTEGER,
    auto_ban_until INTEGER,
    auto_ban_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE provider_signal_sequences (
    reporter TEXT NOT NULL,
    provider TEXT NOT NULL,
    last_sequence INTEGER NOT NULL,
    PRIMARY KEY(reporter, provider)
);

-- ── Provider trust records ────────────────────────

CREATE TABLE provider_trust_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL,
    trustee TEXT NOT NULL,
    action TEXT NOT NULL CHECK(action IN ('trust','distrust')),
    content_hash BLOB,                  -- FK: indexed_entries.content_hash (logical)
    namespace_id TEXT,                  -- FK: indexed_folders.namespace_id (logical)
    created_at INTEGER NOT NULL,
    expires_at INTEGER,
    signature TEXT NOT NULL,
    UNIQUE(provider, trustee, action, content_hash)
);

-- ── Provider bans ─────────────────────────────────

CREATE TABLE provider_bans (
    provider TEXT NOT NULL,
    content_hash BLOB,                  -- NULL = global ban
    reason TEXT NOT NULL,
    banned_at INTEGER NOT NULL,
    expires_at INTEGER,
    PRIMARY KEY(provider, content_hash)
);

-- ── Provider leases ───────────────────────────────

CREATE TABLE provider_leases (
    provider TEXT NOT NULL,
    content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
    ticket TEXT NOT NULL,
    leased_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    signature TEXT NOT NULL,
    PRIMARY KEY(provider, content_hash)
);
CREATE INDEX idx_provider_leases_hash ON provider_leases(content_hash);

-- ── Trust delegations ─────────────────────────────

CREATE TABLE trust_delegations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    delegator TEXT NOT NULL,
    delegate TEXT NOT NULL,
    content_scope TEXT,
    namespace_scope TEXT,               -- FK: indexed_folders.namespace_id (logical)
    permissions_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER,
    signature TEXT NOT NULL,
    UNIQUE(delegator, delegate, content_scope, namespace_scope)
);

-- ── Content attestations ──────────────────────────

CREATE TABLE attestations (
    content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
    attestor TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('verified','corrupted','malicious','safe','unsafe')),
    metadata_json TEXT,
    created_at INTEGER NOT NULL,
    signature TEXT NOT NULL,
    PRIMARY KEY(content_hash, attestor, kind)
);

-- ── Content reports ───────────────────────────────

CREATE TABLE content_reports (
    content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
    reporter TEXT NOT NULL,
    reason TEXT NOT NULL,
    scope TEXT NOT NULL DEFAULT 'global',
    created_at INTEGER NOT NULL,
    PRIMARY KEY(content_hash, reporter, reason)
);

-- ── Provider trust signals ────────────────────────

CREATE TABLE provider_trust_signals (
    reporter TEXT NOT NULL,
    provider TEXT NOT NULL,
    signal_kind TEXT NOT NULL CHECK(signal_kind IN ('ObservedSuccess','ObservedFailure','ObservedCorruption')),
    content_hash BLOB,
    sequence INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    signature TEXT NOT NULL,
    UNIQUE(reporter, provider, sequence)
);
CREATE INDEX idx_trust_signals_provider ON provider_trust_signals(provider);
CREATE INDEX idx_trust_signals_ts ON provider_trust_signals(timestamp);

-- ── Trust streams ─────────────────────────────────

CREATE TABLE trust_streams (
    namespace TEXT PRIMARY KEY,
    subscribed_at INTEGER NOT NULL
);

-- ── Migrate existing link/mirror data to match new schema ──
-- stable_links and link_mirrors already exist (created by initialize_schema).
-- No changes needed unless the schema needs updates.
```

### Why in indexing.sqlite and NOT a separate trust.db

Natural foreign keys exist between these tables and the existing `indexing.sqlite` tables:
- `attestations.content_hash` → `indexed_entries.content_hash`
- `content_reports.content_hash` → `indexed_entries.content_hash`
- `provider_leases.content_hash` → same hash space as `indexed_entries.content_hash`
- `trust_delegations.namespace_scope` → `indexed_folders.namespace_id`
- `trust_delegations.delegator` → same identity space as `wot_metadata.author`
- `attestations.attestor` → same identity space as `wot_metadata.author`
- Existing `moderation_records` table already uses content_hash → indexed_entries
- All trust decisions are made in context of content that is (or should be) indexed

Separating these into `trust.db` would break join queries and require cross-database integrity management.

### Implementation Plan

#### Step 1: Add migration support to IndexingDatabase

File: `syncweb-core/src/indexing.rs`

- Update `SCHEMA_VERSION` from `"1"` to `"2"`
- In `initialize_schema()`: add all new `CREATE TABLE IF NOT EXISTS` statements above
- Add `migrate_v1_to_v2()` function that adds new tables only

#### Step 2: Migrate `IndexingState` (JSON) → SQLite

File: `syncweb-cli/src/cli/indexing.rs`

This is the largest change. Currently every CLI handler in this file:
1. Calls `load_state(data_dir)` → reads entire `indexing-state.json`
2. Modifies it in memory
3. Calls `save_state(data_dir, &state)` → writes entire file

Replace with SQLite operations:

```rust
// Remove these functions:
// fn load_state(data_dir: &Path) -> Result<IndexingState>
// fn save_state(data_dir: &Path, state: &IndexingState) -> Result<()>

// Add instead: each command handler does targeted SQL operations
// on the relevant tables in indexing.sqlite
```

Remove `IndexingState`, `CatalogState`, `FederatedFilterState`, `LinkState`, `ReportRecord` struct definitions (they become unnecessary with SQLite).

Each CLI command handler becomes:
- `indexing filter add` → `INSERT INTO denylist_rules`
- `indexing filter subscribe` → `INSERT OR REPLACE INTO filter_lists`
- `link create` → `INSERT INTO stable_links`
- `link resolve` → `SELECT FROM stable_links`
- `trust delegate` → `INSERT INTO trust_delegations`
- `attest` → `INSERT INTO attestations`
- `report` → `INSERT INTO content_reports`
- `trust provider` → `INSERT INTO provider_trust_records`
- `provider ban` → `INSERT INTO provider_bans`
- `moderation` → `INSERT INTO moderation_records`
- etc.

#### Step 3: Make DenylistService read from SQLite

File: `syncweb-core/src/indexing/denylist.rs`

```rust
pub struct DenylistService {
    database: IndexingDatabase,  // add this field (currently no database reference)
}

impl DenylistService {
    pub fn new(database: IndexingDatabase) -> Self;  // new constructor
    pub fn add(&self, rule: DenylistRule) -> Result<()> {
        // INSERT into denylist_rules table (not just in-memory)
    }
    pub fn check(&self, context: &FilterContext) -> Result<Denied> {
        // SELECT from denylist_rules (not in-memory HashSet)
    }
}
```

#### Step 4: Make LinkResolver read/write from SQLite

File: `syncweb-core/src/indexing/links.rs`

```rust
pub struct LinkResolver {
    database: IndexingDatabase,  // add database reference
    cache: Arc<Mutex<HashMap<...>>>,  // keep in-memory cache for performance
}

impl LinkResolver {
    pub fn new(database: IndexingDatabase) -> Self;
    pub fn publish(&self, pointer: MutablePointer) -> Result<()> {
        // Upsert into stable_links table + update cache
    }
    pub fn resolve(&self, alias: &str) -> Result<Option<LinkResolution>> {
        // Check cache, fall back to SQLite query
    }
}
```

#### Step 5: Migrate daemon IPC handlers

File: `syncweb-core/src/daemon/ipc.rs`

- Replace `load_state`/`save_state` calls in IPC handlers with SQLite operations
- The `handle_indexing_*` IPC methods that currently load/save `IndexingState` now use `IndexingDatabase` methods directly

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/indexing.rs` | Add new tables to schema; bump SCHEMA_VERSION; add migration |
| `syncweb-core/src/indexing/denylist.rs` | Add `IndexingDatabase` field; read/write from SQLite |
| `syncweb-core/src/indexing/links.rs` | Add `IndexingDatabase` field; read/write from SQLite |
| `syncweb-core/src/indexing/reputation.rs` | Add `save_reputation()` / `load_reputation()` methods using SQLite |
| `syncweb-core/src/indexing/resilience.rs` | Persist leases/bans to SQLite tables directly |
| `syncweb-cli/src/cli/indexing.rs` | Replace JSON load/save with SQLite operations; remove `IndexingState` struct |
| `syncweb-core/src/daemon/ipc.rs` | Replace JSON state calls with SQLite calls |
| `syncweb-core/tests/indexing_db_test.rs` | **NEW** — tests for new SQLite tables |

---

## Files to DELETE after migration

| File | Reason |
|---|---|
| `daemon.state` (on-disk, stop writing it) | Replaced by `node.db` daemon_lifecycle |
| `daemon.status` (on-disk, stop writing it) | Replaced by `node.db` daemon_status + folder_status_reports |
| `networks.json` (on-disk, stop writing it) | Replaced by `node.db` network tables |
| `collections.json` (on-disk, stop writing it) | Replaced by `node.db` collection tables |
| `config.toml` (on-disk, stop writing it) | Replaced by `node.db` app_config |
| `filters.toml` (on-disk, stop writing it) | Replaced by `node.db` filter_rules |
| `stats.json` (on-disk, stop writing it) | Replaced by `stats.db` bandwidth tables |
| `indexing-state.json` (on-disk, stop writing it) | Replaced by `indexing.sqlite` trust/moderation/provider tables |
| `syncweb-core/src/storage/config.rs` `save` method | Config now in node.db |
| `syncweb-core/src/daemon/state.rs` `StateFile` struct | Replaced by NodeDatabase |

NOT deleted:
- `identity.key` — key material, correct as plaintext
- `daemon.lock` — flock-based PID lock
- `daemon.sock` — unix domain socket
- `syncweb/data/blobs/` — Iroh FsStore (untouched)
- `syncweb/data/docs/` — Iroh Docs persistent store (untouched)

---

## New Dependency

The project already uses `rusqlite` with `bundled` feature for `indexing.sqlite`. No new crate dependencies needed — the same `rusqlite` handles all three databases.
