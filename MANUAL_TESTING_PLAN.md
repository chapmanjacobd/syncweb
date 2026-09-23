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

### 1.1 First Run / Create

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Run `syncweb folders create ./test-folder` | Creates `./test-folder/`, prints path, namespace, ticket, and `syncweb://` share URL | Check dir exists: `ls -la test-folder/` |
| 2 | Run `syncweb folders create --mode sendonly ./test-sendonly` | Creates folder with SendOnly mode | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM folder_configs;"` |
| 3 | Run `syncweb folders create --network home ./test-network` | Creates folder linked to network "home" | Verify with `syncweb config show networks` |
| 4 | Run `syncweb folders create --mode receiveencrypted ./test-encrypted` | Creates ReceiveEncrypted folder | Check folder list: `syncweb folders` |

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
| 4b | `syncweb --json status` (daemon running) | Single JSON object `{daemon, folders, devices, networks}` with all four keys present | |
| 5 | `syncweb stop` | Prompts "Are you sure…?" (default no); confirm stops daemon, status shows "not running" | `syncweb status` returns error or "no daemon" |
| 6 | Start daemon, then `syncweb stop --force` | Force kills daemon after the same confirmation | Check PID gone: `ps aux | grep syncweb` |
| 7 | `syncweb stop --yes` in a script/pipe | Skips the prompt and stops the daemon; without `--yes` non-interactive runs abort ("aborted") and the daemon stays up — `--json` does not skip the prompt | `syncweb status` after aborted run still shows "daemon: running" |

### 2.2 Reload & Sync Commands

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Daemon running, change config.toml, then `syncweb reload` | Daemon reloads config, no restart needed | Check logs for "config reloaded" |
| 2 | `syncweb sync` | Triggers sync for all folders | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM sync_checkpoints ORDER BY last_updated_at DESC LIMIT 5;"` |
| 3 | `syncweb sync <ns>` | Triggers sync for a specific folder; warns and does nothing if it is not live | |
| 4 | `syncweb folders create ./new-folder` | Creates folder and adds it to the running daemon | `syncweb folders` shows new folder |
| 5 | `syncweb folders leave <namespace-id>` | Removes folder from daemon | `syncweb folders` no longer shows it |

---

## 3. Folder Sync (Core P2P)

### 3.1 Create & Join (Two Devices)

Setup: Node A (alice) and Node B (bob), each with `syncweb` installed.

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice: `syncweb folders create --mode sendreceive ./shared-docs` | Creates folder, prints URL | Save the URL |
| 2 | Alice: `echo "hello world" > shared-docs/test.txt` | File created | |
| 3 | Alice: `syncweb folders import ./shared-docs` | Imports file into blob store | `syncweb ls ./shared-docs` shows test.txt |
| 4 | Bob: `syncweb folders join <alice-url> ./bob-shared` | Joins folder, starts syncing | Wait for discovery (~5-30s) |
| 5 | Bob: `syncweb ls ./bob-shared` | Shows test.txt (lazy, no blob yet) | |
| 6 | Bob: `syncweb download ./bob-shared/test.txt` | Downloads the blob | Check: `cat bob-shared/test.txt` shows "hello world" |
| 7 | Bob: `echo "bob edit" >> bob-shared/test.txt && syncweb folders import ./bob-shared` | Bob imports change | |
| 8 | Alice: wait, then `syncweb download ./shared-docs/test.txt` | Gets Bob's edit | `cat shared-docs/test.txt` shows both lines |

### 3.2 Sync Modes

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice: `syncweb folders create --mode sendonly ./sendonly` | SendOnly folder | |
| 2 | Alice: create file, `syncweb folders import` | File available remotely | |
| 3 | Bob: `join` the folder | Can read but writes are rejected | Bob tries: `echo "x" > sendonly/x.txt && syncweb folders import` → error |
| 4 | Alice: `syncweb folders create --mode receiveonly ./recvonly` | ReceiveOnly folder | |
| 5 | Bob: `join` the folder | Can write but Alice ignores Bob's writes | |
| 6 | Alice: `syncweb folders create --mode receiveencrypted ./enc` | ReceiveEncrypted folder | |
| 7 | Bob: `join` the folder | Can write, but blobs are encrypted at rest | |

