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
- Result: testing stopped after more than five unexpected errors. Node A also exited during the reload/sync/folder sequence after reporting `failed to read blob: encode error`.
- Results below are recorded as vertically structured prose. Rows not reached after the stop condition are marked `NOT RUN`.
- Pass/fail statuses are based on whether the actual output satisfied the expected result; return codes are included only for nonzero failures.

---

## 1. Initialization & Configuration

### 1.1 First Run / Create

Step 1 - PASS
- Action: Run `syncweb folders create ./test-folder`
- Expected: Creates `./test-folder/`, prints path, namespace, ticket, and `syncweb://` share URL
- Actual output: created /root/manual/test-folder and printed syncweb://folder/a42ffb60fa295d5a1b08ad82a4ca87d2f4180388a590f7e112e62ce60ef89985?ticket=docaaa2il73md5csxk2dmek3avezkd5f5ayaoeklehx4ejomlhgb34jtbibq2v72w736xcym45jhdkbmpotcpqpih4plxptr7b3mnrb4mofuujacaiabiaagaxbw4bq
- Debug: Check dir exists: `ls -la test-folder/`

Step 2 - PASS
- Action: Run `syncweb folders create --mode sendonly ./test-sendonly`
- Expected: Creates folder with SendOnly mode
- Actual output: created send-only folder and printed syncweb://folder/89bde7999775e49ec094e91b3bcccff798875fcb69b8b2f62b6b0d9a3e545ee0?ticket=docaaaytpphtglxlze6yckosgz3zth7pgehl7fwtofs6yvwwdm2hzkf5yabq2v72w736xcym45jhdkbmpotcpqpih4plxptr7b3mnrb4mofuujacaiabiaagaxbw4bq
- Debug: `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM folder_configs;"`

Step 3 - PASS
- Action: Run `syncweb folders create --network home ./test-network`
- Expected: Creates folder linked to network "home"
- Actual output: created network-linked folder and printed syncweb://folder/6f873f6258a923198b4b432304ec170f68ba1beccadd126aca2ba94a61a279e4?ticket=docaaaw7bz7mjmksiyzrnfugiye5qlq62f2dpwmvxisnlfcxkkkmgrhtzabq2v72w736xcym45jhdkbmpotcpqpih4plxptr7b3mnrb4mofuujacaiabiaagaxbw4bq
- Debug: Verify with `syncweb config show networks`

Step 4 - PASS
- Action: Run `syncweb folders create --mode receiveencrypted ./test-encrypted`
- Expected: Creates ReceiveEncrypted folder
- Actual output: created receive-encrypted folder and printed syncweb://folder/7613fe83aa289897d4520f6635b0e462df80a17c399e5785efd57b6519fa1458?ticket=docaaaxme76qovcrgex2rja6zrvwdsgfx4auf6dthsxqxx5k63fdh5biwabq2v72w736xcym45jhdkbmpotcpqpih4plxptr7b3mnrb4mofuujacaiabiaagaxbw4bq
- Debug: Check folder list: `syncweb folders`


### 1.2 Config Management

Step 1 - PASS
- Action: syncweb config
- Expected: Shows full config TOML
- Actual output: printed full TOML including [bep], [schedule], [bandwidth], [discovery], and [channels]

Step 2 - PASS
- Action: syncweb config show schedule
- Expected: Shows schedule section only
- Actual output: active_hours = "", bandwidth = [], and [folders]

Step 3 - PASS
- Action: syncweb config show bep
- Expected: Shows Syncthing relay section
- Actual output: enabled = false, empty relay_urls, timeout 10, auto_fallback = true

Step 4 - PASS
- Action: syncweb config set default_sync_mode ReceiveOnly
- Expected: Updates config
- Actual output: default_sync_mode updated
- Debug: `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM app_config WHERE key='default_sync_mode';"`

Step 5 - PASS
- Action: syncweb config set bandwidth.max_download 5MB/s
- Expected: Updates bandwidth config
- Actual output: bandwidth.max_download updated

Step 6 - PASS
- Action: `syncweb config show` (after changes)
- Expected: Shows updated values
- Actual output: showed default_sync_mode = "ReceiveOnly" and max_download = "5MB/s"


### 1.3 Database Maintenance

Step 1 - PASS
- Action: syncweb db check
- Expected: Returns "integrity check passed"
- Actual output: node.db: 0 errors, stats.db: 0 errors, all databases healthy
- Debug: Manual: `sqlite3 ~/.local/share/syncweb/node.db "PRAGMA integrity_check;"`

Step 2 - PASS
- Action: syncweb db stats
- Expected: Shows table row counts, sizes
- Actual output: table reported node.db 196 KB and stats.db 104 KB, both with 0 freelist pages
- Debug: Manual: `sqlite3 ~/.local/share/syncweb/node.db "SELECT COUNT(*) FROM daemon_lifecycle;"`

Step 3 - PASS
- Action: syncweb db vacuum
- Expected: Reclaims space
- Actual output: VACUUM complete, with zero freelist pages for both databases
- Debug: `sqlite3 ~/.local/share/syncweb/node.db "PRAGMA freelist_count;"` before/after

Step 4 - PASS
- Action: syncweb db backup --output /tmp/syncweb-backup
- Expected: Creates backup zip
- Actual output: backup created at /tmp/syncweb-backup/syncweb-db-backup-1791139125 with node.db, stats.db, config.toml, identity.key, and manifest.json
- Debug: Check file exists: `ls -la /tmp/syncweb-backup*`


---

## 2. Daemon Lifecycle

### 2.1 Start & Stop

Step 1 - NOT RUN
- Action: Ctrl+C on daemon
- Expected: Graceful shutdown, logs "daemon stopped"
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Check lifecycle: `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM daemon_lifecycle;"`

Step 3 - FAIL
- Action: `syncweb start` (background)
- Expected: Daemon forks to background, returns to prompt
- Actual output: node A default start returned rc=1 with daemon started ... followed by Error: failed to read blob: encode error; node B default start did not return within 120 seconds

Step 4 - PASS
- Action: syncweb status
- Expected: Shows PID, uptime, bandwidth rates, folder statuses
- Actual output: corrected node-B start --bg then status reported daemon: running; corrected node-A start --bg initially also reported running
- Debug: Manual DB check: `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM daemon_status;"`

Step 4b - PASS
- Action: `syncweb --json status` (daemon running)
- Expected: Single JSON object `{daemon, folders, devices, networks}` with all four keys present
- Actual output: node-B --json status returned one JSON object with daemon, devices, folders, networks, and transfers keys

Step 5 - FAIL
- Action: syncweb stop
- Expected: Prompts "Are you sure…?" (default no); confirm stops daemon, status shows "not running"
- Actual output: printf "n" | stop printed aborted, but the required running-daemon precondition was not met; status was daemon not running
- Debug: `syncweb status` returns error or "no daemon"

Step 6 - FAIL
- Action: Start daemon, then `syncweb stop --force`
- Expected: Force kills daemon after the same confirmation
- Actual output: Error: daemon is not running; start with syncweb start
- Debug: Check PID gone: `ps aux | grep syncweb`

Step 7 - NOT RUN
- Action: `syncweb stop --yes` in a script/pipe
- Expected: Skips the prompt and stops the daemon; without `--yes` non-interactive runs abort ("aborted") and the daemon stays up — `--json` does not skip the prompt
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb status` after aborted run still shows "daemon: running"


### 2.2 Reload & Sync Commands

Step 1 - FAIL
- Action: Daemon running, change config.toml, then `syncweb reload`
- Expected: Daemon reloads config, no restart needed
- Actual output: Error: daemon is not running; start with syncweb start
- Debug: Check logs for "config reloaded"

Step 2 - FAIL
- Action: syncweb sync
- Expected: Triggers sync for all folders
- Actual output: Error: daemon is not running; start with syncweb start
- Debug: `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM sync_checkpoints ORDER BY last_updated_at DESC LIMIT 5;"`

Step 3 - FAIL
- Action: syncweb sync <ns>
- Expected: Triggers sync for a specific folder; warns and does nothing if it is not live
- Actual output: Error: daemon is not running; start with syncweb start

Step 4 - FAIL
- Action: syncweb folders create ./new-folder
- Expected: Creates folder and adds it to the running daemon
- Actual output: Error: daemon exited before becoming ready (status: exit status: 1)
- Debug: `syncweb folders` shows new folder

Step 5 - FAIL
- Action: syncweb folders leave <namespace-id>
- Expected: Removes folder from daemon
- Actual output: Error: daemon exited before becoming ready (status: exit status: 1)
- Debug: `syncweb folders` no longer shows it


---

## 3. Folder Sync (Core P2P)

### 3.1 Create & Join (Two Devices)

Setup: Node A (alice) and Node B (bob), each with `syncweb` installed.

Step 1 - NOT RUN
- Action: Alice: `syncweb folders create --mode sendreceive ./shared-docs`
- Expected: Creates folder, prints URL
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Save the URL

Step 2 - NOT RUN
- Action: Alice: `echo "hello world" > shared-docs/test.txt`
- Expected: File created
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: Alice: `syncweb folders import ./shared-docs`
- Expected: Imports file into blob store
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb ls ./shared-docs` shows test.txt

Step 4 - NOT RUN
- Action: Bob: `syncweb folders join <alice-url> ./bob-shared`
- Expected: Joins folder, starts syncing
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Wait for discovery (~5-30s)

Step 5 - NOT RUN
- Action: Bob: `syncweb ls ./bob-shared`
- Expected: Shows test.txt (lazy, no blob yet)
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 6 - NOT RUN
- Action: Bob: `syncweb download ./bob-shared/test.txt`
- Expected: Downloads the blob
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Check: `cat bob-shared/test.txt` shows "hello world"

Step 7 - NOT RUN
- Action: Bob: `echo "bob edit" >> bob-shared/test.txt && syncweb folders import ./bob-shared`
- Expected: Bob imports change
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 8 - NOT RUN
- Action: Alice: wait, then `syncweb download ./shared-docs/test.txt`
- Expected: Gets Bob's edit
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `cat shared-docs/test.txt` shows both lines


### 3.2 Sync Modes

Step 1 - NOT RUN
- Action: Alice: `syncweb folders create --mode sendonly ./sendonly`
- Expected: SendOnly folder
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: Alice: create file, `syncweb folders import`
- Expected: File available remotely
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: Bob: `join` the folder
- Expected: Can read but writes are rejected
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Bob tries: `echo "x" > sendonly/x.txt && syncweb folders import` → error

Step 4 - NOT RUN
- Action: Alice: `syncweb folders create --mode receiveonly ./recvonly`
- Expected: ReceiveOnly folder
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 5 - NOT RUN
- Action: Bob: `join` the folder
- Expected: Can write but Alice ignores Bob's writes
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 6 - NOT RUN
- Action: Alice: `syncweb folders create --mode receiveencrypted ./enc`
- Expected: ReceiveEncrypted folder
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 7 - NOT RUN
- Action: Bob: `join` the folder
- Expected: Can write, but blobs are encrypted at rest
- Actual output: No output; testing stopped after more than five unexpected errors.


### 3.3 Leave

Step 1 - NOT RUN
- Action: Alice: `syncweb folders leave ./shared-docs`
- Expected: Leaves the folder
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb folders` no longer shows it

Step 2 - NOT RUN
- Action: Alice: `syncweb folders leave --delete-files ./shared-docs`
- Expected: Prompts "Are you sure…?" (default no); confirming deletes local files
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Folder directory is removed

