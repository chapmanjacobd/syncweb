# Syncweb Manual Testing Plan

Only sections and tests that are not fully passing are listed. Fully passing
sections and tests are omitted; status reflects the 2026-10-05 runs.

## Setup & Prerequisites

- Build: `cargo build --release` (binary at `target/release/syncweb`)
- Two machines (or a loopback test with two terminals) needed for most sync tests
- Data dirs: `~/.local/share/syncweb/` (default) — contains `node.db`, `stats.db`, `blobs/`, `docs/`
- Debug log: `RUST_LOG=debug syncweb ...` or `syncweb --verbose ...`
- Trace log: `RUST_LOG=trace syncweb ...`
- SQLite: `sqlite3 ~/.local/share/syncweb/node.db` (and `stats.db`)

## Last Verified Run (provenance)

- VMs: `syncweb-a` (`10.0.3.2`) and `syncweb-b` (`10.0.3.3`) via `iiab-vm` + `nsenter`.
- Binary SHA-256: `d1eeb9673609586641b7ae5f80c00f45f0127847247936075c0b57b6cf2c5f2e`.
- Data: fresh profiles; Alice `shared-docs`/`twoway`/`sendonly`, Bob
  `bob-shared`/`bob-twoway`/`bob-sendonly`; daemons run with `start --bg`.
- Re-verified the ten prior failures against the SHA above. Passing after the
  fixes: JSON envelopes (`folders --json` → `{"folders":[…]}`,
  `find --json` → `{"paths":[…]}`), `status --json` `mode`/`kind`/`sync_count`,
  human-readable filter sizes plus rejection of a wrong-direction bound and of
  garbage (`F5`), media listener (`/`=404, `/media/deadbeef`=400, 64-zero
  hash=404), `--log-file` writing the `daemon started` record immediately on a
  clean start, `start --bg` idempotence on an already-running daemon, and
  `folders create --mode` help listing `receiveencrypted`.

---

## 3.2 Sync Modes — ReceiveEncrypted

- Not yet exercised: a `--mode receiveencrypted` folder joined by a second node.
  Expected: the joiner can write, but blobs are encrypted at rest on the
  receiving side. (`sendonly` / `receiveonly` / two-way `--write` were tested.)

## 4.2 `find` — pattern is positional

- Action: `syncweb find '*' --type f --ext mp3 --local-only ./music`
- Expected: combined filters applied on the disk scan
- Note: the pattern is positional and required; without it `./music` binds to
  the pattern slot and the path defaults to `.` (the scanner then walks `/` and
  hits unreadable special files). `--remote-only` narrows to undownloaded rows.

## 9.3 Package Archive (`.car.zst`)

- Not yet exercised: `syncweb package import --filter 'name!=*.tmp' <archive>`
  drops filtered entries while importing. `import` always installs
  (`--no-install` does not exist); `export` takes filesystem paths, not a
  collection id.

## 10.3 Network Events & Health

- `syncweb network test-relay <RELAY_URL>` — the relay URL is a required
  positional argument (not `--relay-url`); there is no default.
- `syncweb network peers ./nw-docs` (daemon running) — shows the folder's
  inbound peers and a per-blob `% seeded` table; `--json` emits a single
  `{folder, peers, per_blob}` object.
- `syncweb network peers ./nw-docs --json` (no daemon) — honest empty state
  `{folder, peers: [], per_blob: []}` with no guessed values.

## 13.2 Bandwidth Verification

- Sync large files while monitoring `syncweb stats network`; bandwidth is capped
  at the configured limit.
  Debug: `sqlite3 ~/.local/share/syncweb/stats.db "SELECT SUM(bytes) FROM bandwidth_events WHERE direction='download';"`
- Wait for an inactive window, then trigger sync — sync does not start (or is
  delayed).
- `syncweb stats network` — totals, per-folder, per-peer.
- `syncweb stats network --period 24h` — only events from the last 24h.
- `syncweb stats network --since 24h --json` — object with
  `total_upload`/`total_download`/`per_folder`/`per_peer`/`period_start`.
- `syncweb stats network --folder <namespace>` — per-folder breakdown.
- `syncweb stats network --follow --once` — prints the current snapshot and
  exits (cron-safe).
- `syncweb stats network --follow` (Ctrl+C after a moment) — streams sync
  sessions / network events live; under `--json`, one JSON object per line
  (NDJSON).

