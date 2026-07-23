# Plan: Network Gaps & Remaining Storage Improvements

## Divergence

Three prior plans covered JSON→SQLite migration, ephemeral→persistent gaps, and Iroh docs audit. This plan covers everything else: network-level state gaps, the `.syncweb-collection.json` redundancy, database maintenance tooling, and partial-sync progress tracking.

## Decision

Address six distinct gaps:

1. **Network health & event log** — persist network connectivity history and events per-network
2. **Per-network bandwidth/transfer tracking** — correlate stats with network membership
3. **`.syncweb-collection.json` redundancy** — remove the local JSON manifest, read from blob store
4. **Sync progress persistence** — checkpoint partial sync/download progress to survive restarts
5. **Network membership propagation via Iroh docs** — replace ticket-only membership with real-time doc-synced member lists
6. **Database maintenance** — migration framework, vacuum, backup, integrity checks

---

## GAP 1: Network Health & Event Log

### Current State

Network operations (peer joins, peer leaves, relay connections, topic subscriptions) generate no persistent records. The `network test-relay` command runs a connectivity test and discards results. Network member joins/leaves mutate `networks.json` but produce no event history.

### What's Lost on Restart

- History of which peers connected and when
- Relay connectivity uptime/downtime
- Which networks were actively syncing
- How many times a network reconnected after failure
- Peer churn rates

### Fix

Add tables to `stats.db` (the logging/metrics database from Plan 1):

```sql
-- Network connectivity events
CREATE TABLE network_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    network_id TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK(event_type IN (
        'peer_joined','peer_left','sync_started','sync_finished',
        'relay_connected','relay_disconnected','relay_failed',
        'topic_subscribed','topic_unsubscribed',
        'member_added','member_removed','folder_added','folder_removed',
        'ticket_created','ticket_accepted'
    )),
    peer TEXT,
    details TEXT,
    metadata_json TEXT
);
CREATE INDEX idx_network_events_ts ON network_events(timestamp);
CREATE INDEX idx_network_events_network ON network_events(network_id);
CREATE INDEX idx_network_events_type ON network_events(event_type);

-- Network sync sessions
CREATE TABLE network_sync_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    network_id TEXT NOT NULL,
    folder_namespace TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    finished_at INTEGER,
    files_transferred INTEGER NOT NULL DEFAULT 0,
    bytes_transferred INTEGER NOT NULL DEFAULT 0,
    errors INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running','completed','failed','cancelled'))
);
CREATE INDEX idx_network_sessions_network ON network_sync_sessions(network_id);

-- Relay connection health (aggregated, periodic)
CREATE TABLE relay_health (
    relay_url TEXT NOT NULL,
    checked_at INTEGER NOT NULL,
    connected INTEGER NOT NULL,
    latency_ms INTEGER,
    error_message TEXT,
    PRIMARY KEY(relay_url, checked_at)
);
```

### Implementation

**New module:** `syncweb-core/src/net/network_log.rs`

```rust
pub struct NetworkLogger {
    database: Arc<StatsDatabase>,
}

impl NetworkLogger {
    pub fn record_event(&self, network_id: &NetworkId, event: NetworkEventType, peer: Option<PublicKey>, details: Option<&str>) -> Result<()>;
    pub fn record_sync_start(&self, network_id: &NetworkId, folder: NamespaceId) -> Result<i64>;
    pub fn record_sync_finish(&self, session_id: i64, files: u64, bytes: u64, errors: u64, status: &str) -> Result<()>;
    pub fn record_relay_check(&self, relay_url: &str, connected: bool, latency: Option<Duration>, error: Option<&str>) -> Result<()>;

    // Query
    pub fn recent_events(&self, network_id: &NetworkId, limit: usize) -> Result<Vec<NetworkEvent>>;
    pub fn recent_sessions(&self, network_id: &NetworkId, limit: usize) -> Result<Vec<SyncSession>>;
    pub fn relay_uptime(&self, relay_url: &str, window: Duration) -> Result<f64>;  // 0.0-1.0
}
```

**Wire into NetworkManager:**

```rust
// syncweb-core/src/net/network_manager.rs

impl NetworkManager {
    // On every mutation, also log the event
    pub fn create(&mut self, name: &str, options: NetworkOptions) -> Result<NetworkId> {
        // ... existing logic ...
        self.logger.record_event(&id, NetworkEventType::MemberAdded, None, Some("created"))?;
        Ok(id)
    }

    pub fn join(&mut self, ticket: NetworkTicket) -> Result<NetworkId> {
        // ... existing logic ...
        self.logger.record_event(&id, NetworkEventType::MemberAdded, Some(self.local_node), Some("joined"))?;
        Ok(id)
    }
}
```