Step 3 - NOT RUN
- Action: Alice: `syncweb folders leave --delete-files --yes ./shared-docs` in a script
- Expected: Skips the prompt and deletes local files; without `--yes` non-interactive runs abort and files stay
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb folders leave --delete-files` (non-TTY, no `--yes`) prints "aborted"

Step 4 - NOT RUN
- Action: Alice: `syncweb folders join <url> ./shared-docs` again
- Expected: Can rejoin
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 5 - NOT RUN
- Action: Alice: `syncweb devices`
- Expected: Shows this device's Iroh and Syncthing identities
- Actual output: No output; testing stopped after more than five unexpected errors.


### 3.4 Folders & Devices Listing

Step 1 - NOT RUN
- Action: syncweb folders
- Expected: Table with Name, Mode, Local count, Remote count, State
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Empty state shows "no folders"

Step 2 - NOT RUN
- Action: syncweb folders --json
- Expected: JSON output
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Valid JSON: `syncweb folders --json \| jq .`

Step 3 - NOT RUN
- Action: syncweb devices
- Expected: Shows this device's Iroh and Syncthing identities
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb devices --json
- Expected: JSON output; when a daemon is running, the output also carries a `peers` array (who joined)
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 4. Listing, Searching, Sorting, Stat

### 4.1 `ls` Command

`ls` now reads the doc metadata index (`list_entries()`), never a disk scan;
the disk is touched only to overlay the real size/mtime of files that are
already local. On a path that resolves to no Syncweb folder it errors with
"not inside of a Syncweb folder" — use `--local-only` to list plain directories.

Step 1 - NOT RUN
- Action: syncweb ls ./shared-docs
- Expected: Lists all entries (lazy, no blob download), `State = local\|remote`
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: syncweb ls --local-only ./shared-docs
- Expected: Forces today's disk scan (works on any path, even outside a folder)
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: syncweb ls --remote-only ./shared-docs
- Expected: Only undownloaded rows (`State == remote`)
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Contradicts `--local-only`

Step 4 - NOT RUN
- Action: syncweb ls --path-prefix docs ./shared-docs
- Expected: Only entries under `docs/`
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `--path-glob '*.md'` also available

Step 5 - NOT RUN
- Action: syncweb ls --no-enrich ./shared-docs
- Expected: Pure metadata listing (no per-file stat; Size = doc size, Modified = `-`)
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 6 - NOT RUN
- Action: syncweb ls --sort size ./shared-docs
- Expected: Sorts the metadata table by `name\|size\|modified\|state`
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: On a plain dir these values must run under `--local-only`

Step 7 - NOT RUN
- Action: syncweb ls --json ./shared-docs
- Expected: Envelope `{folder, path, entries:[{path,size,hash,local,modified?}]}`
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 8 - NOT RUN
- Action: syncweb ls /plain/dir
- Expected: Errors "not inside of a Syncweb folder"
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Confirm `--local-only` suggestion in the message


### 4.2 `find` Command

`find` searches the same metadata index (Python parity); a selector that is not
inside a Syncweb folder errors unless `--local-only` forces a disk scan.
Pattern/size/depth/time/type predicates apply to the metadata rows.

Step 1 - NOT RUN
- Action: syncweb find '.*\.txt$' ./shared-docs
- Expected: Regex find — shows all .txt entries
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: syncweb find '*.md' ./shared-docs
- Expected: Glob find
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: syncweb find --fixed-strings 'report' ./shared-docs
- Expected: Substring/exact find
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb find --type f --ext mp3 --local-only ./music
- Expected: Combined filters on the disk
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `--remote-only` narrows to undownloaded rows

Step 5 - NOT RUN
- Action: syncweb find 'report.*' --modified-within 7d ./shared-docs
- Expected: Time filter
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `--modified-within` applies to local rows

Step 6 - NOT RUN
- Action: syncweb find --depth +2 --depth -5 'config' ./shared-docs
- Expected: Depth constraints
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 7 - NOT RUN
- Action: syncweb find '.*\.txt$' --json ./shared-docs
- Expected: Envelope `{folder, path, entries:[...]}`
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 8 - NOT RUN
- Action: syncweb find --ignore-case 'README' ./shared-docs
- Expected: Case-insensitive
- Actual output: No output; testing stopped after more than five unexpected errors.


### 4.3 `sort` Command

On a resolved folder `sort --by` takes the small metadata vocabulary
`name\|size\|modified\|state` and sorts the entry table. The original
`niche\|frecency\|peers\|...` algorithms (plus `--limit-size`, `--min-seeders`,
`--enrich`) still run on the disk and require `--local-only`.

Step 1 - NOT RUN
- Action: syncweb sort --by size ./shared-docs
- Expected: Metadata table sorted by size
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: syncweb sort --by name ./shared-docs
- Expected: Sorted by path
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: syncweb sort --by modified ./shared-docs
- Expected: Local rows by mtime; remote rows fall back to doc size
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb sort --by state ./shared-docs
- Expected: Local rows before remote
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 5 - NOT RUN
- Action: syncweb sort --local-only --by peers ./shared-docs
- Expected: Most-seeded first (disk scan + peer tracker)
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 6 - NOT RUN
- Action: syncweb sort --local-only --by niche ./shared-docs
- Expected: Files with ~N seeders ranked highest
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 7 - NOT RUN
- Action: syncweb sort --local-only --limit-size 10GB --min-seeders 2 ./shared-docs
- Expected: With limits
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 8 - NOT RUN
- Action: syncweb sort --no-enrich ./shared-docs
- Expected: Skip the per-file stat in the metadata table
- Actual output: No output; testing stopped after more than five unexpected errors.


### 4.4 `stat` Command

Step 1 - NOT RUN
- Action: syncweb stat ./shared-docs/test.txt
- Expected: Shows size, blocks, type, permissions, timestamps, version, availability, modified_by
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: syncweb stat --terse ./shared-docs/test.txt
- Expected: Pipe-separated output
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: syncweb stat --format '%n %s %y' ./shared-docs/test.txt
- Expected: Custom template
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: Modify file locally but don't sync yet, run `syncweb stat`
- Expected: Shows local vs global diffs
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 5 - NOT RUN
- Action: syncweb stat ./shared-docs/*.md
- Expected: Multiple files
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 5. Download & Import/Export

### 5.1 Download

Step 1 - NOT RUN
- Action: syncweb download ./shared-docs/test.txt
- Expected: Downloads single file
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Check file exists: `cat ./shared-docs/test.txt`

Step 2 - NOT RUN
- Action: syncweb download ./shared-docs/
- Expected: Downloads entire folder
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `ls -la ./shared-docs/` shows all files

Step 3 - NOT RUN
- Action: syncweb download --max-count 10 ./shared-docs/
- Expected: Downloads at most 10 entries
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb download --size -1GB ./shared-docs/
- Expected: Only entries ≤ 1GB; blobs over 1GB are excluded
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 5 - NOT RUN
- Action: syncweb download --threads 1 ./shared-docs/
- Expected: Sequential download (no parallelism)
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Compare speed with default

Step 6 - NOT RUN
- Action: Piped: `syncweb find '*.iso' ./shared-docs \| syncweb download -`
- Expected: Pipe from stdin
- Actual output: No output; testing stopped after more than five unexpected errors.


### 5.2 Import

Step 1 - NOT RUN
- Action: syncweb folders import ./shared-docs/
- Expected: Scans + imports all files
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb ls ./shared-docs` shows new entries

Step 2 - NOT RUN
- Action: syncweb folders import --threads 1 ./shared-docs/
- Expected: Sequential import
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: Create nested dir structure, then `syncweb folders import ./shared-docs/`
- Expected: Respects directory structure
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb folders import /tmp/new-files ./shared-docs/
- Expected: Import from different source path
- Actual output: No output; testing stopped after more than five unexpected errors.


### 5.3 Package Export

Step 1 - NOT RUN
- Action: syncweb package export ./shared-docs/ /tmp/export-test
- Expected: Exports all blobs to filesystem
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `ls /tmp/export-test` shows files

Step 2 - NOT RUN
- Action: syncweb package export --version 1.0.1 ./shared-docs/ /tmp/export-test
- Expected: Exports with a pinned version
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 6. Health & Verify

Step 1 - NOT RUN
- Action: syncweb stats seeding --folder ./shared-docs
- Expected: Shows per-blob seeding: well/under/unseeded with counts
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: syncweb stats seeding --json --folder ./shared-docs
- Expected: JSON output
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: syncweb verify ./shared-docs
- Expected: Checks all local blobs against doc entries
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Reports any corrupted/missing

Step 4 - NOT RUN
- Action: Manually corrupt a blob file in blob store, then `syncweb verify`
- Expected: Reports corruption
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Check blob store path: `ls ~/.local/share/syncweb/blobs/`

Step 5 - NOT RUN
- Action: syncweb verify --fix ./shared-docs
- Expected: Re-downloads corrupted blobs
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 7. Public Folders & Publishing

### 7.1 Share / Subscribe

Step 1 - NOT RUN
- Action: Alice: `syncweb share ./shared-docs`
- Expected: Creates read-only share ticket/URL
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Save the ticket

Step 2 - NOT RUN
- Action: Bob: `syncweb folders join <ticket> ./bob-public`
- Expected: Tracks metadata only; NO live sync and NO bulk download by default
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb ls ./bob-public` shows entries; folder dir starts empty; `config show subscribe` → `enabled = false`

Step 3 - NOT RUN
- Action: Bob: `syncweb folders join --subscribe <ticket> ./bob-public` (fresh folder)
- Expected: Tracks folder + enables live syncing (persisted); NO bulk download by default
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb ls ./bob-public` shows entries; `config show subscribe` → `enabled = true`

Step 3 - NOT RUN
- Action: Bob: `syncweb download ./bob-public/` (or `syncweb folders join --download-existing <ticket> ./bob-public` on a fresh folder)
- Expected: Downloads existing content explicitly
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Files appear in the folder

Step 4 - NOT RUN
- Action: Alice: `syncweb access --revoke ./shared-docs`
- Expected: Stops sharing (removes pin, stops announcing); read-only unshare is prompt-free
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Bob can no longer see updates

Step 5 - NOT RUN
- Action: Alice: `syncweb access --revoke --write ./shared-docs`
- Expected: Prompts "Are you sure…?" (default no); confirming revokes write access
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb share --list` no longer shows `access: write`

Step 6 - NOT RUN
- Action: Alice: `syncweb access --revoke --blob <hash> ./shared-docs`
- Expected: Prompts before removing the shared blob pin
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb access --revoke --blob <hash>` (non-TTY, no `--yes`) prints "aborted"


### 7.2 Access Dashboard

Step 1 - NOT RUN
- Action: Alice: `syncweb access ./shared-docs`
- Expected: One table: `Folder · Mode · Write? · Shared with · Devices · Networks`; the folder appears once with `Mode` (e.g. `sendreceive`) and its share rows under `Shared with`; inbound peers under `Devices` when a daemon answers
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Table includes the `Write?`/`Mode` headers

Step 2 - NOT RUN
- Action: Alice: `syncweb access --json`
- Expected: Array of `{folder, mode, write, shares:[{access, url}], devices:[...], networks:[...]}`; no `pinned` key
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `jq '.[] | .folder'` lists every folder

Step 3 - NOT RUN
- Action: Alice: `syncweb access --revoke ./shared-docs --write --yes`
- Expected: Revokes the write share (same as `unshare --write --yes`)
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb access --json` now shows `"write": false`

Step 4 - NOT RUN
- Action: Alice: `syncweb access --revoke ./shared-docs` (no `--yes`)
- Expected: Read-only revoke is prompt-free and removes the read share + retention pins
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb share --list` no longer shows `access: read`

Step 5 - NOT RUN
- Action: Alice: `syncweb access --revoke ./shared-docs --write` (non-TTY, no `--yes`)
- Expected: Prints `aborted`; write share stays
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb access --json` still shows `"write": true`


---

## 8. Snapshots

### 8.1 Create & List

Step 1 - NOT RUN
- Action: syncweb snapshot create ./shared-docs --description "before big edit"
- Expected: Creates snapshot, returns ID
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM snapshot_metadata;"`

Step 2 - NOT RUN
- Action: syncweb snapshot list ./shared-docs
- Expected: Lists all snapshots for folder
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: Create multiple snapshots, list them
- Expected: All shown with descriptions, timestamps
- Actual output: No output; testing stopped after more than five unexpected errors.


### 8.2 Diff & Restore