## 14. Filter Engine / Watch Mode

- `syncweb watch --show-filters` — shows the loaded filter rules.
- `syncweb watch --dry-run --paths <folder>` — shows what would be
  accepted/rejected.
- `syncweb watch --once <folder>` — imports files matching the rules, rejects
  the rest.
- `syncweb watch --filters /path/to/filters.toml <folder>` — uses a custom filter
  config path. With a daemon running this is sent as a per-folder override and
  persisted; otherwise the daemon's canonical `DATA_DIR/filters.toml` is used.
  Editing `filters.toml` requires `syncweb reload` for the daemon watcher to
  pick it up (same as `config.toml`).

Example `filters.toml`:

```toml
[general]
sort_mode = "niche"
limit_size = "10GB"
min_seeders = 1

[[rules]]
type = "accept"
match = { name = "*.iso", min_size = "100MB" }

[[rules]]
type = "reject"
match = { name = "*.tmp" }
```

- Size fields accept human-readable values (`1KB`, `100MB`, `2GiB`) and bare
  integers; a wrong-direction bound (`min_size = "<5GB"`) or garbage
  (`min_size = "not-a-size"`) makes daemon startup fail with a clear error. All
  verified 2026-10-05.
- The parser only knows the top-level `rules` array (TOML `[[rules]]`). The
  `[general]` table and any singular `[[rule]]` are silently ignored (the config
  struct has no `deny_unknown_fields`), so a typo there is not reported.

## 15. Watch Mode (File Watcher)

- `syncweb watch ./shared-docs` — watches the folder; then touch/create/delete/
  modify a file and confirm the watcher imports the change (logs show
  "file changed: <path>"). The daemon owns the watcher and applies the canonical
  filter engine plus per-folder ignore globs.
- `syncweb watch --once ./shared-docs` — scans once, then exits.
- `.syncignore` with glob patterns — the watcher excludes matching files.

## 16. Conflict Resolution

- Both nodes modify the same file offline, then come online and sync.
  Expected: conflict detected. Text conflicts auto-resolve (LWW) and save a
  `.diff`; binary conflicts produce a `.conflict.<hash>` file preserving both
  versions.
- Debug: check for `.conflict`/`.diff` files.

## 17. Offline Queue

- Go offline, make changes to a synced folder, then come back online.
  Expected: pending changes sync automatically.

## 18. Media Server

