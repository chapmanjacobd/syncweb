# Plan: Ephemeral → Persistent State Gaps

## Divergence

Several in-memory data structures hold state that should survive daemon restarts but is currently lost. Others are intentionally ephemeral (runtime handles, OS resources) and should remain so. This plan identifies which gaps to fix and which to leave.

## Decision

Fix 3 critical/medium gaps by persisting to SQLite (leveraging the databases created in the JSON→SQLite plan). Leave 4 data structures as intentionally ephemeral with justification.

---

## GAP 1 (Critical): ProviderReputationStore

### Current location
`syncweb-core/src/indexing/reputation.rs:157-167`

### What is lost
```rust
pub struct ProviderReputationStore {
    reputations: HashMap<PublicKey, ProviderReputation>,  // ALL history lost
    auto_bans: HashMap<PublicKey, AutoBan>,                // ALL bans reset
    signal_sequences: HashMap<(PublicKey, PublicKey), u64>, // ALL sequence tracking lost
    pending_signals: Vec<ProviderTrustSignal>,             // queued gossip lost
    // ...config, policy, reporter — fine to reconstruct
}
```

### Impact
- A provider with 100 successful fetches and 0 failures is demoted to "unknown" (score 0.5) on restart
- An automatic ban that should last 30 days is canceled on restart (the banned provider can reconnect immediately)
- Gossip deduplication (signal_sequences) resets — duplicate signals accepted
- Pending trust signal batch (queued for gossip publication) is lost

### Fix

Add to `indexing.sqlite`:

```sql
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
```

Implementation:

File: `syncweb-core/src/indexing/reputation.rs`

```rust
impl ProviderReputationStore {
    pub fn new(database: IndexingDatabase, config: ReputationConfig) -> Self {
        let reputations = database.load_all_reputations()?;      // SELECT * FROM provider_reputation
        let signal_sequences = database.load_signal_sequences()?; // SELECT * FROM provider_signal_sequences
        Self {
            reputations,
            config,
            policy: TrustPolicy::new(),
            auto_bans: Self::reconstruct_auto_bans(&reputations),
            signal_sequences,
            pending_signals: Vec::new(),  // pending batch always starts empty
            max_signal_batch: DEFAULT_SIGNAL_BATCH_SIZE,
            reporter: None,
            next_signal_sequence: 1,
        }
    }

    pub fn record_fetch_result(&mut self, provider: PublicKey, success: bool, kind: FetchFailureKind, now: u64) {
        // ... existing logic ...
        // ADD: persist to database after update
        self.database.upsert_reputation(provider, &self.reputations[&provider])?;
        self.database.upsert_ban(provider, self.auto_bans.get(&provider))?;
    }

    pub fn purge_stale(&mut self, now: u64, ttl: Duration) {
        // ... existing logic ...
        // ADD: also DELETE from database
        self.database.delete_stale_reputations(now, ttl)?;
    }
}
```

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/indexing/reputation.rs` | Add `IndexingDatabase` field; persist on write; hydrate on new |
| `syncweb-core/src/indexing.rs` | Add `load_all_reputations`, `upsert_reputation`, etc. to `IndexingDatabase` |

---

## GAP 2 (Medium): ResilienceService Leases & Bans

### Current location
`syncweb-core/src/indexing/resilience.rs` — `ProviderLeaseTracker` (in-memory lease map)

### What is lost
- Leases gathered from gossip during a session (all providers known to be seeding a hash)
- Active provider bans (banned_at, expires_at, reason)
- The `indexing-state.json` partially persists these but is only read once at startup and never written back during runtime

### Impact
- After restart, the node forgets all provider leases — must re-discover seeding providers via gossip
- Active bans are lifted (banned providers can serve content again until re-banned)
- Content that was at replication targets (healthy) shows as unhealthy until new gossip arrives

### Fix

The `provider_leases` and `provider_bans` tables in `indexing.sqlite` (from the JSON→SQLite plan):

```sql
CREATE TABLE provider_leases (
    provider TEXT NOT NULL,
    content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
    ticket TEXT NOT NULL,
    leased_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    signature TEXT NOT NULL,
    PRIMARY KEY(provider, content_hash)
);