### 3.3 Leave

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice: `syncweb folders leave ./shared-docs` | Leaves the folder | `syncweb folders` no longer shows it |
| 2 | Alice: `syncweb folders leave --delete-files ./shared-docs` | Prompts "Are you sure…?" (default no); confirming deletes local files | Folder directory is removed |
| 3 | Alice: `syncweb folders leave --delete-files --yes ./shared-docs` in a script | Skips the prompt and deletes local files; without `--yes` non-interactive runs abort and files stay | `syncweb folders leave --delete-files` (non-TTY, no `--yes`) prints "aborted" |
| 4 | Alice: `syncweb folders join <url> ./shared-docs` again | Can rejoin | |
| 5 | Alice: `syncweb devices` | Shows this device's Iroh and Syncthing identities | |

### 3.4 Folders & Devices Listing

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb folders` | Table with Name, Mode, Local count, Remote count, State | Empty state shows "no folders" |
| 2 | `syncweb folders --json` | JSON output | Valid JSON: `syncweb folders --json \| jq .` |
| 3 | `syncweb devices` | Shows this device's Iroh and Syncthing identities | |
| 4 | `syncweb devices --json` | JSON output | |

---

## 4. Listing, Searching, Sorting, Stat

### 4.1 `ls` Command

`ls` now reads the doc metadata index (`list_entries()`), never a disk scan;
the disk is touched only to overlay the real size/mtime of files that are
already local. On a path that resolves to no Syncweb folder it errors with
"not inside of a Syncweb folder" — use `--local-only` to list plain directories.

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb ls ./shared-docs` | Lists all entries (lazy, no blob download), `State = local\|remote` | |
| 2 | `syncweb ls --local-only ./shared-docs` | Forces today's disk scan (works on any path, even outside a folder) | |
| 3 | `syncweb ls --remote-only ./shared-docs` | Only undownloaded rows (`State == remote`) | Contradicts `--local-only` |
| 4 | `syncweb ls --path-prefix docs ./shared-docs` | Only entries under `docs/` | `--path-glob '*.md'` also available |
| 5 | `syncweb ls --no-enrich ./shared-docs` | Pure metadata listing (no per-file stat; Size = doc size, Modified = `-`) | |
| 6 | `syncweb ls --sort size ./shared-docs` | Sorts the metadata table by `name\|size\|modified\|state` | On a plain dir these values must run under `--local-only` |
| 7 | `syncweb ls --json ./shared-docs` | Envelope `{folder, path, entries:[{path,size,hash,local,modified?}]}` | |
| 8 | `syncweb ls /plain/dir` | Errors "not inside of a Syncweb folder" | Confirm `--local-only` suggestion in the message |

### 4.2 `find` Command

