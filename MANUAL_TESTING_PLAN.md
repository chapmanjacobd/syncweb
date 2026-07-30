# Syncweb Manual Testing Plan

## Setup & Prerequisites

- Build: `cargo build --release` (binary at `target/release/syncweb`)
- Two machines (or a loopback test with two terminals) needed for most sync tests
- Data dirs: `~/.local/share/syncweb/` (default) — contains `node.db`, `stats.db`, `blobs/`, `docs/`
- Debug log: `RUST_LOG=debug syncweb ...` or `syncweb --verbose ...`
- Trace log: `RUST_LOG=trace syncweb ...`
- SQLite debugging (replace `~/.local/share/syncweb` with your `--data-dir`):
  ```bash
  sqlite3 ~/.local/share/syncweb/node.db
  sqlite3 ~/.local/share/syncweb/stats.db
  ```

---

## 1. Initialization & Configuration

### 1.1 First Run / Init

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Run `syncweb init ./test-folder` | Creates `./test-folder/`, outputs `sync://<node-id>/<namespace-id>` URL | Check dir exists: `ls -la test-folder/` |
| 2 | Run `syncweb init --sync-mode sendonly ./test-sendonly` | Creates folder with SendOnly mode | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM folder_configs;"` |
| 3 | Run `syncweb init --label "My Docs" --network home ./test-network` | Creates folder linked to network "home" | Verify with `syncweb config show networks` |
| 4 | Run `syncweb init --sync-mode receiveencrypted ./test-encrypted` | Creates ReceiveEncrypted folder | Check folder list: `syncweb folders` |

### 1.2 Config Management

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb config` | Shows full config TOML | |
| 2 | `syncweb config show schedule` | Shows schedule section only | |
| 3 | `syncweb config show bep` | Shows Syncthing relay section | |
| 4 | `syncweb config set default_sync_mode ReceiveOnly` | Updates config | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM app_config WHERE key='default_sync_mode';"` |
| 5 | `syncweb config set bandwidth.max_download 5MB/s` | Updates bandwidth config | |
| 6 | `syncweb config show` (after changes) | Shows updated values | |

### 1.3 Database Maintenance

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb db check` | Returns "integrity check passed" | Manual: `sqlite3 ~/.local/share/syncweb/node.db "PRAGMA integrity_check;"` |
| 2 | `syncweb db stats` | Shows table row counts, sizes | Manual: `sqlite3 ~/.local/share/syncweb/node.db "SELECT COUNT(*) FROM daemon_lifecycle;"` |
| 3 | `syncweb db vacuum` | Reclaims space | `sqlite3 ~/.local/share/syncweb/node.db "PRAGMA freelist_count;"` before/after |
| 4 | `syncweb db backup --output /tmp/syncweb-backup` | Creates backup zip | Check file exists: `ls -la /tmp/syncweb-backup*` |

---

## 2. Daemon Lifecycle

### 2.1 Start & Stop

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb start --foreground` | Daemon starts in foreground, shows "daemon running" | `RUST_LOG=debug syncweb start --foreground` |
| 2 | Ctrl+C on daemon | Graceful shutdown, logs "daemon stopped" | Check lifecycle: `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM daemon_lifecycle;"` |
| 3 | `syncweb start` (background) | Daemon forks to background, returns to prompt | |
| 4 | `syncweb status` | Shows PID, uptime, bandwidth rates, folder statuses | Manual DB check: `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM daemon_status;"` |
| 5 | `syncweb shutdown` | Daemon stops, status shows "not running" | `syncweb status` returns error or "no daemon" |
| 6 | Start daemon, then `syncweb shutdown --force` | Force kills daemon | Check PID gone: `ps aux | grep syncweb` |

### 2.2 Reload & Sync Commands

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Daemon running, change config.toml, then `syncweb reload` | Daemon reloads config, no restart needed | Check logs for "config reloaded" |
| 2 | `syncweb daemon-sync` | Triggers sync for all folders | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM sync_checkpoints ORDER BY last_updated_at DESC LIMIT 5;"` |
| 3 | `syncweb daemon-sync --namespace <id>` | Triggers sync for specific folder | |
| 4 | `syncweb daemon-add ./new-folder` | Adds folder to running daemon | `syncweb folders` shows new folder |
| 5 | `syncweb daemon-remove <namespace-id>` | Removes folder from daemon | `syncweb folders` no longer shows it |