**Wire into daemon sync cycle:**

```rust
// syncweb-core/src/daemon/daemon.rs — in the automatic sync loop

for (network_id, folders) in &network_folders {
    for folder_namespace in folders {
        let session_id = logger.record_sync_start(network_id, folder_namespace)?;
        let result = sync_engine.sync(folder_namespace, SessionMode::ReconcileOnce).await;
        match result {
            Ok(handle) => {
                // ... wait for completion, collect stats ...
                logger.record_sync_finish(session_id, files, bytes, errors, "completed")?;
            }
            Err(e) => {
                logger.record_sync_finish(session_id, 0, 0, 1, "failed")?;
            }
        }
    }
}
```

**Expose via CLI:**

New subcommand: `syncweb network events <network-id> [--limit N]`
New subcommand: `syncweb network health [--network <id>]`

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/net/network_log.rs` | **NEW** — NetworkLogger implementation |
| `syncweb-core/src/net/mod.rs` | Add `pub mod network_log` |
| `syncweb-core/src/net/network_manager.rs` | Add event logging on mutations |
| `syncweb-core/src/daemon/daemon.rs` | Log sync sessions per network |
| `syncweb-core/src/storage/stats_db.rs` | Add network events/sessions tables to schema |
| `syncweb-cli/src/cli/commands.rs` | Add `NetworkEvents` and `NetworkHealth` subcommands |
| `syncweb-cli/src/main.rs` | Handle new subcommands |

---

## GAP 2: Per-Network Bandwidth Correlation

### Current State

`stats.json` → `stats.db` (Plan 1) tracks bandwidth per-folder and per-peer, but has no concept of networks. A folder may belong to multiple networks, and there's no way to attribute traffic to a specific network.

### Fix

Add a `network_id` column to the `bandwidth_events` table:

```sql
-- In stats.db (add column to existing table from Plan 1)
ALTER TABLE bandwidth_events ADD COLUMN network_id TEXT;
CREATE INDEX idx_bw_network ON bandwidth_events(network_id);

-- Per-network aggregate view
CREATE VIEW network_bandwidth_summary AS
SELECT
    network_id,
    MIN(timestamp) AS period_start,
    MAX(timestamp) AS period_end,
    SUM(CASE WHEN direction = 'upload' THEN bytes ELSE 0 END) AS total_upload,
    SUM(CASE WHEN direction = 'download' THEN bytes ELSE 0 END) AS total_download,
    COUNT(DISTINCT peer) AS active_peers
FROM bandwidth_events
WHERE network_id IS NOT NULL
GROUP BY network_id;
```

The daemon's bandwidth recording path (in SyncEngine or Daemon) knows which network triggered the sync. Pass the `network_id`:

```rust
// syncweb-core/src/sync/engine.rs — in the transfer completion hook

pub fn record_transfer(&self, network_id: Option<NetworkId>, direction: Direction, bytes: u64, peer: Option<&str>) {
    self.stats_db.record_transfer(timestamp, direction, bytes, 1,
        Some(&folder_namespace.to_string()),
        peer,
        network_id.map(|id| id.to_string()).as_deref()
    )?;
}
```

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/storage/stats_db.rs` | Add `network_id` column to schema; update `record_upload/download` signatures |
| `syncweb-core/src/sync/engine.rs` | Pass `network_id` through transfer recording |
| `syncweb-core/src/daemon/daemon.rs` | Pass `network_id` when calling sync for network folders |
| `syncweb-core/src/net/network_manager.rs` | Optionally: expose network_id in folder iteration |

---

## GAP 3: `.syncweb-collection.json` Redundancy

### Current State

Two copies of each collection manifest exist after `collection init`:
1. **In the Iroh blob store** — content-addressed, integrity-guaranteed (the canonical source)
2. **As `.syncweb-collection.json`** — JSON file written alongside the package source directory (`syncweb-cli/src/main.rs:2614`)

The JSON file is a convenience copy that lets `package export` and `package info` read the manifest without hitting the blob store. But this creates a consistency risk: the blob in the store and the JSON file can diverge (e.g., if the user edits the JSON file, or if the package is imported from a ticket where no local manifest file was written).

