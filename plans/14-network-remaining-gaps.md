# Plan: Network Gaps & Remaining Storage Improvements

## Divergence

Three prior plans covered JSON→SQLite migration, ephemeral→persistent gaps, and Iroh docs audit. This plan covers everything else: network-level state gaps, the `.syncweb-collection.json` redundancy, database maintenance tooling, and partial-sync progress tracking.

## Decision

Address six distinct gaps:

1. Network health & event log — persist network connectivity history and events per-network
2. Per-network bandwidth/transfer tracking — correlate stats with network membership
3. `.syncweb-collection.json` redundancy — remove the local JSON manifest, read from blob store
4. Sync progress persistence — checkpoint partial sync/download progress to survive restarts
5. Network membership propagation via Iroh docs — replace ticket-only membership with real-time doc-synced member lists
6. Database maintenance — migration framework, vacuum, backup, integrity checks

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

New module: `syncweb-core/src/net/network_log.rs`

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

Wire into NetworkManager:

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

Wire into daemon sync cycle:

Prerequisite: `SyncEngine::sync()` must accept an optional `session_id: Option<String>` parameter and pass it through to transfer hooks so `record_transfer` can correlate bandwidth events.

```rust
// syncweb-core/src/daemon/daemon.rs — in the automatic sync loop

for (network_id, folders) in &network_folders {
    for folder_namespace in folders {
        let session_id = logger.record_sync_start(network_id, folder_namespace)?;
        let result = sync_engine.sync(
            folder_namespace,
            SessionMode::ReconcileOnce,
            Some(network_id),  // for bandwidth correlation (GAP 2)
            Some(&session_id), // for transfer tracking
        ).await;
        match result {
            Ok(stats) => {
                logger.record_sync_finish(
                    session_id,
                    stats.files_transferred,
                    stats.bytes_transferred,
                    stats.errors,
                    "completed"
                )?;
            }
            Err(e) => {
                logger.record_sync_finish(session_id, 0, 0, 1, "failed")?;
            }
        }
    }
}
```

The `SyncEngine.sync()` signature change:
```rust
pub async fn sync(
    &self,
    folder_namespace: NamespaceId,
    mode: SessionMode,
    network_id: Option<NetworkId>,    // NEW: for GAP 2 bandwidth correlation
    session_id: Option<&str>,          // NEW: for GAP 1 session tracking
) -> Result<SyncResult>;

pub struct SyncResult {
    pub files_transferred: u64,
    pub bytes_transferred: u64,
    pub errors: u64,
    pub new_entries: u64,
    pub conflicts_resolved: u64,
}
```

Breaking change note: This changes the public API of `SyncEngine`. All call sites
must be updated:
- Daemon path (GAP 7): passes `network_id` and `session_id` as shown above.
- CLI direct path (non-daemon `syncweb sync`): passes `None` for both
  `network_id` and `session_id`. The CLI's direct sync path does not participate in
  network-scoped bandwidth correlation or session tracking — those are daemon-only
  concerns. This is a backward-compatible default: `None` means "no network context".

Expose via CLI:

New subcommand: `syncweb network events <network-id> [--limit N]`
New subcommand: `syncweb network health [--network <id>]`

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/net/network_log.rs` | NEW — NetworkLogger implementation |
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

The `network_id` column is already present in `bandwidth_events` from `02-json-to-sqlite-migration.md`. Add indexes and the aggregate view:

```sql
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
1. In the Iroh blob store — content-addressed, integrity-guaranteed (the canonical source)
2. As `.syncweb-collection.json` — JSON file written alongside the package source directory (`syncweb-cli/src/main.rs:2614`)

The JSON file is a convenience copy that lets `package export` and `package info` read the manifest without hitting the blob store. But this creates a consistency risk: the blob in the store and the JSON file can diverge (e.g., if the user edits the JSON file, or if the package is imported from a ticket where no local manifest file was written).

### Files

| File | Line | Operation |
|------|------|-----------|
| `syncweb-cli/src/main.rs` | 2599-2601 | `manifest_path()` returns `<path>/.syncweb-collection.json` |
| `syncweb-cli/src/main.rs` | 2613-2616 | `save_manifest()` writes `manifest.to_bytes()` (JSON) to disk |
| `syncweb-core/src/daemon/ipc.rs` | 1191-1192 | `tokio::fs::read(&manifest_path)` reads it back |

### Fix

Remove the local JSON file entirely. Manifests are already in the blob store. Reading from the blob store is fast (local hashing + disk read). The blob store is the canonical, content-addressed source.

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

New module: `syncweb-core/src/sync/checkpoint.rs`

