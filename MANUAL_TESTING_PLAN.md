# Syncweb Manual Testing Plan

Only sections and tests that are not fully passing are listed. Fully passing
sections and tests are omitted; status reflects the 2026-10-05 runs against
binary SHA-256 `d8df56b292da232bd3f135aef515410f2436831923ccf3420f4944715bcd9ccc`.

## Setup & Prerequisites

- Build: `cargo build --release` (binary at `target/release/syncweb`)
- Two machines (or a loopback test with two terminals) needed for most sync tests
- Data dirs: `~/.local/share/syncweb/` (default) — contains `node.db`, `stats.db`, `blobs/`, `docs/`
- Debug log: `RUST_LOG=debug syncweb ...` or `syncweb --verbose ...`
- Trace log: `RUST_LOG=trace syncweb ...`
- SQLite: `sqlite3 ~/.local/share/syncweb/node.db` (and `stats.db`)

## Last Verified Run (provenance)

- VMs: `syncweb-a` (`10.0.3.2`) and `syncweb-b` (`10.0.3.3`) via `iiab-vm` +
  `nsenter` (enter the container-init PID under the `systemd-nspawn@…` unit,
  not the supervisor).
- Binary SHA-256: `d8df56b292da232bd3f135aef515410f2436831923ccf3420f4944715bcd9ccc`.
- Data: fresh profiles; Alice `shared`/`twoway`, Bob `received`/`shared`; daemons
  run with `start --bg --no-relay`. Folder paths were registered absolute; see
  `NF-9`.
- Regression tests: `cargo nextest run` → 668 passed. New coverage:
  `syncweb-cli/tests/daemon_integration_test.rs`:
  `test_watch_dry_run_missing_path_is_reported`,
  `test_subscribe_transfer_records_bandwidth_stats`,
  `test_relative_folder_paths_are_absolute_in_daemon`,
  `test_watch_once_honors_syncignore`,
  `test_inactive_schedule_window_pauses_folder`;
  `syncweb-core/src/filter.rs`:
  `config_parsing_tests::unknown_top_level_tables_are_rejected`.
- VM network isolation: `iiab-vm` installs a bridge-forward rule that drops
  container-to-container traffic:
  `nft list table bridge iiab` → `iifname "vb-*" oifname "vb-*" drop`. This is
  an L2 bridge filter, so it only kills direct container-to-container
  traffic (ARP `INCOMPLETE`, ping 100% loss). Two ways around it:
  - Direct: temporarily allow it, e.g.
    `nft insert rule bridge iiab forward iifname "vb-syncweb-a" oifname "vb-syncweb-b" accept`
    (plus the reverse). Used for this run: ping A↔B `0% packet loss`, direct
    LAN transfers succeeded.
  - Relay: run the daemons with relay mode enabled (the default; do not
    pass `--no-relay`). Relay traffic is routed out the container's default
    gateway and NAT'd, so it never traverses the bridge `forward` hook.

### Fixed Since the Previous Run

- Removed the unimplemented `SyncMode::ReceiveEncrypted` (and the
  `--mode receiveencrypted` spelling) everywhere: the enum, `Display`/`FromStr`,
  CLI help, docs, and tests. It never encrypted anything; `receiveonly` is the
  supported read-only mode. `--mode receiveencrypted` now fails with
  `Error: invalid sync mode: receiveencrypted`. See
  `docs/conflict-resolution-plan.md` for the follow-up conflict work.
- `NF-1` `--no-daemon` hangs: every embedded command now fails fast with
  `Error: a daemon owns <data-dir>; stop it before using --no-daemon` instead of
  blocking on the store lock (`access`, `folders`, `ls`, … verified; rc=1).
- `NF-2` `folders create` was not idempotent per path: repeated
  `folders create ./shared` now reuses the existing registration
  (`folders: 1`, `folder_mounts` rows: `1`) in both the daemon and embedded
  paths.
- `NF-3` fresh cross-VM transfer: was environmental (see bridge isolation
  above). With the accept rule the fresh join transfers correctly. Not a code
  bug.
- Read-only watcher retry loop: `SyncwebFolder` now records whether its replica
  holds the write key, and the daemon skips watchers (and ignores queued
  events) for read-only folders. Verified 0 retry warnings, 0 errors, content
  still delivered.
- `NF-4` `watch --dry-run --paths` no longer aborts on a non-existent path. A
  missing path is now evaluated (size 0) and reported:
  ```text
  $ syncweb --data-dir /root/nf4 watch --dry-run --paths ./shared/alpha.txt ./shared/x.tmp ./shared
  accept	./shared/alpha.txt
  accept	./shared/x.tmp
  accept	./shared/alpha.txt
  RC=0
  ```
  (Previously: `accept ./shared/alpha.txt` then
  `Error: No such file or directory (os error 2)`, rc=1.)