### Files

| File | Line | Operation |
|------|------|-----------|
| `syncweb-cli/src/main.rs` | **2599-2601** | `manifest_path()` returns `<path>/.syncweb-collection.json` |
| `syncweb-cli/src/main.rs` | **2613-2616** | `save_manifest()` writes `manifest.to_bytes()` (JSON) to disk |
| `syncweb-core/src/daemon/ipc.rs` | **1191-1192** | `tokio::fs::read(&manifest_path)` reads it back |

### Fix

**Remove the local JSON file entirely.** Manifests are already in the blob store. Reading from the blob store is fast (local hashing + disk read). The blob store is the canonical, content-addressed source.

For `package export`:
```rust
// Instead of reading .syncweb-collection.json:
let manifest_hash = collection_head.manifest_hash;  // from doc entry
let manifest_bytes = blob_store.get(manifest_hash).await?;
let manifest = CollectionManifest::from_bytes(&manifest_bytes)?;
```

For `package info`:
```rust
// Instead of reading .syncweb-collection.json:
// Manifest hash is either in the blob ticket or from version history
let manifest_bytes = blob_store.get(manifest_hash).await?;
let manifest = CollectionManifest::from_bytes(&manifest_bytes)?;
```

The `collections.json` → `node.db` (Plan 1) already tracks `manifest_hash` per installed collection, so the hash lookup path exists.

### Validation

After removing the write, verify that every code path that reads `.syncweb-collection.json` has an alternative:
- `package export` — uses `manifest_hash` from collection state → blob store
- `package info` — uses ticket hash or collection state → blob store
- `package publish` — generates manifest from source, stores to blob, doesn't need on-disk copy
- `package verify` — reads from blob store via `CollectionManifest::blob_id()`

### Files to modify

| File | Change |
|---|---|
| `syncweb-cli/src/main.rs` | Remove `manifest_path()` and `save_manifest()`; update callers to use blob store |
| `syncweb-core/src/daemon/ipc.rs` | Remove `tokio::fs::read(&manifest_path)`; read from blob store |
| `syncweb-core/src/folder/collection.rs` | Ensure `CollectionHead` has `manifest_hash` field accessible to all callers |

---

## GAP 4: Sync Progress Persistence (Partial Download Checkpointing)

### Current State

When syncing a folder, all progress tracking is in-memory via `TransferStats`. If the daemon is restarted mid-sync, all progress is lost and the folder must be re-synced from scratch. For large folders, this can mean re-downloading gigabytes of data.

The blob store already has individual blobs — if 350 of 400 files were downloaded before the crash, those 350 blobs are still locally available. But the sync engine doesn't know this and will trigger downloads for all 400 again (though Iroh's protocol may skip blobs already present).

### What Needs Persistence

For each folder being synced, track:
- Which entry keys have been processed
- Which entry keys are pending download
- Which entry keys failed (with retry count)
- Total progress (processed / total)

This lets a restarted sync resume from where it left off.

### Fix

Add tables to `node.db`:

```sql
-- Sync checkpoints per folder
CREATE TABLE sync_checkpoints (
    namespace_id TEXT NOT NULL,
    session_id TEXT NOT NULL,           -- UUID for this sync session
    total_entries INTEGER NOT NULL,
    processed_entries INTEGER NOT NULL DEFAULT 0,
    failed_entries INTEGER NOT NULL DEFAULT 0,
    bytes_total INTEGER,
    bytes_transferred INTEGER NOT NULL DEFAULT 0,
    started_at INTEGER NOT NULL,
    last_updated_at INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','running','completed','failed','cancelled')),
    PRIMARY KEY(namespace_id, session_id)
);

-- Individual entry progress within a sync session
CREATE TABLE sync_entry_progress (
    namespace_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    entry_key BLOB NOT NULL,
    hash BLOB NOT NULL,
    size INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','downloading','completed','failed','skipped')),
    retries INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(namespace_id, session_id, entry_key),
    FOREIGN KEY(namespace_id, session_id) REFERENCES sync_checkpoints(namespace_id, session_id) ON DELETE CASCADE
);
```

### Implementation

**New module:** `syncweb-core/src/sync/checkpoint.rs`