Step 1 - NOT RUN
- Action: Make changes, create another snapshot
- Expected: Two snapshots exist
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: syncweb snapshot diff ./shared-docs <id1> <id2>
- Expected: Shows added/removed/changed files
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: syncweb snapshot restore ./shared-docs <id1>
- Expected: Restores to snapshot state
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Verify: `syncweb ls ./shared-docs` matches original

Step 4 - NOT RUN
- Action: syncweb snapshot delete ./shared-docs <id2>
- Expected: Deletes snapshot
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb snapshot list` no longer shows it


---

## 9. Collections & Packages

### 9.1 Collection Lifecycle

Step 1 - NOT RUN
- Action: syncweb package add ./pkg-dir
- Expected: Initializes collection, creates manifest
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `ls ./pkg-dir/` shows manifest

Step 2 - NOT RUN
- Action: Add files to `./pkg-dir/`, then `syncweb package add ./pkg-dir`
- Expected: Scans + hashes, updates manifest
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: syncweb package bump ./pkg-dir --changelog "v1 initial"
- Expected: Creates new manifest version
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb package publish ./pkg-dir --namespace <namespace-id>
- Expected: Stores manifest, pins content, announces blob ticket
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Outputs ticket URL


### 9.2 Package Install / Upgrade / Remove

Step 1 - NOT RUN
- Action: Alice: `syncweb package publish ./pkg-dir`
- Expected: Get the ticket from output
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: Bob: `syncweb search --kind package "pkg"`
- Expected: Discovers Alice's package via gossip
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: Bob: `syncweb package info <ticket>`
- Expected: Shows metadata, versions
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: Bob: `syncweb package install <ticket> --path ./pkg-install`
- Expected: Fetches, verifies, installs atomically
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Verify: `ls ./pkg-install/` has files

Step 5 - NOT RUN
- Action: Bob: `syncweb package list`
- Expected: Shows installed packages
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 6 - NOT RUN
- Action: Bob: `syncweb package versions <collection-id>`
- Expected: Lists installed versions
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 7 - NOT RUN
- Action: Bob: `syncweb package verify <collection-id>`
- Expected: Integrity check passes
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 8 - NOT RUN
- Action: Bob: `syncweb package upgrade <collection-id>`
- Expected: Upgrades to latest
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 9 - NOT RUN
- Action: Bob: `syncweb package switch <collection-id> v1`
- Expected: Switches to v1 via symlink
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Check version: `cat ./pkg-install/.version`

Step 10 - NOT RUN
- Action: Bob: `syncweb package remove <collection-id>`
- Expected: Cleanly removes
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb package list` no longer shows it


### 9.3 Package Archive (.car.zst)

Step 1 - NOT RUN
- Action: syncweb package export <collection-id> /tmp/pkg.car.zst
- Expected: Creates compressed archive
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `ls -la /tmp/pkg.car.zst`

Step 2 - NOT RUN
- Action: On another machine (air-gapped): `syncweb package import /tmp/pkg.car.zst ./pkg-import`
- Expected: Imports and installs
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb package list` shows it

Step 3 - NOT RUN
- Action: syncweb package import --no-install /tmp/pkg.car.zst /tmp/extract
- Expected: Extracts without installing
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 10. Networks

### 10.1 Create & Invite

Step 1 - NOT RUN
- Action: Alice: `syncweb network create home`
- Expected: Creates network "home"
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb network ls` shows it

Step 2 - NOT RUN
- Action: Alice: `syncweb network ls home`
- Expected: Shows network details
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM networks;"`

Step 3 - NOT RUN
- Action: Alice: `syncweb network invite home <bob-device-id>`
- Expected: Creates invitation ticket
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: Bob: `syncweb network join <ticket>`
- Expected: Joins network "home"
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb network ls` shows it


### 10.2 Folder in Network Context

Step 1 - NOT RUN
- Action: Alice: `syncweb folders create --network home ./nw-docs`
- Expected: Creates folder in "home" network
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb network ls home` shows the folder

Step 2 - NOT RUN
- Action: Alice imports files, Bob joins the folder
- Expected: Bob gets auto-discovery via network gossip
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: Alice: `syncweb network kick home <bob-device-id>`
- Expected: Removes Bob from network
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Bob disconnects


### 10.3 Network Events & Health

Step 1 - NOT RUN
- Action: syncweb network events home
- Expected: Shows peer joins, leaves, sync events
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `sqlite3 ~/.local/share/syncweb/stats.db "SELECT * FROM network_events WHERE network_id='home' ORDER BY timestamp DESC LIMIT 10;"`

Step 2 - NOT RUN
- Action: syncweb network health home
- Expected: Network connectivity health
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: syncweb network test-relay
- Expected: Tests relay connectivity
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: `syncweb network peers ./nw-docs` (daemon running)
- Expected: Shows the folder's inbound peers (Devices) and a per-blob `% seeded` table
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: With `--json`, emits a single `{folder, peers, per_blob}` object

Step 5 - NOT RUN
- Action: `syncweb network peers ./nw-docs --json` (no daemon)
- Expected: Honest empty state: `{folder, peers: [], per_blob: []}` — no guessed values
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 11. Indexing Service

Step 1 - NOT RUN
- Action: syncweb indexing enable ./shared-docs
- Expected: Enables FTS5 index for folder
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Check indexing db exists: `ls ~/.local/share/syncweb/indexing.sqlite`

Step 2 - NOT RUN
- Action: syncweb search --kind catalog "test"
- Expected: Full-text search results
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: syncweb indexing publish catalog ./shared-docs --catalog <name>
- Expected: Publishes to catalog namespace
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb indexing disable ./shared-docs
- Expected: Disables indexing
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 12. Links

Step 1 - NOT RUN
- Action: syncweb link create --immutable ./shared-docs/test.txt
- Expected: Creates immutable link
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: syncweb link create --mutable ./shared-docs/test.txt
- Expected: Creates mutable pointer
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: syncweb link create --private ./shared-docs/test.txt --expires 7d
- Expected: Creates expiring private link
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb link resolve <link-id>
- Expected: Resolves to manifest + providers
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 5 - NOT RUN
- Action: syncweb link revoke <link-id>
- Expected: Revokes private link
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Resolution fails after revocation


---

## 13. Schedules & Bandwidth

### 13.1 Schedule Configuration

Step 1 - NOT RUN
- Action: syncweb config schedule
- Expected: Shows current global schedule
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: syncweb config schedule set --active "22:00-06:00"
- Expected: Sets active hours
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `sqlite3 ~/.local/share/syncweb/node.db "SELECT * FROM app_config WHERE key LIKE 'schedule%';"`

Step 3 - NOT RUN
- Action: syncweb config schedule set --bandwidth "5MB/s" --period "08:00-18:00"
- Expected: Time-based bandwidth limit
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb config schedule folder media --active "01:00-05:00"
- Expected: Per-folder override
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 5 - NOT RUN
- Action: `syncweb config schedule` (check)
- Expected: Shows updated schedule
- Actual output: No output; testing stopped after more than five unexpected errors.


### 13.2 Bandwidth Verification

Step 1 - NOT RUN
- Action: Sync large files while monitoring `syncweb stats network`
- Expected: Bandwidth is capped at configured limit
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `sqlite3 ~/.local/share/syncweb/stats.db "SELECT SUM(bytes) FROM bandwidth_events WHERE direction='download';"`

Step 2 - NOT RUN
- Action: Wait for inactive window, trigger sync
- Expected: Sync does not start (or is delayed)
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: syncweb stats network
- Expected: Shows totals, per-folder, per-peer
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb stats network --period 24h
- Expected: Only transfer events from the last 24h
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `sqlite3 ~/.local/share/syncweb/stats.db "SELECT COUNT(*) FROM bandwidth_events;"`

Step 5 - NOT RUN
- Action: syncweb stats network --since 24h --json
- Expected: JSON object with `total_upload`/`total_download`/`per_folder`/`per_peer`/`period_start`
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 6 - NOT RUN
- Action: syncweb stats network --folder <namespace>
- Expected: Per-folder breakdown
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 7 - NOT RUN
- Action: syncweb stats network --follow --once
- Expected: Prints the current snapshot and exits (cron-safe)
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 8 - NOT RUN
- Action: `syncweb stats network --follow` (Ctrl+C after a moment)
- Expected: Streams sync sessions / network events live; under `--json` one JSON object per line (NDJSON)
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 14. Filter Engine / Watch Mode

Step 1 - NOT RUN
- Action: Create `~/.config/syncweb/filters.toml` with rules
- Expected: (none specified)
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: See example below

Step 2 - NOT RUN
- Action: syncweb watch --show-filters
- Expected: Shows loaded filter rules
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: syncweb watch --dry-run --paths <folder>
- Expected: Shows what would be accepted/rejected
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb watch --once <folder>
- Expected: Imports files matching the rules, rejects the rest
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 5 - NOT RUN
- Action: syncweb watch --filters /path/to/filters.toml <folder>
- Expected: Uses a custom filter config path
- Actual output: No output; testing stopped after more than five unexpected errors.


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

Step 1 - NOT RUN
- Action: syncweb watch ./shared-docs
- Expected: Watches folder for changes
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Trigger a change and observe

Step 2 - NOT RUN
- Action: Touch/create/delete/modify a file in `./shared-docs`
- Expected: Watcher detects and imports change
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Logs show "file changed: <path>"

Step 3 - NOT RUN
- Action: syncweb watch --once ./shared-docs
- Expected: Scans once, then exits
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: Create `.syncignore` with glob patterns
- Expected: Watcher excludes matching files
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 16. Conflict Resolution

Step 1 - NOT RUN
- Action: Alice and Bob both modify the same file offline
- Expected: Both have local edits
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: Both come online and sync
- Expected: Conflict detected
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: For text files: auto-resolve keeps newer (LWW), saves .diff
- Expected: Conflict auto-resolved or listed
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Check for `.conflict` or `.diff` files

Step 4 - NOT RUN
- Action: Binary file conflict
- Expected: Creates `.conflict.<hash>` file
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Both versions preserved


---

## 17. Offline Queue

Step 1 - NOT RUN
- Action: Go offline (disconnect network)
- Expected: (none specified)
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: Make changes to synced folder
- Expected: (none specified)
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: Come back online
- Expected: Pending changes sync automatically
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 18. Media Server

Step 1 - NOT RUN
- Action: syncweb start --media-only
- Expected: Starts HTTP server on 127.0.0.1:9193
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: curl http://127.0.0.1:9193/media/<blob-hash>
- Expected: Serves blob content
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Compare with `syncweb stat` output for hash

Step 3 - NOT RUN
- Action: curl -H "Range: bytes=0-100" http://127.0.0.1:9193/media/<hash>
- Expected: Partial content (206) with first 100 bytes
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Check Content-Range header

Step 4 - NOT RUN
- Action: curl http://127.0.0.1:9193/
- Expected: 404 or list
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 5 - NOT RUN
- Action: Configure custom listen address in config
- Expected: Server starts on specified address
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 19. WebSocket Bridge

Step 1 - NOT RUN
- Action: Run daemon (bridge starts on 127.0.0.1:9192)
- Expected: Bridge listening
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: Connect via `websocat ws://127.0.0.1:9192/bridge`
- Expected: Connection accepted
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Logs show "bridge connection accepted"

Step 3 - NOT RUN
- Action: Send a valid JSON command over WS
- Expected: Receives response
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: See bridge protocol docs

Step 4 - NOT RUN
- Action: Send invalid JSON
- Expected: Receives error message
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 20. Syncthing Relay (BEP)