---

## 3. Folder Sync (Core P2P)

### 3.1 Create & Join (Two Devices)

Setup: Node A (alice) and Node B (bob), each with `syncweb` installed.

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice: `syncweb init --sync-mode sendreceive ./shared-docs` | Creates folder, prints URL | Save the URL |
| 2 | Alice: `echo "hello world" > shared-docs/test.txt` | File created | |
| 3 | Alice: `syncweb import ./shared-docs` | Imports file into blob store | `syncweb ls ./shared-docs` shows test.txt |
| 4 | Bob: `syncweb join <alice-url> ./bob-shared` | Joins folder, starts syncing | Wait for discovery (~5-30s) |
| 5 | Bob: `syncweb ls ./bob-shared` | Shows test.txt (lazy, no blob yet) | |
| 6 | Bob: `syncweb download ./bob-shared/test.txt` | Downloads the blob | Check: `cat bob-shared/test.txt` shows "hello world" |
| 7 | Bob: `echo "bob edit" >> bob-shared/test.txt && syncweb import ./bob-shared` | Bob imports change | |
| 8 | Alice: wait, then `syncweb download ./shared-docs/test.txt` | Gets Bob's edit | `cat shared-docs/test.txt` shows both lines |

### 3.2 Sync Modes

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice: `syncweb init --sync-mode sendonly ./sendonly` | SendOnly folder | |
| 2 | Alice: create file, `syncweb import` | File available remotely | |
| 3 | Bob: `join` the folder | Can read but writes are rejected | Bob tries: `echo "x" > sendonly/x.txt && syncweb import` → error |
| 4 | Alice: `syncweb init --sync-mode receiveonly ./recvonly` | ReceiveOnly folder | |
| 5 | Bob: `join` the folder | Can write but Alice ignores Bob's writes | |
| 6 | Alice: `syncweb init --sync-mode receiveencrypted ./enc` | ReceiveEncrypted folder | |
| 7 | Bob: `join` the folder | Can write, but blobs are encrypted at rest | |

### 3.3 Leave / Drop

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice: `syncweb leave ./shared-docs` | Leaves the folder | `syncweb folders` no longer shows it |
| 2 | Alice: `syncweb join <url> ./shared-docs` again | Can rejoin | |
| 3 | Alice: `syncweb devices` | Lists Bob's device | |
| 4 | Alice: `syncweb drop <bob-device-id>` | Removes Bob's access | Bob sees disconnection in logs |

### 3.4 Folders & Devices Listing

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb folders` | Table with Name, Mode, Local count, Remote count, State | Empty state shows "no folders" |
| 2 | `syncweb folders --json` | JSON output | Valid JSON: `syncweb folders --json \| jq .` |
| 3 | `syncweb devices` | Lists peers, connection status | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM folder_peers;"` |
| 4 | `syncweb devices --bep` | Shows Syncthing-compatible DeviceIds | |
| 5 | `syncweb devices --json` | JSON output | |

---

## 4. Listing, Searching, Sorting, Stat

### 4.1 `ls` Command

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb ls ./shared-docs` | Lists all entries (lazy, no blob download) | |
| 2 | `syncweb ls --threads 1 ./shared-docs` | Sequential (slower) listing | Compare speed with default (parallel) |
| 3 | `syncweb ls ./shared-docs/nested/` | Lists subdirectory entries | |
| 4 | `syncweb ls --json ./shared-docs` | JSON output per entry | |

### 4.2 `find` Command

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb find '.*\.txt$' ./shared-docs` | Regex find — shows all .txt files | |
| 2 | `syncweb find --glob '/*.md' ./shared-docs` | Glob find | |
| 3 | `syncweb find --fixed-string 'report' ./shared-docs` | Substring/exact find | |
| 4 | `syncweb find --type f --ext mp3 --min-size 10MB --max-size 500MB ./music` | Combined filters | |
| 5 | `syncweb find 'report.*' --modified-within 7d ./shared-docs` | Time filter | |
| 6 | `syncweb find --depth +2 --depth -5 'config' ./shared-docs` | Depth constraints | |
| 7 | `syncweb find '.*\.txt$' --json ./shared-docs` | JSON output | |
| 8 | `syncweb find --ignore-case 'README' ./shared-docs` | Case-insensitive | |