```rust
pub struct SyncCheckpoint {
    namespace_id: NamespaceId,
    session_id: String,
    database: NodeDatabase,
}

impl SyncCheckpoint {
    pub fn new(database: &NodeDatabase, namespace_id: NamespaceId) -> Result<Self>;

    /// Initialize with total entry count. Returns session_id.
    pub fn initialize(&self, total_entries: usize) -> Result<String>;

    /// Mark an entry as completed.
    pub fn mark_completed(&self, entry_key: &[u8], hash: Hash, size: u64) -> Result<()>;

    /// Mark an entry as failed with error message.
    pub fn mark_failed(&self, entry_key: &[u8], error: &str) -> Result<()>;

    /// Mark an entry as skipped (already present locally).
    pub fn mark_skipped(&self, entry_key: &[u8]) -> Result<()>;

    /// Get all pending entries for this session.
    pub fn pending_entries(&self) -> Result<Vec<PendingEntry>>;

    /// Get overall progress.
    pub fn progress(&self) -> Result<CheckpointProgress>;

    /// Mark session as completed.
    pub fn complete(&self) -> Result<()>;

    /// Load the most recent unfinished checkpoint for a folder.
    pub fn resume(namespace_id: NamespaceId) -> Result<Option<Self>>;
}

#[derive(Clone, Debug)]
pub struct CheckpointProgress {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub pending: usize,
    pub bytes_transferred: u64,
    pub bytes_total: Option<u64>,
    pub percentage: f64,
}
```

### Wire into SyncEngine

```rust
// syncweb-core/src/sync/engine.rs — in run_intent()

async fn run_intent(folder: SyncwebFolder, mode: SessionMode, ...) -> Result<()> {
    let checkpoint = match SyncCheckpoint::resume(folder.namespace_id())? {
        Some(cp) => {
            tracing::info!("resuming sync checkpoint {}", cp.session_id());
            cp
        }
        None => {
            let entries = docs_engine.list_latest(folder.doc()).await?;
            let cp = SyncCheckpoint::new(&node_db, folder.namespace_id())?;
            cp.initialize(entries.len())?;
            cp
        }
    };

    // Only process entries that are still pending
    for entry in checkpoint.pending_entries()? {
        match download_entry(&entry).await {
            Ok(_) => checkpoint.mark_completed(&entry.key, entry.hash, entry.size)?,
            Err(e) => checkpoint.mark_failed(&entry.key, &e.to_string())?,
        }
    }

    if checkpoint.progress()?.failed == 0 {
        checkpoint.complete()?;
    }
}
```

### Cleanup

On successful completion, delete the checkpoint records (they're transient operational state). On daemon startup, check for dangling checkpoints and clean up sessions older than 7 days (stale/crashed sessions).

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/sync/checkpoint.rs` | **NEW** — SyncCheckpoint implementation |
| `syncweb-core/src/sync/mod.rs` | Add `pub mod checkpoint` |
| `syncweb-core/src/sync/engine.rs` | Integrate checkpoint into `run_intent()` |
| `syncweb-core/src/storage/node_db.rs` | Add sync_checkpoints schema; add CRUD methods |

---

## GAP 5: Database Maintenance Framework

### Rationale

After migrating 6 JSON files + 2 TOML files into 3 SQLite databases (plus the pre-existing indexing.sqlite), the project needs:
- A unified schema migration system
- Periodic VACUUM for space reclamation
- Backup tooling
- Integrity verification

### Schema Migration System

**New module:** `syncweb-core/src/storage/migrate.rs`

```rust
pub trait Migration {
    fn version(&self) -> i64;
    fn description(&self) -> &'static str;
    fn up(&self, connection: &Connection) -> Result<()>;
    // fn down(&self, connection: &Connection) -> Result<()>;  // future
}

pub struct MigrationRunner {
    migrations: Vec<Box<dyn Migration>>,
}

impl MigrationRunner {
    pub fn new() -> Self;

    /// Register a migration. Migrations must be registered in version order.
    pub fn add_migration(&mut self, migration: Box<dyn Migration>);

    /// Run all pending migrations for a database.
    /// Uses a `schema_version` table to track current state.
    pub fn run(&self, connection: &Connection) -> Result<usize>;  // returns count applied

    /// Check if any migrations are pending.
    pub fn pending_count(&self, connection: &Connection) -> Result<usize>;
}
```

Each database (`node.db`, `stats.db`, `indexing.sqlite`) uses the same `MigrationRunner` with its own set of migrations:

```rust
// syncweb-core/src/storage/node_db.rs

fn create_migrations() -> MigrationRunner {
    let mut runner = MigrationRunner::new();
    runner.add_migration(Box::new(CreateDaemonTables));
    runner.add_migration(Box::new(CreateNetworkTables));
    runner.add_migration(Box::new(CreateCollectionTables));
    runner.add_migration(Box::new(CreateConfigTables));
    runner.add_migration(Box::new(CreateFilterTables));
    runner.add_migration(Box::new(CreateSyncCheckpointTables));
    runner
}

impl NodeDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.busy_timeout(Duration::from_secs(5))?;

        let runner = create_migrations();
        let applied = runner.run(&connection)?;
        if applied > 0 {
            tracing::info!(applied, "applied node.db schema migrations");
        }

        Ok(Self { connection: Arc::new(Mutex::new(connection)) })
    }
}
```

### VACUUM / Maintenance

Add to each database:

```rust
impl NodeDatabase {
    /// Run VACUUM to reclaim space. Should be called periodically and after large deletes.
    pub fn vacuum(&self) -> Result<()> {
        self.with_connection(|conn| {
            conn.execute_batch("VACUUM")
                .map_err(|e| SyncwebError::operation("vacuum failed", e))
        })
    }