```rust
pub struct SyncCheckpoint {
    namespace_id: NamespaceId,
    session_id: String,
    database: NodeDatabase,
}

impl SyncCheckpoint {
    pub fn new(database: &NodeDatabase, namespace_id: NamespaceId) -> Result<Self>;

    /// Create a session (total_entries is unknown for streaming; starts at 0).
    pub fn create_session(&self) -> Result<String>;

    /// Mark an entry as completed (downloaded successfully).
    pub fn mark_completed(&self, entry_key: &[u8], hash: Hash, size: u64) -> Result<()>;

    /// Mark an entry as failed with error message.
    pub fn mark_failed(&self, entry_key: &[u8], error: &str) -> Result<()>;

    /// Mark an entry as skipped (already present locally or deleted).
    pub fn mark_skipped(&self, entry_key: &[u8], hash: Hash, size: u64) -> Result<()>;

    /// Get all completed entries for this session (for resume filtering).
    pub fn completed_entries(&self) -> Result<Vec<EntryProgress>>;

    /// Get all failed entries for this session (for retry).
    pub fn failed_entries(&self) -> Result<Vec<EntryProgress>>;

    /// Get overall progress.
    pub fn progress(&self) -> Result<CheckpointProgress>;

    /// Mark session as completed.
    pub fn complete(&self) -> Result<()>;

    /// Mark session as incomplete (has leftovers for next resume).
    pub fn mark_incomplete(&self) -> Result<()>;

    /// Load the most recent unfinished checkpoint for a folder.
    pub fn resume(namespace_id: NamespaceId) -> Result<Option<Self>>;
}

pub struct EntryProgress {
    pub entry_key: Vec<u8>,
    pub hash: Hash,
    pub size: u64,
    pub status: String,
    pub retries: u32,
    pub error_message: Option<String>,
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

Sync is streaming — entries arrive via `LiveEvent`, not as a pre-known total. The checkpoint must handle incremental discovery:

```rust
// syncweb-core/src/sync/engine.rs — in run_intent()

async fn run_intent(folder: SyncwebFolder, mode: SessionMode, ...) -> Result<()> {
    // Check for an unfinished checkpoint on previous session
    let maybe_cp = SyncCheckpoint::resume(folder.namespace_id())?;
    let mut completed: HashSet<Vec<u8>> = HashSet::new();
    let mut failed: Vec<(Vec<u8>, String)> = Vec::new();
    let mut checkpoint = None;

    if let Some(cp) = maybe_cp {
        tracing::info!("resuming sync checkpoint: {}", cp.session_id());
        // Replay completed entries from the checkpoint so we don't re-download
        for entry in cp.completed_entries()? {
            completed.insert(entry.entry_key);
        }
        // Re-queue failed entries for retry (up to max_retries)
        for entry in cp.failed_entries()? {
            if entry.retries < 3 {
                failed.push((entry.entry_key, entry.error_message));
            }
        }
        checkpoint = Some(cp);
    }

    // Subscribe to live events and process entries as they arrive
    let mut stream = docs_engine.watch(folder.doc()).await?;
    while let Some(event) = stream.next().await {
        // NOTE: The exact fields of LiveEvent depend on the iroh-docs version.
        // Current iroh-docs exports LiveEvent via iroh::docs::store::LiveEvent.
        // The Insert variant provides entry content (key, hash, size) through
        // an Entry trait, not as raw fields. Actual destructuring:
        //   LiveEvent::InsertLocal { entry } or LiveEvent::InsertRemote { entry, .. }
        // Adjust this code during implementation to match the actual API.
        match event {
            LiveEvent::Insert { key, hash, size } => {
                if completed.contains(&key) {
                    continue; // already handled in checkpoint
                }
                // Check if blob already exists locally
                let already_local = blob_store.has(hash).await.unwrap_or(false);
                if already_local {
                    if let Some(ref cp) = checkpoint {
                        cp.mark_skipped(&key, hash, size)?;
                    }
                    continue;
                }
                // Need to download
                match download_entry(key, hash, size).await {
                    Ok(_) => {
                        completed.insert(key.clone());
                        if let Some(ref cp) = checkpoint {
                            cp.mark_completed(&key, hash, size)?;
                        }
                    }
                    Err(e) => {
                        failed.push((key.clone(), e.to_string()));
                        if let Some(ref cp) = checkpoint {
                            cp.mark_failed(&key, &e.to_string())?;
                        }
                    }
                }
            }
            LiveEvent::Delete { key } => {
                // Remove from checkpoint if present
                if let Some(ref cp) = checkpoint {
                    cp.mark_skipped(&key, Hash::default(), 0)?;
                }
            }
            LiveEvent::SyncFinished => break,
            _ => {}
        }
    }

    if let Some(ref cp) = checkpoint {
        if failed.is_empty() {
            cp.complete()?;
        } else {
            cp.mark_incomplete()?; // leave for next resume attempt
        }
    }
}
```

Key design points:
- No upfront total count — entries are discovered incrementally via `LiveEvent`
- Resume by filtering — completed entries from the checkpoint are skipped
- Failed entries have retry limit — after 3 failures, the sync fails (no infinite retry)
- Already-local blobs are skipped — check `blob_store.has(hash)` before downloading

### Cleanup

On successful completion, delete the checkpoint records (they're transient operational state). On daemon startup, check for dangling checkpoints and clean up sessions older than 7 days (stale/crashed sessions).

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/sync/checkpoint.rs` | NEW — SyncCheckpoint implementation |
| `syncweb-core/src/sync/mod.rs` | Add `pub mod checkpoint` |
| `syncweb-core/src/sync/engine.rs` | Integrate checkpoint into `run_intent()` |
| `syncweb-core/src/storage/node_db.rs` | Add sync_checkpoints schema; add CRUD methods |

---

## GAP 5: Database Maintenance Framework

### Rationale

