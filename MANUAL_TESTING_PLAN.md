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

## Manual Test Run: 2026-10-04

- Version: `syncweb-v0.1.1-alpha.7` (tag `7b682f53a591d88f8f1f7ac378b7afe097be2de4`), built from source.
- Nodes: `iiab-vm` containers `syncweb-a` (`10.0.3.2`) and `syncweb-b` (`10.0.3.3`), clean Ubuntu cloud images created with `--skip-install`; IIAB was not installed.
- Result: testing stopped after more than five unexpected errors. Alpha.7 uses `syncweb start --bg` for background mode; the documented `syncweb start` default-background behavior and `start --foreground` flag are not available. Node A also exited during the reload/sync/folder sequence after reporting `failed to read blob: encode error`.
- Every table below has a `Pass / Fail / Output` column first. Rows not reached after the stop condition are marked `NOT RUN`.

---

## 1. Initialization & Configuration

### 1.1 First Run / Create

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| PASS - rc=0; created /root/manual/test-folder and printed `syncweb://folder/a42ffb60fa295d5a1b08ad82a4ca87d2f4180388a590f7e112e62ce60ef89985?ticket=docaaa2il73md5csxk2dmek3avezkd5f5ayaoeklehx4ejomlhgb34jtbibq2v72w736xcym45jhdkbmpotcpqpih4plxptr7b3mnrb4mofuujacaiabiaagaxbw4bq` |  1 | Run `syncweb folders create ./test-folder` | Creates `./test-folder/`, prints path, namespace, ticket, and `syncweb://` share URL | Check dir exists: `ls -la test-folder/` |
| PASS - rc=0; created send-only folder and printed `syncweb://folder/89bde7999775e49ec094e91b3bcccff798875fcb69b8b2f62b6b0d9a3e545ee0?ticket=docaaaytpphtglxlze6yckosgz3zth7pgehl7fwtofs6yvwwdm2hzkf5yabq2v72w736xcym45jhdkbmpotcpqpih4plxptr7b3mnrb4mofuujacaiabiaagaxbw4bq` |  2 | Run `syncweb folders create --mode sendonly ./test-sendonly` | Creates folder with SendOnly mode | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM folder_configs;"` |
| PASS - rc=0; created network-linked folder and printed `syncweb://folder/6f873f6258a923198b4b432304ec170f68ba1beccadd126aca2ba94a61a279e4?ticket=docaaaw7bz7mjmksiyzrnfugiye5qlq62f2dpwmvxisnlfcxkkkmgrhtzabq2v72w736xcym45jhdkbmpotcpqpih4plxptr7b3mnrb4mofuujacaiabiaagaxbw4bq` |  3 | Run `syncweb folders create --network home ./test-network` | Creates folder linked to network "home" | Verify with `syncweb config show networks` |
| PASS - rc=0; created receive-encrypted folder and printed `syncweb://folder/7613fe83aa289897d4520f6635b0e462df80a17c399e5785efd57b6519fa1458?ticket=docaaaxme76qovcrgex2rja6zrvwdsgfx4auf6dthsxqxx5k63fdh5biwabq2v72w736xcym45jhdkbmpotcpqpih4plxptr7b3mnrb4mofuujacaiabiaagaxbw4bq` |  4 | Run `syncweb folders create --mode receiveencrypted ./test-encrypted` | Creates ReceiveEncrypted folder | Check folder list: `syncweb folders` |

### 1.2 Config Management

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| PASS - rc=0; printed full TOML including `[bep]`, `[schedule]`, `[bandwidth]`, `[discovery]`, and `[channels]` |  1 | `syncweb config` | Shows full config TOML | |
| PASS - rc=0; `active_hours = ""`, `bandwidth = []`, and `[folders]` |  2 | `syncweb config show schedule` | Shows schedule section only | |
| PASS - rc=0; `enabled = false`, empty `relay_urls`, timeout `10`, `auto_fallback = true` |  3 | `syncweb config show bep` | Shows Syncthing relay section | |
| PASS - rc=0; `default_sync_mode updated` |  4 | `syncweb config set default_sync_mode ReceiveOnly` | Updates config | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM app_config WHERE key='default_sync_mode';"` |
| PASS - rc=0; `bandwidth.max_download updated` |  5 | `syncweb config set bandwidth.max_download 5MB/s` | Updates bandwidth config | |
| PASS - rc=0; showed `default_sync_mode = "ReceiveOnly"` and `max_download = "5MB/s"` |  6 | `syncweb config show` (after changes) | Shows updated values | |

