# Plan: Iroh Docs Usage Audit

## Divergence

Iroh docs (`iroh-docs`) provides a CRDT-based multi-writer document store with peer-to-peer synchronization. It is used throughout the codebase for several purposes — some correct, some questionable. This plan audits every docs usage site, categorizes it, and prescribes fixes for misuse.

## Background: What Iroh Docs Provides

Iroh docs is a key-value document store where:
- Each document ("doc") is a namespace identified by a `NamespaceId`
- Multiple authors can write to the same doc concurrently
- CRDT merges resolve conflicts automatically (latest-writer-wins per key)
- Docs sync between peers via QUIC connections
- Each key maps to a content-addressed blob hash + size
- Live events (`LiveEvent`) stream insertions, deletions, and sync state changes

This makes Iroh docs ideal for:
- **Mutable, shared state** that multiple peers concurrently update
- **Folder synchronization** where entries change over time
- **Discoverable metadata** that should sync with peer discovery

Iroh docs is NOT ideal for:
- **Immutable data** that never changes after creation
- **Single-writer data** that only the local node ever reads (use blobs or SQLite instead)
- **High-frequency counters** or rapidly-updating state (use SQLite or in-memory)
- **Key material** or secrets that should never leave the local machine

## Audit: Every Docs Usage Site

### 1. Folder Entry Storage (CORRECT)

**File:** `syncweb-core/src/folder/syncweb_folder.rs` (via `DocsEngine`)

**Usage:** Each synchronized folder has an Iroh doc where keys are file paths (as byte strings) and values are blob references (hash + size). The doc acts as the canonical index of what files exist in the folder.

**Assessment: CORRECT.** This is the primary and most appropriate use of Iroh docs. Multiple peers can concurrently add, modify, or delete files. The CRDT merge provides automatic conflict resolution. The doc is the synchronization unit.

**No change needed.**

---

### 2. Folder Metadata Key (CORRECT)

**File:** `syncweb-core/src/folder/manager.rs:18`

**Usage:** A single doc entry under key `b"sys/syncweb/mode"` stores the `SyncMode` string (`"SendReceive"`, `"ReceiveOnly"`, etc.).

**Assessment: CORRECT.** This is a tiny piece of metadata that benefits from being inside the doc — it syncs with the folder, and any peer can read it to understand the folder's intent. Less than 50 bytes.

**No change needed.**

---

### 3. Capability Grants (CORRECT)

**File:** `syncweb-core/src/folder/syncweb_folder.rs` (`grant` method)

**Usage:** Uses Iroh docs' built-in `set_capability()` and `list_capabilities()` API to grant/revoke read/write access to peers. Capabilities are stored inside the docs persistent store as part of the namespace metadata.

**Assessment: CORRECT.** This is Iroh's own permission system. The capabilities are small and tightly coupled to the doc's access control. No custom storage needed.

**No change needed.**

---

### 4. Public Blob Subscription (OVERUSE)

**File:** `syncweb-core/src/folder/manager.rs:87-113` (`subscribe_public`)

**Usage:** When subscribing to a public blob ticket, creates a new doc namespace, writes a single entry `b"public/content"` pointing to the blob hash, and wraps it as a `SyncwebFolder` with `SyncMode::PublicReadOnly`.

**Why this is overuse:**
- The blob is immutable (content-addressed) — there is nothing to sync
- Only one key (`b"public/content"`) ever exists in this doc
- Only the local node ever writes to it (single-writer)
- The doc namespace overhead (CRDT log, peer tracking, gossip metadata) is incurred for a single pointer
- A `SyncwebFolder` is created solely to conform to the folder API, not because the data is a folder
- Memory: each doc namespace consumes ~hundreds of bytes in the docs store and has a live sync task

**Fix: Refactor to use BlobStore directly.**

Create a lightweight `PublicSubscription` abstraction that doesn't create a doc:

```rust
// New type in syncweb-core/src/folder/
pub struct PublicSubscription {
    ticket: BlobTicket,
    hash: Hash,
    size: u64,
    label: String,
}

impl PublicSubscription {
    pub fn new(ticket: BlobTicket, bytes: &[u8]) -> Self {
        Self {
            ticket,
            hash: ticket.hash(),
            size: bytes.len() as u64,
            label: String::from_utf8_lossy(&blake3::hash(bytes).as_bytes()[..8].to_vec())
                .to_string(),
        }
    }

    pub fn hash(&self) -> Hash { self.hash }
    pub fn size(&self) -> u64 { self.size }
    pub fn ticket(&self) -> &BlobTicket { &self.ticket }
    pub fn label(&self) -> &str { &self.label }
}
```

Update `FolderManager::subscribe_public()`:
```rust
pub async fn subscribe_public(&self, ticket: &BlobTicket) -> Result<PublicSubscription> {
    self.blob_store.fetch(&self.endpoint, ticket).await?;
    let bytes = self.blob_store.get(ticket.hash()).await?;
    Ok(PublicSubscription::new(ticket.clone(), &bytes))
}
```

Update `DaemonHandle` and IPC to track `PublicSubscription` alongside `SyncwebFolder` in the folder registry. Commands like `ls`, `find`, `health` that currently operate on folders should accept either a `SyncwebFolder` or a `PublicSubscription` via a common trait or enum:

```rust
pub enum FolderOrSubscription {
    Folder(SyncwebFolder),
    Subscription(PublicSubscription),
}

impl FolderOrSubscription {
    pub fn namespace_id(&self) -> String {
        match self {
            Self::Folder(f) => f.namespace_id().to_string(),
            Self::Subscription(s) => format!("blob:{}", s.hash()),
        }
    }

    pub async fn list_entries(&self) -> Result<Vec<EntryLike>> { ... }
    pub fn path(&self) -> Option<&Path> { ... }
    pub fn label(&self) -> &str { ... }
}
```

**Files to modify:**

| File | Change |
|---|---|
| `syncweb-core/src/folder/public_subscription.rs` | **NEW** — PublicSubscription type |
| `syncweb-core/src/folder/mod.rs` | Add `pub mod public_subscription` |
| `syncweb-core/src/folder/manager.rs` | Change `subscribe_public` return type; remove doc creation |
| `syncweb-core/src/daemon/daemon.rs` | Track subscriptions alongside folders |
| `syncweb-core/src/daemon/ipc.rs` | Accept subscription in list/list commands |
| `syncweb-core/src/daemon/state.rs` | `FolderEntry` → `FolderOrSubscription` in status |
| `syncweb-core/src/node/docs_engine.rs` | No change (subscribe_public was the only doc consumer) |

---

### 5. Collection Publication (CORRECT)

**File:** `syncweb-core/src/folder/collection.rs:457-509` (`CollectionStore::publish`)

**Usage:** A `CollectionHead` (manifest hash + version) is stored as a doc entry in a collection-specific namespace. The doc acts as a mutable pointer to the latest published version.

**Assessment: CORRECT.** This is the canonical "mutable pointer" pattern. The blob stores the immutable manifest; the doc entry stores the current version. Peers only need to sync the doc to discover the latest version, not download all historical manifests.

**No change needed.**

---

### 6. Catalog Publishing (CORRECT)

**File:** `syncweb-core/src/indexing/catalog.rs` — `CatalogService`

**Usage:** Each catalog has a dedicated doc namespace. Catalog records (folder entries with metadata) are stored as doc entries keyed by `{folder_namespace_id}/{entry_key}`. Catalogs sync between peers for distributed search.

**Why the doc is appropriate:**
- Catalogs are mutable (records are added/updated as folders change)
- Multiple publishers can contribute to a catalog (multi-writer)
- Peers can subscribe to a catalog and receive live updates
- The doc provides structured, queryable metadata

**Assessment: CORRECT.** Catalogs are exactly the kind of shared, mutable, multi-writer metadata that Iroh docs was designed for.

**No change needed.**

---

### 7. Mutable Links / Signed Pointers (ADEQUATE)

**File:** `syncweb-core/src/indexing/links.rs` — `MutablePointer` stored in Iroh docs