`find` searches the same metadata index (Python parity); a selector that is not
inside a Syncweb folder errors unless `--local-only` forces a disk scan.
Pattern/size/depth/time/type predicates apply to the metadata rows.

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb find '.*\.txt$' ./shared-docs` | Regex find — shows all .txt entries | |
| 2 | `syncweb find '*.md' ./shared-docs` | Glob find | |
| 3 | `syncweb find --fixed-strings 'report' ./shared-docs` | Substring/exact find | |
| 4 | `syncweb find --type f --ext mp3 --local-only ./music` | Combined filters on the disk | `--remote-only` narrows to undownloaded rows |
| 5 | `syncweb find 'report.*' --modified-within 7d ./shared-docs` | Time filter | `--modified-within` applies to local rows |
| 6 | `syncweb find --depth +2 --depth -5 'config' ./shared-docs` | Depth constraints | |
| 7 | `syncweb find '.*\.txt$' --json ./shared-docs` | Envelope `{folder, path, entries:[...]}` | |
| 8 | `syncweb find --ignore-case 'README' ./shared-docs` | Case-insensitive | |

### 4.3 `sort` Command

On a resolved folder `sort --by` takes the small metadata vocabulary
`name\|size\|modified\|state` and sorts the entry table. The original
`niche\|frecency\|peers\|...` algorithms (plus `--limit-size`, `--min-seeders`,
`--enrich`) still run on the disk and require `--local-only`.

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb sort --by size ./shared-docs` | Metadata table sorted by size | |
| 2 | `syncweb sort --by name ./shared-docs` | Sorted by path | |
| 3 | `syncweb sort --by modified ./shared-docs` | Local rows by mtime; remote rows fall back to doc size | |
| 4 | `syncweb sort --by state ./shared-docs` | Local rows before remote | |
| 5 | `syncweb sort --local-only --by peers ./shared-docs` | Most-seeded first (disk scan + peer tracker) | |
| 6 | `syncweb sort --local-only --by niche ./shared-docs` | Files with ~N seeders ranked highest | |
| 7 | `syncweb sort --local-only --limit-size 10GB --min-seeders 2 ./shared-docs` | With limits | |
| 8 | `syncweb sort --no-enrich ./shared-docs` | Skip the per-file stat in the metadata table | |

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
| 3 | `syncweb download --max-count 10 ./shared-docs/` | Downloads at most 10 entries | |
| 4 | `syncweb download --size -1GB ./shared-docs/` | Only entries ≤ 1GB; blobs over 1GB are excluded | |
| 5 | `syncweb download --threads 1 ./shared-docs/` | Sequential download (no parallelism) | Compare speed with default |
| 6 | Piped: `syncweb find '*.iso' ./shared-docs \| syncweb download -` | Pipe from stdin | |

### 5.2 Import

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb folders import ./shared-docs/` | Scans + imports all files | `syncweb ls ./shared-docs` shows new entries |
| 2 | `syncweb folders import --threads 1 ./shared-docs/` | Sequential import | |
| 3 | Create nested dir structure, then `syncweb folders import ./shared-docs/` | Respects directory structure | |
| 4 | `syncweb folders import /tmp/new-files ./shared-docs/` | Import from different source path | |

### 5.3 Package Export

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb package export ./shared-docs/ /tmp/export-test` | Exports all blobs to filesystem | `ls /tmp/export-test` shows files |
| 2 | `syncweb package export --version 1.0.1 ./shared-docs/ /tmp/export-test` | Exports with a pinned version | |

---