### 1.3 Database Maintenance

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| PASS - rc=0; `node.db: 0 errors`, `stats.db: 0 errors`, all databases healthy |  1 | `syncweb db check` | Returns "integrity check passed" | Manual: `sqlite3 ~/.local/share/syncweb/node.db "PRAGMA integrity_check;"` |
| PASS - rc=0; table reported `node.db 196 KB` and `stats.db 104 KB`, both with `0` freelist pages |  2 | `syncweb db stats` | Shows table row counts, sizes | Manual: `sqlite3 ~/.local/share/syncweb/node.db "SELECT COUNT(*) FROM daemon_lifecycle;"` |
| PASS - rc=0; `VACUUM complete`, with zero freelist pages for both databases |  3 | `syncweb db vacuum` | Reclaims space | `sqlite3 ~/.local/share/syncweb/node.db "PRAGMA freelist_count;"` before/after |
| PASS - rc=0; backup created at `/tmp/syncweb-backup/syncweb-db-backup-1791139125` with node.db, stats.db, config.toml, identity.key, and manifest.json |  4 | `syncweb db backup --output /tmp/syncweb-backup` | Creates backup zip | Check file exists: `ls -la /tmp/syncweb-backup*` |

---

## 2. Daemon Lifecycle

### 2.1 Start & Stop

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| FAIL - rc=2; `error: unexpected argument '--foreground' found` |  1 | `syncweb start --foreground` | Daemon starts in foreground, shows "daemon running" | `RUST_LOG=debug syncweb start --foreground` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Ctrl+C on daemon | Graceful shutdown, logs "daemon stopped" | Check lifecycle: `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM daemon_lifecycle;"` |
| FAIL - node A default `start` returned rc=1 with `daemon started ...` followed by `Error: failed to read blob: encode error`; node B default `start` did not return within 120 seconds |  3 | `syncweb start` (background) | Daemon forks to background, returns to prompt | |
| PASS - corrected node-B `start --bg` then `status` reported `daemon: running`; corrected node-A `start --bg` initially also reported running |  4 | `syncweb status` | Shows PID, uptime, bandwidth rates, folder statuses | Manual DB check: `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM daemon_status;"` |
| PASS - node-B `--json status` returned one JSON object with `daemon`, `devices`, `folders`, `networks`, and `transfers` keys | 4b | `syncweb --json status` (daemon running) | Single JSON object `{daemon, folders, devices, networks}` with all four keys present | |
| FAIL - `printf "n" | stop` printed `aborted`, but the required running-daemon precondition was not met; status was `daemon not running`  5 | `syncweb stop` | Prompts "Are you sure…?" (default no); confirm stops daemon, status shows "not running" | `syncweb status` returns error or "no daemon" |
| FAIL - `Error: daemon is not running; start with `syncweb start`  6 | Start daemon, then `syncweb stop --force` | Force kills daemon after the same confirmation | Check PID gone: `ps aux | grep syncweb` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  7 | `syncweb stop --yes` in a script/pipe | Skips the prompt and stops the daemon; without `--yes` non-interactive runs abort ("aborted") and the daemon stays up — `--json` does not skip the prompt | `syncweb status` after aborted run still shows "daemon: running" |

### 2.2 Reload & Sync Commands

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| FAIL - `Error: daemon is not running; start with `syncweb start` |  1 | Daemon running, change config.toml, then `syncweb reload` | Daemon reloads config, no restart needed | Check logs for "config reloaded" |
| FAIL - `Error: daemon is not running; start with `syncweb start` |  2 | `syncweb sync` | Triggers sync for all folders | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM sync_checkpoints ORDER BY last_updated_at DESC LIMIT 5;"` |
| FAIL - `Error: daemon is not running; start with `syncweb start` |  3 | `syncweb sync <ns>` | Triggers sync for a specific folder; warns and does nothing if it is not live | |
| FAIL - `Error: daemon exited before becoming ready (status: exit status: 1)` |  4 | `syncweb folders create ./new-folder` | Creates folder and adds it to the running daemon | `syncweb folders` shows new folder |
| FAIL - `Error: daemon exited before becoming ready (status: exit status: 1)` |  5 | `syncweb folders leave <namespace-id>` | Removes folder from daemon | `syncweb folders` no longer shows it |

---

## 3. Folder Sync (Core P2P)

### 3.1 Create & Join (Two Devices)

Setup: Node A (alice) and Node B (bob), each with `syncweb` installed.

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Alice: `syncweb folders create --mode sendreceive ./shared-docs` | Creates folder, prints URL | Save the URL |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Alice: `echo "hello world" > shared-docs/test.txt` | File created | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Alice: `syncweb folders import ./shared-docs` | Imports file into blob store | `syncweb ls ./shared-docs` shows test.txt |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Bob: `syncweb folders join <alice-url> ./bob-shared` | Joins folder, starts syncing | Wait for discovery (~5-30s) |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | Bob: `syncweb ls ./bob-shared` | Shows test.txt (lazy, no blob yet) | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  6 | Bob: `syncweb download ./bob-shared/test.txt` | Downloads the blob | Check: `cat bob-shared/test.txt` shows "hello world" |
| NOT RUN - stop condition reached (>5 unexpected errors) |  7 | Bob: `echo "bob edit" >> bob-shared/test.txt && syncweb folders import ./bob-shared` | Bob imports change | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  8 | Alice: wait, then `syncweb download ./shared-docs/test.txt` | Gets Bob's edit | `cat shared-docs/test.txt` shows both lines |

### 3.2 Sync Modes

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Alice: `syncweb folders create --mode sendonly ./sendonly` | SendOnly folder | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Alice: create file, `syncweb folders import` | File available remotely | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Bob: `join` the folder | Can read but writes are rejected | Bob tries: `echo "x" > sendonly/x.txt && syncweb folders import` → error |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Alice: `syncweb folders create --mode receiveonly ./recvonly` | ReceiveOnly folder | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | Bob: `join` the folder | Can write but Alice ignores Bob's writes | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  6 | Alice: `syncweb folders create --mode receiveencrypted ./enc` | ReceiveEncrypted folder | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  7 | Bob: `join` the folder | Can write, but blobs are encrypted at rest | |

### 3.3 Leave

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Alice: `syncweb folders leave ./shared-docs` | Leaves the folder | `syncweb folders` no longer shows it |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Alice: `syncweb folders leave --delete-files ./shared-docs` | Prompts "Are you sure…?" (default no); confirming deletes local files | Folder directory is removed |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Alice: `syncweb folders leave --delete-files --yes ./shared-docs` in a script | Skips the prompt and deletes local files; without `--yes` non-interactive runs abort and files stay | `syncweb folders leave --delete-files` (non-TTY, no `--yes`) prints "aborted" |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Alice: `syncweb folders join <url> ./shared-docs` again | Can rejoin | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | Alice: `syncweb devices` | Shows this device's Iroh and Syncthing identities | |

### 3.4 Folders & Devices Listing

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb folders` | Table with Name, Mode, Local count, Remote count, State | Empty state shows "no folders" |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb folders --json` | JSON output | Valid JSON: `syncweb folders --json \| jq .` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb devices` | Shows this device's Iroh and Syncthing identities | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb devices --json` | JSON output; when a daemon is running, the output also carries a `peers` array (who joined) | |

