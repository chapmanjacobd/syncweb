# Plan: JSON → SQLite Migration (Plan 1)

## Problem
Three JSON files hold persistent state but lack durability, queryability, and migration safety:
- `indexing-state.json` — WOT, provider reputation, denylist, links, reports, attestations
- `networks.json` — network membership, tickets, folders, invites
- `collections.json` — collection manifests, heads, pins

Daemon crashes corrupt them. No schema evolution. CLI and daemon race on writes.

## Decision
Migrate to three SQLite databases with WAL mode, foreign keys, and migrations.

---

## Database 1: `indexing.sqlite`

### Tables
```sql
-- Schema versioning (used by migration framework)
CREATE TABLE schema_version (
    version INTEGER NOT NULL
);

-- WoT / Trust
CREATE TABLE trust_roots (
    id INTEGER PRIMARY KEY,
    pubkey TEXT NOT NULL UNIQUE,           -- hex
    label TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE trust_delegations (
    id INTEGER PRIMARY KEY,
    from_pubkey TEXT NOT NULL,
    to_pubkey TEXT NOT NULL,
    scope TEXT,
    max_depth INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    revoked_at INTEGER,
    FOREIGN KEY(from_pubkey) REFERENCES trust_roots(pubkey)
    -- NOTE: No FK on to_pubkey -- delegates do not need to be trust roots.
    -- The whole point of delegation is extending trust to entities that are
    -- NOT already in the trust root set (see 05-trust-delegate-tdd.md).
);

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

-- Denylist / Filters
CREATE TABLE denylist_rules (
    id INTEGER PRIMARY KEY,
    pattern TEXT NOT NULL,
    rule_type TEXT NOT NULL CHECK(rule_type IN ('hash','glob','path')),
    source TEXT,
    created_at INTEGER NOT NULL,
    expires_at INTEGER
);

CREATE TABLE filter_lists (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    url TEXT NOT NULL,
    last_fetched INTEGER,
    last_sequence INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1
);

-- Links
CREATE TABLE stable_links (
    alias TEXT PRIMARY KEY,
    content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
    sequence INTEGER NOT NULL DEFAULT 0,
    issuer TEXT NOT NULL,
    signature BLOB NOT NULL CHECK(length(signature) = 64),
    scope TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE link_mirrors (
    alias TEXT NOT NULL,
    provider TEXT NOT NULL,
    added_at INTEGER NOT NULL,
    PRIMARY KEY(alias, provider),
    FOREIGN KEY(alias) REFERENCES stable_links(alias)
);

CREATE TABLE revoked_links (
    capability_hash BLOB PRIMARY KEY CHECK(length(capability_hash) = 32),
    revoked_at INTEGER NOT NULL
);

-- Reports / Moderation
CREATE TABLE reports (
    id INTEGER PRIMARY KEY,
    content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
    reason TEXT NOT NULL,
    reporter TEXT,                    -- hex pubkey, nullable for local-only reports
    signature TEXT,                   -- hex-encoded Ed25519 signature (128 chars), nullable for local-only reports
    created_at INTEGER NOT NULL
);

-- Attestations
CREATE TABLE attestations (
    id INTEGER PRIMARY KEY,
    content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
    kind TEXT NOT NULL CHECK(kind IN ('license','provenance','derivative')),
    value TEXT NOT NULL,
    sequence INTEGER NOT NULL DEFAULT 1,
    issuer TEXT NOT NULL,
    signature TEXT NOT NULL CHECK(length(signature) = 128),
    created_at INTEGER NOT NULL,
    UNIQUE(issuer, content_hash, kind, sequence)
);

-- Provider leases (from gossip, used by ResilienceService)
CREATE TABLE provider_leases (
    provider TEXT NOT NULL,
    content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
    ticket TEXT NOT NULL,
    leased_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    signature TEXT NOT NULL,
    PRIMARY KEY(provider, content_hash)
);

-- Provider bans (from ResilienceService)
CREATE TABLE provider_bans (
    provider TEXT NOT NULL,
    content_hash BLOB,
    reason TEXT NOT NULL,
    banned_at INTEGER NOT NULL,
    expires_at INTEGER,
    PRIMARY KEY(provider, content_hash)
);

-- Indexes
CREATE INDEX idx_trust_delegations_from ON trust_delegations(from_pubkey);
CREATE INDEX idx_trust_delegations_to ON trust_delegations(to_pubkey);
CREATE INDEX idx_reports_content ON reports(content_hash);
CREATE INDEX idx_attestations_content ON attestations(content_hash);
```