## 6. Health & Verify

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb stats seeding --folder ./shared-docs` | Shows per-blob seeding: well/under/unseeded with counts | |
| 2 | `syncweb stats seeding --json --folder ./shared-docs` | JSON output | |
| 3 | `syncweb verify ./shared-docs` | Checks all local blobs against doc entries | Reports any corrupted/missing |
| 4 | Manually corrupt a blob file in blob store, then `syncweb verify` | Reports corruption | Check blob store path: `ls ~/.local/share/syncweb/blobs/` |
| 5 | `syncweb verify --fix ./shared-docs` | Re-downloads corrupted blobs | |

---

## 7. Public Folders & Publishing

### 7.1 Share / Subscribe

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice: `syncweb share ./shared-docs` | Creates read-only share ticket/URL | Save the ticket |
| 2 | Bob: `syncweb folders join <ticket> ./bob-public` | Tracks metadata only; NO live sync and NO bulk download by default | `syncweb ls ./bob-public` shows entries; folder dir starts empty; `config show subscribe` → `enabled = false` |
| 3 | Bob: `syncweb folders join --subscribe <ticket> ./bob-public` (fresh folder) | Tracks folder + enables live syncing (persisted); NO bulk download by default | `syncweb ls ./bob-public` shows entries; `config show subscribe` → `enabled = true` |
| 3 | Bob: `syncweb download ./bob-public/` (or `syncweb folders join --download-existing <ticket> ./bob-public` on a fresh folder) | Downloads existing content explicitly | Files appear in the folder |
| 4 | Alice: `syncweb access --revoke ./shared-docs` | Stops sharing (removes pin, stops announcing); read-only unshare is prompt-free | Bob can no longer see updates |
| 5 | Alice: `syncweb access --revoke --write ./shared-docs` | Prompts "Are you sure…?" (default no); confirming revokes write access | `syncweb share --list` no longer shows `access: write` |
| 6 | Alice: `syncweb access --revoke --blob <hash> ./shared-docs` | Prompts before removing the shared blob pin | `syncweb access --revoke --blob <hash>` (non-TTY, no `--yes`) prints "aborted" |

### 7.2 Access Dashboard

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice: `syncweb access ./shared-docs` | One table: `Folder · Mode · Write? · Shared with · Networks`; the folder appears once with `Mode` (e.g. `sendreceive`) and its share rows under `Shared with` | Table includes the `Write?`/`Mode` headers |
| 2 | Alice: `syncweb access --json` | Array of `{folder, mode, write, shares:[{access, url}], networks:[...]}`; no `pinned` key | `jq '.[] | .folder'` lists every folder |
| 3 | Alice: `syncweb access --revoke ./shared-docs --write --yes` | Revokes the write share (same as `unshare --write --yes`) | `syncweb access --json` now shows `"write": false` |
| 4 | Alice: `syncweb access --revoke ./shared-docs` (no `--yes`) | Read-only revoke is prompt-free and removes the read share + retention pins | `syncweb share --list` no longer shows `access: read` |
| 5 | Alice: `syncweb access --revoke ./shared-docs --write` (non-TTY, no `--yes`) | Prints `aborted`; write share stays | `syncweb access --json` still shows `"write": true` |

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

---

## 9. Collections & Packages

### 9.1 Collection Lifecycle

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb package add ./pkg-dir` | Initializes collection, creates manifest | `ls ./pkg-dir/` shows manifest |
| 2 | Add files to `./pkg-dir/`, then `syncweb package add ./pkg-dir` | Scans + hashes, updates manifest | |
| 3 | `syncweb package bump ./pkg-dir --changelog "v1 initial"` | Creates new manifest version | |
| 4 | `syncweb package publish ./pkg-dir --namespace <namespace-id>` | Stores manifest, pins content, announces blob ticket | Outputs ticket URL |

### 9.2 Package Install / Upgrade / Remove

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice: `syncweb package publish ./pkg-dir` | Get the ticket from output | |
| 2 | Bob: `syncweb search --kind package "pkg"` | Discovers Alice's package via gossip | |
| 3 | Bob: `syncweb package info <ticket>` | Shows metadata, versions | |
| 4 | Bob: `syncweb package install <ticket> --path ./pkg-install` | Fetches, verifies, installs atomically | Verify: `ls ./pkg-install/` has files |
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
| 1 | Alice: `syncweb folders create --network home ./nw-docs` | Creates folder in "home" network | `syncweb network ls home` shows the folder |
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
| 2 | `syncweb search --kind catalog "test"` | Full-text search results | |
| 3 | `syncweb indexing publish catalog ./shared-docs --catalog <name>` | Publishes to catalog namespace | |
| 4 | `syncweb indexing disable ./shared-docs` | Disables indexing | |

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

## 13. Schedules & Bandwidth

### 13.1 Schedule Configuration

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb config schedule` | Shows current global schedule | |
| 2 | `syncweb config schedule set --active "22:00-06:00"` | Sets active hours | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM app_config WHERE key LIKE 'schedule%';"` |
| 3 | `syncweb config schedule set --bandwidth "5MB/s" --period "08:00-18:00"` | Time-based bandwidth limit | |
| 4 | `syncweb config schedule folder media --active "01:00-05:00"` | Per-folder override | |
| 5 | `syncweb config schedule` (check) | Shows updated schedule | |