    /// Return estimated database size in bytes.
    pub fn size_on_disk(&self) -> Result<u64> {
        // ... fs::metadata ...
    }

    /// Return freed page count (indicates whether VACUUM would help).
    pub fn freelist_count(&self) -> Result<i64> {
        // PRAGMA freelist_count
    }
}
```

Add a daemon maintenance task that runs periodically (every 24h by default):
```rust
// In Daemon::run_inner()
let maintenance_interval = Duration::from_hours(24);
let node_db = self.node_db.clone();
let stats_db = self.stats_db.clone();
tokio::spawn(async move {
    loop {
        tokio::time::sleep(maintenance_interval).await;
        for db in [&node_db, &stats_db] {
            if db.freelist_count()? > 100 {
                db.vacuum()?;
            }
        }
    }
});
```

### Integrity Check

```rust
impl NodeDatabase {
    /// Run integrity_check PRAGMA. Returns list of errors (empty = healthy).
    pub fn check_integrity(&self) -> Result<Vec<String>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare("PRAGMA integrity_check")?;
            let errors: Vec<String> = stmt.query_map([], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .filter(|s: &String| s != "ok")
                .collect();
            Ok(errors)
        })
    }
}
```

Expose via CLI:
- `syncweb db check` — runs integrity check on all databases
- `syncweb db vacuum` — manually trigger vacuum
- `syncweb db stats` — shows database sizes, table row counts

### Backup

```rust
impl NodeDatabase {
    /// Create a backup using SQLite's online backup API.
    pub fn backup(&self, backup_path: impl AsRef<Path>) -> Result<()> {
        // Uses rusqlite::backup::Backup API for hot-backup
    }
}
```

Expose via CLI:
- `syncweb db backup [--output <path>]` — backs up all databases to a directory or single archive

### Files to modify/create

| File | Change |
|---|---|
| `syncweb-core/src/storage/migrate.rs` | **NEW** — Migration trait and MigrationRunner |
| `syncweb-core/src/storage/mod.rs` | Add `pub mod migrate` |
| `syncweb-core/src/storage/node_db.rs` | Add migration registration; add vacuum/check/backup methods |
| `syncweb-core/src/storage/stats_db.rs` | Add migration registration; add vacuum/check/backup methods |
| `syncweb-core/src/indexing.rs` | Refactor `initialize_schema` to use MigrationRunner |
| `syncweb-core/src/daemon/daemon.rs` | Add periodic maintenance task |
| `syncweb-cli/src/cli/commands.rs` | Add `DbCommand` with `check`, `vacuum`, `backup`, `stats` subcommands |
| `syncweb-cli/src/main.rs` | Wire `db` subcommand |

---

## GAP 6: Network Membership Propagation via Iroh Docs

### Current State

Network membership is managed entirely through one-time tickets:
1. Owner creates a network
2. Owner generates a `NetworkTicket` containing the current member list, shared secret, and folder set
3. New member imports the ticket → gets a snapshot of membership
4. If owner kicks a member later, the kicked member only discovers this when their gossip connection is rejected — there is no real-time notification
5. If the owner adds a new member, existing members don't learn about it automatically
6. The member list in `networks.json` → `node.db` (Plan 1) is purely local — each node has its own potentially stale copy

### Impact

- **Kicked members** continue trying to connect until they time out or manually leave
- **Newly added members** are invisible to existing peers until they exchange tickets manually
- **Two nodes with tickets from different points in time** may have different member lists, causing confusion about who is in the network
- **No single source of truth** — each node's local copy can diverge
- Network members can't see the full member list without asking the owner for a fresh ticket

### Fix: Signed Membership Doc

Store network membership as a **signed document entry** in a per-network Iroh docs namespace. Every network gets a dedicated doc. The owner writes a signed membership list; all members sync the doc and verify signatures.

#### Architecture

```
Network "my-project" (NetworkId::from_name("my-project"))
  └── Doc namespace = derive_namespace(network_id, shared_secret)
       ├── key: "sys/network/members"     → signed member list (written by owner)
       ├── key: "sys/network/info"        → network metadata (name, label, created_at)
       └── key: "sys/network/folders"     → associated folder namespaces