---

## Database 2: `node.db` (Networks + Collections)

### Tables
```sql
-- Identity
CREATE TABLE identity (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    secret_key BLOB NOT NULL CHECK(length(secret_key) = 32),
    created_at INTEGER NOT NULL
);

-- Networks
CREATE TABLE networks (
    network_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    label TEXT,
    owner TEXT NOT NULL,                 -- hex pubkey
    shared_secret BLOB NOT NULL CHECK(length(shared_secret) = 32),
    doc_ticket TEXT,                      -- for membership doc (GAP 6)
    created_at INTEGER NOT NULL
);

CREATE TABLE network_members (
    network_id TEXT NOT NULL,
    member TEXT NOT NULL,                 -- hex pubkey
    role TEXT NOT NULL DEFAULT 'member' CHECK(role IN ('admin','member')),
    joined_at INTEGER NOT NULL,
    PRIMARY KEY(network_id, member),
    FOREIGN KEY(network_id) REFERENCES networks(network_id) ON DELETE CASCADE
);

CREATE TABLE network_folders (
    network_id TEXT NOT NULL,
    namespace_id TEXT NOT NULL,
    label TEXT,
    added_at INTEGER NOT NULL,
    PRIMARY KEY(network_id, namespace_id),
    FOREIGN KEY(network_id) REFERENCES networks(network_id) ON DELETE CASCADE
);

CREATE TABLE network_invites (
    id INTEGER PRIMARY KEY,
    network_id TEXT NOT NULL,
    invite_ticket TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER,
    max_uses INTEGER,
    uses INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(network_id) REFERENCES networks(network_id) ON DELETE CASCADE
);

-- Collections
CREATE TABLE collections (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    namespace_id TEXT NOT NULL UNIQUE,    -- iroh docs namespace
    manifest_hash BLOB NOT NULL CHECK(length(manifest_hash) = 32),
    version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE collection_versions (
    collection_id INTEGER NOT NULL,
    version INTEGER NOT NULL,
    manifest_hash BLOB NOT NULL CHECK(length(manifest_hash) = 32),
    created_at INTEGER NOT NULL,
    PRIMARY KEY(collection_id, version),
    FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

CREATE TABLE collection_pins (
    collection_id INTEGER NOT NULL,
    entry_key BLOB NOT NULL,
    content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
    pinned_at INTEGER NOT NULL,
    PRIMARY KEY(collection_id, entry_key),
    FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

-- Folders (local folder registry)
CREATE TABLE folders (
    namespace_id TEXT PRIMARY KEY,
    label TEXT,
    sync_mode TEXT NOT NULL CHECK(sync_mode IN ('send_receive','receive_only','send_only','public_readonly')),
    created_at INTEGER NOT NULL,
    last_synced_at INTEGER
);

-- Sync checkpoints (GAP 4)
CREATE TABLE sync_checkpoints (
    namespace_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
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

CREATE TABLE sync_entry_progress (
    namespace_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    entry_key BLOB NOT NULL,
    content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
    size INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','downloading','completed','failed','skipped')),
    retries INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(namespace_id, session_id, entry_key),
    FOREIGN KEY(namespace_id, session_id) REFERENCES sync_checkpoints(namespace_id, session_id) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX idx_network_members_network ON network_members(network_id);
CREATE INDEX idx_network_folders_network ON network_folders(network_id);
CREATE INDEX idx_collections_namespace ON collections(namespace_id);
CREATE INDEX idx_sync_checkpoints_status ON sync_checkpoints(status);
CREATE INDEX idx_sync_entry_progress_status ON sync_entry_progress(status);
```