### 4.3 `sort` Command

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb sort ./shared-docs` | Default sort (niche + frecency) | |
| 2 | `syncweb sort --sort peers ./shared-docs` | Most-seeded first | |
| 3 | `syncweb sort --sort niche ./shared-docs` | Files with ~N seeders ranked highest | |
| 4 | `syncweb sort --sort +niche ./shared-docs` | Most niche first | |
| 5 | `syncweb sort --sort -niche ./shared-docs` | Least niche first | |
| 6 | `syncweb sort --sort frecency ./shared-docs` | Popular + recent first | |
| 7 | `syncweb sort --sort peers --sort time ./shared-docs` | Multi-criteria | |
| 8 | `syncweb sort --limit-size 10GB --min-seeders 2 ./shared-docs` | With limits | |

### 4.4 `stat` Command

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb stat ./shared-docs/test.txt` | Shows size, blocks, type, permissions, timestamps, version, availability, modified_by | |
| 2 | `syncweb stat --terse ./shared-docs/test.txt` | Pipe-separated output | |
| 3 | `syncweb stat --format '%n %s %y' ./shared-docs/test.txt` | Custom template | |
| 4 | Modify file locally but don't sync yet, run `syncweb stat` | Shows local vs global diffs | |
| 5 | `syncweb stat ./shared-docs/*.md` | Multiple files | |

---

## 5. Download & Import/Export

### 5.1 Download

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb download ./shared-docs/test.txt` | Downloads single file | Check file exists: `cat ./shared-docs/test.txt` |
| 2 | `syncweb download ./shared-docs/` | Downloads entire folder | `ls -la ./shared-docs/` shows all files |
| 3 | `syncweb download --limit 10 ./shared-docs/` | Downloads at most 10 entries | |
| 4 | `syncweb download --size 1GB ./shared-docs/` | Skips blobs >1GB | |
| 5 | `syncweb download --threads 1 ./shared-docs/` | Sequential download (no parallelism) | Compare speed with default |
| 6 | Piped: `syncweb find '*.iso' ./shared-docs \| syncweb download -` | Pipe from stdin | |

### 5.2 Import

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb import ./shared-docs/` | Scans + imports all files | `syncweb ls ./shared-docs` shows new entries |
| 2 | `syncweb import --threads 1 ./shared-docs/` | Sequential import | |
| 3 | Create nested dir structure, then `syncweb import ./shared-docs/` | Respects directory structure | |
| 4 | `syncweb import /tmp/new-files ./shared-docs/` | Import from different source path | |

### 5.3 Export

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb export ./shared-docs/ /tmp/export-test` | Exports all blobs to filesystem | `ls /tmp/export-test` shows files |
| 2 | `syncweb export --threads 1 ./shared-docs/ /tmp/export-test` | Sequential export | Compare speed with default |

---

## 6. Health & Verify

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb health ./shared-docs` | Shows per-blob seeding: well/under/unseeded with counts | |
| 2 | `syncweb health --json ./shared-docs` | JSON output | |
| 3 | `syncweb verify ./shared-docs` | Checks all local blobs against doc entries | Reports any corrupted/missing |
| 4 | Manually corrupt a blob file in blob store, then `syncweb verify` | Reports corruption | Check blob store path: `ls ~/.local/share/syncweb/blobs/` |
| 5 | `syncweb verify --fix ./shared-docs` | Re-downloads corrupted blobs | |

---

## 7. Public Folders & Publishing

### 7.1 Publish / Subscribe

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice: `syncweb publish ./shared-docs` | Creates public blob ticket, outputs URL | Save the ticket |
| 2 | Alice: `syncweb publish --limit 100 --size 10GB ./shared-docs` | Published with limits | |
| 3 | Bob: `syncweb subscribe <ticket> ./bob-public` | Subscribes to public folder | `syncweb ls ./bob-public` shows files |
| 4 | Bob: `syncweb download ./bob-public/` | Downloads content | |
| 5 | Alice: `syncweb unpublish ./shared-docs` | Removes pin, stops announcing | Bob can no longer see updates |