**Usage:** Signed mutable pointers (alias → hash + sequence + signature) are stored as doc entries in link-specific doc namespaces. The doc syncs pointer updates between peers.

**Why the doc is used:**
- Pointers change over time (new versions)
- Multiple peers may discover and cache the same pointer
- Doc sync ensures pointer updates propagate

**Why it's borderline:**
- This could also work with simple gossip messages (announce pointer update on a topic)
- The doc approach adds overhead for what is essentially a single-value-per-key update
- But the doc provides persistent storage that survives the gossip window

**Assessment: ADEQUATE.** The docs approach works and provides persistence that gossip alone doesn't. No urgent change needed, but if link resolution becomes a performance bottleneck, consider a hybrid: gossip for discovery + SQLite for persistence.

**No immediate change.**

---

### 8. Sync Engine Live Events (CORRECT)

**File:** `syncweb-core/src/sync/engine.rs:196` — `self.docs_engine.watch(folder.doc())`

**Usage:** The sync engine subscribes to `LiveEvent` streams from folder docs to drive synchronization intents. When an entry is inserted remotely, the engine decides whether to download the blob.

**Assessment: CORRECT.** Live events from docs are the proper way to be notified of remote changes. This is the reactive synchronization model that Iroh docs provides.

**No change needed.**

---

### 9. Indexing Service Live Events (CORRECT)

**File:** `syncweb-core/src/indexing.rs:1055` — `folder.docs_engine().watch(folder.doc())`

**Usage:** When a folder is enabled for indexing, the `IndexingService` subscribes to its doc's live events to index new entries in SQLite for FTS5 search.

**Assessment: CORRECT.** The indexing service consumes doc events to build the search index. This is a read-only consumer layered on top of the doc — appropriate architecture.

**No change needed.**

---

### 10. Integrity Verification (CORRECT)

**File:** `syncweb-core/src/verify.rs:59` — `self.docs_engine.list_latest(folder.doc())`

**Usage:** The `IntegrityChecker` lists all entries in a folder's doc, then checks each referenced blob for hash integrity.

**Assessment: CORRECT.** The doc is the canonical list of folder entries. Reading from it to verify blobs is the right data source.

**No change needed.**

---

### 11. Health Check (CORRECT)

**File:** `syncweb-core/src/daemon/ipc.rs` (handle_health_check)

**Usage:** Lists doc entries and checks if referenced blobs are locally available, then computes seeding health.

**Assessment: CORRECT.** Same pattern as verify — the doc is the entry list.

**No change needed.**

---

### 12. Snapshot (CORRECT — bypasses docs)

**File:** `syncweb-core/src/snapshot.rs:458-475` (`SnapshotStore::store`)

**Usage:** Snapshots are stored as pinned blobs (using naming convention `syncweb/snapshot/{id}/manifest`), NOT as doc entries. The `restore_for_folder` method does write back to the doc, but the snapshot itself lives entirely in the blob store.

**Assessment: CORRECT.** Snapshots are immutable point-in-time captures. No CRDT needed. The pin naming convention provides discovery. This is the right pattern — use blobs for immutable data, docs for mutable data.

**No change needed.**

---

## UNDERUSE: Where Docs Could Be Added (but shouldn't right now)

### Potential Use 1: Network Membership as Docs

**Current:** `networks.json` (now `node.db` via Plan 1) stores network member lists. Membership changes require explicit ticket exchange — no automatic propagation of kicks or joins.

**Idea:** Store signed membership lists as doc entries in a per-network doc namespace. Members sync the doc and see real-time membership changes (owner adds/kicks members, signed by owner key).

**Verdict: IMPLEMENTED in plans/network-remaining-gaps.md (GAP 6).** A full implementation plan with data structures, signature scheme, namespace derivation, lifecycle, edge cases, and migration strategy is in that plan. This is no longer deferred.

### Potential Use 2: Config Distribution

**Current:** `config.toml` / `node.db` stores local-only config. Multi-device users must manually copy config.

**Idea:** Store config in a dedicated doc namespace, synced between the user's devices. Config changes on one device propagate automatically.