```

The doc namespace is derived deterministically from the network ID and shared secret, so all members can compute the same namespace without exchanging it.

#### Data Structures

```rust
/// The canonical, owner-signed list of network members.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedMemberList {
    pub network_id: String,             // NetworkId as string
    pub owner: String,                  // PublicKey as string (the signer)
    pub sequence: u64,                  // Monotonic counter, prevents replay
    pub members: Vec<MemberEntry>,      // Current member set
    pub updated_at: u64,                // Unix timestamp
    pub signature: String,              // hex-encoded Ed25519 signature over the above fields
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemberEntry {
    pub key: String,                    // PublicKey as string
    pub joined_at: u64,
    pub role: MemberRole,               // Future: admin/member distinction
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MemberRole {
    Admin,
    Member,
}
```

#### Signature Scheme

```rust
const MEMBER_LIST_SIGNATURE_CONTEXT: &[u8] = b"syncweb/network-membership/v1\0";

impl SignedMemberList {
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        assert_eq!(
            self.owner,
            hex::encode(signing_key.verifying_key().to_bytes())
        );
        let mut unsigned = self.clone();
        unsigned.signature = String::new();
        let message = serde_json::to_vec(&unsigned)?;
        let mut signed_bytes = Vec::new();
        signed_bytes.extend_from_slice(MEMBER_LIST_SIGNATURE_CONTEXT);
        signed_bytes.extend_from_slice(&message);
        self.signature = hex::encode(signing_key.sign(&signed_bytes).to_bytes());
        Ok(())
    }

    pub fn verify(&self) -> Result<()> {
        let key_bytes = hex::decode(&self.owner)?;
        let verifying_key = VerifyingKey::from_bytes(&key_bytes.into())?;
        let signature_bytes = hex::decode(&self.signature)?;
        let signature = Signature::from_slice(&signature_bytes)?;
        let mut unsigned = self.clone();
        unsigned.signature = String::new();
        let message = serde_json::to_vec(&unsigned)?;
        let mut signed_bytes = Vec::new();
        signed_bytes.extend_from_slice(MEMBER_LIST_SIGNATURE_CONTEXT);
        signed_bytes.extend_from_slice(&message);
        verifying_key.verify(&signed_bytes, &signature)?;
        Ok(())
    }
}
```

#### Doc Namespace Derivation

```rust
/// Derive the deterministic Iroh docs namespace for a network.
///
/// Uses the network ID and shared secret so that:
/// - All members can independently compute the same namespace
/// - An attacker who knows the network ID but not the shared secret cannot
///   compute the namespace (content-addressing provides privacy by obscurity)
pub fn network_doc_namespace(network_id: NetworkId, shared_secret: &[u8; 32]) -> NamespaceId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"syncweb/network-doc/v1\0");
    hasher.update(network_id.as_bytes());
    hasher.update(shared_secret);
    NamespaceId::from_bytes(hasher.finalize().into())
}
```

#### Lifecycle

**Owner creates network:**
1. Generate `network_doc_namespace(network_id, shared_secret)`
2. Create or open the doc with that namespace
3. Write `sys/network/info` with network metadata
4. Write `sys/network/members` with `SignedMemberList { members: [owner], sequence: 1, ... }`
5. Write `sys/network/folders` with folder namespace list
6. Store the doc as a `Doc` handle; members can now receive live updates

**Owner adds a member:**
1. Increment sequence
2. Add `MemberEntry { key: new_member, role: Member, joined_at: now }` to the list
3. Sign and write to `sys/network/members`
4. The doc syncs to all connected members automatically
5. The new member still needs to join by importing a ticket (to get the shared secret and doc namespace), but existing members see the change immediately

**Owner kicks a member:**
1. Increment sequence
2. Remove `MemberEntry` for the kicked member
3. Sign and write to `sys/network/members`
4. The doc syncs to ALL members including the kicked one
5. On receiving the updated member list, the kicked node:
   - Verifies the owner's signature
   - Sees it's no longer in the list
   - Auto-leaves the network (unsubscribes gossip topic, closes doc)
   - Log/emit a "kicked from network" event

**New member joins (receives ticket):**
1. Ticket contains: `network_id`, `shared_secret`, `owner PublicKey`, and relay/bootstrap info
2. New member derives `network_doc_namespace(network_id, shared_secret)`
3. Opens the doc (may need to fetch it from the network first via Iroh's doc ticket mechanism)
4. Reads `sys/network/members` → verifies owner signature → discovers full member list
5. Subscribes to the doc for live updates (future membership changes propagate automatically)

**Any member detects changes:**
1. Doc live event fires for `sys/network/members` key
2. Read the new entry, verify signature
3. Compare old and new member lists
4. If local node is no longer in the list → auto-leave
5. If new members added → update gossip topic peer set
6. If members removed → update gossip topic peer set

#### Integration with Existing NetworkManager

The `NetworkManager` becomes a hybrid:
- **Local state** (`node.db`): network metadata (name, label, owner key, shared secret), folder associations
- **Synced state** (Iroh docs): member list (single source of truth for membership)

```rust
pub struct NetworkManager {
    database: NodeDatabase,                // local metadata, folders
    local_node: PublicKey,
    member_list_docs: HashMap<NetworkId, Doc>,  // live doc handles
}