### 7.2 Public List

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb public list` | Lists announced public folders | May be empty if none announced |

---

## 8. Snapshots

### 8.1 Create & List

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb snapshot create ./shared-docs --description "before big edit"` | Creates snapshot, returns ID | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM snapshot_metadata;"` |
| 2 | `syncweb snapshot list ./shared-docs` | Lists all snapshots for folder | |
| 3 | Create multiple snapshots, list them | All shown with descriptions, timestamps | |

### 8.2 Diff & Restore

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Make changes, create another snapshot | Two snapshots exist | |
| 2 | `syncweb snapshot diff ./shared-docs <id1> <id2>` | Shows added/removed/changed files | |
| 3 | `syncweb snapshot restore ./shared-docs <id1>` | Restores to snapshot state | Verify: `syncweb ls ./shared-docs` matches original |
| 4 | `syncweb snapshot delete ./shared-docs <id2>` | Deletes snapshot | `syncweb snapshot list` no longer shows it |

### 8.3 Alias Commands

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb backup ./shared-docs --description "test backup"` | Same as snapshot create | |
| 2 | `syncweb restore ./shared-docs <id>` | Same as snapshot restore | |
| 3 | `syncweb snapshots ./shared-docs` | Same as snapshot list | |

---

## 9. Collections & Packages

### 9.1 Collection Lifecycle

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb collection init ./pkg-dir` | Initializes collection, creates manifest | `ls ./pkg-dir/` shows manifest |
| 2 | Add files to `./pkg-dir/`, then `syncweb collection add ./pkg-dir` | Scans + hashes, updates manifest | |
| 3 | `syncweb collection versions ./pkg-dir --changelog "v1 initial"` | Creates new manifest version | |
| 4 | `syncweb collection publish ./pkg-dir` | Stores manifest, pins content, announces blob ticket | Outputs ticket URL |

### 9.2 Package Install / Upgrade / Remove

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice: publish collection | Get the ticket from output | |
| 2 | Bob: `syncweb package search ./ --query "pkg"` | Discovers Alice's package via gossip | |
| 3 | Bob: `syncweb package info <collection-id>` | Shows metadata, versions | |
| 4 | Bob: `syncweb package install <collection-id> ./pkg-install` | Fetches, verifies, installs atomically | Verify: `ls ./pkg-install/` has files |
| 5 | Bob: `syncweb package list` | Shows installed packages | |
| 6 | Bob: `syncweb package versions <collection-id>` | Lists installed versions | |
| 7 | Bob: `syncweb package verify <collection-id>` | Integrity check passes | |
| 8 | Bob: `syncweb package upgrade <collection-id>` | Upgrades to latest | |
| 9 | Bob: `syncweb package switch <collection-id> v1` | Switches to v1 via symlink | Check version: `cat ./pkg-install/.version` |
| 10 | Bob: `syncweb package remove <collection-id>` | Cleanly removes | `syncweb package list` no longer shows it |

### 9.3 Package Archive (.car.zst)

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb package export <collection-id> /tmp/pkg.car.zst` | Creates compressed archive | `ls -la /tmp/pkg.car.zst` |
| 2 | On another machine (air-gapped): `syncweb package import /tmp/pkg.car.zst ./pkg-import` | Imports and installs | `syncweb package list` shows it |
| 3 | `syncweb package import --no-install /tmp/pkg.car.zst /tmp/extract` | Extracts without installing | |

---

## 10. Networks

### 10.1 Create & Invite

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice: `syncweb network create home` | Creates network "home" | `syncweb network ls` shows it |
| 2 | Alice: `syncweb network ls home` | Shows network details | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM networks;"` |
| 3 | Alice: `syncweb network invite home <bob-device-id>` | Creates invitation ticket | |
| 4 | Bob: `syncweb network join <ticket>` | Joins network "home" | `syncweb network ls` shows it |

### 10.2 Folder in Network Context

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice: `syncweb init --network home ./nw-docs` | Creates folder in "home" network | `syncweb network ls home` shows the folder |
| 2 | Alice imports files, Bob joins the folder | Bob gets auto-discovery via network gossip | |
| 3 | Alice: `syncweb network kick home <bob-device-id>` | Removes Bob from network | Bob disconnects |