- `NF-8` `stats network` now accounts for daemon-driven subscription transfers.
  The daemon attaches a live `BandwidthAccountant` to each supervised intent; the
  `folders join --subscribe` path wakes the supervisor immediately so it is
  running before the initial transfer. A fresh 50 000 000 B join produced:
  ```text
  total_upload:  0 B
  total_download: 47 MB
  sqlite3 stats.db ... → download|50000021|3
  ```
  (Previously `total_download: 0 B` and an empty `bandwidth_events`.) Live
  transfers added after subscription are accounted too (5 MB / 6 000 000 B
  event seen). Uploads are still not counted (no seeding-side hook).
- `NF-9` relative folder paths were stored verbatim, but the daemon's watcher
  runs with a different working directory, so `folders join ./received` logged
  `managed folder path does not exist; watcher deferred` and never
  materialized/watched the folder. `folders create`/`folders join` now resolve
  the path to an absolute path before handing it to the daemon:
  ```text
  $ cd /root/reltest && syncweb --data-dir /root/reltest folders create --write --no-share ./shared
  ... Path: /root/reltest/shared ...
  grep -c "does not exist" daemon.log → 0
  ```
- `NF-10` `.syncignore` is now honored by the scanner. `Scanner`/`Importer`
  read `.syncignore` from the scan root, merge its non-comment lines with the
  `--exclude` globs, and always exclude the ignore file itself. Verified on
  `syncweb-a`:
  ```text
  $ syncweb --data-dir /root/nf10/data --no-daemon watch --once /root/nf10/dir
  imported: 1
  $ syncweb --data-dir /root/nf10/data --no-daemon ls /root/nf10/dir
  | keep.txt | 5 B | ... | local |
  ```
  Regression: `test_watch_once_honors_syncignore`.
- `NF-12` an inactive schedule window now pauses syncing. `Daemon::run_cycle`
  only starts supervision for folders whose window is active and cancels any
  live session outside it; `run_trigger` (`sync <namespace>`) consults
  `schedule_active_for` instead of activating unconditionally. Verified on
  `syncweb-a` with `active_hours` two hours in the future:
  ```text
  $ syncweb --data-dir /root/nf12/data folders
  | ... | sendreceive | /root/nf12/dir | no | - | 0 | 0 |
  ```
  (Previously: `Active yes` / `Session: active`.) Regression:
  `test_inactive_schedule_window_pauses_folder`.
- Section 14: `FilterConfig` now uses `deny_unknown_fields`, so a mistyped
  `[general]` table or a singular `[[rule]]` is reported instead of silently
  ignored:
  ```text
  $ syncweb --no-daemon watch --show-filters --filters /root/sec14-bad.toml
  Error: failed to parse filter configuration: TOML parse error at line 1, column 2
    |
  1 | [general]
  ```
  A valid `[[rules]]` file still parses. Regression:
  `config_parsing_tests::unknown_top_level_tables_are_rejected`.

---

## 13.2 Bandwidth Verification

- FAIL (`NF-11`): the configured bandwidth cap is not enforced. With a global
  window `[[schedule.bandwidth]] hours = "00:00-23:59" max_download = "2M"` on
  the receiver, a live subscription transfer delivered a 10 000 000 B payload
  in ~0.24 s (~42 MB/s), far above the 2 MB/s cap. Reproduced on `syncweb-b`
  (receiver) subscribed to `syncweb-a` by polling `stats network` while
  `syncweb-a` imported a 10 MB blob:
  ```text
  B total_download=10000016 delta=10000000 elapsed=0.237720021
  ```
  `SyncEngine` only pauses *after* `ContentReady`, but iroh-docs has already
  eagerly downloaded the blob by then, so the throttle cannot bound the
  transfer. iroh-blobs has no download-rate knob, and iroh-docs eager
  downloads are not gated by a bandwidth policy, so a real fix needs a
  download-policy + explicit throttled fetch design.

## 16. Conflict Resolution

- FAIL (`NF-6`): no conflict handling exists. Alice (`sendreceive`) and Bob
  (`sendreceive`, joined from a `share --write` ticket) started from
  `note.txt = "base version"`; both daemons were stopped, each side rewrote
  `note.txt`, imported with `--no-daemon`, and the daemons were restarted and
  synced. Afterwards each node still had its own text and no `.diff` or
  `.conflict.<hash>` file:
  ```text
  A note.txt: alice text version   B note.txt: bob text version
  A conflicts: (none)              B conflicts: (none)
  ```
  Divergent entries simply do not converge on disk. `docs/offline-conflict.md`
  describes the intended LWW + `.diff`/`.conflict.<hash>` UX, but no code
  implements it, and there is no automatic materialization of remote entries.

## 17. Offline Queue

