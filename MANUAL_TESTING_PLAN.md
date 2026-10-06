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

## Last Verified Run (provenance)

- VMs: `syncweb-a` (`10.0.3.2`) and `syncweb-b` (`10.0.3.3`) via `iiab-vm` +
  `nsenter` (enter the container-init PID under the `systemd-nspawn@…` unit,
  not the supervisor).
- Binary SHA-256: `e37534b7a32f7c9a5c12aa7caf676261fdfe8fd99ccba45e99d352083b145242`.
- Data: fresh profiles; Alice `shared`/`twoway`, Bob `received`/`shared`; daemons
  run with `start --bg --no-relay`. Folder paths were registered absolute; see
  `NF-9`. Offline edits were imported with `--no-daemon watch --once`.
- Regression tests: `cargo nextest run` → 686 passed. Coverage added for the
  fixes below: `syncweb-cli/tests/daemon_integration_test.rs`:
  `test_watch_dry_run_missing_path_is_reported`,
  `test_subscribe_transfer_records_bandwidth_stats`,
  `test_relative_folder_paths_are_absolute_in_daemon`,
  `test_watch_once_honors_syncignore`,
  `test_inactive_schedule_window_pauses_folder`,
  `test_writable_subscribe_materializes_remote_entries`,
  `test_offline_conflict_materializes_conflict_copy`;
  `syncweb-cli/tests/cli_test.rs`: `config_transfer_limits_round_trip`;
  `syncweb-core/src/filter.rs`:
  `config_parsing_tests::unknown_top_level_tables_are_rejected`;
  `syncweb-core/src/transfer_limits.rs`: `tests::*`;
  `syncweb-core/src/daemon/supervisor.rs`:
  `tests::retry_window_is_measured_from_first_attempt`;
  `syncweb-core/src/sync/conflict.rs`: `tests::*`.
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
- `NF-11` rate throttling was removed and replaced with a quota policy.
  The post-`ContentReady` `throttle_bandwidth` sleep is gone (`SyncEngine` no
  longer takes `SubscribeParams.bandwidth`); iroh-blobs has no rate knob and
  iroh-docs eager downloads made it unenforceable. In its place,
  `syncweb-core/src/transfer_limits.rs` implements a qBittorrent-style policy:
  `[transfer_limits]` sets `max_download_per_hour/day/month` and
  `max_upload_per_hour/day/month` byte quotas, and `TransferQueue` holds blobs
  until their full size fits every rolling window for that direction. When a
  download quota is configured the daemon switches managed folders to the
  metadata-only iroh-docs download policy and fetches queued blobs explicitly
  within budget. Verified on the VM pair with `syncweb-a` publishing
  `seed.txt` (5 B), `small.bin` (100 KB), and `big.bin` (10 MB) and a fresh
  `syncweb-b` joined `--subscribe` with a 1 MB/month download limit:
  ```text
  B ls:  big.bin (9 MB) remote | seed.txt 5 B local | small.bin 100 KB local
  B stats network → total_download: 102405   # 5 + 102400 fetched
  ```
  Raising the limit to 20 MB/month (picked up without a restart) let the queue
  drain on the next cycle:
  ```text
  B ls:  big.bin (9 MB) local
  B stats network → total_download: 10102405
  ```
  Regression: `transfer_limits::tests::*`,
  `integration::config_test::transfer_limits_round_trip_and_parse`,
  `config_transfer_limits_round_trip`.
- Intent retries now use exponential backoff bounded by a hard window measured
  from the first attempt, default 3 months
  (`DaemonConfig::retry_window` = `DEFAULT_RETRY_WINDOW`), replacing the old
  3-attempt cap. A failing intent (e.g. `Remote peer aborted sync: NotFound`)
  keeps retrying instead of giving up after three tries. Regression:
  `daemon::supervisor::tests::retry_window_is_measured_from_first_attempt`.
- `NF-6` (Section 16) conflict resolution: no conflict handling existed —
  divergent edits never converged on disk and no `.diff`/`.conflict.<hash>`
  file was written. Now `materialize_folder_content`
  (`syncweb-core/src/sync/conflict.rs`) enumerates every CRDT variant per key
  (`docs_engine.list_all_entries`, plain `Query::all()`), selects the LWW
  winner (greatest record `timestamp()`, ties broken deterministically by
  author bytes), writes the winner at the original path, and writes each loser
  as `<path>.conflict.<8-hex-hash>`. The daemon's per-folder reconciler
  (`daemon.rs` `build_reconciler`, attached via `SyncEngine::with_reconciler`)
  coalesces `ContentReady`/`InsertRemote`/`SyncFinished` sync events into a
  trailing-edge materialization pass. The watcher/Importer no longer re-stamps
  a materialized winner as a new local edit (identical hash short-circuits
  `set_blob`), and conflict copies (`.conflict.*`) are excluded from scans and
  never re-imported. `folders create` now enables live sync for the owner too,
  so the creator's mount receives remote edits. Verified on the VM pair after
  both sides edited `note.txt` offline:
  ```text
  A note.txt: bob text version        B note.txt: bob text version
  A note.txt.conflict.4a313a7e alice  B note.txt.conflict.4a313a7e alice
  ```
  The loser was preserved on both sides as `note.txt.conflict.4a313a7e`.
  Regressions: `test_offline_conflict_materializes_conflict_copy`,
  `sync::conflict::tests::*`.