CREATE TABLE provider_bans (
    provider TEXT NOT NULL,
    content_hash BLOB,
    reason TEXT NOT NULL,
    banned_at INTEGER NOT NULL,
    expires_at INTEGER,
    PRIMARY KEY(provider, content_hash)
);
```

Implementation:

File: `syncweb-core/src/indexing/resilience.rs`

```rust
impl ResilienceService {
    pub fn new(database: IndexingDatabase, config: ResilienceConfig) -> Self {
        let leases = database.load_active_leases()?;  // SELECT WHERE expires_at > now
        let bans = database.load_active_bans()?;      // SELECT WHERE expires_at IS NULL OR expires_at > now
        let tracker = ProviderLeaseTracker::new(config.budget.observation_ttl);
        for lease in leases { tracker.record(lease)?; }
        for ban in bans { tracker.ban_provider_from_record(ban)?; }
        Self { tracker: Arc::new(Mutex::new(tracker)), ... }
    }

    pub fn ban_provider(&self, provider: PublicKey, reason: String, hash: Option<Hash>, duration: Option<Duration>) -> Result<()> {
        // ... existing logic ...
        // ADD: persist to database
        self.database.insert_provider_ban(provider, hash, &reason, now, expires_at)?;
        Ok(())
    }
}
```

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/indexing/resilience.rs` | Add database-backed lease/ban persistence |
| `syncweb-core/src/indexing.rs` | Add lease/ban query methods to `IndexingDatabase` |

---

## GAP 3 (Medium): DenylistService State Divergence

### Current location
`syncweb-core/src/indexing/denylist.rs` — `DenylistService` (in-memory HashSet)

### What is lost
- Rules added during daemon runtime are held in memory + written to `indexing-state.json`
- But the JSON file is loaded once at CLI invocation time. If the daemon adds rules via IPC and the CLI re-loads state, the in-memory DenylistService and the JSON file can diverge
- The SQLite `denylist_rules` table already exists BUT IS NOT USED by the DenylistService

### Impact
- Filter rules added during a daemon session are not properly persisted when the daemon keeps running and CLI commands modify state independently
- Subscription to federated filter lists can be lost

### Fix

The `denylist_rules` and `filter_lists` tables already exist in `indexing.sqlite`. Make `DenylistService` use them directly:

File: `syncweb-core/src/indexing/denylist.rs`

```rust
pub struct DenylistService {
    database: IndexingDatabase,          // ADD this field (currently absent)
    rules: Arc<Mutex<Vec<DenylistRule>>>, // in-memory cache
    lists: Arc<Mutex<Vec<FilterList>>>,   // in-memory cache
}

impl DenylistService {
    pub fn new(database: IndexingDatabase) -> Self {
        let rules = database.load_denylist_rules()?;  // SELECT * FROM denylist_rules
        let lists = database.load_filter_lists()?;    // SELECT * FROM filter_lists
        Self {
            database,
            rules: Arc::new(Mutex::new(rules)),
            lists: Arc::new(Mutex::new(lists)),
        }
    }

    pub fn add(&self, rule: DenylistRule) -> Result<()> {
        // INSERT OR REPLACE into denylist_rules
        self.database.upsert_denylist_rule(&rule)?;
        // update in-memory cache
        self.rules.lock()?.push(rule);
        Ok(())
    }

    pub fn subscribe(&self, list: &FilterList) -> Result<bool> {
        // INSERT OR REPLACE into filter_lists
        self.database.upsert_filter_list(list)?;
        // update in-memory cache
        // ... dedup by sequence ...
        Ok(changed)
    }
}
```

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/indexing/denylist.rs` | Add SQLite persistence; read from DB on init |
| `syncweb-core/src/indexing.rs` | Add denylist/filter query methods if missing |
| `syncweb-core/src/indexing.rs` | `IndexingService::new()` passes database to `DenylistService::new()` |

---

## GAP 4 (Low): LinkResolver

### Current location
`syncweb-core/src/indexing/links.rs` — `LinkResolver` (in-memory HashMap of pointers, mirrors, revoked links)

### What is lost
- Mutable pointers discovered from peers via Iroh docs sync are added to the in-memory resolver but NOT persisted
- Only links explicitly created by the local user are persisted (via `indexing-state.json`)
- After restart, peer-discovered pointers must be re-synced

### Impact
- Link resolution for peer-discovered content is slower after restart (must wait for re-sync)
- Links resolved during a session are momentarily unavailable after restart

### Fix

The `stable_links` and `link_mirrors` tables already exist in `indexing.sqlite`. The `LinkResolver` should write-through to them:

File: `syncweb-core/src/indexing/links.rs`

```rust
pub struct LinkResolver {
    database: IndexingDatabase,
    pointers: Arc<Mutex<HashMap<String, MutablePointer>>>,
    mirrors: Arc<Mutex<HashMap<String, Vec<Mirror>>>>,
    revoked: Arc<Mutex<HashSet<PrivateLink>>>,
}