With 3 SQLite databases (the ones created in Plan 02: `indexing.sqlite`, `node.db`, `stats.db`), the project needs:
- Periodic VACUUM for space reclamation
- Backup tooling
- Integrity verification

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
let indexing_db = self.indexing_db.clone();
tokio::spawn(async move {
    loop {
        tokio::time::sleep(maintenance_interval).await;
        for db in [&node_db, &stats_db, &indexing_db] {
            if let Ok(count) = db.freelist_count() {
                if count > 100 {
                    let _ = db.vacuum();
                }
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
|---|---|---|
| `syncweb-core/src/storage/node_db.rs` | Add vacuum/check/backup methods |
| `syncweb-core/src/storage/indexing_db.rs` | Add vacuum/check/backup methods |
| `syncweb-core/src/storage/stats_db.rs` | Add vacuum/check/backup methods |
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

- Kicked members continue trying to connect until they time out or manually leave
- Newly added members are invisible to existing peers until they exchange tickets manually
- Two nodes with tickets from different points in time may have different member lists, causing confusion about who is in the network
- No single source of truth — each node's local copy can diverge
- Network members can't see the full member list without asking the owner for a fresh ticket

### Fix: Signed Membership Doc

Store network membership as a signed document entry in a per-network Iroh docs namespace. Every network gets a dedicated doc. The owner writes a signed membership list; all members sync the doc and verify signatures.

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
/// This derivation is used only by the OWNER at network creation time.
/// New members do NOT derive the namespace — they receive it via the doc_ticket
/// in the NetworkTicket. The derivation exists so the owner can pre-compute
/// the namespace before creating the doc, and to verify that a received
/// doc_ticket maps to the expected network (defense in depth).
///
/// Security note: The shared_secret provides privacy-by-obscurity for the
/// namespace. An attacker who knows the network_id but not the shared_secret
/// cannot derive the namespace to probe for the doc. However, the doc_ticket
/// is the authoritative reference for the doc location — if derivation and
/// ticket disagree, trust the ticket.
pub fn network_doc_namespace(network_id: NetworkId, shared_secret: &[u8; 32]) -> NamespaceId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"syncweb/network-doc/v1\0");
    hasher.update(network_id.as_bytes());
    hasher.update(shared_secret);
    NamespaceId::from_bytes(hasher.finalize().into())
}
```

Owner pre-computes at network creation:
```rust
// In NetworkManager::create()
let shared_secret = generate_secret();
let doc_namespace = network_doc_namespace(network_id, &shared_secret);
let doc = docs_engine.create_doc(doc_namespace).await?;
let doc_ticket = doc.share(ShareMode::Read, AddrInfoOptions::default()).await?.to_string();

// Write initial membership
let members = SignedMemberList {
    network_id: network_id.to_string(),
    owner: local_key.public().to_string(),
    sequence: 1,
    members: vec![MemberEntry { key: local_key.public().to_string(), joined_at: now(), role: MemberRole::Admin }],
    updated_at: now(),
    signature: String::new(),
};
members.sign(&local_key)?;
doc.set_bytes(author, b"sys/network/members", serde_json::to_vec(&members)?).await?;
doc.set_bytes(author, b"sys/network/info", serde_json::to_vec(&NetworkInfo { ... })?).await?;

// Store in node.db
database.insert_network(NetworkRecord {
    id: network_id,
    name,
    owner: local_key.public(),
    shared_secret,
    doc_ticket,  // <-- PRE-COMPUTED
    created_at: now(),
})?;
```

New member joins with ticket (no derivation needed):
```rust
// In NetworkManager::join(ticket)
let doc = docs_engine.import_ticket(ticket.doc_ticket).await?;  // Direct!
let members_bytes = doc.get(author, b"sys/network/members").await?.unwrap().content_bytes();
let members: SignedMemberList = serde_json::from_slice(&members_bytes)?;
members.verify()?;
// Verify we're in the member list (or it's an invite-any network)
```

This eliminates the circular dependency entirely. The doc_ticket is the single source of truth for the doc location.

#### Integration with Existing NetworkManager

The `NetworkManager` becomes a hybrid:
- Local state (`node.db`): network metadata (name, label, owner key, shared secret), folder associations
- Synced state (Iroh docs): member list (single source of truth for membership)

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

The `NetworkTicket` now carries the doc ticket (so new members can find the namespace) in addition to the shared secret. The doc_ticket is generated by the owner at network creation time and embedded in every invite.

```rust
pub struct NetworkTicket {
    pub network_id: NetworkId,
    pub name: String,
    pub owner: PublicKey,
    pub shared_secret: [u8; 32],
    pub doc_ticket: String,               // Iroh DocTicket for the membership doc (pre-computed by owner)
    pub invited_node: Option<PublicKey>,  // None = invite-any ticket
}
```

Breaking the circular dependency:
1. Owner creates network → generates `shared_secret` → derives `doc_namespace` → creates doc → gets `doc_ticket`
2. Owner stores `doc_ticket` in `networks` table (new column)
3. When inviting, owner includes `doc_ticket` in the `NetworkTicket`
4. New member receives ticket → has `shared_secret` (to verify) AND `doc_ticket` (to open doc directly)
5. No derivation needed at join time — the doc_ticket is the authoritative reference

#### Edge Cases

1. Two owners writing concurrently: The `sys/network/members` key is single-writer (only the owner writes). If two nodes claim to be owner, CRDT merge picks the latest write. Members should verify the owner field matches the expected owner and reject entries signed by anyone else.

2. Owner rotates signing key: If the owner generates a new keypair, they must issue a transition entry signed by the OLD key authorizing the NEW key. This is a future concern — initially, owner key rotation is unsupported (re-create the network if needed).

3. Member joins while owner is offline: The ticket-based join path still works via relay/bootstrap peers. The new member fetches the doc from any connected peer (not just the owner). Signature verification ensures authenticity even without direct owner connectivity.

4. Shared secret compromise: If the shared secret is leaked, an attacker can derive the doc namespace but cannot write to `sys/network/members` (only the owner's signature is accepted). They can read the member list (privacy by obscurity is weak, but acceptable for discovery).

5. Stale sequence numbers: If a member receives a member list with `sequence <= current_sequence`, it's rejected as a replay. The owner must always increment the sequence counter.

#### Migration from existing ticket-only model

For networks created before this change:
1. Existing networks have their member lists in `node.db` / `networks.json`
2. On owner's first sync after upgrade: read local member list, derive doc namespace, create the doc, write initial signed member list
3. Owner generates `doc_ticket` and stores it in `networks` table
4. Existing members with tickets: the ticket format is extended to include `doc_ticket`. On next connection, the member opens the doc and discovers the canonical member list
5. Networks with no shared secret: generate one retroactively, distribute via new ticket (requires owner to re-invite)

CLI changes:
- `syncweb network invite` now outputs a ticket containing `doc_ticket`
- Old tickets without `doc_ticket` still work (fallback to derivation) but log a deprecation warning
- `syncweb network join` accepts both old and new ticket formats

### Files to modify/create

| File | Change |
|---|---|
| `syncweb-core/src/net/membership_doc.rs` | NEW — `SignedMemberList`, `MemberEntry`, signature/verification, namespace derivation |
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
|---|---|---|---|---|---|
| 1 | Network events/sessions tables (GAP 1) | Plan 1 (node.db, stats.db exist) | Schema is defined upfront in each database open |
| 2 | Network bandwidth correlation (GAP 2) | Plan 1 (stats.db exists) | Adds column to existing table |
| 3 | Network membership propagation via docs (GAP 6) | Plan 1 (node.db exists), Iroh docs available | Uses network doc namespaces; needs NodeDatabase for local metadata |
| 4 | Sync checkpointing (GAP 4) | Plan 1 (node.db exists) | Schema + engine integration |
| 5 | `.syncweb-collection.json` removal (GAP 3) | Plan 1 (collections in node.db) | Simplifies code, removes duplicate state |
| 6 | Maintenance tasks (remaining GAP 5) | None | Vacuum + backup are standalone utilities |

---

## Files Summary

| File | Action |
|---|---|---|
| `syncweb-core/src/net/network_log.rs` | NEW |
| `syncweb-core/src/net/membership_doc.rs` | NEW |
| `syncweb-core/src/sync/checkpoint.rs` | NEW |
| `syncweb-core/src/storage/node_db.rs` | Add sync checkpoints, backup/vacuum |
| `syncweb-core/src/storage/stats_db.rs` | Add network events, backup/vacuum |
| `syncweb-core/src/net/network.rs` | Add `doc_ticket` to `NetworkTicket`; add namespace derivation |
| `syncweb-core/src/net/network_manager.rs` | Add membership doc integration; add auto-leave on kick; add NetworkLogger |
| `syncweb-core/src/daemon/daemon.rs` | Wire network logger, membership docs, checkpoints, maintenance task |
| `syncweb-core/src/sync/engine.rs` | Integrate checkpoints; pass network_id to stats |
| `syncweb-cli/src/main.rs` | Remove .syncweb-collection.json writes; add `db` and `network events/health` commands |
| `syncweb-cli/src/cli/commands.rs` | Update `network invite` output for doc tickets |
| `syncweb-core/src/daemon/ipc.rs` | Remove .syncweb-collection.json reads |

---

## GAP 7: Daemon Integration of NetworkManager

### Current State

`NetworkManager` is only instantiated by the CLI (`syncweb-cli/src/main.rs:2942`) for synchronous commands like `network create`, `network join`, `network leave`, etc. It is never present in the daemon process. The daemon constructs `IdentityManager`, `IrohNode`, `FolderManager`, `SyncEngine`, `ManagedPool`, `IpcServer`, and `IntentSupervisor` — but no `NetworkManager`.

### Impact

- All network state (`members`, `folders`, `shared_secret`, invite management) is invisible to the daemon
- The gossip topic subscription (`NetworkManager::subscribe()`) is never called — networks have no real-time membership presence
- Network membership docs (GAP 6) cannot be opened or synced without a daemon-side doc handle
- Network events (GAP 1) and bandwidth correlation (GAP 2) have no daemon-side trigger — the `NetworkLogger` never receives events
- Network-scoped access control (GAP 8) has no enforcement point
- CLI `network join` creates a network and stores it in `node.db`, but the daemon never sees it until a full restart re-reads the database

### Fix

Add `NetworkManager` to the daemon's `DaemonInner` struct and wire it through startup and all network-relevant code paths.

```rust
// syncweb-core/src/daemon/daemon.rs

pub struct DaemonInner {
    // ... existing fields ...
    pub network_manager: Arc<tokio::sync::RwLock<NetworkManager>>,
}

impl DaemonInner {
    async fn new(config: &DaemonConfig) -> Result<Self> {
        // ... existing setup ...
        let network_manager = NetworkManager::open(&node_db, local_node_id)?;
        Ok(DaemonInner { network_manager: Arc::new(tokio::sync::RwLock::new(network_manager)), ... })
    }

    async fn run_inner(self: Arc<Self>) -> Result<()> {
        // On startup: subscribe to gossip for every existing network
        for network in self.network_manager.read().await.list()? {
            self.gossip_service.subscribe_bookmarked_topics(&[network.topic])?;
        }

        // On startup: open membership docs for live sync (GAP 6)
        for network in self.network_manager.read().await.list()? {
            if let Some(doc_ticket) = network.doc_ticket() {
                self.docs_engine.import_ticket(doc_ticket).await?;
                self.docs_engine.watch(network.membership_doc()).await?;
            }
        }

        // Wire network_id into the sync cycle
        self.sync_engine.set_network_resolver(move |namespace_id: NamespaceId| {
            self.network_manager.read().await.network_for_folder(namespace_id)
        });

        // ... rest of run_inner ...
    }
}
```

Wiring network_id into sync intents:

```rust
// In DaemonInner::run_folder_intent():
let network_id = net_mgr.network_for_folder(&folder.namespace_id());
self.network_logger.record_sync_start(&network_id, folder.namespace_id())?;
let result = self.sync_engine.sync(folder.doc(), network_id, mode).await;
match result {
    Ok(stats) => self.network_logger.record_sync_finish(session_id, stats.files, stats.bytes, 0, "completed")?,
    Err(e) => self.network_logger.record_sync_finish(session_id, 0, 0, 1, "failed")?,
}
```

Expose via IPC for CLI commands that need runtime state:

```rust
// syncweb-core/src/daemon/ipc.rs

IpcRequest::NetworkInvite { network_id, device } => {
    let mut net_mgr = self.network_manager.write().await;
    let ticket = net_mgr.invite(&network_id, device)?;
    Ok(IpcResponse::NetworkTicket { ticket: ticket.encode() })
}

IpcRequest::NetworkKick { network_id, device } => {
    let mut net_mgr = self.network_manager.write().await;
    net_mgr.kick(&network_id, &device)?;
    Ok(IpcResponse::Ok)
}

IpcRequest::NetworkLeave { network_id } => {
    let mut net_mgr = self.network_manager.write().await;
    net_mgr.leave(&network_id)?;
    Ok(IpcResponse::Ok)
}
```

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/daemon/daemon.rs` | Add `NetworkManager` field; wire through `new()`, `run_inner()`, `run_folder_intent()`, `run_collection_intent()` |
| `syncweb-core/src/daemon/ipc.rs` | Add network mutation IPC handlers; route `NetworkInvite`, `NetworkKick`, `NetworkLeave` to daemon |
| `syncweb-core/src/net/network_manager.rs` | Add `network_for_folder()` lookup; add `open()` constructor for daemon use; add `list()` iterator |
| `syncweb-core/src/sync/engine.rs` | Accept optional `network_id` parameter in sync/reconcile methods |

---

## GAP 8: Network Access Control & Isolation

### Current State

Blob access (`blob_store.rs`), folder operations (`folder/`), and sync engine (`sync/engine.rs`) have zero awareness of networks. There is no access control check that gates blob downloads, folder entry listing, or sync participation on network membership. A peer can:

- Read any folder's doc if they possess the namespace capability, regardless of network association
- Download any blob if they know the hash, regardless of which network the blob "belongs to"
- Discover peers and sync data for any folder, even ones not associated with their networks

### Impact

- No data isolation between networks — blobs and folders are globally accessible
- A user who joins network "work" can potentially access data from network "personal" if they discover the namespace ID
- Invite-only networks provide zero security — the `shared_secret` is generated but never checked, and anyone with a gossiped namespace ID can sync the folder
- Multi-tenant or organization-separated deployments are impossible
- Compliance with data isolation requirements (e.g., per-project access boundaries) cannot be met

### Why This Matters

The fundamental purpose of a "network" in Syncweb is to group peers and folders into trust boundaries. Without access control, networks are reduced to cosmetic labels. A project collaboration network, a family sharing network, and a public distribution network must not leak data to each other.

### Fix: Three-Layer Access Control

#### Layer 1: Folder Association Enforcement

A namespace can only be synced if it is explicitly associated with a network the local node belongs to. On daemon startup, the folder registry only includes folders whose `NamespaceId` appears in at least one `network_folders` row where the local node is a `network_members` entry:

```rust
impl NetworkManager {
    /// Returns true if the local node is a member of at least one network
    /// that has the given folder namespace associated with it.
    pub fn can_access_folder(&self, namespace_id: &NamespaceId) -> Result<bool> {
        let local_key = self.local_node.to_string();
        self.database.with_connection(|conn| -> Result<bool> {
            let mut stmt = conn.prepare(
                "SELECT 1 FROM network_folders nf
                 JOIN network_members nm ON nf.network_id = nm.network_id
                 WHERE nf.namespace_id = ?1 AND nm.member = ?2
                 LIMIT 1"
            )?;
            Ok(stmt.exists(params![namespace_id.to_string(), &local_key])?)
        })
    }
}
```

#### Layer 2: Blob Download Gating

Before downloading a blob, verify the blob is referenced by a folder entry in at least one network the local node belongs to. This prevents blind hash-based downloading of blobs that have leaked via side channels:

```rust
pub struct NetworkContext {
    network_manager: Arc<tokio::sync::RwLock<NetworkManager>>,
    network_id: NetworkId,
}

impl NetworkContext {
    /// Check if the given blob hash appears as an entry in any folder
    /// associated with this network.
    pub async fn can_access_blob(&self, hash: &Hash) -> Result<bool> {
        let mgr = self.network_manager.read().await;
        let folders = mgr.folders_for_network(&self.network_id)?;
        for namespace_id in folders {
            let doc = self.docs_engine.open_doc(namespace_id).await?;
            let entries = self.docs_engine.list_latest(&doc).await?;
            for entry in entries {
                if entry.hash == *hash {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}
```

Optimization: Instead of scanning doc entries per download, build a reverse index on sync completion — a `blob_folders` table in SQLite that maps `(hash, namespace_id)` to associate a blob with all folders that reference it, then join through `network_folders` for network membership:

```sql
-- Blob→folder index (populated on sync completion, incremental update on LiveEvent::Insert)
CREATE TABLE blob_folders (
    content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
    namespace_id TEXT NOT NULL,
    entry_key BLOB NOT NULL,
    added_at INTEGER NOT NULL,
    PRIMARY KEY(content_hash, namespace_id, entry_key)
);
CREATE INDEX idx_blob_folders_hash ON blob_folders(content_hash);

-- O(1) lookup: is this blob accessible in any of my networks?
-- SELECT 1 FROM blob_folders bf
-- JOIN network_folders nf ON bf.namespace_id = nf.namespace_id
-- JOIN network_members nm ON nf.network_id = nm.network_id
-- WHERE bf.content_hash = ? AND nm.member = ?
-- LIMIT 1
```

Performance requirement: The `blob_folders` reverse index MUST be built BEFORE
enabling access control checks on blob downloads. The initial O(n) scan (iterating all
doc entries) is only acceptable during the index build phase. Once the index is
populated, all access checks use the O(1) SQL query. The index is maintained
incrementally: on each `LiveEvent::Insert`, insert into `blob_folders`; on
`LiveEvent::Delete`, delete from `blob_folders`.

#### Layer 3: Peer Discovery Scoping

Gossip peer discovery should be scoped to network membership. When syncing a folder for network X, only bootstrap from peers that are members of network X (not all connected peers):

```rust
let members = self.network_manager.members_of_network(&network_id)?;
let peer_ids: Vec<PeerId> = members.iter()
    .filter_map(|pk| self.peer_map.lookup(pk))
    .collect();
self.docs_engine.start_sync(folder.doc(), peer_ids).await?;
```

### Edge Cases

1. Public blobs / read-only subscriptions: Public blobs subscribed via `subscribe_public` should be accessible without network membership (they are intentionally public).

2. Cross-network folder sharing: If a folder needs to be in two networks intentionally (e.g., "team-x" and "team-y" both get the docs folder), both networks must explicitly add the folder. The `network_folders` composite primary key `(network_id, namespace_id)` supports this.

3. Owner-only content: Invite-only network members who aren't the owner should not have owner-level capabilities (kick, modify member list). The `SignedMemberList.owner` field from GAP 6 is verified before writes.

4. Performance: The blob access check at Layer 2 adds overhead. Mitigate by caching the reverse index in memory (populated on sync completion) and building a `blob_network_index` table for O(1) lookups.

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/net/network_manager.rs` | Add `can_access_folder()`, `networks_for_folder()`, `folders_for_network()`, `members_of_network()` |
| `syncweb-core/src/node/blob_store.rs` | Add `download_with_network()` gated by `NetworkContext` |
| `syncweb-core/src/sync/engine.rs` | Accept `NetworkContext`; gate sync initiation on `can_access_folder()`; scope peer discovery |
| `syncweb-core/src/daemon/daemon.rs` | Construct `NetworkContext` before calls to `SyncEngine`; pass context to blob downloads |
| `syncweb-core/src/net/network_context.rs` | NEW — `NetworkContext` type with `can_access_blob()` and network-scoped operations |
| `syncweb-core/src/storage/node_db.rs` | Add `blob_folders` table for reverse-lookup optimization; maintain on sync completion |

---

## Test Plan: Network Isolation & Integration

This section defines tests that verify the network isolation and integration behavior described in GAPs 6--8, plus the existing gaps 1--5. These are integration tests that spin up real daemon instances with Iroh nodes — distinct from the existing `network_test.rs` which tests only the data model CRUD in isolation.

### Test Suite: Network Isolation (`syncweb-core/tests/network_isolation_test.rs`)

#### Test 1: Cross-network blob isolation

Setup:
1. Spin up 3 IrohNode instances: owner-A, member-A, member-B (all on relay)
2. Create network "alpha" (owner-A)
3. Create network "beta" (owner-A)
4. member-A joins "alpha" only; member-B joins "beta" only
5. Owner-A creates folder "alpha-docs" → add to network "alpha"
6. Owner-A creates folder "beta-docs" → add to network "beta"
7. Owner-A adds blob `x` to "alpha-docs" folder (write to alpha doc)
8. Owner-A adds blob `y` to "beta-docs" folder (write to beta doc)

Assertions:
- member-A can list entries and download blobs from "alpha-docs"
- member-A cannot list entries from "beta-docs" (not a member of beta)
- member-B can list entries and download blobs from "beta-docs"
- member-B cannot list entries from "alpha-docs"

#### Test 2: Same namespace label, different networks, no data leakage

Setup:
1. Spin up 4 IrohNode instances: owner-A, owner-B, member-A, member-B
2. Create network "team-a" (owner-A) and network "team-b" (owner-B)
3. owner-A creates folder "docs" and adds to "team-a"
4. owner-B creates folder "docs" (a different namespace ID) and adds to "team-b"
5. member-A joins "team-a"; member-B joins "team-b"
6. owner-A writes blob with content `"team a content"` to "team-a/docs"
7. owner-B writes blob with content `"team b content"` to "team-b/docs"

Assertions:
- member-A syncs "docs" and gets content `"team a content"` (NOT `"team b content"`)
- member-B syncs "docs" and gets content `"team b content"` (NOT `"team a content"`)
- Network "team-a" members cannot enumerate or sync folders that are only in "team-b"
- The `network_folders` table correctly isolates the two docs entries

#### Test 3: Blob content identical across networks, access still isolated

Setup:
1. Spin up 3 IrohNode instances: owner-A, member-A, member-B
2. Create network "alpha" and "beta" (owner-A)
3. member-A joins "alpha"; member-B joins "beta"
4. owner-A creates folder "alpha-docs" → network "alpha"
5. owner-A creates folder "beta-docs" → network "beta"
6. Add identical blob content `b"shared-content"` to both folders (same hash, same bytes)
7. Verify both folders reference the same blob hash

Assertions:
- member-A can retrieve the blob through "alpha-docs" (access gated by folder membership)
- member-B can retrieve the blob through "beta-docs"
- member-A cannot discover or enumerate "beta-docs" entries (folder is not in their networks)
- The blob itself is content-addressed (not network-scoped), but access to the doc that references it is network-gated

#### Test 4: Invite-only network rejects unauthorized access

Setup:
1. Spin up 2 IrohNode instances: owner-A, outsider
2. Create invite-only network "secure" (owner-A, `invite_only = true`)
3. owner-A creates folder "secure-docs" → network "secure"
4. owner-A adds blob `secret.txt`
5. outsider discovers the folder namespace via gossip but does NOT possess a valid ticket

Assertions:
- outsider cannot open the folder doc (namespace capability not granted)
- outsider cannot fetch blobs from the doc's entries
- After GAP 6 (signed membership docs): outsider cannot verify membership in the network
- `NetworkManager::can_access_folder()` returns `false` for the outsider

#### Test 5: Network membership propagation (post GAP 6)

Setup:
1. Spin up 3 IrohNode instances: owner-A, member-X, member-Y
2. Create network "team" (owner-A) with membership doc
3. member-X joins via ticket → membership doc is synced
4. member-X and member-Y are connected via relay

Assertions:
- owner-A invites member-Y → writes `SignedMemberList` with both X and Y as members
- member-X's membership doc live event fires → member-X sees new member Y in list
- member-X's gossip peer set is updated to include member-Y
- owner-A kicks member-X → writes `SignedMemberList` without X
- member-X receives the update → verifies signature → auto-leaves network
- member-X's folder access for network folders is revoked within 5 seconds

### Test Suite: Daemon Integration (`syncweb-cli/tests/daemon_integration_test.rs` additions)

#### Test: Two daemons on different networks

Setup:
1. Start daemon-A with network "alpha" and folder "alpha-docs"
2. Start daemon-B with network "beta" and folder "beta-docs"
3. Both daemons connected to the same relay
4. Add blob content `"alpha-data"` to daemon-A's "alpha-docs"
5. Add blob content `"beta-data"` to daemon-B's "beta-docs"

Assertions:
- daemon-A's blob does not appear in daemon-B's blob store
- daemon-B's blob does not appear in daemon-A's blob store
- IPC `list` from daemon-A shows only "alpha-docs", not "beta-docs"
- IPC `network list` from daemon-A shows only "alpha"

#### Test: Daemon sync respects network scoping on restart

Setup:
1. Start daemon with network "team", folder "docs" in network
2. Add blobs to "docs", sync completes
3. Restart the daemon
4. On startup, daemon reads `network_folders` and `network_members` from `node.db`

Assertions:
- `can_access_folder("docs")` returns `true` after restart
- `can_access_folder("unrelated-folder")` returns `false` (folder exists but not in any network)
- Sync engine only reconciles folders with valid network access
- No sync is initiated for folders outside the node's networks

### Test Suite: Bandwidth & Event Logging (post GAP 1 & 2)

#### Test: Network events recorded on membership changes

Assertions:
- `network_events` table has `member_added` event for join
- `network_events` table has `member_removed` event for kick
- `network_events` table has `ticket_created` event for invite
- `network_events` table has `ticket_accepted` event for join

#### Test: Bandwidth attributed to correct network

Setup:
1. Create networks "alpha" and "beta"
2. Add 10MB blob to "alpha" folder, sync with "alpha" member
3. Add 5MB blob to "beta" folder, sync with "beta" member

Assertions:
- `network_bandwidth_summary` shows ~10MB for "alpha", ~5MB for "beta"
- No bandwidth from "alpha" sync is attributed to "beta"
- Per-peer bandwidth records have the correct `network_id`

### Test Suite: Sync Checkpointing (post GAP 4)

#### Test: Checkpoint survives daemon restart

Setup: Create folder with 100 blobs, kill daemon after 40 completed.

Assertions:
- `sync_entry_progress` has 40 entries `completed`, 60 `pending`
- After resume, only 60 blobs are downloaded (not all 100)
- `sync_checkpoints` status transitions: `running` → `running` (after restart) → `completed`

#### Test: Failed entries retried on resume

Assertions:
- Failed entries recorded with `retries > 0`
- On resume, failed entries are re-attempted
- `sync_checkpoints.failed_entries` decrements as retries succeed

### Files to create/modify for tests

| File | Action |
|---|---|
| `syncweb-core/tests/integration/network_isolation_test.rs` | NEW — Tests 1--5 defined above |
| `syncweb-core/tests/integration/daemon_integration_test.rs` | Add network-aware daemon tests |
| `syncweb-core/tests/integration/network_test.rs` | Add bandwidth correlation and event logging assertions |
| `syncweb-core/tests/integration/sync_test.rs` | Add checkpoint resume tests |
| `syncweb-cli/tests/cli_test.rs` | Add CLI test: network create with same name across separate daemon instances |

---

## Implementation Order (Updated)

| Step | Gap | Depends On | Rationale |
|---|---|---|---|
| 1 | Network events/sessions tables (GAP 1) | Plan 1 (node.db, stats.db exist) | Schema is defined upfront in each database open |
| 2 | Network bandwidth correlation (GAP 2) | Plan 1 (stats.db exists) | Adds column to existing table |
| 3 | Daemon integration of NetworkManager (GAP 7) | GAP 1 (node.db has network tables) | Without daemon integration, GAPs 6 and 8 have no runtime context |
| 4 | Network membership propagation via docs (GAP 6) | GAP 7 (NetworkManager in daemon) | Network docs need daemon-side NetworkManager for doc lifecycle |
| 5 | Network access control & isolation (GAP 8) | GAP 7 (NetworkManager in daemon), GAP 6 (signed membership lists) | Access checks depend on daemon runtime; signed lists provide verifiable membership |
| 6 | Sync checkpointing (GAP 4) | Plan 1 (node.db exists) | Schema + engine integration |
| 7 | `.syncweb-collection.json` removal (GAP 3) | Plan 1 (collections in node.db) | Simplifies code, removes duplicate state |
| 8 | Maintenance tasks (GAP 5) | None | Vacuum + backup are standalone utilities |
| 9 | Network isolation & integration tests | GAPs 1--8 | Write tests alongside each gap, not deferred to end |

---

## Files Summary (Updated)

| File | Action |
|---|---|
| `syncweb-core/src/net/network_log.rs` | NEW |
| `syncweb-core/src/net/membership_doc.rs` | NEW |
| `syncweb-core/src/net/network_context.rs` | NEW |
| `syncweb-core/src/sync/checkpoint.rs` | NEW |
| `syncweb-core/src/storage/node_db.rs` | Add sync checkpoints, blob_folders table, backup/vacuum; add network folder/member lookup queries |
| `syncweb-core/src/storage/stats_db.rs` | Add network events, backup/vacuum; add `network_id` to bandwidth_events |
| `syncweb-core/src/net/network.rs` | Add `doc_ticket` to `NetworkTicket`; add namespace derivation |
| `syncweb-core/src/net/network_manager.rs` | Add membership doc integration; add auto-leave on kick; add NetworkLogger; add `can_access_folder()`, `networks_for_folder()`, `folders_for_network()`, `members_of_network()`; add `open()` and `network_for_folder()` for daemon use |
| `syncweb-core/src/daemon/daemon.rs` | Add `NetworkManager` field; wire through `new()`, `run_inner()`, `run_folder_intent()`; construct `NetworkContext` for sync calls; subscribe to network gossip on startup |
| `syncweb-core/src/daemon/ipc.rs` | Remove .syncweb-collection.json reads; add network mutation IPC handlers |
| `syncweb-core/src/sync/engine.rs` | Integrate checkpoints; accept `NetworkContext`; pass `network_id` to stats; gate sync on `can_access_folder()` |
| `syncweb-core/src/node/blob_store.rs` | Add `download_with_network()` gated by `NetworkContext` |
| `syncweb-cli/src/main.rs` | Remove .syncweb-collection.json writes; add `db` and `network events/health` commands |
| `syncweb-cli/src/cli/commands.rs` | Update `network invite` output for doc tickets |
| `syncweb-core/tests/network_isolation_test.rs` | NEW — Cross-network isolation tests |
| `syncweb-cli/tests/daemon_integration_test.rs` | Add network-aware daemon tests |
| `syncweb-core/tests/network_test.rs` | Add bandwidth/event logging assertions |
| `syncweb-core/tests/sync_checkpoint_test.rs` | Add checkpoint resume tests |
| `syncweb-cli/tests/cli_test.rs` | Add network isolation CLI tests |