impl NetworkManager {
    pub async fn join(&mut self, ticket: NetworkTicket) -> Result<NetworkId> {
        // 1. Extract network_id, shared_secret, owner from ticket
        // 2. Derive doc namespace
        // 3. Open doc: docs_engine.import_ticket(doc_ticket).await?  (or open if exists)
        // 4. Subscribe to doc for live updates
        // 5. Read sys/network/info → verify network metadata
        // 6. Read sys/network/members → verify signature → accept member list
        // 7. Save local metadata (name, label, shared_secret) to node.db
        // 8. Store doc handle for live updates
        Ok(id)
    }

    pub async fn kick(&mut self, network_id: NetworkId, member: &PublicKey) -> Result<()> {
        // 1. Verify local node IS the owner
        // 2. Read current member list from doc
        // 3. Remove member, increment sequence, sign, write to doc
        // 4. Doc sync propagates the change to all members
        Ok(())
    }

    /// Called on doc live event for sys/network/members:
    async fn on_membership_changed(&self, network_id: NetworkId, new_list: SignedMemberList) -> Result<()> {
        new_list.verify()?;
        if !new_list.members.iter().any(|m| m.key == local_node.to_string()) {
            // We've been kicked!
            self.auto_leave(network_id).await?;
        } else {
            // Update local cache and gossip peer set
            self.update_peer_set(network_id, &new_list.members).await?;
        }
        Ok(())
    }
}
```

#### Ticket Changes

The `NetworkTicket` now carries the doc ticket (so new members can find the namespace) in addition to the shared secret:

```rust
pub struct NetworkTicket {
    pub network_id: NetworkId,
    pub name: String,
    pub owner: PublicKey,
    pub shared_secret: [u8; 32],
    pub doc_ticket: String,               // Iroh DocTicket for the membership doc
    pub invited_node: Option<PublicKey>,  // None = invite-any ticket
}
```

#### Edge Cases

1. **Two owners writing concurrently**: The `sys/network/members` key is single-writer (only the owner writes). If two nodes claim to be owner, CRDT merge picks the latest write. Members should verify the owner field matches the expected owner and reject entries signed by anyone else.

2. **Owner rotates signing key**: If the owner generates a new keypair, they must issue a transition entry signed by the OLD key authorizing the NEW key. This is a future concern — initially, owner key rotation is unsupported (re-create the network if needed).

3. **Member joins while owner is offline**: The ticket-based join path still works via relay/bootstrap peers. The new member fetches the doc from any connected peer (not just the owner). Signature verification ensures authenticity even without direct owner connectivity.

4. **Shared secret compromise**: If the shared secret is leaked, an attacker can derive the doc namespace but cannot write to `sys/network/members` (only the owner's signature is accepted). They can read the member list (privacy by obscurity is weak, but acceptable for discovery).

5. **Stale sequence numbers**: If a member receives a member list with `sequence <= current_sequence`, it's rejected as a replay. The owner must always increment the sequence counter.

#### Migration from existing ticket-only model

For networks created before this change:
1. Existing networks have their member lists in `node.db` / `networks.json`
2. On owner's first sync after upgrade: read local member list, derive doc namespace, create the doc, write initial signed member list
3. Existing members with tickets: the ticket carries the doc_ticket. On next connection, the member opens the doc and discovers the canonical member list
4. Networks with no shared secret: generate one retroactively, distribute via new ticket

### Files to modify/create

| File | Change |
|---|---|
| `syncweb-core/src/net/membership_doc.rs` | **NEW** — `SignedMemberList`, `MemberEntry`, signature/verification, namespace derivation |
| `syncweb-core/src/net/mod.rs` | Add `pub mod membership_doc` |
| `syncweb-core/src/net/network.rs` | Add `doc_ticket` field to `NetworkTicket`; add doc namespace derivation |
| `syncweb-core/src/net/network_manager.rs` | Add doc-based member list sync; integrate with Iroh docs; add auto-leave on kick |
| `syncweb-core/src/daemon/daemon.rs` | Wire `DocsEngine` into `NetworkManager`; subscribe to membership docs on startup |
| `syncweb-core/src/net/network_log.rs` | Add `MemberKicked` and `MemberAdded` event types |
| `syncweb-cli/src/cli/commands.rs` | Update `network invite` output to include doc ticket |
| `syncweb-cli/src/main.rs` | Handle auto-kick notification in CLI output |

---

## Implementation Order

| Step | Gap | Depends On | Rationale |
|---|---|---|---|---|
| 1 | Migration framework (GAP 5) | Nothing | Foundation for all other DB schema changes |
| 2 | Network events/sessions tables (GAP 1) | GAP 5 (migration) | Schema changes need migration runner |
| 3 | Network bandwidth correlation (GAP 2) | GAP 5, Plan 1 (stats.db exists) | Adds column to existing table |
| 4 | Network membership propagation via docs (GAP 6) | Plan 1 (node.db exists), Iroh docs available | Uses network doc namespaces; needs NodeDatabase for local metadata |
| 5 | Sync checkpointing (GAP 4) | GAP 5, Plan 1 (node.db exists) | Schema + engine integration |
| 6 | `.syncweb-collection.json` removal (GAP 3) | Plan 1 (collections in node.db) | Simplifies code, removes duplicate state |
| 7 | Maintenance tasks (remaining GAP 5) | GAP 5 migration foundation | Vacuum + backup need databases to exist first |

---

## Files Summary

| File | Action |
|---|---|---|
| `syncweb-core/src/storage/migrate.rs` | **NEW** |
| `syncweb-core/src/net/network_log.rs` | **NEW** |
| `syncweb-core/src/net/membership_doc.rs` | **NEW** |
| `syncweb-core/src/sync/checkpoint.rs` | **NEW** |
| `syncweb-core/src/storage/node_db.rs` | Add migrations, sync checkpoints, backup/vacuum |
| `syncweb-core/src/storage/stats_db.rs` | Add migrations, network events, backup/vacuum |
| `syncweb-core/src/net/network.rs` | Add `doc_ticket` to `NetworkTicket`; add namespace derivation |
| `syncweb-core/src/net/network_manager.rs` | Add membership doc integration; add auto-leave on kick; add NetworkLogger |
| `syncweb-core/src/daemon/daemon.rs` | Wire network logger, membership docs, checkpoints, maintenance task |
| `syncweb-core/src/sync/engine.rs` | Integrate checkpoints; pass network_id to stats |
| `syncweb-core/src/indexing.rs` | Refactor to use MigrationRunner |
| `syncweb-cli/src/main.rs` | Remove .syncweb-collection.json writes; add `db` and `network events/health` commands |
| `syncweb-cli/src/cli/commands.rs` | Update `network invite` output for doc tickets |
| `syncweb-core/src/daemon/ipc.rs` | Remove .syncweb-collection.json reads |