### 10.3 Network Events & Health

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb network events home` | Shows peer joins, leaves, sync events | `sqlite3 ~/.local/share/syncweb/stats.db "SELECT * FROM network_events WHERE network_id='home' ORDER BY timestamp DESC LIMIT 10;"` |
| 2 | `syncweb network health home` | Network connectivity health | |
| 3 | `syncweb network test-relay` | Tests relay connectivity | |

---

## 11. Indexing Service

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb indexing enable ./shared-docs` | Enables FTS5 index for folder | Check indexing db exists: `ls ~/.local/share/syncweb/indexing.sqlite` |
| 2 | `syncweb indexing search "test" --in ./shared-docs` | Full-text search results | |
| 3 | `syncweb indexing health ./shared-docs` | Lease/health status | |
| 4 | `syncweb indexing publish --catalog ./shared-docs` | Publishes to catalog namespace | |
| 5 | `syncweb indexing meta add ./shared-docs --key "description" --value "My docs"` | Adds signed metadata | |
| 6 | `syncweb indexing disable ./shared-docs` | Disables indexing | |

---

## 12. Links

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb link create --immutable ./shared-docs/test.txt` | Creates immutable link | |
| 2 | `syncweb link create --mutable ./shared-docs/test.txt` | Creates mutable pointer | |
| 3 | `syncweb link create --private ./shared-docs/test.txt --expires 7d` | Creates expiring private link | |
| 4 | `syncweb link resolve <link-id>` | Resolves to manifest + providers | |
| 5 | `syncweb link revoke <link-id>` | Revokes private link | Resolution fails after revocation |

---

## 13. Mirroring

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb mirror <provider-node-id>` | Mirrors all blobs from provider | Shows progress |
| 2 | `syncweb mirror --network home` | Mirrors all blobs in network "home" | |
| 3 | `syncweb mirror <provider-node-id> --dry-run` | Shows what would be mirrored | |
| 4 | `syncweb mirror <provider-node-id> --no-share` | Downloads but doesn't re-share | |

---

## 14. Schedules & Bandwidth

### 14.1 Schedule Configuration

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb schedule` | Shows current global schedule | |
| 2 | `syncweb schedule set --active "22:00-06:00"` | Sets active hours | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM app_config WHERE key LIKE 'schedule%';"` |
| 3 | `syncweb schedule set --bandwidth "5MB/s" --period "08:00-18:00"` | Time-based bandwidth limit | |
| 4 | `syncweb schedule folder media --active "01:00-05:00"` | Per-folder override | |
| 5 | `syncweb schedule` (check) | Shows updated schedule | |

### 14.2 Bandwidth Verification

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Sync large files while monitoring `syncweb stats` | Bandwidth is capped at configured limit | `sqlite3 ~/.local/share/syncweb/stats.db "SELECT SUM(bytes) FROM bandwidth_events WHERE direction='download';"` |
| 2 | Wait for inactive window, trigger sync | Sync does not start (or is delayed) | |
| 3 | `syncweb stats` | Shows totals, per-folder, per-peer | |
| 4 | `syncweb stats --period 24h` | Last 24 hours | |
| 5 | `syncweb stats --folder <namespace>` | Per-folder breakdown | |

---

## 15. Filter Engine / Automatic Mode

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Create `~/.config/syncweb/filters.toml` with rules | | See example below |
| 2 | `syncweb automatic --show-filters` | Shows loaded filter rules | |
| 3 | `syncweb automatic --dry-run` | Shows what would be matched without downloading | |
| 4 | `syncweb automatic` | Runs rules-based auto-sync daemon | |
| 5 | Edit filters.toml, then `syncweb automatic --reload` | Reloads filters without restart | |
| 6 | `syncweb config set filter.config_path /path/to/filters.toml` | Custom filter config path | |

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

---