### 13.2 Bandwidth Verification

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Sync large files while monitoring `syncweb stats network` | Bandwidth is capped at configured limit | `sqlite3 ~/.local/share/syncweb/stats.db "SELECT SUM(bytes) FROM bandwidth_events WHERE direction='download';"` |
| 2 | Wait for inactive window, trigger sync | Sync does not start (or is delayed) | |
| 3 | `syncweb stats network` | Shows totals, per-folder, per-peer | |
| 4 | `syncweb stats network --period 24h` | Only transfer events from the last 24h | `sqlite3 ~/.local/share/syncweb/stats.db "SELECT COUNT(*) FROM bandwidth_events;"` |
| 5 | `syncweb stats network --since 24h --json` | JSON object with `total_upload`/`total_download`/`per_folder`/`per_peer`/`period_start` | |
| 6 | `syncweb stats network --folder <namespace>` | Per-folder breakdown | |
| 7 | `syncweb stats network --follow --once` | Prints the current snapshot and exits (cron-safe) | |
| 8 | `syncweb stats network --follow` (Ctrl+C after a moment) | Streams sync sessions / network events live; under `--json` one JSON object per line (NDJSON) | |

---

## 14. Filter Engine / Watch Mode

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Create `~/.config/syncweb/filters.toml` with rules | | See example below |
| 2 | `syncweb watch --show-filters` | Shows loaded filter rules | |
| 3 | `syncweb watch --dry-run --paths <folder>` | Shows what would be accepted/rejected | |
| 4 | `syncweb watch --once <folder>` | Imports files matching the rules, rejects the rest | |
| 5 | `syncweb watch --filters /path/to/filters.toml <folder>` | Uses a custom filter config path | |

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

## 15. Watch Mode (File Watcher)

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb watch ./shared-docs` | Watches folder for changes | Trigger a change and observe |
| 2 | Touch/create/delete/modify a file in `./shared-docs` | Watcher detects and imports change | Logs show "file changed: <path>" |
| 3 | `syncweb watch --once ./shared-docs` | Scans once, then exits | |
| 4 | Create `.syncignore` with glob patterns | Watcher excludes matching files | |

---

## 16. Conflict Resolution

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Alice and Bob both modify the same file offline | Both have local edits | |
| 2 | Both come online and sync | Conflict detected | |
| 3 | For text files: auto-resolve keeps newer (LWW), saves .diff | Conflict auto-resolved or listed | Check for `.conflict` or `.diff` files |
| 4 | Binary file conflict | Creates `.conflict.<hash>` file | Both versions preserved |

---

## 17. Offline Queue

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Go offline (disconnect network) | | |
| 2 | Make changes to synced folder | | |
| 3 | Come back online | Pending changes sync automatically | |

---

## 18. Media Server

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb start --media-only` | Starts HTTP server on 127.0.0.1:9193 | |
| 2 | `curl http://127.0.0.1:9193/media/<blob-hash>` | Serves blob content | Compare with `syncweb stat` output for hash |
| 3 | `curl -H "Range: bytes=0-100" http://127.0.0.1:9193/media/<hash>` | Partial content (206) with first 100 bytes | Check Content-Range header |
| 4 | `curl http://127.0.0.1:9193/` | 404 or list | |
| 5 | Configure custom listen address in config | Server starts on specified address | |

---

## 19. WebSocket Bridge

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Run daemon (bridge starts on 127.0.0.1:9192) | Bridge listening | |
| 2 | Connect via `websocat ws://127.0.0.1:9192/bridge` | Connection accepted | Logs show "bridge connection accepted" |
| 3 | Send a valid JSON command over WS | Receives response | See bridge protocol docs |
| 4 | Send invalid JSON | Receives error message | |

---

## 20. Syncthing Relay (BEP)

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Configure BEP: `syncweb config set bep.enabled true` | BEP enabled | `syncweb config show bep` |
| 2 | `syncweb config set bep.relay_urls '["tcp://relay.syncthing.net:22270"]'` | Sets relay | |
| 3 | Two nodes behind CGNAT (or simulate with firewall) | Connection falls back to Syncthing relay | Logs show "relay connected" |
| 4 | `syncweb network test-relay` | Tests relay connectivity | Logs latency and status |
| 5 | `syncweb devices` | Shows DeviceIds | Verify format matches Syncthing DeviceId |