---

## 4. Listing, Searching, Sorting, Stat

### 4.1 `ls` Command

`ls` now reads the doc metadata index (`list_entries()`), never a disk scan;
the disk is touched only to overlay the real size/mtime of files that are
already local. On a path that resolves to no Syncweb folder it errors with
"not inside of a Syncweb folder" — use `--local-only` to list plain directories.

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb ls ./shared-docs` | Lists all entries (lazy, no blob download), `State = local\|remote` | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb ls --local-only ./shared-docs` | Forces today's disk scan (works on any path, even outside a folder) | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb ls --remote-only ./shared-docs` | Only undownloaded rows (`State == remote`) | Contradicts `--local-only` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb ls --path-prefix docs ./shared-docs` | Only entries under `docs/` | `--path-glob '*.md'` also available |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb ls --no-enrich ./shared-docs` | Pure metadata listing (no per-file stat; Size = doc size, Modified = `-`) | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  6 | `syncweb ls --sort size ./shared-docs` | Sorts the metadata table by `name\|size\|modified\|state` | On a plain dir these values must run under `--local-only` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  7 | `syncweb ls --json ./shared-docs` | Envelope `{folder, path, entries:[{path,size,hash,local,modified?}]}` | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  8 | `syncweb ls /plain/dir` | Errors "not inside of a Syncweb folder" | Confirm `--local-only` suggestion in the message |