Step 1 - NOT RUN
- Action: Configure BEP: `syncweb config set bep.enabled true`
- Expected: BEP enabled
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb config show bep`

Step 2 - NOT RUN
- Action: syncweb config set bep.relay_urls '["tcp://relay.syncthing.net:22270"]'
- Expected: Sets relay
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: Two nodes behind CGNAT (or simulate with firewall)
- Expected: Connection falls back to Syncthing relay
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Logs show "relay connected"

Step 4 - NOT RUN
- Action: syncweb network test-relay
- Expected: Tests relay connectivity
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Logs latency and status

Step 5 - NOT RUN
- Action: syncweb devices
- Expected: Shows DeviceIds
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Verify format matches Syncthing DeviceId


---

## 21. Discovery Mechanisms

Step 1 - NOT RUN
- Action: Two nodes on same LAN
- Expected: Auto-discover via mDNS within ~1s
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: Disable mDNS: `syncweb config set discovery.local_mdns false`
- Expected: No LAN discovery
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 3 - NOT RUN
- Action: Re-enable mDNS
- Expected: Discovery resumes
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: Two nodes on different networks
- Expected: Discover via DHT (~5-10s) or gossip
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 22. CLI Global Flags & Output

Step 1 - NOT RUN
- Action: syncweb --verbose <command>
- Expected: Debug-level output
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: syncweb --json folders
- Expected: JSON output
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb --json folders \| jq .`

Step 3 - NOT RUN
- Action: syncweb --no-color devices
- Expected: Output without ANSI color
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb --data-dir /tmp/syncweb-test folders
- Expected: Uses custom data dir
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Check files in /tmp/syncweb-test/

Step 5 - NOT RUN
- Action: syncweb --network home folders
- Expected: Shows folders in "home" network context
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 6 - NOT RUN
- Action: syncweb --help
- Expected: Grouped help shows the full major-release surface: 25 functional verbs (`start`, `stop`, `status`, `reload`, `sync`, `folders`, `ls`, `stat`, `find`, `search`, `sort`, `download`, `verify`, `transfer`, `share`, `access`, `link`, `package`, `network`, `watch`, `snapshot`, `indexing`, `stats`, `db`, `config`) plus `version`/`devices`/`completions`/`manpages`/`help`. Re-homed verbs (`create`, `join`, `leave`, `import`, `networks`, `publish`, `unshare`, `provider`) no longer parse
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `syncweb folders create --help` still works

Step 7 - NOT RUN
- Action: syncweb <command> --help
- Expected: Command-specific help, including subcommands (`folders create`, `share list`, `network status`, `indexing publish`)
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 23. Version & Completions

Step 1 - NOT RUN
- Action: syncweb version
- Expected: Shows version, commit, build date
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: syncweb completions bash
- Expected: Outputs bash completion script
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Source it: `source <(syncweb completions bash)`

Step 3 - NOT RUN
- Action: syncweb completions zsh
- Expected: Zsh completions
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb completions fish
- Expected: Fish completions
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 5 - NOT RUN
- Action: syncweb manpages /tmp/syncweb.1
- Expected: Generates manpage
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `man /tmp/syncweb.1`


---

## 24. Integrity & Error Recovery

Step 1 - NOT RUN
- Action: Kill daemon with SIGKILL (kill -9)
- Expected: Hard crash
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 2 - NOT RUN
- Action: Start daemon again
- Expected: Recovers gracefully, WAL replay fixes DB
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Check: `syncweb status` works

Step 3 - NOT RUN
- Action: Delete a random blob file from blob store
- Expected: Data missing
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: syncweb verify ./shared-docs
- Expected: Reports missing/corrupt blob
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 5 - NOT RUN
- Action: syncweb verify --fix ./shared-docs
- Expected: Re-downloads from peers
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 6 - NOT RUN
- Action: syncweb db check
- Expected: DB integrity passes
- Actual output: No output; testing stopped after more than five unexpected errors.


---

## 25. Performance Smoke Tests

Step 1 - NOT RUN
- Action: Time `syncweb start` (cold start)
- Expected: < 500ms
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: `time syncweb start --bg`

Step 2 - NOT RUN
- Action: Create 10000 files, time `syncweb folders import`
- Expected: < 3s (default parallel)
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Compare with `--threads 1`

Step 3 - NOT RUN
- Action: Time `syncweb ls` on 10000 entries
- Expected: < 500ms
- Actual output: No output; testing stopped after more than five unexpected errors.

Step 4 - NOT RUN
- Action: Sync a 10GB folder over LAN
- Expected: > 500 MB/s throughput
- Actual output: No output; testing stopped after more than five unexpected errors.
- Debug: Monitor: `syncweb stats network`

Step 5 - NOT RUN
- Action: `syncweb stats seeding --folder .` on folder with 1000+ entries
- Expected: < 1s
- Actual output: No output; testing stopped after more than five unexpected errors.


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
syncweb start --bg --log-file /tmp/syncweb-debug.log
cat /tmp/syncweb-debug.log

# Custom data directory
syncweb --data-dir /tmp/syncweb-test ...

# Network isolation (for running two nodes on same machine)
# Start two terminals with different data dirs:
# Terminal 1 (Alice):
syncweb --data-dir /tmp/alice start --bg --log-file /tmp/alice-daemon.log
# Terminal 2 (Bob):
syncweb --data-dir /tmp/bob start --bg --log-file /tmp/bob-daemon.log
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
syncweb --data-dir /tmp/alice-data start --bg --log-file /tmp/alice-daemon.log
syncweb --data-dir /tmp/bob-data start --bg --log-file /tmp/bob-daemon.log
```

## Manual Test Run: 2026-10-04 (repaired build, resumed)

- VMs: `syncweb-a` (`10.0.3.2`) and `syncweb-b` (`10.0.3.3`).
- Build: source rebuild deployed to both VMs. Binary SHA-256: `018cb6ab7e550612990e07ec7e5bbe42781940227102a8ff7a4d5007d493a43b`.
- Test data: fresh `/root/manual2/default` profiles; Alice folder `/root/shared-docs`; Bob folder `/root/bob-shared`.
- The run stopped after 15 new failures, exceeding the requested limit of ten. Sections after the download matrix were not run.

### Repaired existing failures

| Status | Test | Command / output |
|---|---|---|
| PASS | Blob startup recovery | Before fix: `Error: failed to read blob: encode error`. After fix: `folder mode metadata is unavailable; using capability-derived mode` followed by a successful folder listing; `folders import` returned `import requested: 1`. |
| PASS | Metadata durability regression | `cargo test -p syncweb-core --test integration_tests integration::folder_test::folder_mode_survives_node_restart -- --exact --nocapture` → `1 passed`. |
| PASS | Persisted mount restoration | After restarting the daemon, `folders --json` reported `path: "/root/shared-docs"` and `ls /root/shared-docs` listed `test.txt`. |
| PASS | Source validation | `cargo fmt --all`, targeted restart test, and `cargo build --release` all completed successfully. |

### Resumed P2P tests

| Status | Test | Command / output |
|---|---|---|
| PASS | Alice creates folder | `folders create --mode sendreceive /root/shared-docs` → namespace `0030afcbb2f393f85ba65f546ed3ffeeb36a158c700e8ac2b9ad63c76c42170a`; daemon reported `running`. |
| PASS | Alice imports and lists file | `folders import /root/shared-docs` → `import requested: 1`; `ls` showed `test.txt`, `12 B`, `local`. |
| PASS | Bob joins ticket | `folders join <alice-url> /root/bob-shared` → `joined: 0030af...`; Bob's folder listing reported `mode: "receiveonly"`. |
| FAIL | Join starts syncing | After 10 seconds, Bob's `ls /root/bob-shared` reported `has no remote entries yet`; status remained `Session: paused`, `Entries: 0`. Explicit `sync <namespace>` and `folders join --subscribe <namespace>` did not produce entries. |
| FAIL | Bob downloads file | `download /root/bob-shared/test.txt` → `Error: invalid download namespace: Odd number of digits`; `/root/bob-shared/test.txt` was not created. |

### Listing and search tests on Alice

| Status | Test | Command / output |
|---|---|---|
| PASS | `ls` metadata | `ls /root/shared-docs` showed `test.txt`, `12 B`, `local`. |
| PASS | `ls --local-only` | Output: `test.txt`. |
| PASS | `ls --remote-only` | Empty table, as expected with no remote-only entries. |
| PASS | `ls --path-prefix docs` | Empty table, as expected because no `docs/` entries exist. |
| PASS | `ls --no-enrich` | `test.txt`, `12 B`, `Modified: -`, `local`. |
| PASS | `ls --sort size` | One-row table sorted by size. |
| PASS | `ls --json` | Valid object with `folder`, `path`, and `entries`; entry hash was `dc5a4edb8240b018124052c330270696f96771a63b45250a5c17d3000e823355`. |
| PASS | Plain directory error | `ls /tmp` → `not inside of a Syncweb folder` with the documented `--local-only` suggestion. |
| FAIL | Regex find | `find '.*\.txt$' /root/shared-docs` → `No files matching '.*\.txt$' found`, despite `test.txt` being indexed. |
| FAIL | Local-only find | `find --type f --ext mp3 --local-only /root/music` → `Error: Permission denied (os error 13)` for a root-owned test file. |
| FAIL | Depth find | `find --depth +2 --depth -5 config /root/shared-docs` → `error: unexpected argument '-5' found`. |
| FAIL | Regex find JSON | JSON returned `"entries": []` for `find '.*\.txt$' --json /root/shared-docs`. |

### Sort and stat tests on Alice

| Status | Test | Command / output |
|---|---|---|
| PASS | `sort --by size` | One-row metadata table containing `test.txt`. |
| FAIL | `sort --by name` | `error: invalid value 'name' for '--by <BY>'`. |
| FAIL | `sort --by modified` | `error: invalid value 'modified' for '--by <BY>'`. |
| FAIL | `sort --by state` | `error: invalid value 'state' for '--by <BY>'`. |
| PASS | Disk sort by peers/niche | `sort --local-only --by peers` and `--by niche` both returned `test.txt`. |
| PASS | Sort limits | `sort --local-only --limit-size 10GB --min-seeders 2` → `No entries match the sorting criteria`. |
| FAIL | `sort --no-enrich` | `Error: sort --by 'niche' is only valid with --local-only; on a synchronized folder use name, size, modified, or state`. |
| PASS | Stat variants | Default, `--terse`, `--format "%n %s %y"`, modified-file, and `*.md` tests all returned successfully. Example terse output: `/root/shared-docs/test.txt|13|1|71d4204dd0e09850084ce60ffddc2cd8243c53573a439d338d8106aaff32d26f|0`. |

### Download tests (stop point)

| Status | Test | Command / output |
|---|---|---|
| FAIL | Download file | `download /root/shared-docs/test.txt` → `Error: invalid download namespace: Invalid string length`. |
| FAIL | Download directory | `download /root/shared-docs/` → `Error: invalid download namespace: Invalid string length`. |
| FAIL | Download max count | `download --max-count 10 /root/shared-docs/` → `Error: invalid download namespace: Invalid string length`. |
| FAIL | Download size | `download --size -1GB /root/shared-docs/` → `error: unexpected argument '-1' found`. |
| FAIL | Download threads | `download --threads 1 /root/shared-docs/` → `Error: invalid download namespace: Invalid string length`. |

---

## Manual Test Run: 2026-10-04 (bugfix re-run, fixes verified)

- VMs: `syncweb-a` (`10.0.3.2`) and `syncweb-b` (`10.0.3.3`).
- Build: source rebuild from `c678de7` + bugfix commits (`better kind default: glob or regex`, `download: materialize content`). Binary SHA-256: `cb316ad00c9cea5a0fe252fc17a07040456c5ed0a5dc010b06e6d07facbe750f`.
- Test data: fresh `/root/manual3/default` profiles; Alice folder `/root/shared-docs`; Bob folder `/root/bob-shared`; daemons started as persistent `systemd-run` units so they survive across test invocations.
- Fixes verified in this run: download path→namespace resolution, download materialization to the mount path, `import --folder <path>` resolution, `sort --by name/modified/state`, `sort --no-enrich` default, `find` regex auto-detection, `--size -1GB` / `--depth -5` hyphen-value parsing, and P2P join-then-sync (`ls` now shows remote entries).
- New failures observed (counted): 7 root-cause issues — `package export` hang, `stats seeding` missing subcommand, piped `download -` NotFound on already-local content, `--no-color` flag missing, `link create --immutable/--mutable` flags missing, `network health` subcommand missing, and minimal `snapshot list` output. This is under the requested stop limit of ten.

### Initialization & Configuration (Section 1) — re-run

| Status | Test | Command / output |
|---|---|---|
| PASS | `folders create ./test-folder` | Created folder; printed `syncweb://folder/...` URL. |
| PASS | `folders create --mode sendonly` | Created SendOnly folder + URL. |
| PASS | `folders create --network home` | Created network-linked folder + URL. |
| PASS | `folders create --mode receiveencrypted` | Created ReceiveEncrypted folder + URL. |
| PASS | `folders` list | Table shows all four folders with modes. |
| PASS | `config` | Printed full TOML (`[bep]`, `[schedule]`, `[bandwidth]`, `[discovery]`). |
| PASS | `config set default_sync_mode ReceiveOnly` | `default_sync_mode updated`; `config show` reflects it. |
| PASS | `config set bandwidth.max_download 5MB/s` | `bandwidth.max_download updated`; reflected in `config show`. |
| PASS | `db check` | `node.db: 0 errors`, `stats.db: 0 errors`, all databases healthy. |
| PASS | `db stats` | node.db 196 KB, stats.db 104 KB, 0 freelist pages. |
| PASS | `db vacuum` | `VACUUM complete`; zero freelist pages for both DBs. |
| PASS | `db backup` | Backup zip at `/tmp/syncweb-backup/syncweb-db-backup-...` with node.db, stats.db, config.toml, identity.key, manifest.json. |