---

## 21. Discovery Mechanisms

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Two nodes on same LAN | Auto-discover via mDNS within ~1s | |
| 2 | Disable mDNS: `syncweb config set discovery.local_mdns false` | No LAN discovery | |
| 3 | Re-enable mDNS | Discovery resumes | |
| 4 | Two nodes on different networks | Discover via DHT (~5-10s) or gossip | |

---

## 22. CLI Global Flags & Output

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb --verbose <command>` | Debug-level output | |
| 2 | `syncweb --json folders` | JSON output | `syncweb --json folders \| jq .` |
| 3 | `syncweb --no-color devices` | Output without ANSI color | |
| 4 | `syncweb --data-dir /tmp/syncweb-test folders` | Uses custom data dir | Check files in /tmp/syncweb-test/ |
| 5 | `syncweb --network home folders` | Shows folders in "home" network context | |
| 6 | `syncweb --help` | Grouped help shows the full major-release surface: 25 functional verbs (`start`, `stop`, `status`, `reload`, `sync`, `folders`, `ls`, `stat`, `find`, `search`, `sort`, `download`, `verify`, `transfer`, `share`, `access`, `link`, `package`, `network`, `watch`, `snapshot`, `indexing`, `stats`, `db`, `config`) plus `version`/`devices`/`completions`/`manpages`/`help`. Re-homed verbs (`create`, `join`, `leave`, `import`, `networks`, `publish`, `unshare`, `provider`) no longer parse | `syncweb folders create --help` still works |
| 7 | `syncweb <command> --help` | Command-specific help, including subcommands (`folders create`, `share list`, `network status`, `indexing publish`) | |

---

## 23. Version & Completions

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | `syncweb version` | Shows version, commit, build date | |
| 2 | `syncweb completions bash` | Outputs bash completion script | Source it: `source <(syncweb completions bash)` |
| 3 | `syncweb completions zsh` | Zsh completions | |
| 4 | `syncweb completions fish` | Fish completions | |
| 5 | `syncweb manpages /tmp/syncweb.1` | Generates manpage | `man /tmp/syncweb.1` |

---

## 24. Integrity & Error Recovery

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Kill daemon with SIGKILL (kill -9) | Hard crash | |
| 2 | Start daemon again | Recovers gracefully, WAL replay fixes DB | Check: `syncweb status` works |
| 3 | Delete a random blob file from blob store | Data missing | |
| 4 | `syncweb verify ./shared-docs` | Reports missing/corrupt blob | |
| 5 | `syncweb verify --fix ./shared-docs` | Re-downloads from peers | |
| 6 | `syncweb db check` | DB integrity passes | |

---

## 25. Performance Smoke Tests

| Step | Action | Expected Result | Debug |
|------|--------|-----------------|-------|
| 1 | Time `syncweb start` (cold start) | < 500ms | `time syncweb start --foreground` |
| 2 | Create 10000 files, time `syncweb folders import` | < 3s (default parallel) | Compare with `--threads 1` |
| 3 | Time `syncweb ls` on 10000 entries | < 500ms | |
| 4 | Sync a 10GB folder over LAN | > 500 MB/s throughput | Monitor: `syncweb stats network` |
| 5 | `syncweb stats seeding --folder .` on folder with 1000+ entries | < 1s | |

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
syncweb --data-dir /tmp/alice-data create /tmp/alice-files/shared
# Copy the URL

# Terminal 2: Bob
mkdir -p /tmp/bob-data /tmp/bob-files
syncweb --data-dir /tmp/bob-data join <URL> /tmp/bob-files/shared

# Start both daemons
syncweb --data-dir /tmp/alice-data start --foreground   # Terminal 1
syncweb --data-dir /tmp/bob-data start --foreground     # Terminal 2
```