- `NF-7` (Section 17) offline queue: remote entries stayed doc-only — files
  created offline by the peer never appeared on disk (and in the previous run
  did not even propagate reliably). Now the reconciler materializes remote
  entries of writable folders automatically, so a file added offline by one
  side appears on the peer's disk without any explicit `download`. Verified:
  ```text
  A ls:  alice-only.txt bob-only.txt note.txt
  A disk: alice-only.txt bob-only.txt note.txt note.txt.conflict.4a313a7e
  B disk: alice-only.txt bob-only.txt note.txt note.txt.conflict.4a313a7e
  ```
  No `WARN`/`retry`/`NotFound` lines in either daemon log. Regressions:
  `test_writable_subscribe_materializes_remote_entries`,
  `test_offline_conflict_materializes_conflict_copy`.

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
  There is no BEP TLS handshake, session establishment, or QUIC tunnel, and
  `bep.*` is never consumed by the daemon/sync transport (it is only read by
  `network test-relay` via `relay_config()`). The actual runtime relay is
  iroh's own `RelayMode`. No "relay connected" log is emitted.
- `network test-relay`: `tcp://relay.syncthing.net:22270` from the previous run
  no longer resolves (`relay.syncthing.net` is NXDOMAIN as of 2026-10-05/06;
  the live pool endpoint is `relays.syncthing.net`). Against a live pool relay
  the command succeeds:
  ```text
  $ syncweb network test-relay tcp://91.189.82.132:22067
  relay reachable: tcp://91.189.82.132:22067
  ```
- `syncweb devices` Syncthing DeviceId format should still be confirmed against
  a reference implementation.

## 21. Discovery Mechanisms

- Untested: two nodes on different networks discovering via DHT (~5-10 s) or
  gossip. On a shared bridge the VM pair discovers each other and syncs in
  both modes: default relay (no `nft` accept rule needed) and `--no-relay`
  with the direct accept rule (beacon/mDNS). The base-version propagation and
  the offline-conflict reruns above both rely on this discovery working
  end-to-end on the `iiab-vm` bridge.

## 24. Integrity & Error Recovery

- BLOCKED: real corruption/repair (delete a blob, `verify` reports it, `--fix`
  re-downloads) is not reproducible because blob content lives inside the
  `redb` store — there is no per-blob file to delete or corrupt.
- Non-destructive `verify` passes on both nodes of the sync pair:
  ```text
  A verify /root/mt/shared   → total: 3, verified: 3, corrupted: 0, missing: 0, valid: true
  B verify /root/mt/received → total: 3, verified: 3, corrupted: 0, missing: 0, valid: true
  ```

## 25. Performance Smoke Tests

- INCONCLUSIVE: `iiab-vm` nodes have only 2.4 GB allocated, so a 10 GB folder
  cannot be staged. Scaled-down smoke: 512 MB transferred end-to-end over the
  bridge in ~1.29 s ≈ 396 MB/s (target is > 500 MB/s for 10 GB over LAN). This
  is a throughput observation, not a clean pass or fail.

---

## Known Limitations / Notes

- Blob corruption/repair tests (Sections 6, 18, 24) are not reproducible while
  blob content lives inside the `redb` store; there is no per-blob file to
  corrupt.
- `receiveencrypted` was removed as a `SyncMode` (it never had an
  encryption-at-rest implementation); `--mode receiveencrypted` is now rejected
  as an invalid mode. Use `receiveonly` for read-only joins.
- Conflict resolution (LWW winner at the original path + `<path>.conflict.<hash>`
  losers) is implemented at materialize time and is also run automatically by
  the daemon reconciler for writable folders (`docs/conflict-resolution-plan.md`
  Phase 1 + a guarded Phase 2; the watcher/Importer guards prevent the
  materialized winner from being re-stamped as a new local edit). Text `.diff`
  files and `syncweb conflicts` are not implemented yet.
- `.syncignore` lines are merged with `--exclude` globs by
  `Scanner`/`Importer`; the ignore file itself is always excluded (NF-10 fix).
- A folder outside its scheduled `active_hours` is not supervised at all
  (`run_cycle` cancels the live session; `sync <namespace>` refuses), so it
  reports `Active no` / `Session: paused` until the window opens (NF-12 fix).
- `FilterConfig` rejects unknown top-level tables, so `[general]` typos and a
  singular `[[rule]]` are parse errors (Section 14 fix).
- Transfer limits are configured under `[transfer_limits]`
  (`max_download_per_hour`, `_per_day`, `_per_month`, and the `max_upload_*`
  equivalents). The legacy `[[schedule.bandwidth]]` and `[bandwidth] max_*`
  rate settings have been removed entirely; `schedule` now only controls
  `active_hours`.
- Upload quotas are enforced by pausing live sessions when the upload budget is
  exhausted, but no upload byte hook exists yet (uploads are still not
  counted), so upload gating is structural until that hook lands.
- A single blob larger than every configured download cap stays queued
  indefinitely (by design: it can never fit the rolling budget).
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