### 4.2 `find` Command

`find` searches the same metadata index (Python parity); a selector that is not
inside a Syncweb folder errors unless `--local-only` forces a disk scan.
Pattern/size/depth/time/type predicates apply to the metadata rows.

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb find '.*\.txt$' ./shared-docs` | Regex find — shows all .txt entries | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb find '*.md' ./shared-docs` | Glob find | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb find --fixed-strings 'report' ./shared-docs` | Substring/exact find | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb find --type f --ext mp3 --local-only ./music` | Combined filters on the disk | `--remote-only` narrows to undownloaded rows |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb find 'report.*' --modified-within 7d ./shared-docs` | Time filter | `--modified-within` applies to local rows |
| NOT RUN - stop condition reached (>5 unexpected errors) |  6 | `syncweb find --depth +2 --depth -5 'config' ./shared-docs` | Depth constraints | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  7 | `syncweb find '.*\.txt$' --json ./shared-docs` | Envelope `{folder, path, entries:[...]}` | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  8 | `syncweb find --ignore-case 'README' ./shared-docs` | Case-insensitive | |

### 4.3 `sort` Command

On a resolved folder `sort --by` takes the small metadata vocabulary
`name\|size\|modified\|state` and sorts the entry table. The original
`niche\|frecency\|peers\|...` algorithms (plus `--limit-size`, `--min-seeders`,
`--enrich`) still run on the disk and require `--local-only`.

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb sort --by size ./shared-docs` | Metadata table sorted by size | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb sort --by name ./shared-docs` | Sorted by path | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb sort --by modified ./shared-docs` | Local rows by mtime; remote rows fall back to doc size | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb sort --by state ./shared-docs` | Local rows before remote | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb sort --local-only --by peers ./shared-docs` | Most-seeded first (disk scan + peer tracker) | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  6 | `syncweb sort --local-only --by niche ./shared-docs` | Files with ~N seeders ranked highest | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  7 | `syncweb sort --local-only --limit-size 10GB --min-seeders 2 ./shared-docs` | With limits | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  8 | `syncweb sort --no-enrich ./shared-docs` | Skip the per-file stat in the metadata table | |

### 4.4 `stat` Command

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb stat ./shared-docs/test.txt` | Shows size, blocks, type, permissions, timestamps, version, availability, modified_by | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb stat --terse ./shared-docs/test.txt` | Pipe-separated output | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb stat --format '%n %s %y' ./shared-docs/test.txt` | Custom template | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Modify file locally but don't sync yet, run `syncweb stat` | Shows local vs global diffs | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb stat ./shared-docs/*.md` | Multiple files | |

---

## 5. Download & Import/Export

### 5.1 Download

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb download ./shared-docs/test.txt` | Downloads single file | Check file exists: `cat ./shared-docs/test.txt` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb download ./shared-docs/` | Downloads entire folder | `ls -la ./shared-docs/` shows all files |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb download --max-count 10 ./shared-docs/` | Downloads at most 10 entries | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb download --size -1GB ./shared-docs/` | Only entries ≤ 1GB; blobs over 1GB are excluded | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb download --threads 1 ./shared-docs/` | Sequential download (no parallelism) | Compare speed with default |
| NOT RUN - stop condition reached (>5 unexpected errors) |  6 | Piped: `syncweb find '*.iso' ./shared-docs \| syncweb download -` | Pipe from stdin | |

### 5.2 Import

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb folders import ./shared-docs/` | Scans + imports all files | `syncweb ls ./shared-docs` shows new entries |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb folders import --threads 1 ./shared-docs/` | Sequential import | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Create nested dir structure, then `syncweb folders import ./shared-docs/` | Respects directory structure | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb folders import /tmp/new-files ./shared-docs/` | Import from different source path | |