**Verdict: DEFER.** Config is intentionally device-specific (paths, relay preferences, thread counts). Sharing config across devices is actively harmful — a sync interval that works on a powerful desktop won't work on a laptop. If cross-device config sync is desired, it should be opt-in and scoped to specific config keys (e.g., `filter_rules` only).

### Potential Use 3: Watcher/PendingWatch Persistence

**Current:** File watchers are re-registered from scratch on daemon restart.

**Verdict: NO CHANGE.** Watchers must be registered with the OS (`inotify`/`FSEvents`). You can't persist a file descriptor. Re-registration from folder paths (stored in Iroh docs as folder entries) is the correct approach.

---

## Summary of Changes

### Immediate Fix

| # | Issue | Severity | Action |
|---|---|---|---|
| 4 | Public blob subscription wastes a Doc namespace | Medium | Refactor to `PublicSubscription` using BlobStore directly |

### Adequate (no change)

| # | Site | Verdict |
|---|---|---|
| 1 | Folder entry storage | Correct |
| 2 | Folder metadata key | Correct |
| 3 | Capability grants | Correct |
| 5 | Collection publication | Correct |
| 6 | Catalog publishing | Correct |
| 7 | Mutable links / signed pointers | Adequate (could optimize later) |
| 8 | Sync engine live events | Correct |
| 9 | Indexing service live events | Correct |
| 10 | Integrity verification | Correct |
| 11 | Health check | Correct |
| 12 | Snapshot (blobs only) | Correct |

### Deferred to Future

| # | Idea | Reason |
|---|---|---|
| Network membership via docs | Feature, not bug fix |
| Config distribution via docs | Harmful default; needs opt-in design |

---

## Implementation Detail: PublicSubscription Refactor

### New types

```rust
// syncweb-core/src/folder/public_subscription.rs

use iroh_blobs::{Hash, ticket::BlobTicket};

#[derive(Clone, Debug)]
pub struct PublicSubscription {
    ticket: BlobTicket,
    hash: Hash,
    size: u64,
    label: String,
}

impl PublicSubscription {
    pub fn new(ticket: BlobTicket, bytes: &[u8]) -> Self {
        Self {
            hash: ticket.hash(),
            ticket,
            size: bytes.len() as u64,
            label: format_short_hash(&ticket.hash()),
        }
    }

    pub fn hash(&self) -> Hash { self.hash }
    pub fn size(&self) -> u64 { self.size }
    pub fn ticket(&self) -> &BlobTicket { &self.ticket }
    pub fn label(&self) -> &str { &self.label }
}

fn format_short_hash(hash: &Hash) -> String {
    let hex = hash.to_string();
    format!("{}..", &hex[..12])
}
```

### FolderManager change

```rust
// syncweb-core/src/folder/manager.rs

pub async fn subscribe_public(&self, ticket: &BlobTicket) -> Result<PublicSubscription> {
    self.blob_store.fetch(&self.endpoint, ticket).await?;
    let bytes = self.blob_store.get(ticket.hash()).await?;
    self.subscriptions
        .write()
        .await
        .insert(ticket.hash(), PublicSubscription::new(ticket.clone(), &bytes));
    Ok(PublicSubscription::new(ticket.clone(), &bytes))
}
```

### IPC change

In `syncweb-core/src/daemon/ipc.rs`, the `FolderEntry` enum already supports a `PublicSubscription` variant-like structure. The `folders` response format adds a `kind: "subscription"` field for public blobs. This should map to the `PublicSubscription` type instead of a dummy `SyncwebFolder`.

### Commands that need `PublicSubscription` support

| Command | Current behavior with public subscription | After refactor |
|---|---|---|
| `ls` | Shows single `public/content` entry | Shows blob hash, size, direct access |
| `find` | N/A (not applicable to single blobs) | Can search by hash |
| `health` | Shows `public/content` as single entry | Shows blob seeding status |
| `download` | Fetches blob (works correctly) | Works correctly with `PublicSubscription.hash()` |
| `unsubscribe` | Drops the doc namespace | Untracks the subscription |
| `verify` | Hashes the single blob | Uses blob store directly |
| `stat` | Shows doc namespace stats | Shows blob store stats |