## 16. Watch Mode (File Watcher)

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb watch ./shared-docs` | Watches folder for changes | Trigger a change and observe |
| 2 | Touch/create/delete/modify a file in `./shared-docs` | Watcher detects and imports change | Logs show "file changed: <path>" |
| 3 | `syncweb watch --once ./shared-docs` | Scans once, then exits | |
| 4 | Create `.syncignore` with glob patterns | Watcher excludes matching files | |

---

## 17. Conflict Resolution

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice and Bob both modify the same file offline | Both have local edits | |
| 2 | Both come online and sync | Conflict detected | |
| 3 | `syncweb conflicts ./shared-docs` | Lists unresolved conflicts | |
| 4 | For text files: auto-resolve keeps newer (LWW), saves .diff | Conflict auto-resolved or listed | Check for `.conflict` or `.diff` files |
| 5 | `syncweb conflicts --auto-resolve` | Resolves all automatically | |
| 6 | `syncweb conflicts resolve <id> --keep-local` | Keep local version | |
| 7 | `syncweb conflicts resolve <id> --keep-remote` | Keep remote version | |
| 8 | Binary file conflict | Creates `.conflict.<hash>` file | Both versions preserved |

---

## 18. Offline Queue

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Go offline (disconnect network) | | |
| 2 | Make changes to synced folder | | |
| 3 | `syncweb pending` | Shows pending changes queued | |
| 4 | Come back online | Pending changes sync automatically | |
| 5 | `syncweb pending` (after sync) | No pending changes | |

---

## 19. Media Server

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb media` (daemon must be running) | Starts HTTP server on 127.0.0.1:9193 | |
| 2 | `curl http://127.0.0.1:9193/media/<blob-hash>` | Serves blob content | Compare with `syncweb stat` output for hash |
| 3 | `curl -H "Range: bytes=0-100" http://127.0.0.1:9193/media/<hash>` | Partial content (206) with first 100 bytes | Check Content-Range header |
| 4 | `curl http://127.0.0.1:9193/` | 404 or list | |
| 5 | Configure custom listen address in config | Server starts on specified address | |

---

## 20. WebSocket Bridge

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Run daemon (bridge starts on 127.0.0.1:9192) | Bridge listening | |
| 2 | Connect via `websocat ws://127.0.0.1:9192/bridge` | Connection accepted | Logs show "bridge connection accepted" |
| 3 | Send a valid JSON command over WS | Receives response | See bridge protocol docs |
| 4 | Send invalid JSON | Receives error message | |

---

## 21. Syncthing Relay (BEP)

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Configure BEP: `syncweb config set bep.enabled true` | BEP enabled | `syncweb config show bep` |
| 2 | `syncweb config set bep.relay_urls '["tcp://relay.syncthing.net:22270"]'` | Sets relay | |
| 3 | Two nodes behind CGNAT (or simulate with firewall) | Connection falls back to Syncthing relay | Logs show "relay connected" |
| 4 | `syncweb network test-relay` | Tests relay connectivity | Logs latency and status |
| 5 | `syncweb devices --bep` | Shows DeviceIds | Verify format matches Syncthing DeviceId |

---

## 22. Discovery Mechanisms

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Two nodes on same LAN | Auto-discover via mDNS within ~1s | |
| 2 | Disable mDNS: `syncweb config set discovery.local_mdns false` | No LAN discovery | |
| 3 | Re-enable mDNS | Discovery resumes | |
| 4 | Two nodes on different networks | Discover via DHT (~5-10s) or gossip | |

---

## 23. CLI Global Flags & Output

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb --verbose <command>` | Debug-level output | |
| 2 | `syncweb --json folders` | JSON output | `syncweb --json folders \| jq .` |
| 3 | `syncweb --no-color devices` | Output without ANSI color | |
| 4 | `syncweb --data-dir /tmp/syncweb-test folders` | Uses custom data dir | Check files in /tmp/syncweb-test/ |
| 5 | `syncweb --network home folders` | Shows folders in "home" network context | |
| 6 | `syncweb --help` | Shows help | |
| 7 | `syncweb <command> --help` | Command-specific help | |

---

## 24. Version & Completions

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb version` | Shows version, commit, build date | |
| 2 | `syncweb completions bash` | Outputs bash completion script | Source it: `source <(syncweb completions bash)` |
| 3 | `syncweb completions zsh` | Zsh completions | |
| 4 | `syncweb completions fish` | Fish completions | |
| 5 | `syncweb manpages /tmp/syncweb.1` | Generates manpage | `man /tmp/syncweb.1` |

---