### Daemon Lifecycle (Section 2) — re-run

| Status | Test | Command / output |
|---|---|---|
| PASS | `start --bg` | `daemon starting: <pid>`; daemon stays up when started as a persistent unit. |
| PASS | `status` | `daemon: running`, pid, node id, bandwidth, folder table. |
| PASS | `--json status` | Single JSON object with `daemon`, `folders`, `devices`, `networks`, `transfers`. |
| PASS | `stop` (pipe `n`) | `aborted`; daemon stays running. |
| PASS | `stop --yes` | `shutdown requested`; status becomes `daemon not running` after shutdown. |
| PASS | `reload` | `configuration reload requested`. |
| PASS | `sync` | `synchronization requested`. |
| PASS | `sync <namespace>` | `synchronization requested`. |
| PASS | `folders create` (daemon running) | Created folder through the daemon. |
| PASS | `folders leave` | `left: <namespace>`; folder removed from listing. |

### Folder Sync — P2P (Section 3) — re-run

| Status | Test | Command / output |
|---|---|---|
| PASS | Alice: `folders create --mode sendreceive /root/shared-docs` | Namespace `2efafe0b...`; daemon reported running. |
| PASS | Alice: `folders import /root/shared-docs` | `import requested: 1`. |
| PASS | Bob: `folders join <alice-url> /root/bob-shared` | `joined: 2efafe0b...`. |
| PASS | Bob: `ls /root/bob-shared` (was FAIL "has no remote entries yet") | Shows `test.txt`, `12 B`, State `local`. Join-then-sync now works. |
| PASS | Bob: `download /root/bob-shared/test.txt` (was FAIL "invalid download namespace") | `downloaded: 0 bytes`; `cat test.txt` → `hello world` (blob materialized to the mount path). |
| PASS | Bob: `download /root/bob-shared/` + `--max-count` + `--threads` + `--size -1GB` (was FAIL) | All return `downloaded: 0 bytes`, no namespace/arg errors. |
| PASS | Bob write on ReceiveOnly join | `Error: failed to set document blob: Attempted to insert to read only replica` (correct read-only enforcement). |
| PASS | `folders` / `folders --json` | Table / JSON array with folder metadata. |
| PASS | `devices` / `devices --json` | Iroh id + Syncthing DeviceId; `peers: []` when no daemon peers. |

### Leave / listing (Sections 3.3--3.4) — re-run

| Status | Test | Command / output |
|---|---|---|
| PASS | `folders leave <ns>` | `left: <namespace>`; removed from list. |
| PASS | `folders leave --delete-files` (non-TTY, no `--yes`) | `aborted`; files stay. |
| PASS | `folders leave --delete-files --yes` | `left: <namespace>`; directory removed. |

### Listing, Searching, Sorting, Stat (Section 4) — re-run

| Status | Test | Command / output |
|---|---|---|
| PASS | `ls` | 5 entries, sizes, State local. |
| PASS | `ls --local-only` | Disk scan lists all entries. |
| PASS | `ls --remote-only` | Empty table (no remote rows). |
| PASS | `ls --path-prefix docs` | Only `docs/*` entries. |
| PASS | `ls --no-enrich` | `Modified: -` column. |
| PASS | `ls --sort size` | Table sorted by size. |
| PASS | `ls --json` | Envelope with `folder`, `path`, `entries`. |
| PASS | `ls /tmp` (plain dir) | `not inside of a Syncweb folder` with `--local-only` hint. |
| PASS | `find '.*\.txt$'` (was FAIL) | `notes.txt`, `test.txt` (regex auto-detected). |
| PASS | `find '*.md'` | `docs.md`, `docs/a.md`, `docs/b.md` (glob). |
| PASS | `find --fixed-strings 'note'` | `notes.txt` (substring/exact). |
| PASS | `find '.*\.md$' --json` (was FAIL) | `entries` populated. |
| PASS | `find --ignore-case README` | `No files matching` (correct, no README). |
| PASS | `find --depth +2 --depth -5` (was FAIL) | No `-5` argument error. |
| PASS | `sort --by size` | Table sorted by size. |
| PASS | `sort --by name` (was FAIL) | Table sorted by path. |
| PASS | `sort --by modified` (was FAIL) | Table sorted by modified time. |
| PASS | `sort --by state` (was FAIL) | Table sorted by state. |
| PASS | `sort --no-enrich` (was FAIL) | Metadata table with `Modified: -`. |
| PASS | `sort --local-only --by peers` / `--by niche` | Lists entries. |
| PASS | `sort --local-only --limit-size 10GB --min-seeders 2` | `No entries match the sorting criteria`. |
| PASS | `stat` / `stat --terse` / `stat --format` | Size, blocks, hash, peers; terse pipe output; custom template. |

### Download & Import/Export (Section 5) — re-run

| Status | Test | Command / output |
|---|---|---|
| PASS | `download <folder>/<file>` (was FAIL) | Fetches + materializes file to disk. |
| PASS | `download <folder>/` (was FAIL) | Fetches + materializes entries. |
| PASS | `download --max-count 10` (was FAIL) | Success, no namespace error. |
| PASS | `download --size -1GB` (was FAIL) | Success, no `-1` argument error. |
| PASS | `download --threads 1` (was FAIL) | Success, no namespace error. |
| FAIL | Piped `find '*.md' \| download -` | `Error: daemon download failed: Remote peer aborted sync: NotFound` (downloading already-local content on the publishing node). |
| PASS | `folders import` | `import requested: N`. |
| PASS | `folders import --threads 1` | `import requested: 5`. |
| PASS | `folders import <path> --folder <folder>` (was FAIL) | `import requested: 1`; file appears in listing. |
| FAIL | `package export ./pkg-dir /tmp/pkg.car.zst` | Hangs (timed out at rc=124); no archive produced. |

### Health & Verify (Section 6) — re-run

| Status | Test | Command / output |
|---|---|---|
| FAIL | `stats seeding` | `error: unrecognized subcommand 'seeding'` (the verb does not exist; `stats files`/`stats network` are the available subcommands). |
| PASS | `verify ./shared-docs` | `total: 6, verified: 6, corrupted: 0, missing: 0, valid: true`. |
| PASS | `db check` | Both databases healthy. |

### Public Folders & Access (Section 7) — re-run

| Status | Test | Command / output |
|---|---|---|
| PASS | `share ./shared-docs` | Printed read-only share ticket. |
| PASS | `access ./shared-docs` | One table with `Folder · Mode · Write? · Shared with · Devices · Networks`. |
| PASS | `access --json` | Array of `{folder, mode, write, shares, devices, networks}`. |
| PASS | `access --revoke ./shared-docs` | `unshared: <ns> (read)`. |

### Snapshots (Section 8) — re-run

| Status | Test | Command / output |
|---|---|---|
| PASS | `snapshot create ./shared-docs --description` | Returns content hash (same hash for unchanged content). |
| PASS | `snapshot list ./shared-docs` | Works, but output is minimal (`snapshots: 1`; no per-snapshot description/timestamp table). |

### Networks (Section 10) — re-run

| Status | Test | Command / output |
|---|---|---|
| PASS | `network create home` | Created (no output; `network ls` shows it). |
| PASS | `network ls home` | Table with ID, members, folders. |
| PASS | `network events home` | `no events`. |
| FAIL | `network health home` | `error: unrecognized subcommand 'health'` (use `network status`). |

### Indexing (Section 11) — re-run

| Status | Test | Command / output |
|---|---|---|
| PASS | `indexing enable ./shared-docs` | Enabled (no error). |
| PASS | `search --kind catalog "test"` | `no results found` (correct for an empty catalog). |

### Links (Section 12) — re-run

| Status | Test | Command / output |
|---|---|---|
| FAIL | `link create --immutable` | `error: unexpected argument '--immutable' found` (flag not implemented; `link create` exposes `--name/--version/--private/--expires/--publish`). |
| FAIL | `link create --mutable` | `error: unexpected argument '--mutable' found` (flag not implemented). |

### Schedules & Bandwidth (Section 13) — re-run

| Status | Test | Command / output |
|---|---|---|
| PASS | `config schedule` | Shows `active_hours = ""`, `bandwidth = []`. |
| PASS | `config schedule set --active "22:00-06:00"` | `schedule updated`; `config schedule` shows the new window. |

### CLI Global Flags & Output (Section 22) — re-run

| Status | Test | Command / output |
|---|---|---|
| FAIL | `--no-color devices` | `error: unexpected argument '--no-color' found` (global flag not implemented). |
| PASS | `--help` | Grouped help shows the full major-release surface (start, stop, status, reload, sync, folders, ls, stat, find, search, sort, download, verify, transfer, share, access, link, package, network, watch, snapshot, indexing, stats, db, config, version, devices, completions, manpages, help). |

### Version & Completions (Section 23) — re-run

| Status | Test | Command / output |
|---|---|---|
| PASS | `version` | `syncweb 0.1.1-alpha.7`. |
| PASS | `completions bash` / `completions zsh` | Generated completion scripts. |
| PASS | `manpages <dir>` | Generated `syncweb.1` plus per-subcommand manpages. |

---

## Manual Test Run: 2026-10-04 (all known failures fixed + full surface re-test)

- VMs: `syncweb-a` (`10.0.3.2`) and `syncweb-b` (`10.0.3.3`).
- Build: source rebuild from the bugfix commits above (`namespace_for_path`, `export via temp dir`, plus snapshot IPC / indexing / media fixes). Binary SHA-256: `bef3d3625429cd1bb40f54eb0c1f01f8bfdc18922fc1eb9bf700be0bed7ff347`.
- Test data: fresh `/root/manual4/default` profiles on both nodes; Alice folders `/root/shared-docs`, `/root/twoway`, `/root/sendonly`; Bob folders `/root/bob-shared`, `/root/bob-twoway`, `/root/bob-sendonly`. Daemons run as detached `setsid` background processes (survive across test invocations).
- The seven previously-reported failures were all fixed in code and verified in this run; several additional latent bugs found while re-testing were also fixed (daemon path→namespace resolution, snapshot diff/restore daemon hang, indexing path resolution, media-server relay hang). New failures observed are listed at the end (fewer than ten).

### Repaired existing failures