impl LinkResolver {
    pub fn new(database: IndexingDatabase) -> Self {
        let pointers = database.load_all_pointers()?;   // SELECT from stable_links
        let mirrors = database.load_all_mirrors()?;    // SELECT from link_mirrors
        // ... reconstruct caches ...
    }

    pub fn publish(&self, pointer: MutablePointer) -> Result<()> {
        // UPSERT into stable_links table
        self.database.upsert_link(&pointer)?;
        // update cache
        self.pointers.lock()?.insert(pointer.alias.clone(), pointer);
        Ok(())
    }
}
```

### Files to modify

| File | Change |
|---|---|
| `syncweb-core/src/indexing/links.rs` | Add database persistence layer |
| `syncweb-core/src/indexing.rs` | Add link query methods |

---

## INTENTIONALLY EPHEMERAL (No Changes Needed)

### 1. FolderManager in-memory cache

Location: `syncweb-core/src/folder/manager.rs:27`

```rust
folders: Arc<RwLock<HashMap<NamespaceId, SyncwebFolder>>>
```

**Justification:** Iroh docs persistent store (`data/docs/`) is the source of truth. On daemon restart, `FolderManager::list()` calls `docs.inner().list()` which enumerates all namespaces from the Iroh docs store. The in-memory `HashMap` is a performance optimization, not state.

### 2. IntentTasks / IntentControls

Location: `syncweb-core/src/daemon/daemon.rs:93-94`

```rust
intent_tasks: Mutex<HashMap<NamespaceId, JoinHandle<()>>>,
intent_controls: IntentControls,  // Arc<Mutex<HashMap<NamespaceId, IntentControl>>>
```

**Justification:** These hold active `tokio::task::JoinHandle` objects to running async tasks. JoinHandles are OS-level resources (references to live futures on the tokio runtime). They cannot be serialized and would be immediately stale after deserialization. Sync intents are re-created on demand by `syncweb automatic` or explicit `syncweb sync` commands.

### 3. FsWatcher / PendingWatch

Location: `syncweb-core/src/daemon/daemon.rs:95-96`

```rust
watchers: Mutex<HashMap<String, FsWatcher>>,
pending_watch_events: Mutex<HashMap<String, PendingWatch>>,
```

**Justification:** `FsWatcher` wraps OS-level filesystem notification handles (`inotify` on Linux). These are file descriptors — impossible to serialize. They must be re-registered with the OS on daemon startup. The daemon re-registers watchers for all watched folders in its startup sequence (reading folder paths from Iroh docs).

### 4. ScheduleManager

Location: `syncweb-core/src/daemon/daemon.rs:89`

```rust
schedule_manager: tokio::sync::RwLock<Option<ScheduleManager>>,
```

**Justification:** Parsed from `config.toml` (now `node.db` after Plan 1) at daemon startup. The schedule config is a pure function of the persisted config — no runtime state to preserve.

### 5. MemoryLookup

Location: `syncweb-core/src/node/iroh_node.rs:1` and usage

**Justification:** Iroh's internal address lookup table. Populated from active connections and announced endpoints. Connections must be re-established after restart regardless — serializing stale addresses would be harmful.

### 6. Gossip topic subscriptions (PackageCatalog, TopicTracker, etc.)

**Justification:** Gossip topics are network subscriptions — TCP connections and protocol state. They expire when the connection drops. On restart, topics are re-subscribed using the persisted network membership (from `node.db`).

---

## Implementation Order

1. **GAP 2 first** — ResilienceService leases/bans must be persisted because the JSON→SQLite plan already creates the `provider_leases` and `provider_bans` tables. This gap is zero new schema, just wiring.
2. **GAP 1 second** — ProviderReputationStore requires the `provider_reputation` and `provider_signal_sequences` tables from the JSON→SQLite plan. Depends on those tables existing.
3. **GAP 3 third** — DenylistService already has tables in indexing.sqlite, just needs wiring. Depends on `IndexingDatabase` being the source of truth.
4. **GAP 4 last** — LinkResolver is lowest impact and already half-persisted via the existing `stable_links` table.

---

## Changes Summary

| Gap | Severity | Tables Added | Key Files |
|---|---|---|---|
| ProviderReputationStore | Critical | `provider_reputation`, `provider_signal_sequences` | `indexing/reputation.rs` |
| ResilienceService | Medium | `provider_leases`, `provider_bans` (already in Plan 1) | `indexing/resilience.rs` |
| DenylistService | Medium | `denylist_rules`, `filter_lists` (already exist) | `indexing/denylist.rs` |
| LinkResolver | Low | `stable_links`, `link_mirrors` (already exist) | `indexing/links.rs` |