---

## Database 3: `stats.db` (Metrics + Network Logs)

### Tables
```sql
-- Bandwidth (per folder, per peer, per network)
CREATE TABLE bandwidth_events (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL,
    direction TEXT NOT NULL CHECK(direction IN ('upload','download')),
    bytes INTEGER NOT NULL,
    entries INTEGER NOT NULL DEFAULT 1,
    namespace_id TEXT,                    -- folder namespace
    peer TEXT,                            -- hex pubkey
    network_id TEXT                       -- network correlation (validated at app layer, not via FK)
);

-- Transfer sessions (GAP 1)
CREATE TABLE network_sync_sessions (
    id INTEGER PRIMARY KEY,
    network_id TEXT NOT NULL,
    folder_namespace TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    finished_at INTEGER,
    files_transferred INTEGER NOT NULL DEFAULT 0,
    bytes_transferred INTEGER NOT NULL DEFAULT 0,
    errors INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running','completed','failed','cancelled'))
);

-- Network events (GAP 1)
CREATE TABLE network_events (
    id INTEGER PRIMARY KEY,
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

-- Relay health
CREATE TABLE relay_health (
    relay_url TEXT NOT NULL,
    checked_at INTEGER NOT NULL,
    connected INTEGER NOT NULL,
    latency_ms INTEGER,
    error_message TEXT,
    PRIMARY KEY(relay_url, checked_at)
);

-- Sync stats (11-stats-tdd.md)
CREATE TABLE sync_stats (
    id INTEGER PRIMARY KEY,
    network_id TEXT,
    folder_namespace TEXT,
    round_number INTEGER NOT NULL,
    files_synced INTEGER NOT NULL DEFAULT 0,
    conflicts_resolved INTEGER NOT NULL DEFAULT 0,
    timestamp INTEGER NOT NULL
);

-- Views
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

---

## Migration Framework

### `syncweb-core/src/storage/migration.rs`
```rust
use rusqlite::{Connection, params};
use std::path::Path;

pub const INDEXING_DB_VERSION: u32 = 3;
pub const NODE_DB_VERSION: u32 = 2;
pub const STATS_DB_VERSION: u32 = 1;