### 5.3 Package Export

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb package export ./shared-docs/ /tmp/export-test` | Exports all blobs to filesystem | `ls /tmp/export-test` shows files |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb package export --version 1.0.1 ./shared-docs/ /tmp/export-test` | Exports with a pinned version | |

---

## 6. Health & Verify

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb stats seeding --folder ./shared-docs` | Shows per-blob seeding: well/under/unseeded with counts | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb stats seeding --json --folder ./shared-docs` | JSON output | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb verify ./shared-docs` | Checks all local blobs against doc entries | Reports any corrupted/missing |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Manually corrupt a blob file in blob store, then `syncweb verify` | Reports corruption | Check blob store path: `ls ~/.local/share/syncweb/blobs/` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb verify --fix ./shared-docs` | Re-downloads corrupted blobs | |

---

## 7. Public Folders & Publishing

### 7.1 Share / Subscribe

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Alice: `syncweb share ./shared-docs` | Creates read-only share ticket/URL | Save the ticket |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Bob: `syncweb folders join <ticket> ./bob-public` | Tracks metadata only; NO live sync and NO bulk download by default | `syncweb ls ./bob-public` shows entries; folder dir starts empty; `config show subscribe` → `enabled = false` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Bob: `syncweb folders join --subscribe <ticket> ./bob-public` (fresh folder) | Tracks folder + enables live syncing (persisted); NO bulk download by default | `syncweb ls ./bob-public` shows entries; `config show subscribe` → `enabled = true` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Bob: `syncweb download ./bob-public/` (or `syncweb folders join --download-existing <ticket> ./bob-public` on a fresh folder) | Downloads existing content explicitly | Files appear in the folder |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Alice: `syncweb access --revoke ./shared-docs` | Stops sharing (removes pin, stops announcing); read-only unshare is prompt-free | Bob can no longer see updates |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | Alice: `syncweb access --revoke --write ./shared-docs` | Prompts "Are you sure…?" (default no); confirming revokes write access | `syncweb share --list` no longer shows `access: write` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  6 | Alice: `syncweb access --revoke --blob <hash> ./shared-docs` | Prompts before removing the shared blob pin | `syncweb access --revoke --blob <hash>` (non-TTY, no `--yes`) prints "aborted" |

### 7.2 Access Dashboard

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Alice: `syncweb access ./shared-docs` | One table: `Folder · Mode · Write? · Shared with · Devices · Networks`; the folder appears once with `Mode` (e.g. `sendreceive`) and its share rows under `Shared with`; inbound peers under `Devices` when a daemon answers | Table includes the `Write?`/`Mode` headers |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Alice: `syncweb access --json` | Array of `{folder, mode, write, shares:[{access, url}], devices:[...], networks:[...]}`; no `pinned` key | `jq '.[] | .folder'` lists every folder |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Alice: `syncweb access --revoke ./shared-docs --write --yes` | Revokes the write share (same as `unshare --write --yes`) | `syncweb access --json` now shows `"write": false` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Alice: `syncweb access --revoke ./shared-docs` (no `--yes`) | Read-only revoke is prompt-free and removes the read share + retention pins | `syncweb share --list` no longer shows `access: read` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | Alice: `syncweb access --revoke ./shared-docs --write` (non-TTY, no `--yes`) | Prints `aborted`; write share stays | `syncweb access --json` still shows `"write": true` |

---

## 8. Snapshots