- FAIL / unstable (`NF-7`): after both daemons were stopped, Alice created
  `alice-only.txt` and Bob created `bob-only.txt`; after restart, import, and
  `sync`, the new files did **not** appear on the peer at all in this run —
  Alice's `ls` listed only `alice-only.txt` + `note.txt`, Bob's only
  `bob-only.txt` + `note.txt`:
  ```text
  A ls: alice-only.txt note.txt
  B ls: bob-only.txt note.txt
  ```
  Bob's daemon log showed the supervised intent exhausting its retries:
  ```text
  WARN sync failed ... err="Remote peer aborted sync: NotFound"
  WARN supervised intent stopped ... retry_count=3, error="Remote peer aborted
  sync: NotFound"
  ```
  A previous run did observe doc-level propagation of distinct files, so the
  behavior is not deterministic; it is coupled to `NF-6`
  (no conflict/materialization) and peer re-discovery/serving timing. No file
  is materialized to disk in any case.

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
  There is no BEP TLS handshake, session establishment, or QUIC tunnel, and
  `bep.*` is never consumed by the daemon/sync transport (it is only read by
  `network test-relay` via `relay_config()`). The actual runtime relay is
  iroh's own `RelayMode`. No "relay connected" log is emitted.
- `syncweb devices` Syncthing DeviceId format should still be confirmed against
  a reference implementation.

## 21. Discovery Mechanisms

- Untested: two nodes on different networks discovering via DHT (~5-10 s) or
  gossip.

## 24. Integrity & Error Recovery

- BLOCKED: real corruption/repair (delete a blob, `verify` reports it, `--fix`
  re-downloads) is not reproducible because blob content lives inside the
  `redb` store — there is no per-blob file to delete or corrupt.

## 25. Performance Smoke Tests

- INCONCLUSIVE: `iiab-vm` nodes have only 2.4 GB allocated, so a 10 GB folder
  cannot be staged. Scaled-down smoke: 512 MB transferred end-to-end over the
  bridge in ~1.29 s ≈ 396 MB/s (target is > 500 MB/s for 10 GB over LAN). This
  is a throughput observation, not a clean pass or fail.

## New Failures Found (2026-10-05)

These are the still-open new failures found while exercising the sections
above. The run stopped short of ten.

(NF-9 relative folder paths, NF-10 `.syncignore`, and NF-12 inactive schedule
windows were found in previous runs and fixed; they are listed under "Fixed
Since the Previous Run". NF-11 below remains open.)

### NF-11 configured bandwidth cap is not enforced

- Repro: receiver config `[[schedule.bandwidth]] hours="00:00-23:59"
  max_download="2M"`; sender publishes a 10 MB blob over a live subscription.
- Expected: download bounded near 2 MB/s (~5 s).
- Actual: a 10 000 000 B payload arrived in ~0.24 s (~42 MB/s).
- Location: `SyncEngine::throttle_bandwidth` runs on `ContentReady`, after
  iroh-docs has already fetched the blob, so it cannot bound the transfer.
  iroh-blobs exposes no download-rate limit and iroh-docs eager downloads are
  not gated by a bandwidth policy, so a real fix needs a download-policy plus
  explicit throttled-fetch design.

---

## Known Limitations / Notes

- Blob corruption/repair tests (Sections 6, 18, 24) are not reproducible while
  blob content lives inside the `redb` store; there is no per-blob file to
  corrupt.
- `receiveencrypted` was removed as a `SyncMode` (it never had an
  encryption-at-rest implementation); `--mode receiveencrypted` is now rejected
  as an invalid mode. Use `receiveonly` for read-only joins.
- Conflict resolution (`docs/offline-conflict.md`) and automatic
  materialization of remote entries are unimplemented; see
  `docs/conflict-resolution-plan.md` for options and the recommended approach.
- Offline reconnect after a daemon restart is unstable: a supervised intent can
  exhaust its retries with `Remote peer aborted sync: NotFound` when the peer
  has the folder open but is not itself running a live-sync intent (NF-7).
- `.syncignore` lines are merged with `--exclude` globs by
  `Scanner`/`Importer`; the ignore file itself is always excluded (NF-10 fix).
- A folder outside its scheduled `active_hours` is not supervised at all
  (`run_cycle` cancels the live session; `sync <namespace>` refuses), so it
  reports `Active no` / `Session: paused` until the window opens (NF-12 fix).
- `FilterConfig` rejects unknown top-level tables, so `[general]` typos and a
  singular `[[rule]]` are parse errors (Section 14 fix).
- `bep.*` config and `syncweb devices` Syncthing identity exist, but there is
  no BEP transport wired into sync.
- The VM pair is not offline: DNS and HTTPS egress work, and both direct
  (with an `nft` accept rule) and relay-mode P2P deliver across the
  `iiab-vm` bridge isolation.
- Relative folder paths are resolved to absolute paths by `folders create` and
  `folders join` (NF-9 fix).
- Plan-syntax corrections already applied in-body: `find` requires a positional
  pattern; `package upgrade <manifest-ticket>` (not the collection id);
  `package remove <collection> <version>` (version required);
  `package export <package-dir> <output>` (filesystem paths, not a collection
  id); `package import` always installs (`--no-install` does not exist);
  `link create --expires <unix-ts>` (not a duration);
  `network test-relay <RELAY_URL>` (required positional); `config set
  discovery.mdns <bool>` (not `discovery.local_mdns`); the node-database debug
  queries use real tables (`folder_mounts`, `shares`; there is no
  `folder_configs`, `snapshot_metadata`, or `folder_peers` table).

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