## 25. Integrity & Error Recovery

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Kill daemon with SIGKILL (kill -9) | Hard crash | |
| 2 | Start daemon again | Recovers gracefully, WAL replay fixes DB | Check: `syncweb status` works |
| 3 | Delete a random blob file from blob store | Data missing | |
| 4 | `syncweb verify ./shared-docs` | Reports missing/corrupt blob | |
| 5 | `syncweb verify --fix ./shared-docs` | Re-downloads from peers | |
| 6 | `syncweb db check` | DB integrity passes | |

---

## 26. Performance Smoke Tests

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Time `syncweb start` (cold start) | < 500ms | `time syncweb start --foreground` |
| 2 | Create 10000 files, time `syncweb import` | < 3s (default parallel) | Compare with `--threads 1` |
| 3 | Time `syncweb ls` on 10000 entries | < 500ms | |
| 4 | Sync a 10GB folder over LAN | > 500 MB/s throughput | Monitor: `syncweb stats` |
| 5 | `syncweb health` on folder with 1000+ entries | < 1s | |

---

## Debugging SQL Queries (Node Database)

```bash
# Open node database
sqlite3 ~/.local/share/syncweb/node.db

# Check daemon lifecycle
SELECT * FROM daemon_lifecycle;

# Check daemon status
SELECT * FROM daemon_status;

# List all folders
SELECT * FROM folder_configs;

# List folder status reports
SELECT * FROM folder_status_reports;

# Check sync sessions
SELECT * FROM sync_checkpoints ORDER BY last_updated_at DESC LIMIT 20;

# Check sync progress
SELECT * FROM sync_entry_progress WHERE status = 'failed';

# List networks
SELECT * FROM networks;

# List network members
SELECT * FROM network_members;

# Check filter rules
SELECT * FROM filter_rules;

# Check installed collections
SELECT * FROM installed_collections;

# Snapshots
SELECT * FROM snapshot_metadata;

# App config
SELECT * FROM app_config;

# Peer tracking
SELECT * FROM folder_peers;

# Check schema version
SELECT * FROM schema_version;
```

## Debugging SQL Queries (Stats Database)

```bash
sqlite3 ~/.local/share/syncweb/stats.db

# Bandwidth totals
SELECT direction, SUM(bytes) FROM bandwidth_events GROUP BY direction;

# Recent transfers
SELECT * FROM bandwidth_events ORDER BY timestamp DESC LIMIT 20;

# Per-folder bandwidth
SELECT folder_namespace, SUM(bytes) FROM bandwidth_events GROUP BY folder_namespace;

# Network events
SELECT * FROM network_events ORDER BY timestamp DESC LIMIT 20;

# Sync sessions
SELECT * FROM network_sync_sessions ORDER BY started_at DESC LIMIT 10;

# Relay health
SELECT * FROM relay_health ORDER BY checked_at DESC LIMIT 10;

# Daemon log
SELECT * FROM daemon_log ORDER BY timestamp DESC LIMIT 20;

# Full bandwidth summary
SELECT * FROM network_bandwidth_summary;
```

## Environment Variables

```bash
# Set log level
export RUST_LOG=debug   # or trace, info, warn, error

# Log to file
export RUST_LOG=debug
syncweb start --foreground 2>&1 | tee /tmp/syncweb-debug.log

# Custom data directory
syncweb --data-dir /tmp/syncweb-test ...

# Network isolation (for running two nodes on same machine)
# Start two terminals with different data dirs:
# Terminal 1 (Alice):
syncweb --data-dir /tmp/alice start --foreground
# Terminal 2 (Bob):
syncweb --data-dir /tmp/bob start --foreground
```

## Two-Node Loopback Test Setup

For testing on a single machine:

```bash
# Terminal 1: Alice
mkdir -p /tmp/alice-data /tmp/alice-files
syncweb --data-dir /tmp/alice-data init /tmp/alice-files/shared
# Copy the URL

# Terminal 2: Bob
mkdir -p /tmp/bob-data /tmp/bob-files
syncweb --data-dir /tmp/bob-data join <URL> /tmp/bob-files/shared

# Start both daemons
syncweb --data-dir /tmp/alice-data start --foreground   # Terminal 1
syncweb --data-dir /tmp/bob-data start --foreground     # Terminal 2
```