### 8.1 Create & List

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb snapshot create ./shared-docs --description "before big edit"` | Creates snapshot, returns ID | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM snapshot_metadata;"` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb snapshot list ./shared-docs` | Lists all snapshots for folder | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Create multiple snapshots, list them | All shown with descriptions, timestamps | |

### 8.2 Diff & Restore

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Make changes, create another snapshot | Two snapshots exist | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb snapshot diff ./shared-docs <id1> <id2>` | Shows added/removed/changed files | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb snapshot restore ./shared-docs <id1>` | Restores to snapshot state | Verify: `syncweb ls ./shared-docs` matches original |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb snapshot delete ./shared-docs <id2>` | Deletes snapshot | `syncweb snapshot list` no longer shows it |

---

## 9. Collections & Packages

### 9.1 Collection Lifecycle

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb package add ./pkg-dir` | Initializes collection, creates manifest | `ls ./pkg-dir/` shows manifest |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Add files to `./pkg-dir/`, then `syncweb package add ./pkg-dir` | Scans + hashes, updates manifest | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb package bump ./pkg-dir --changelog "v1 initial"` | Creates new manifest version | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb package publish ./pkg-dir --namespace <namespace-id>` | Stores manifest, pins content, announces blob ticket | Outputs ticket URL |

### 9.2 Package Install / Upgrade / Remove

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Alice: `syncweb package publish ./pkg-dir` | Get the ticket from output | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Bob: `syncweb search --kind package "pkg"` | Discovers Alice's package via gossip | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Bob: `syncweb package info <ticket>` | Shows metadata, versions | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Bob: `syncweb package install <ticket> --path ./pkg-install` | Fetches, verifies, installs atomically | Verify: `ls ./pkg-install/` has files |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | Bob: `syncweb package list` | Shows installed packages | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  6 | Bob: `syncweb package versions <collection-id>` | Lists installed versions | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  7 | Bob: `syncweb package verify <collection-id>` | Integrity check passes | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  8 | Bob: `syncweb package upgrade <collection-id>` | Upgrades to latest | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  9 | Bob: `syncweb package switch <collection-id> v1` | Switches to v1 via symlink | Check version: `cat ./pkg-install/.version` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  10 | Bob: `syncweb package remove <collection-id>` | Cleanly removes | `syncweb package list` no longer shows it |

### 9.3 Package Archive (.car.zst)

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb package export <collection-id> /tmp/pkg.car.zst` | Creates compressed archive | `ls -la /tmp/pkg.car.zst` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | On another machine (air-gapped): `syncweb package import /tmp/pkg.car.zst ./pkg-import` | Imports and installs | `syncweb package list` shows it |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb package import --no-install /tmp/pkg.car.zst /tmp/extract` | Extracts without installing | |

---

## 10. Networks

### 10.1 Create & Invite

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Alice: `syncweb network create home` | Creates network "home" | `syncweb network ls` shows it |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Alice: `syncweb network ls home` | Shows network details | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM networks;"` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Alice: `syncweb network invite home <bob-device-id>` | Creates invitation ticket | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Bob: `syncweb network join <ticket>` | Joins network "home" | `syncweb network ls` shows it |

### 10.2 Folder in Network Context

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Alice: `syncweb folders create --network home ./nw-docs` | Creates folder in "home" network | `syncweb network ls home` shows the folder |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Alice imports files, Bob joins the folder | Bob gets auto-discovery via network gossip | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Alice: `syncweb network kick home <bob-device-id>` | Removes Bob from network | Bob disconnects |

### 10.3 Network Events & Health

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb network events home` | Shows peer joins, leaves, sync events | `sqlite3 ~/.local/share/syncweb/stats.db "SELECT * FROM network_events WHERE network_id='home' ORDER BY timestamp DESC LIMIT 10;"` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb network health home` | Network connectivity health | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb network test-relay` | Tests relay connectivity | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb network peers ./nw-docs` (daemon running) | Shows the folder's inbound peers (Devices) and a per-blob `% seeded` table | With `--json`, emits a single `{folder, peers, per_blob}` object |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb network peers ./nw-docs --json` (no daemon) | Honest empty state: `{folder, peers: [], per_blob: []}` — no guessed values | |

---