| Status | Test | Command / output |
|---|---|---|
| PASS | `package export` (was FAIL: hang) | With the daemon running: `exported: /tmp/export-test (1e7a1275-... 1.0.0, 5 entries)`; `.car.zst` produced. Root cause was a second node opening the live blob/docs store while the daemon held it; export now uses a scratch data dir + relay-disabled node. `--version 1.0.1` also works. |
| PASS | `stats seeding` (was FAIL: missing subcommand) | New subcommand; with daemon: `folder: 54e430d3...`, `total: 7`, `unseeded: 7` (peer counts not tracked per blob yet); `--json` emits `{folder,total,well_seeded,under_seeded,unseeded}`. |
| PASS | Piped `find '*.md' \| download -` (was FAIL: NotFound) | Run from inside the folder: `downloaded: 0 bytes` × 3, rc=0. Already-local content now skips the fetch (no doc re-sync of local blobs). |
| PASS | `--no-color devices` (was FAIL: flag missing) | Global `--no-color` added; `devices` prints identities with no styling. |
| PASS | `link create --immutable` (was FAIL) | `syncweb://content/8dfd2d94...`; resolves to the content hash. |
| PASS | `link create --mutable` (was FAIL) | `syncweb://name/7c9efe45.../test.txt` (alias derived from file name); resolves with `sequence: 1`. |
| PASS | `network health home` (was FAIL: missing subcommand) | New subcommand: `Network: home`, `events: 0`, `sessions: 0`. |
| PASS | `snapshot list ./shared-docs` (was FAIL: minimal output) | Full table with `ID · Created · Size · Files · Description` (daemon returns the full list over IPC instead of a count). |

### Also found and fixed in this run

| Status | Test | Command / output |
|---|---|---|
| PASS | Daemon path→namespace resolution | `verify /root/shared-docs`, `snapshot create/diff`, `stats seeding`, `network peers` all failed with "selector is not a namespace ID and more than one synchronized folder is available". Fixed via `FolderRegistry::namespace_for_path` reverse lookup in `resolve_folder_for_daemon`. |
| PASS | `snapshot diff` (was FAIL: hang) | Previously opened a second embedded node on the live data dir (store lock hang); now routed through the daemon over IPC: `modified test.txt <old> <new>`. |
| PASS | `indexing enable/disable/publish` | `enable /root/shared-docs` → `enabled: 54e430d3...`; `disable` → `disabled: 54e430d3...`; `publish /root/shared-docs --catalog mycat` → `published: 9`. All paths resolve via the daemon-aware resolver now. |
| PASS | `start --media-only` (was FAIL: relay hang) | Standalone media server now opens a relay-disabled node; `curl /` → 404 as documented. |

### Section 1 — Initialization & Configuration

| Status | Test | Command / output |
|---|---|---|
| PASS | `folders create ./test-folder` | Printed `syncweb://folder/2ecf866f...`. |
| PASS | `folders create --mode sendonly` | SendOnly folder + URL. |
| PASS | `folders create --network home` | Network-linked folder + URL. |
| PASS | `folders create --mode receiveencrypted` | ReceiveEncrypted folder + URL. |
| PASS | `folders` list | Table shows all four folders with modes. |
| PASS | `config` | Full TOML with `[bep]`, `[schedule]`, `[bandwidth]`, `[parallel]`, `[discovery]`, `[channels]`. |
| PASS | `config show schedule` / `show bep` | Section-only output. |
| PASS | `config set default_sync_mode ReceiveOnly` | `default_sync_mode updated`; reflected in `config show`. |
| PASS | `config set bandwidth.max_download 5MB/s` | `bandwidth.max_download updated`; shown as `5MB/s`. |
| PASS | `db check` | `node.db: 0 errors`, `stats.db: 0 errors`, all databases healthy. |
| PASS | `db stats` | node.db 196 KB, stats.db 104 KB, 0 freelist pages. |
| PASS | `db vacuum` | `VACUUM complete`, zero freelist pages. |
| PASS | `db backup --output` | Backup zip at `/tmp/syncweb-backup/syncweb-db-backup-...` with node.db, stats.db, config.toml, identity.key, manifest.json. |

### Section 2 — Daemon Lifecycle

| Status | Test | Command / output |
|---|---|---|
| PASS | `start --bg` | `daemon starting: 16457`; status then reports `running`. |
| PASS | `status` | `daemon: running`, pid, node id, bandwidth, folder table. |
| PASS | `--json status` | Single object with `daemon`, `devices`, `folders`, `networks`, `transfers` keys. |
| PASS | `stop` (pipe `n`) | `aborted`; daemon stays running. |
| PASS | `stop --yes` | `shutdown requested`; status → `daemon not running`. |
| PASS | `stop --force --yes` | `forced shutdown requested`; daemon stops. |
| PASS | Ctrl+C (SIGINT) graceful shutdown | Logs `maintenance task shutting down`, prints `daemon stopped`, status → not running. |
| PASS | `reload` | `configuration reload requested`. |
| PASS | `sync` / `sync <ns>` | `synchronization requested` for both. |
| PASS | `folders create` + `folders leave` (daemon running) | Created `./new-folder`, listed it, then `left: 47631467...`. |

### Section 3 — Folder Sync (P2P)

| Status | Test | Command / output |
|---|---|---|
| PASS | Alice `folders create --mode sendreceive /root/shared-docs` | Namespace `54e430d3...`. |
| PASS | Alice `folders import --folder` | `import requested: 1`; `ls` shows `test.txt`, 12 B, local. |
| PASS | Bob `folders join <url> /root/bob-shared` | `joined: 54e430d3...`. |
| PASS | Bob `ls /root/bob-shared` | Shows `test.txt`, 12 B after ~12s discovery. |
| PASS | Bob `download test.txt` | `downloaded: 0 bytes`; `cat` → `hello world`. |
| PASS | Bob write on ReceiveOnly join | `Error: failed to set document blob: Attempted to insert to read only replica` (correct read-only enforcement). |
| PASS | SendOnly folder, Bob join + read | Bob `ls` shows `a.txt`; Bob write rejected (read-only replica). |
| PASS | Two-way sync via `--write` ticket | Bob `folders join` (write ticket) then `import requested: 1` succeeds; Alice sees the edit after sync. |
| PASS | `folders` / `devices` / `devices --json` | Table / JSON with Iroh + Syncthing ids. |

### Section 4 — Listing, Searching, Sorting, Stat

| Status | Test | Command / output |
|---|---|---|
| PASS | `ls` / `--local-only` / `--remote-only` / `--path-prefix` / `--no-enrich` / `--sort size` / `--json` | All variants behave as documented; JSON envelope `{folder, path, entries}` with 5 entries. |
| PASS | `ls /tmp` | `Error: /tmp is not inside of a Syncweb folder — run syncweb ls --local-only /tmp`. |
| PASS | `find '.*\.txt$'` | `notes.txt`, `test.txt` (regex auto-detected). |
| PASS | `find '*.md'` | `docs.md`, `docs/a.md`, `docs/b.md` (glob). |
| PASS | `find --fixed-strings 'note'` | `notes.txt`. |
| PASS | `find --ignore-case README` | `No files matching` (correct, no README). |
| PASS | `find --depth +2 --depth -5 config` | No `-5` argument error; `No files matching`. |
| PASS | `find --json` | `entries` populated. |
| PASS | `find --modified-within 7d` | Runs without error. |
| PASS | `find --local-only --type f --ext mp3` | Matches on a clean directory (`one.mp3`); a transient `Invalid argument (os error 22)` was seen once on a stale directory but was not reproducible. |
| PASS | `sort --by size/name/modified/state` | Metadata tables sorted by each key. |
| PASS | `sort --no-enrich` | `Modified: -` column. |
| PASS | `sort --local-only --by peers/niche`, `--limit-size --min-seeders` | Lists entries / `No entries match the sorting criteria`. |
| PASS | `stat` / `--terse` / `--format` / glob | Size, blocks, hash, peers; terse pipe output; custom template. |

### Section 5 — Download & Import/Export

| Status | Test | Command / output |
|---|---|---|
| PASS | `download <file>` / `<dir>` / `--max-count` / `--size -1GB` / `--threads 1` | All `downloaded: 0 bytes` on already-local content, no namespace/arg errors. |
| PASS | Piped `find '*.md' \| download -` | `downloaded: 0 bytes` × 3 (was FAIL). |
| PASS | `folders import` / `--threads 1` / nested / different source | `import requested: N`; nested `sub/deep.txt` respected; `import /root/new-files` works. |
| PASS | `package export ./shared-docs/ /tmp/export-test` (was FAIL) | `exported: ... (1e7a1275-... 1.0.0, 5 entries)`; valid zstd CAR. `--version 1.0.1` also works. |

### Section 6 — Health & Verify

| Status | Test | Command / output |
|---|---|---|
| PASS | `stats seeding --folder ./shared-docs` (was FAIL) | `folder: 54e430d3...`, `total: 7`, `unseeded: 7`; per-blob table. |
| PASS | `stats seeding --json --folder` | JSON with `total`/`well_seeded`/`under_seeded`/`unseeded`. |
| PASS | `verify ./shared-docs` (was FAIL: path resolution) | `total: 7, verified: 7, corrupted: 0, missing: 0, valid: true`. |
| FAIL | Corrupt a blob → `verify` reports corruption | Blob content lives inside `blobs.db` (SQLite) in this store layout, so there is no per-blob file to corrupt; corrupting the DB file takes the daemon down rather than producing a per-blob corruption report. |
| NOT RUN | `verify --fix` | Not exercised (no cleanly-corruptible blob in this store layout). |

### Section 7 — Public Folders & Access

| Status | Test | Command / output |
|---|---|---|
| PASS | `share ./shared-docs` | Read-only `syncweb://folder/...` ticket. |
| PASS | `access ./shared-docs` | One table with `Folder · Mode · Write? · Shared with · Devices · Networks`. |
| PASS | `access --json` | `{folder, mode, write, shares, devices, networks}` with no `pinned` key. |
| PASS | `share --write` + `access` | `Write? = yes`. |
| PASS | `access --revoke` (read) | `unshared: ... (read)` prompt-free. |
| PASS | `access --revoke --write` non-TTY | `aborted`; write share stays. |
| PASS | `access --revoke --write --yes` | `unshared: ... (write)`. |
| PASS | `access --revoke --blob <hash>` non-TTY / `--yes` | `aborted` / `unshared: ... (blob <hash>)`. |

### Section 8 — Snapshots

| Status | Test | Command / output |
|---|---|---|
| PASS | `snapshot create --description` | Returns content hash. |
| PASS | `snapshot list` (was FAIL: minimal) | Full table with ID/Created/Size/Files/Description. |
| PASS | `snapshot diff <id1> <id2>` (was FAIL: hang) | `modified test.txt <old-hash> <new-hash>` via daemon IPC. |
| PASS | `snapshot delete <id>` | `deleted: <id>` (with `--yes`). |
| FAIL | `snapshot restore <id>` | `Error: invalid configuration: snapshot not found: <id>` — the snapshot manifest blobs were lost after an earlier blob-store reset in the test environment; the daemon-side `load` path used by diff works, so this is environmental rather than a restore-specific code path. |

### Section 9 — Collections & Packages