pub fn migrate_indexing_db(conn: &Connection) -> Result<()> {
    // Create schema_version table if it doesn't exist (v0 → v1 transition)
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);
         INSERT INTO schema_version (version) SELECT 0 WHERE NOT EXISTS (SELECT 1 FROM schema_version);"
    )?;

    let version: u32 = conn.query_row(
        "SELECT version FROM schema_version",
        [],
        |r| r.get(0)
    ).unwrap_or(0);

    if version == 0 {
        // v1: initial schema (above)
        conn.execute_batch(INITIAL_INDEXING_SCHEMA)?;
        conn.execute("UPDATE schema_version SET version = 1", [])?;
    }
    if version <= 1 {
        // v2: add attestations table (07-attest-tdd.md)
        conn.execute_batch("
            CREATE TABLE attestations (
                id INTEGER PRIMARY KEY,
                content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
                kind TEXT NOT NULL CHECK(kind IN ('license','provenance','derivative')),
                value TEXT NOT NULL,
                sequence INTEGER NOT NULL DEFAULT 1,
                issuer TEXT NOT NULL,
                signature TEXT NOT NULL CHECK(length(signature) = 128),
                created_at INTEGER NOT NULL,
                UNIQUE(issuer, content_hash, kind, sequence)
            );
        ")?;
        conn.execute("UPDATE schema_version SET version = 2", [])?;
    }
    if version <= 2 {
        // v3: add reports table (08-report-tdd.md)
        conn.execute_batch("
            CREATE TABLE reports (
                id INTEGER PRIMARY KEY,
                content_hash BLOB NOT NULL CHECK(length(content_hash) = 32),
                reason TEXT NOT NULL,
                reporter TEXT,
                signature TEXT,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX idx_reports_content ON reports(content_hash);
        ")?;
        conn.execute("UPDATE schema_version SET version = 3", [])?;
    }
    Ok(())
}

pub fn open_indexing_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    migrate_indexing_db(&conn)?;
    Ok(conn)
}
```

### JSON Import (one-time)
```rust
pub fn import_indexing_state(json_path: &Path, db: &Connection) -> Result<()> {
    let state: IndexingStateJson = serde_json::from_reader(File::open(json_path)?)?;
    let tx = db.transaction()?;

    // WoT
    for root in state.wot.trust_roots {
        tx.execute(
            "INSERT OR IGNORE INTO trust_roots (pubkey, label, created_at) VALUES (?1, ?2, ?3)",
            params![root.pubkey, root.label, root.created_at]
        )?;
    }
    for d in state.wot.delegations {
        tx.execute(
            "INSERT OR IGNORE INTO trust_delegations (from_pubkey, to_pubkey, scope, max_depth, created_at, revoked_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![d.from, d.to, d.scope, d.max_depth, d.created_at, d.revoked_at]
        )?;
    }

    // Provider reputation
    for (provider, rep) in state.provider_reputation.reputations {
        tx.execute(
            "INSERT OR REPLACE INTO provider_reputation (provider, total_fetches, successful_fetches, failed_fetches,
                 consecutive_failures, last_success_at, last_failure_at, auto_ban_until, auto_ban_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                provider, rep.total_fetches, rep.successful_fetches, rep.failed_fetches,
                rep.consecutive_failures, rep.last_success_at, rep.last_failure_at,
                rep.auto_ban_until, rep.auto_ban_count
            ]
        )?;
    }
    for ((reporter, provider), seq) in state.provider_reputation.signal_sequences {
        tx.execute(
            "INSERT OR REPLACE INTO provider_signal_sequences (reporter, provider, last_sequence) VALUES (?1, ?2, ?3)",
            params![reporter, provider, seq]
        )?;
    }

    // Denylist
    for rule in state.denylist.rules {
        tx.execute(
            "INSERT INTO denylist_rules (pattern, rule_type, source, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![rule.pattern, rule.rule_type, rule.source, rule.created_at, rule.expires_at]
        )?;
    }
    for list in state.denylist.lists {
        tx.execute(
            "INSERT OR REPLACE INTO filter_lists (name, url, last_fetched, last_sequence, enabled) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![list.name, list.url, list.last_fetched, list.last_sequence, list.enabled as i32]
        )?;
    }

    // Links
    for ptr in state.links.pointers {
        tx.execute(
            "INSERT OR REPLACE INTO stable_links (alias, content_hash, sequence, issuer, signature, scope, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![ptr.alias, ptr.content_hash, ptr.sequence, ptr.issuer, ptr.signature, ptr.scope, ptr.created_at]
        )?;
    }
    for (alias, mirrors) in state.links.mirrors {
        for m in mirrors {
            tx.execute(
                "INSERT OR IGNORE INTO link_mirrors (alias, provider, added_at) VALUES (?1, ?2, ?3)",
                params![alias, m.provider, m.added_at]
            )?;
        }
    }
    for rev in state.links.revoked {
        tx.execute(
            "INSERT OR IGNORE INTO revoked_links (capability_hash, revoked_at) VALUES (?1, ?2)",
            params![rev.capability_hash, rev.revoked_at]
        )?;
    }

    // Reports
    for r in state.reports {
        tx.execute(
            "INSERT INTO reports (content_hash, reason, reporter, signature, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![r.content, r.reason, r.reporter, r.signature, r.created_at]
        )?;
    }

    // Attestations
    for a in state.attestations {
        tx.execute(
            "INSERT INTO attestations (content_hash, kind, value, sequence, issuer, signature, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![a.content_hash, a.kind, a.value, a.sequence, a.issuer, a.signature, a.created_at]
        )?;
    }

    tx.commit()?;
    Ok(())
}
```

---

## Integration Points

| Plan | Depends On | Tables Used |
|------|------------|-------------|
| `03-ephemeral-to-persistent.md` GAP 1 | `provider_reputation`, `provider_signal_sequences` | Reputation persistence |
| `03-ephemeral-to-persistent.md` GAP 2 | `provider_leases`, `provider_bans` (added above) | Resilience persistence |
| `03-ephemeral-to-persistent.md` GAP 3 | `denylist_rules`, `filter_lists` | Denylist persistence |
| `03-ephemeral-to-persistent.md` GAP 4 | `stable_links`, `link_mirrors` | LinkResolver persistence |
| `14-network-remaining-gaps.md` GAP 1 | `network_events`, `network_sync_sessions`, `relay_health` | Network logging |
| `14-network-remaining-gaps.md` GAP 2 | `bandwidth_events.network_id` | Per-network bandwidth |
| `14-network-remaining-gaps.md` GAP 4 | `sync_checkpoints`, `sync_entry_progress` | Sync checkpointing |
| `14-network-remaining-gaps.md` GAP 5 | All three DBs: `vacuum`, `check_integrity`, `backup` | Maintenance |
| `14-network-remaining-gaps.md` GAP 6 | `networks.doc_ticket`, `network_members` | Membership docs |
| `14-network-remaining-gaps.md` GAP 8 | `network_folders`, `blob_network_index` | Access control |
| `11-stats-tdd.md` | `sync_stats`, `bandwidth_events` | SyncStatsCollector |
| `10-health-tdd.md` | `provider_reputation` (via ResilienceService) | Health peer counts |
| `08-report-tdd.md` | `reports` | Moderation reports |
| `07-attest-tdd.md` | `attestations` | Attestation storage |
| `09-link-tdd.md` | `stable_links`, `link_mirrors`, `revoked_links` | Link persistence |

---

## Files to Create/Modify

| File | Action |
|------|--------|
| `syncweb-core/src/storage/migration.rs` | NEW — migration framework |
| `syncweb-core/src/storage/indexing_db.rs` | NEW — `IndexingDatabase` with CRUD for all indexing tables |
| `syncweb-core/src/storage/node_db.rs` | NEW — `NodeDatabase` with networks, collections, folders, checkpoints |
| `syncweb-core/src/storage/stats_db.rs` | NEW — `StatsDatabase` with bandwidth, sync stats, network events |
| `syncweb-core/src/storage/mod.rs` | NEW — re-exports, `open_all()` helper |
| `syncweb-core/src/indexing.rs` | Replace `IndexingState` JSON I/O with `IndexingDatabase` calls |
| `syncweb-core/src/net/network.rs` | Replace `networks.json` I/O with `NodeDatabase` |
| `syncweb-core/src/folder/collection.rs` | Replace `collections.json` I/O with `NodeDatabase` |
| `syncweb-core/src/daemon/daemon.rs` | Open all three DBs on startup; pass to services |
| `syncweb-cli/src/main.rs` | Import JSON → SQLite on first run (detect `indexing-state.json`) |
| `syncweb-core/tests/migration_test.rs` | NEW — test migrations v0→v1→v2→v3 |

---

## Rollback Safety

- All migrations are additive only (no DROP COLUMN, no RENAME TABLE)
- WAL mode: readers never block writers, writers never block readers
- `PRAGMA foreign_keys=ON` enforced at open
- JSON files kept as backup until first successful daemon restart with SQLite