## 11. Indexing Service

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb indexing enable ./shared-docs` | Enables FTS5 index for folder | Check indexing db exists: `ls ~/.local/share/syncweb/indexing.sqlite` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb search --kind catalog "test"` | Full-text search results | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb indexing publish catalog ./shared-docs --catalog <name>` | Publishes to catalog namespace | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb indexing disable ./shared-docs` | Disables indexing | |

---

## 12. Links

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb link create --immutable ./shared-docs/test.txt` | Creates immutable link | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb link create --mutable ./shared-docs/test.txt` | Creates mutable pointer | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb link create --private ./shared-docs/test.txt --expires 7d` | Creates expiring private link | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb link resolve <link-id>` | Resolves to manifest + providers | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb link revoke <link-id>` | Revokes private link | Resolution fails after revocation |

---

## 13. Schedules & Bandwidth

### 13.1 Schedule Configuration

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb config schedule` | Shows current global schedule | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb config schedule set --active "22:00-06:00"` | Sets active hours | `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM app_config WHERE key LIKE 'schedule%';"` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb config schedule set --bandwidth "5MB/s" --period "08:00-18:00"` | Time-based bandwidth limit | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb config schedule folder media --active "01:00-05:00"` | Per-folder override | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb config schedule` (check) | Shows updated schedule | |

### 13.2 Bandwidth Verification

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Sync large files while monitoring `syncweb stats network` | Bandwidth is capped at configured limit | `sqlite3 ~/.local/share/syncweb/stats.db "SELECT SUM(bytes) FROM bandwidth_events WHERE direction='download';"` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Wait for inactive window, trigger sync | Sync does not start (or is delayed) | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb stats network` | Shows totals, per-folder, per-peer | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb stats network --period 24h` | Only transfer events from the last 24h | `sqlite3 ~/.local/share/syncweb/stats.db "SELECT COUNT(*) FROM bandwidth_events;"` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb stats network --since 24h --json` | JSON object with `total_upload`/`total_download`/`per_folder`/`per_peer`/`period_start` | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  6 | `syncweb stats network --folder <namespace>` | Per-folder breakdown | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  7 | `syncweb stats network --follow --once` | Prints the current snapshot and exits (cron-safe) | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  8 | `syncweb stats network --follow` (Ctrl+C after a moment) | Streams sync sessions / network events live; under `--json` one JSON object per line (NDJSON) | |

---

## 14. Filter Engine / Watch Mode

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Create `~/.config/syncweb/filters.toml` with rules | | See example below |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb watch --show-filters` | Shows loaded filter rules | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb watch --dry-run --paths <folder>` | Shows what would be accepted/rejected | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb watch --once <folder>` | Imports files matching the rules, rejects the rest | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb watch --filters /path/to/filters.toml <folder>` | Uses a custom filter config path | |

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

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb watch ./shared-docs` | Watches folder for changes | Trigger a change and observe |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Touch/create/delete/modify a file in `./shared-docs` | Watcher detects and imports change | Logs show "file changed: <path>" |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb watch --once ./shared-docs` | Scans once, then exits | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Create `.syncignore` with glob patterns | Watcher excludes matching files | |

---

## 16. Conflict Resolution

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Alice and Bob both modify the same file offline | Both have local edits | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Both come online and sync | Conflict detected | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | For text files: auto-resolve keeps newer (LWW), saves .diff | Conflict auto-resolved or listed | Check for `.conflict` or `.diff` files |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Binary file conflict | Creates `.conflict.<hash>` file | Both versions preserved |

---

## 17. Offline Queue

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Go offline (disconnect network) | | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Make changes to synced folder | | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Come back online | Pending changes sync automatically | |

---

## 18. Media Server

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb start --media-only` | Starts HTTP server on 127.0.0.1:9193 | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `curl http://127.0.0.1:9193/media/<blob-hash>` | Serves blob content | Compare with `syncweb stat` output for hash |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `curl -H "Range: bytes=0-100" http://127.0.0.1:9193/media/<hash>` | Partial content (206) with first 100 bytes | Check Content-Range header |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `curl http://127.0.0.1:9193/` | 404 or list | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | Configure custom listen address in config | Server starts on specified address | |

---