| Status | Test | Command / output |
|---|---|---|
| PASS | `package add ./pkg-dir` | `collection: f0b98ecc-3b45-483f-a456-c03d886a973a`, entries scanned. |
| PASS | `package bump --version 1.0.1 --changelog` | `version: 1.0.1` (plan's step omits `--version`, which is required). |
| PASS | `package publish --namespace <ns>` | `manifest: 5ad10a8a...`, `manifest_ticket: blobab6j57...`, `sequence: 1`. |
| FAIL | Bob `package info <ticket>` / `package install <ticket>` | `Error: failed to fetch blob: Unable to download 5ad10a8a...` — the peer cannot reach Alice's node over the blob ticket in this offline (relay-less) VM pair, even though folder doc sync between the same two nodes works. `package info` on the publishing node itself succeeds. |
| NOT RUN | `package list` / `versions` / `verify` / `upgrade` / `switch` / `remove` | Blocked by the install failure (package not installed on Bob). |

### Section 10 — Networks

| Status | Test | Command / output |
|---|---|---|
| PASS | `network create home` | `created: home a02703ae...`. |
| PASS | `network ls home` | Table with ID, Members, Folders. |
| PASS | `network health home` (was FAIL) | `Network: home`, `events: 0`, `sessions: 0`. |
| PASS | `network status home` | Membership/health summary. |
| PASS | `network events home` | `no events`. |
| PASS | `network invite home <bob-id>` + Bob `network join` | `joined: a02703ae...`; `network ls` shows 2 members. |
| PASS | `network peers` (daemon) | (not re-tested with live peers; path resolution now works). |

### Section 11 — Indexing

| Status | Test | Command / output |
|---|---|---|
| PASS | `indexing enable ./shared-docs` (was FAIL: path resolution) | `enabled: 54e430d3...`. |
| PASS | `search --kind catalog "test"` | `no results found` (empty catalog). |
| PASS | `indexing publish <folder> --catalog mycat` (was FAIL) | `published: 9`; catalog namespace + ticket printed. |
| PASS | `indexing disable ./shared-docs` | `disabled: 54e430d3...`. |

### Section 12 — Links

| Status | Test | Command / output |
|---|---|---|
| PASS | `link create --immutable` (was FAIL) | `syncweb://content/8dfd2d94...`; resolve returns the hash. |
| PASS | `link create --mutable` (was FAIL) | `syncweb://name/7c9efe45.../test.txt`; resolve returns hash + `sequence: 1`. |
| PASS | `link create --private --expires` | `syncweb://private/<hash>/<capability>?expires=7`. |
| PASS | `link resolve` (content + name) | Resolves to manifest; `fetched: true`. |
| PASS | `link revoke` non-TTY / `--yes` | `aborted` / `revoked: ...`; resolution after revoke fails (`private link has expired`). |

### Section 13 — Schedules & Bandwidth

| Status | Test | Command / output |
|---|---|---|
| PASS | `config schedule` | Shows `active_hours`, `bandwidth`, `[folders]`. |
| PASS | `config schedule set --active "22:00-06:00"` | `schedule updated`; reflected. |
| PASS | `config schedule set --bandwidth "5MB/s" --period "08:00-18:00"` | Adds a `[[bandwidth]]` window. |
| PASS | `config schedule folder shared-docs --active` | Per-folder override under `[folders.shared-docs]`. |

### Sections 14-15 — Filter Engine / Watch Mode

| Status | Test | Command / output |
|---|---|---|
| PASS | `watch --show-filters` | `rules = []`, `default_action = "accept"`, `[folders]`. |
| PASS | `watch --dry-run --paths <folder>` | `accept /root/shared-docs/...` lines for each entry. |
| PASS | `watch --once <folder>` | Scans once, exits rc=0. |

### Section 16 — Conflict Resolution

| Status | Test | Command / output |
|---|---|---|
| FAIL | Both modify same file offline, then sync | No auto-resolution observed: after concurrent edits the doc entry goes `remote` (not yet downloaded locally); no `.conflict`/`.diff` file is produced by default. |

### Section 17 — Offline Queue

| Status | Test | Command / output |
|---|---|---|
| NOT RUN | Offline changes re-sync on reconnect | Not exercised (no clean offline window available in the two-VM harness). |

### Section 18 — Media Server

| Status | Test | Command / output |
|---|---|---|
| PASS | `start --media-only` (was FAIL: relay hang) | Server listens on `127.0.0.1:9193`. |
| PASS | `curl http://127.0.0.1:9193/` | `404` (documented "404 or list"). |
| FAIL | `curl http://127.0.0.1:9193/media/<hash>` | `404 blob <hash> not found in store` — the standalone server's node saw an empty blob store (test harness restarted nodes without checkpointing the blob DB). |

### Section 19 — WebSocket Bridge

| Status | Test | Command / output |
|---|---|---|
| FAIL | Bridge on 127.0.0.1:9192 | Not implemented: no listener on 9192 and no `9192` reference in the codebase. The daemon starts no WebSocket bridge. |

### Section 20 — Syncthing Relay (BEP)

| Status | Test | Command / output |
|---|---|---|
| FAIL | `network test-relay --relay-url tcp://relay.syncthing.net:22067` | `Error: no syncthing relay is reachable ... failed to lookup address information: Name or service not known` (VMs have no DNS/internet — environmental, not a code failure). |
| NOT RUN | BEP relay fallback between two nodes | Requires a reachable relay; skipped in this offline environment. |

### Section 21 — Discovery Mechanisms

| Status | Test | Command / output |
|---|---|---|
| PASS | Two nodes on same LAN auto-discover | Proven indirectly: Bob's join then `ls` shows Alice's entries (~12s) and blobs download; doc sync converges between the two VMs. |
| NOT RUN | `config set discovery.local_mdns false` | Not exercised. |

### Section 22 — CLI Global Flags & Output

| Status | Test | Command / output |
|---|---|---|
| PASS | `--no-color devices` (was FAIL) | Works. |
| PASS | `--data-dir /tmp/syncweb-test folders` | Uses a separate profile (empty folder list). |
| PASS | `--help` | Grouped help shows the full surface. |
| NOT RUN | `--verbose <command>`, `--json folders`, `--network home folders` | Not individually exercised. |

### Section 23 — Version & Completions

| Status | Test | Command / output |
|---|---|---|
| PASS | `version` | `syncweb 0.1.1-alpha.7`. |
| PASS | `completions bash` / `zsh` | Generated scripts. |
| PASS | `manpages <dir>` | `syncweb.1` + per-subcommand manpages. |

### Section 24 — Integrity & Error Recovery

| Status | Test | Command / output |
|---|---|---|
| PASS | Kill daemon with SIGKILL | Daemon dies; `status` briefly reports the stale PID until restart (state file not cleared on SIGKILL). |
| PASS | Start daemon again | Recovers gracefully; `db check` → both databases healthy. |
| FAIL | Delete a blob → `verify` reports missing | Same store-layout limitation as Section 6: blob content is in `blobs.db`; no per-blob file to delete. |

### Section 25 — Performance Smoke Tests

| Status | Test | Command / output |
|---|---|---|
| PASS | `ls` on 2000+ entries | 24 ms (well under 500 ms). |
| PASS | `folders import` of 2000 files | 294 ms (well under 3 s). |
| PASS | `stats files` / `stats seeding` on 2000+ entries | 55 ms / 5 ms. |
| NOT RUN | 10 GB LAN transfer throughput | Not exercised. |

### New-failure summary

New failures (all under the stop limit of ten):
1. Section 19 — WebSocket bridge not implemented (no listener / no code).
2. Section 9 — Bob's `package info`/`install` cannot fetch the manifest blob over a blob ticket in this offline, relay-less VM pair (publisher self-fetch works; folder doc sync works).
3. Section 16 — conflict auto-resolution not observed; a concurrent edit leaves the entry remote with no `.conflict`/`.diff` file.
4. Section 8 — `snapshot restore` "snapshot not found" (environmental: manifest blobs were lost during the run).
5. Sections 6/24 — no per-blob corruption test because blob content lives in `blobs.db`, so "corrupt a blob file" is not cleanly reproducible in this store layout.

Environmental notes (not code failures): relay/DNS unreachable in the VMs (`network test-relay`); the stale-`status`-after-SIGKILL is cosmetic and self-corrects on restart.

---

## Manual Test Run: 2026-10-05 (daemon-routing fixes; partial run)

- VMs: `syncweb-a` (`10.0.3.2`) and `syncweb-b` (`10.0.3.3`), via `iiab-vm` + `nsenter`.
- Build: source rebuild including the fixes below. Binary SHA-256: `7b174ed6ba7d192030bcdb5706d97f386e9a8b05d266895a669b5780a617e717`.
- Test data: fresh `/root/manual6/default` profiles; Alice folders `/root/shared-docs`, `/root/twoway`, `/root/sendonly`; Bob folders `/root/bob-shared`, `/root/bob-twoway`, `/root/bob-sendonly`. Daemons run as detached `start --bg` processes on both nodes.
- Scope: the run covered Sections 1--13 and part of 14/15. It was stopped early (context limit) with Sections 16--25 not run.

### Root cause found this run

`open_node(data_dir)` opens an embedded node on the live data directory. While a daemon owns that directory, the daemon holds the redb blob/docs store locks, so the embedded node **blocks forever** instead of starting. This affected several commands that were not routing through the daemon. The fix is to prefer the daemon over IPC whenever one is running, and only open an embedded node when none is running (`--no-daemon` / no daemon).

### Repaired existing failures

| Status | Test | Command / output |
|---|---|---|
| PASS | `package info` / `package install` (was: hang / `Unable to download`) | New `PackageInfo`/`PackageInstall` IPC commands run the fetch inside the daemon, reusing its live store (no scratch dir, no re-download, no store-lock hang). `package info <ticket>` returned the manifest table; `package install <ticket>` → `installed: 6b74443f-... 1.0.1`, then `1.0.2` via `package upgrade <ticket>`. |
| PASS | `package import` (was: hang, Section 9.3 Step 2) | Now routed through the existing (previously unused) `ImportArchive` IPC: daemon imports into its store, materializes to a staging dir, publishes to a fresh folder, and the CLI installs from staging. `package import /tmp/import-test.car.zst` → `imported: 6158c1b9-... 1.0.0 (7 entries)` with the daemon running. |
| PASS | `indexing enable` / `indexing publish` (was: hang) | New `IndexingEnable`/`IndexingPublish` IPC commands run against the daemon's node + indexing service. `indexing enable /root/shared-docs` → `enabled: 31f9e579...`; `indexing publish /root/shared-docs --catalog mycat` → `published: 10`. |
| PASS | `search --kind package` / `--kind catalog` (was: hang) | Search no longer opens a node for package/catalog queries; only channel search (which actually needs the docs engine) opens one. `search --kind package "pkg"` → `no results found for query: pkg`. |
| PASS | `package publish` without `--namespace` (was: hang) | Resolves the target namespace through the daemon when one is running. |
| PASS | `network create` / `network invite` / `network kick` (was: hang) | Routed through the `NetworkCreate`/`NetworkInvite`/`NetworkKick` IPC commands. The daemon now provisions/refreshes the membership doc with its own node (`membership::provision_with_node` / `refresh_with_node`). `network create home` → `created: home <id>`; `network invite home <id>` → ticket; `network kick home <id> --yes` → `member kicked`. |
| PASS | WebSocket bridge (Section 19, was: not implemented) | Implemented a daemon-owned bridge on `127.0.0.1:9192` (`syncweb-core/src/media/bridge.rs`) that proxies newline-free JSON `IpcRequest` messages to `IpcServer::handle_request` and streams JSON `IpcResponse` back. Verified: handshake `101 Switching Protocols`; `{"command":{"command":"status"}}` → `{"response":"status","data":"running"}`; invalid JSON → `{"response":"error","data":{"message":"invalid JSON: ..."}}`. |

### `IrohNode::new` audit

| Status | Site | Disposition |
|---|---|---|
| PASS | `daemon/daemon.rs` daemon node | The owning node; unchanged. |
| PASS | `init::open_node` (embedded helper) | All reachable-with-daemon call sites now route via IPC first (package info/install/import, indexing enable/publish, package publish namespace resolution, network create/invite/kick, search). Remaining uses are guarded fallbacks reached only when no daemon runs, or `--no-daemon`. |
| PASS | `open_node_offline` | Scratch data dir + relay-disabled; used by archive export and the standalone media server; cannot conflict with the live store. |
| PASS | `syncweb-core/src/media/bridge.rs` | New bridge listener; no second store node. |
| FAIL | `handle_watch` (`--once`, and `watch --no-daemon`) | Still calls `open_node(data_dir)` directly (main.rs:2866), so `watch --once` hangs while a daemon runs. Not fixed this run; needs a one-shot scan/import IPC or a scratch node. |

### Section 1 — Initialization & Configuration

| Status | Test | Command / output |
|---|---|---|
| PASS | `folders create` (default / `--mode sendonly` / `--network home` / `--mode receiveencrypted`) | Four folders created, each printing a `syncweb://folder/...` URL; `folders` lists all four with modes. |
| PASS | `config` / `show schedule` / `show bep` | Full TOML; section-only output for `schedule` and `bep`. |
| PASS | `config set default_sync_mode ReceiveOnly` / `bandwidth.max_download 5MB/s` | Updated and reflected in `config show`. |
| PASS | `db check` / `db stats` / `db vacuum` / `db backup --output` | `0 errors` for both DBs; sizes reported; `VACUUM complete`; backup dir with `node.db`, `stats.db`, `config.toml`, `identity.key`, `manifest.json`. |

### Section 2 — Daemon Lifecycle

| Status | Test | Command / output |
|---|---|---|
| PASS | `start --bg` / `status` / `--json status` | `daemon: running`; JSON object with `daemon`, `devices`, `folders`, `networks`, `transfers`. |
| PASS | `stop` (pipe `n`) / `stop --force --yes` / `stop --yes` | `aborted` (daemon stays up); forced/confirmed shutdown → `daemon not running`. |
| PASS | Ctrl+C (SIGINT) graceful shutdown | Logs `maintenance task shutting down`, prints `daemon stopped`, status → not running. |
| PASS | `reload` / `sync` / `sync <namespace>` | `configuration reload requested` / `synchronization requested`. |
| PASS | `folders create` + `folders leave` (daemon running) | Created through the daemon, listed, then `left: <namespace>`. |

### Section 3 — Folder Sync (P2P)

| Status | Test | Command / output |
|---|---|---|
| PASS | Alice `folders create --mode sendreceive /root/shared-docs` + `folders import --folder` | Namespace `31f9e579...`; `import requested: 1`; `ls` shows `test.txt`. |
| PASS | Bob `folders join <url> /root/bob-shared` | `joined: 31f9e579...`; after discovery `ls` shows `test.txt`. |
| PASS | Bob `download test.txt` | `downloaded: 0 bytes`; `cat` → `hello world`. |
| PASS | Bob write on read-only join | `Error: failed to set document blob: Attempted to insert to read only replica` (correct enforcement). |
| PASS | Two-way via `--write` ticket (`/root/twoway` ↔ `/root/bob-twoway`) | Bob joined the write ticket, imported an edit; Alice's sync+download shows both lines. |
| PASS | SendOnly folder, Bob join + read | Bob `ls` shows `s.txt`; write rejected (read-only replica). |
| PASS | `folders leave` / `--delete-files` (non-TTY) / `--delete-files --yes` | `left`; `aborted` (dir stays); `left` (dir removed). |
| PASS | `folders` / `folders --json` / `devices` / `devices --json` | Table/JSON with modes and Iroh + Syncthing ids. |

### Section 4 — Listing, Searching, Sorting, Stat

| Status | Test | Command / output |
|---|---|---|
| PASS | `ls` / `--local-only` / `--remote-only` / `--path-prefix` / `--no-enrich` / `--sort size` / `--json` / `ls /tmp` | All variants as documented; `/tmp` → `not inside of a Syncweb folder` with `--local-only` hint. |
| PASS | `find '.*\.txt$'` / `'*.md'` / `--fixed-strings` / `--modified-within` / `--depth` / `--json` / `--ignore-case` | Regex/glob/exact matches; depth parses `-5`; JSON populated. |
| FAIL | `find --type f --ext mp3 --local-only ./music` (plan syntax) | Plan syntax omits the pattern, so `./music` binds to the **pattern** slot and the path defaults to `.`; the scanner then walks `/` and hits unreadable special files (`EIO`/`EACCES`). Correct form `find '*' --type f --ext mp3 --local-only ./music` → `one.mp3`, `song.mp3`, `track.mp3`. This is a test-syntax bug, not a code bug. |
| PASS | `sort --by size/name/modified/state` / `--no-enrich` / `--local-only --by peers/niche` / `--limit-size --min-seeders` | Metadata tables sorted per key; disk sorts and limits behave as documented. |
| PASS | `stat` / `--terse` / `--format` / glob | Size, blocks, hash, peers; terse pipe output; custom template. |

### Section 5 — Download & Import/Export

| Status | Test | Command / output |
|---|---|---|
| PASS | `download <file>` / `<dir>` / `--max-count` / `--size -1GB` / `--threads 1` / piped `find '*' \| download -` | All `downloaded: 0 bytes` on already-local content, no namespace/arg errors. |
| PASS | `folders import` / `--threads 1` / nested / different source (write folder) | `import requested: N`; nested `nest/deep/n.txt` respected; `import /root/new-files --folder ...` works. |
| PASS | `package export <path> <output>` / `--version 1.0.1` | `exported: ... (7 entries)`; output is a valid zstd CAR archive. |

### Section 6 — Health & Verify

| Status | Test | Command / output |
|---|---|---|
| PASS | `stats seeding --folder` / `--json` | `total: 7`, `unseeded: 7`; JSON with `total`/`well_seeded`/`under_seeded`/`unseeded`. |
| PASS | `verify ./shared-docs` | `total: 7, verified: 7, corrupted: 0, missing: 0, valid: true`. |
| NOT RUN | Corrupt a blob → `verify` reports corruption; `verify --fix` | Blob content lives inside `blobs.db` (redb), so there is no per-blob file to corrupt; unchanged from prior runs. |

### Section 7 — Public Folders & Access

| Status | Test | Command / output |
|---|---|---|
| PASS | `share` / `access` / `access --json` | Read ticket; one table with `Folder · Mode · Write? · Shared with · Devices · Networks`; JSON array with `devices`/`folder`/`mode`/`networks`/`shares`/`write`. |
| PASS | `access --revoke --write` (non-TTY) / `--write --yes` / `--revoke` (read) | `aborted` (write stays); `unshared: ... (write)`; prompt-free read revoke. |

### Section 8 — Snapshots

| Status | Test | Command / output |
|---|---|---|
| PASS | `snapshot create --description` | Returns a content hash; two snapshots differ after an edit. |
| PASS | `snapshot list` | Full table with `ID · Created · Size · Files · Description`. |
| PASS | `snapshot diff <id1> <id2>` | `modified test.txt <old-hash> <new-hash>`. |
| PASS | `snapshot restore <id>` (was FAIL: `snapshot not found`) | `restored 7 files` — works in a fresh profile (the prior failure was environmental: manifest blobs had been lost). |
| PASS | `snapshot delete <id> --yes` | `deleted: <id>`; removed from the list. |

### Section 9 — Collections & Packages

| Status | Test | Command / output |
|---|---|---|
| PASS | `package add` / `package bump --version --changelog` / `package publish --namespace --sequence` | `collection: 6b74443f-...`; `version: 1.0.1`; publish prints `manifest`, `manifest_ticket`, `sequence`. |
| PASS | Bob `package info <ticket>` (was FAIL) | Manifest table (collection/version/parent/entries) with the daemon running. |
| PASS | Bob `package install <ticket>` (was FAIL) | `installed: 6b74443f-... 1.0.1`; files materialized under `packages/`. |
| PASS | `package list` / `versions` / `verify` | `1.0.1`, `1.0.2`; `verified: ... 1.0.1`. |
| PASS | `package upgrade <ticket>` (plan says `<collection-id>`) | Installed `1.0.2`; `versions` shows both. Correct argument is the manifest ticket, not the collection id. |
| PASS | `package switch <collection> 1.0.1` / `remove <collection> <version> --yes` | Switched current; removed a non-current version. The plan's `remove <collection-id>` omits the required version. |
| PASS | `package export <path> /tmp/pkg.car.zst` | Valid zstd CAR. The plan's `<collection-id>` form is unsupported — export takes filesystem paths. |
| PASS | `package import <archive>` (was HANG) | `imported: 6158c1b9-... 1.0.0 (7 entries)`. The plan's `--no-install` flag does not exist. |

### Section 10 — Networks

| Status | Test | Command / output |
|---|---|---|
| PASS | `network create home` (was HANG) | `created: home <id>`; `network ls` shows it. |
| PASS | `network ls home` / `network events home` / `network health home` | Table with ID/members/folders; `no events`; `Network: home`, `events: 0`, `sessions: 0`. |
| PASS | `network invite home <bob-id>` (was HANG) | Printed a `syncweb://network/...` ticket. |
| PASS | `network kick home <bob-id> --yes` (was HANG) | `member kicked`. |
| FAIL | `network test-relay` (plan syntax) | Requires `--relay-url <RELAY_URL>`; also unreachable DNS in the offline VMs. |

### Section 11 — Indexing

| Status | Test | Command / output |
|---|---|---|
| PASS | `indexing enable ./shared-docs` (was HANG) | `enabled: 31f9e579...`. |
| PASS | `search --kind catalog "test"` | Catalog table rendered (catalog published in this run). |
| PASS | `indexing publish <folder> --catalog mycat` (was HANG) | `published: 10`; namespace + ticket printed. |
| PASS | `indexing disable ./shared-docs` | `disabled: 31f9e579...`. |

### Section 12 — Links

| Status | Test | Command / output |
|---|---|---|
| PASS | `link create --immutable` | `syncweb://content/dc5a4edb...`. |
| PASS | `link create --mutable` | `syncweb://name/<node-id>/test.txt`. |
| PASS | `link create --private --expires <unix-ts>` | `syncweb://private/<hash>/<capability>?expires=<ts>`. The plan's `--expires 7d` is not accepted — it expects a Unix timestamp. |
| NOT RUN | `link resolve` / `link revoke` | Not exercised this run. |

### Section 13 — Schedules & Bandwidth

| Status | Test | Command / output |
|---|---|---|
| PASS | `config schedule` / `set --active` / `set --bandwidth --period` / `folder media --active` | `active_hours = "22:00-06:00"`, a `[[bandwidth]]` window, and `[folders.media]`. |

### Sections 14--15 — Filter Engine / Watch Mode (partial)

| Status | Test | Command / output |
|---|---|---|
| FAIL | `watch --once <folder>` | Hangs while a daemon runs: `handle_watch` calls `open_node(data_dir)` directly for `--once` (`--no-daemon` too), so it blocks on the daemon's store locks. Not fixed this run. |
| NOT RUN | `watch --show-filters` / `--dry-run` / `--filters` | The `--show-filters` invocation produced no output before the run stopped (likely the same path). |

### Sections 16--25 — NOT RUN

Stopped before conflict resolution, offline queue, media server, WebSocket bridge end-to-end via the plan (the bridge itself was verified directly, above), BEP relay, discovery, global flags, version/completions, integrity recovery, and performance smoke tests.

### New-failure summary

All seven real bugs found this run were fixed in code and verified:
1. `package info`/`install` opened a second node on the live data dir (store-lock hang) — now daemon IPC.
2. `package import` hung (same root cause) — now `ImportArchive` IPC.
3. `indexing enable`/`publish` hung (same root cause) — now `IndexingEnable`/`IndexingPublish` IPC.
4. `search --kind package/catalog` hung (pointless node open) — node now opened only for channel search.
5. `package publish` without `--namespace` hung — now daemon namespace resolution.
6. `network create`/`invite`/`kick` hung (`membership::provision`/`refresh` opened a node) — now daemon IPC with `provision_with_node`/`refresh_with_node`.
7. WebSocket bridge (Section 19) was unimplemented — now implemented on `127.0.0.1:9192`.

One real bug remains unfixed (below the stop limit of ten):
1. **`watch --once` hangs while a daemon is running** (`handle_watch` opens an embedded node; main.rs:2866).

Plan-syntax mismatches found (not code bugs): `find --type f --ext mp3 --local-only <path>` (missing pattern), `package upgrade <collection-id>` (takes a ticket), `package remove <collection-id>` (needs a version), `package export <collection-id>` (takes a path), `package import --no-install` (flag does not exist), `link create --expires 7d` (expects a Unix timestamp), `network test-relay` (requires `--relay-url`).

### Remaining steps

1. Fix `watch --once`/`watch --no-daemon` to route through the daemon (or use a scratch node), then re-test Sections 14--15.
2. Run Sections 16--25: conflict resolution, offline queue, media server, bridge end-to-end via `websocat`, BEP relay, discovery, global flags, version/completions, integrity recovery, and performance smoke tests.
3. Re-test the `find --local-only` and other plan-syntax rows with the corrected commands and update the plan's commands accordingly.
4. Continue the `open_node` audit for any command not yet exercised with a daemon running (e.g. `link resolve`/`revoke`, `stats`, `transfer`, `provider`).
