# Syncweb Manual Testing Plan

Only sections and tests that are not fully passing are listed. Fully passing
sections and tests are omitted; status reflects the 2026-10-06/07 runs against
binary SHA-256 `59206740eb5f0e482fb30c67f001883ca59f467e8d19922726f01e692099b412`.

## Setup & Prerequisites

- Build: `cargo build --release` (binary at `target/release/syncweb`)
- Two machines (or a loopback test with two terminals) needed for most sync tests
- Data dirs: `~/.local/share/syncweb/` (default) — contains `node.db`, `stats.db`, `blobs/`, `docs/`
- Debug log: `RUST_LOG=debug syncweb ...` or `syncweb --verbose ...`
- Trace log: `RUST_LOG=trace syncweb ...`
- SQLite: `sqlite3 ~/.local/share/syncweb/node.db` (and `stats.db`)
- VMs: `syncweb-a` (`10.0.3.2`) and `syncweb-b` (`10.0.3.3`) via `iiab-vm` + `nsenter`.

---

## 21. Discovery Mechanisms

- Verified 2026-10-07 (`syncweb-a`/`syncweb-b`, direct peer IP blocked with an
  `nft` drop rule to emulate separate networks, daemons started with
  `--no-mdns --no-beacon`): a folder created on node A is joined and synced by
  node B and the content downloads/syncs. The custom Syncthing relay transport
  (`Custom(53575342_...)` paths in the A daemon logs) and the default iroh
  relay both carried traffic; the `distributed-topic-tracker` (BitTorrent DHT)
  gossip `TopicActor`/`BubbleMerge` actors start in both daemons.
- Verified 2026-10-07: `syncweb config set discovery.mdns false` (and
  `discovery.beacon false`) now disables the lookups on daemon start, and
  re-enabling with `true` resumes them.
- Still untested: isolating **pure** DHT/gossip peer discovery across truly
  distinct networks. The share ticket always embeds the creator's relay/direct
  addresses, so the join above bootstraps off the ticket rather than DHT-only
  discovery. A clean cross-network DHT-only run (peer address unknown at
  bootstrap) has not been performed.

---

## Next Up (2026-10-07)

### Reverse push across CGNAT (creator→joiner over the BEP relay)

- Inbound relay datagrams on the receiving side are attributed to an
  all-zero node id (`Custom(53575342_01de697bf50d…->53575342_010000…)`), so
  the joiner→creator pull direction works but a **creator that dials a
  relay-only joiner** (sendonly push, or any fresh creator-initiated
  connection) has no resolvable custom address for the joiner.
- Need: exchange relay device ids for the reverse direction (e.g. the joiner
  advertises its custom address back over the connection, or a deferred
  `register_session`-time node-id attribution after the QUIC handshake
  completes), then verify `folders` push/reconcile with direct IP blocked and
  `--no-relay`.

### DHT-only cross-network discovery

- Isolate pure DHT/gossip peer discovery with **no** bootstrap addresses in
  the ticket: craft/share a ticket with `AddrInfoOptions::Id`-style node-id
  only (or clear the peer's address lookup) so the joiner must resolve the
  creator over the BitTorrent DHT topic tracker, then measure discovery
  (~5-10 s) and sync. Currently the ticket embeds direct + relay addresses, so
  this path is not exercised.

---

## New Failures Found & Fixed (2026-10-06/07)

These regressed since the last recorded run and were found while re-verifying;
both are fixed in `59206740…` and no longer reproduce.

### NF-1 `start --bg` dropped discovery flags

- `syncweb --data-dir D start --bg --no-mdns --no-beacon` spawns the daemon
  **without** `--no-mdns`/`--no-beacon`/`--beacon-port`/`--discovery-interface`;
  the child used only `--no-relay`. mDNS and the UDP beacon therefore came back
  on for `--bg` daemons. Fixed in `spawn_daemon_process` (all discovery flags
  are forwarded). Manual re-check: the child `ps` line now shows
  `--no-mdns --no-beacon`.

### NF-2 daemon ignored persisted `discovery.*` config

- `syncweb config set discovery.mdns false` was persisted but the daemon still
  logged `mDNS address lookup registered` on start; only the CLI `--no-mdns`
  flag had any effect. Fixed: `handle_start` now seeds `daemon_config.discovery`
  from `app_config.discovery` (CLI flags only ever turn a mechanism off or
  override the beacon port/interface). Manual re-check: with `mdns = false`
  (`config show discovery`) the daemon logs **no** `mDNS address lookup
  registered` line; after `config set discovery.mdns true` it logs one.

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