## 19. WebSocket Bridge

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Run daemon (bridge starts on 127.0.0.1:9192) | Bridge listening | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Connect via `websocat ws://127.0.0.1:9192/bridge` | Connection accepted | Logs show "bridge connection accepted" |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Send a valid JSON command over WS | Receives response | See bridge protocol docs |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Send invalid JSON | Receives error message | |

---

## 20. Syncthing Relay (BEP)

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Configure BEP: `syncweb config set bep.enabled true` | BEP enabled | `syncweb config show bep` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb config set bep.relay_urls '["tcp://relay.syncthing.net:22270"]'` | Sets relay | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Two nodes behind CGNAT (or simulate with firewall) | Connection falls back to Syncthing relay | Logs show "relay connected" |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb network test-relay` | Tests relay connectivity | Logs latency and status |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb devices` | Shows DeviceIds | Verify format matches Syncthing DeviceId |

---

## 21. Discovery Mechanisms

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Two nodes on same LAN | Auto-discover via mDNS within ~1s | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Disable mDNS: `syncweb config set discovery.local_mdns false` | No LAN discovery | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Re-enable mDNS | Discovery resumes | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Two nodes on different networks | Discover via DHT (~5-10s) or gossip | |

---

## 22. CLI Global Flags & Output

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb --verbose <command>` | Debug-level output | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb --json folders` | JSON output | `syncweb --json folders \| jq .` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb --no-color devices` | Output without ANSI color | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb --data-dir /tmp/syncweb-test folders` | Uses custom data dir | Check files in /tmp/syncweb-test/ |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb --network home folders` | Shows folders in "home" network context | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  6 | `syncweb --help` | Grouped help shows the full major-release surface: 25 functional verbs (`start`, `stop`, `status`, `reload`, `sync`, `folders`, `ls`, `stat`, `find`, `search`, `sort`, `download`, `verify`, `transfer`, `share`, `access`, `link`, `package`, `network`, `watch`, `snapshot`, `indexing`, `stats`, `db`, `config`) plus `version`/`devices`/`completions`/`manpages`/`help`. Re-homed verbs (`create`, `join`, `leave`, `import`, `networks`, `publish`, `unshare`, `provider`) no longer parse | `syncweb folders create --help` still works |
| NOT RUN - stop condition reached (>5 unexpected errors) |  7 | `syncweb <command> --help` | Command-specific help, including subcommands (`folders create`, `share list`, `network status`, `indexing publish`) | |

---

## 23. Version & Completions

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | `syncweb version` | Shows version, commit, build date | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | `syncweb completions bash` | Outputs bash completion script | Source it: `source <(syncweb completions bash)` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | `syncweb completions zsh` | Zsh completions | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb completions fish` | Fish completions | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb manpages /tmp/syncweb.1` | Generates manpage | `man /tmp/syncweb.1` |

---

## 24. Integrity & Error Recovery

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Kill daemon with SIGKILL (kill -9) | Hard crash | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Start daemon again | Recovers gracefully, WAL replay fixes DB | Check: `syncweb status` works |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Delete a random blob file from blob store | Data missing | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | `syncweb verify ./shared-docs` | Reports missing/corrupt blob | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb verify --fix ./shared-docs` | Re-downloads from peers | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  6 | `syncweb db check` | DB integrity passes | |

---

## 25. Performance Smoke Tests

| Pass / Fail / Output | Step | Action | Expected Result | Debug |
|---------------------|------|--------|-----------------|-------|
| NOT RUN - stop condition reached (>5 unexpected errors) |  1 | Time `syncweb start` (cold start) | < 500ms | `time syncweb start --foreground` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  2 | Create 10000 files, time `syncweb folders import` | < 3s (default parallel) | Compare with `--threads 1` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  3 | Time `syncweb ls` on 10000 entries | < 500ms | |
| NOT RUN - stop condition reached (>5 unexpected errors) |  4 | Sync a 10GB folder over LAN | > 500 MB/s throughput | Monitor: `syncweb stats network` |
| NOT RUN - stop condition reached (>5 unexpected errors) |  5 | `syncweb stats seeding --folder .` on folder with 1000+ entries | < 1s | |

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