- `curl http://127.0.0.1:9193/media/<blob-hash>` — serves blob content; compare
  against the `syncweb stat` hash. Must be run against a populated blob store
  (the standalone server's node must see the blobs).
- `curl -H "Range: bytes=0-100" http://127.0.0.1:9193/media/<hash>` — partial
  content (206) with the first 100 bytes; check the `Content-Range` header.
- Custom listen address in config — server starts on the specified address.
- Malformed hashes are rejected without dropping the connection: a short hash
  (`/media/deadbeef`) returns `400`, an unknown 64-hex hash returns `404`, and
  `/` returns `404`. Verified 2026-10-05.

## 19. WebSocket Bridge

- The daemon owns a bridge on `127.0.0.1:9192` (newline-free JSON `IpcRequest`
  in, JSON `IpcResponse` out). Verified directly (handshake `101 Switching
  Protocols`, `status` command, invalid-JSON error), but not yet end-to-end
  through the plan:
- `websocat ws://127.0.0.1:9192/bridge` — connection accepted.
- Send a valid JSON command over WS — receives a response.
- Send invalid JSON — receives an error message.

## 20. Syncthing Relay (BEP)

- Configure BEP (`syncweb config set bep.enabled true`,
  `syncweb config set bep.relay_urls '["tcp://relay.syncthing.net:22270"]'`),
  then connect two nodes behind CGNAT. Expected: the connection falls back to
  the Syncthing relay; logs show "relay connected".
- `syncweb network test-relay tcp://relay.example.net:22067` — logs
  latency and status. Requires a reachable relay/DNS.
- `syncweb devices` — verify the DeviceId format matches Syncthing.

## 21. Discovery Mechanisms

- `syncweb config set discovery.mdns false` — no LAN discovery; re-enabling
  (`discovery.mdns true`) resumes discovery. The key is `discovery.mdns`, not
  `discovery.local_mdns`.
- Two nodes on different networks — discover via DHT (~5-10s) or gossip.

## 22. CLI Global Flags & Output

- `syncweb --verbose <command>` — debug-level output.
- `syncweb --json folders` — JSON output; arrays are wrapped in a named key
  (`{"folders":[…]}`), never a bare top-level array. Same for `find`
  (`{"paths":[…]}`) and `access` (`{"folders":[…]}`). Verified 2026-10-05.
- `syncweb --network home folders` — folders in the "home" network context.

## 24. Integrity & Error Recovery

- Delete a blob, then `syncweb verify ./shared-docs` — reports missing/corrupt;
  `syncweb verify --fix ./shared-docs` re-downloads from peers.
- Blocked: blob content lives inside the redb store, so there is no per-blob
  file to delete or corrupt in the current store layout.

## 25. Performance Smoke Tests

- Sync a 10GB folder over LAN — expected > 500 MB/s; monitor with
  `syncweb stats network`.

## New Failures Found (2026-10-05, beyond the ten-failure pass)

These were discovered while re-verifying the ten fixes against
`d1eeb967…`. Per the testing rule the run stopped here; they are logged, not
yet fixed.

### NF-1 `--no-daemon` hangs when a daemon owns the store (some commands)

- Repro (daemon running on the same store):
  `syncweb --data-dir /root/plan --no-daemon access`
- Expected: a fast error like the one `watch` gives — "a daemon owns
  `<data-dir>`; stop it before using --no-daemon". Actual: the process blocks
  indefinitely holding the embedded store; it had to be `kill -9`ed twice.
- Notes: `--no-daemon watch --once` and other embedded commands *do* return the
  daemon-owned error, so the guard is inconsistent across commands.

### NF-2 `folders create` is not idempotent per path

- Repro (daemon running): call `syncweb folders create ./alice/shared`
  repeatedly. Actual: each call registers a **new** namespace for the same
  path (observed 5+ namespaces for one `./alice/shared`), so `ls`, `watch` and
  `sync` may operate on a different registration than the URL that was shared.
- Expected: either reuse the existing registration for that path or fail with a
  clear "already exists" error.
- Impact: a shared URL can point at a namespace the sender no longer
  imports into, producing `Remote peer aborted sync: NotFound` on the joiner.

### NF-3 Cross-VM transfer from a freshly joined folder yields no content

- Repro: fresh stores on both nodes (`--no-relay`), Alice `folders create
  ./shared` → `watch --once ./shared` → share URL; Bob `folders join <url>
  ./bob` → `download ./bob`.
- Expected: Alice's two files download to Bob. Actual: `downloaded: 0 bytes`;
  `ls ./bob` reports "no remote entries yet"; the peer index never propagates.
  In a polluted store the same path surfaced
  `Remote peer aborted sync: NotFound`.
- Notes: this leaves `F3` (relative-path download) only partially verified — the
  original `path is not absolute` failure is gone, but a successful end-to-end
  cross-VM download was not observed. May be environmental (both nodes on
  `--no-relay`, no mDNS/relay across the VM network) or a real propagation bug;
  not yet distinguished.

---

## Known Limitations / Notes

- Blob corruption/repair tests (Sections 6, 18, 24) are not reproducible while
  blob content lives inside the redb store; there is no per-blob file to
  corrupt.
- `network test-relay` cannot succeed in the offline VM pair (no DNS/internet);
  this is environmental, not a code failure.
- Commands that need the live store must route through the daemon. Opening a
  second embedded node on a live data dir blocks on the store locks; several
  hangs found earlier came from this and were fixed by IPC routing (see the
  `IrohNode::new` audit). **Still failing** for at least `access --no-daemon`
  (see New Failures below).
- Plan-syntax corrections already applied in-body: `find` requires a positional
  pattern; `package upgrade <manifest-ticket>` (not the collection id);
  `package remove <collection> <version>` (version required);
  `package export <package-dir> <output>` (filesystem paths, not a collection
  id); `package import` always installs (`--no-install` does not exist);
  `link create --expires <unix-ts>` (not a duration);
  `network test-relay <RELAY_URL>`; `config set discovery.mdns <bool>`
  (not `discovery.local_mdns`); the node-database debug queries were corrected
  to real tables (`folder_mounts`, `shares`; there is no `folder_configs`,
  `snapshot_metadata`, or `folder_peers` table).

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
