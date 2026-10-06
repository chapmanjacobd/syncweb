# Syncweb Manual Testing Plan

Only sections and tests that are not fully passing are listed. Fully passing
sections and tests are omitted; status reflects the 2026-10-05/06 runs against
binary SHA-256 `e37534b7a32f7c9a5c12aa7caf676261fdfe8fd99ccba45e99d352083b145242`.

## Setup & Prerequisites

- Build: `cargo build --release` (binary at `target/release/syncweb`)
- Two machines (or a loopback test with two terminals) needed for most sync tests
- Data dirs: `~/.local/share/syncweb/` (default) — contains `node.db`, `stats.db`, `blobs/`, `docs/`
- Debug log: `RUST_LOG=debug syncweb ...` or `syncweb --verbose ...`
- Trace log: `RUST_LOG=trace syncweb ...`
- SQLite: `sqlite3 ~/.local/share/syncweb/node.db` (and `stats.db`)

---

## 20. Syncthing Relay (BEP)

- Config is accepted but not wired into the data path:
  ```text
  $ syncweb config set bep.enabled true            → bep.enabled updated
  $ syncweb config set bep.relay_urls '["tcp://relay.syncthing.net:22270"]'
  [bep]
  enabled = true
  relay_urls = ["tcp://relay.syncthing.net:22270"]
  auto_fallback = true
  $ syncweb devices
  syncthing: EYLZ753-SQ4CHF2-WUJJ5L3-HDQY6KH-KAAPNMF-TMJFKE5-7SEFTAS-KN73MAQ
  ```
  the live pool endpoint is `relays.syncthing.net`

## 21. Discovery Mechanisms

- Untested: two nodes on different networks discovering via DHT (~5-10 s) or
  gossip. On a shared bridge the VM pair discovers each other and syncs in
  both modes: default relay (no `nft` accept rule needed) and `--no-relay`
  with the direct accept rule (beacon/mDNS). The base-version propagation and
  the offline-conflict reruns above both rely on this discovery working
  end-to-end on the `iiab-vm` bridge.

## Debugging SQL Queries (Node Database)

```bash
sqlite3 ~/.local/share/syncweb/node.db

SELECT * FROM daemon_lifecycle;
SELECT * FROM daemon_status;
SELECT * FROM folder_mounts;
SELECT * FROM folder_status_reports;
SELECT * FROM sync_checkpoints ORDER BY last_updated_at DESC LIMIT 20;
SELECT * FROM sync_entry_progress WHERE status = 'failed';
SELECT * FROM networks;
SELECT * FROM network_members;
SELECT * FROM filter_rules;
SELECT * FROM installed_collections;
SELECT * FROM shares;
SELECT * FROM app_config;
SELECT * FROM schema_version;
```

## Debugging SQL Queries (Stats Database)

```bash
sqlite3 ~/.local/share/syncweb/stats.db

SELECT direction, SUM(bytes) FROM bandwidth_events GROUP BY direction;
SELECT * FROM bandwidth_events ORDER BY timestamp DESC LIMIT 20;
SELECT folder_namespace, SUM(bytes) FROM bandwidth_events GROUP BY folder_namespace;
SELECT * FROM network_events ORDER BY timestamp DESC LIMIT 20;
SELECT * FROM network_sync_sessions ORDER BY started_at DESC LIMIT 10;
SELECT * FROM relay_health ORDER BY checked_at DESC LIMIT 10;
SELECT * FROM daemon_log ORDER BY timestamp DESC LIMIT 20;
SELECT * FROM network_bandwidth_summary;
```

## Environment Variables

```bash
# Set log level
export RUST_LOG=debug   # or trace, info, warn, error

# Log to file
syncweb start --bg --log-file /tmp/syncweb-debug.log
cat /tmp/syncweb-debug.log

# Custom data directory
syncweb --data-dir /tmp/syncweb-test ...

# Two nodes on one machine (separate data dirs)
syncweb --data-dir /tmp/alice start --bg --log-file /tmp/alice-daemon.log
syncweb --data-dir /tmp/bob start --bg --log-file /tmp/bob-daemon.log
```
