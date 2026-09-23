# Plan 01 — Metadata-first `ls`/`find`/`sort`: the doc index is the source; the filesystem only enriches (Python-impl parity)

Priority: HIGH · Status: Draft · Owner: `syncweb-cli`
Depends on: — · Serves story #4 (Ari: browse a huge remote library before
fetching) and cross-cutting theme #2 ("lazy fetch surprises everyone") —
the lazy-listing escape hatch that makes eager `join` (plan 02) safe

## Goal

Restore the documented "List doc entries (lazy)" behavior
(`docs/commands.md:35`, `docs/commands.md:477` — :35 describes `find`'s
metadata-only design; the "List doc entries (lazy)" `ls` row is at :477) and, in one move, the rest of
that promised family (`find`, `sort`). The original Python client is the
behavior spec: those commands read the **metadata index only** and the disk is
touched **at most to enrich a row** (real size/mtime for files already on
disk). Today the Rust CLI is the exact inverse — `ls`/`find`/`sort` are full
`ParallelScanner`/`FindEngine` directory walks that print nothing for a
joined-but-not-yet-downloaded folder (plan 02 makes that state the norm right
after `join`).

## Evidence (verified in code)

- **Python implementation is the behavior spec** (`../syncweb-py/`):
  - `syncweb/cmds/ls.py:95` `cmd_ls`; `path2fid` (ls.py:9-20) resolves a path
    against each configured folder's mount root and aborts with
    `"X is not inside of a Syncweb folder"` otherwise; the listing data comes
    from Syncthing's `db/browse` + `db/file` metadata REST
    (`syncweb/syncthing.py:711-729`; the `files` reader is at :711-718, the
    `file` metadata reader at :720-729), never a disk walk; the long form's
    Size/Modified come from that metadata.
  - `syncweb/cmds/find.py:179` `cmd_find` (:224 `args.st.files(...)`) and
    `syncweb/cmds/sort.py:164` `cmd_sort` are likewise `db/browse` reads.
    `stat.py` too. Nothing smaller than a single fstat touches the disk.
- **Rust today is the opposite:** `handle_ls` (syncweb-cli/src/main.rs:4298)
  always scans via `ParallelScanner` (main.rs:4317); `handle_find`
  (main.rs:4336-4410) and `handle_sort` (main.rs:4449+) scan disk and never
  consult the folder's doc entries.
- **The lazy source of truth already exists:** `folder.list_entries()`
  (doc entries, no blob fetch) — used today only inside
  `download_joined_folder` (main.rs:2546) and by the daemon's join-download
  handler (ipc.rs:1235). `folder.has_local(hash)` (syncweb_folder.rs:116)
  decides local/remote without a scan.
- **Folder mount paths are already persisted**, so Python-style path→folder
  resolution is a small addition, not a new protocol:
  - `FolderStatusReport.path` (syncweb-core/src/daemon/state.rs:82) is
    persisted in `folder_status_reports` (node_db.rs:555, loaded at
    node_db.rs:640-655) and served over IPC by `FolderList` (ipc.rs:662-665).
  - **Gap:** the reports are written only by the daemon status save; embedded
    `--no-daemon` nodes and stale daemon data aren't reliable, and
    `SyncwebFolder` does not carry its mount path (`path()` → `None`,
    syncweb_folder.rs:282-284). One core addition is therefore in scope: a
    namespace→mount-path registry written at create/join in both modes — the
    Rust analogue of Python's folder `"path"` field surfaced by Syncthing's
    `config/folders` API.
- Existing resolver plumbing handles the "selector is a namespace id" case:
  `manager.resolve(path)` (manager.rs:276, already used by `stats files`,
  main.rs:1954) and `manager.resolve_namespace` (manager.rs:287, used at
  main.rs:2602/2653/2755/2876). The new registry handles the "selector is a
  path inside a mount root" case that `resolve_namespace` cannot
  (manager.rs:287-297 matches a namespace id or the sole folder only).
- Filter plumbing exists to reuse: `ContentFilter` (syncweb-cli/src/cli/filter.rs:9)
  provides path-prefix/glob for the listing; `FindQuery`
  (syncweb-core/src/search.rs:29) provides pattern/size/depth/time/type for
  find. Plan 03 lands the unified `ContentFilterArgs`; this plan uses only
  today's `ContentFilter` (path-prefix/glob, with plan 03's `--path-glob`
  spelling from day one) and defers the depth/ext/size predicates to plan 03.

## Scope guard

- Changes `ls`, **`find`, and `sort`** to metadata-first via **one shared
  resolver + one shared entry-listing helper** guarded by `--local-only`
  (below), which preserves today's exact disk-scan code path for scripts.
- Two core additions permitted: a **folder mount-path registry** (new node_db
  rows upserted at create/join in embedded and daemon modes; read by the
  resolver) and **one read-only IPC command** (`ListEntries { folder_selection }
  -> Vec<EntryRow>`, where `EntryRow = {path, hash, size, local, modified?}`,
  mirroring the existing `IpcCommand::StatsFiles` pattern at
  ipc.rs:133/782). The IPC is required because in daemon-connected runs the CLI
  has no `SyncwebFolder` *or* blob store: the daemon owns the node, and opening
  a second embedded node on the same `data_dir` conflicts (endpoint binds the
  same node identity; `Docs::persistent` store is single-instance). The IPC
  response carries the daemon-computed `local` flag (`has_local`) and, when the
  file is local, the stat-enriched `size`/`modified`; the CLI only renders. The
  CLI therefore lists entries over IPC when a daemon is connected and via
  `folder.list_entries()`/`has_local()` when embedded — reads never scan the
  disk in either mode.
- `download`/`verify` keep their current behavior; their `--remote-only` and
  filter unification arrive in plan 03.

## Steps

### 1. Shared metadata listing helper: `print_folder_entries`

- Extract the doc-metadata walk from the `join --download-all` flow
  (main.rs:2546-2560) **minus its disk-write step** into a reusable function
  (must be `async` — `list_entries`/`has_local` are async, so `handle_ls`
  main.rs:4298, `handle_find` main.rs:4336, and `handle_sort` main.rs:4449
  all become `async fn`):
  - Signature:
    `async fn print_folder_entries(folder: &SyncwebFolder, mount_root: &Path, filter: &ContentFilter, output_json: bool) -> Result<()>`.
    `mount_root` comes from the resolver (step 2) because `SyncwebFolder`
    carries no mount path (`path()` → `None`, syncweb_folder.rs:282-284).
    `find`/`sort` do **not** call this helper as-is: they share its entry-walk
    + stat-enrichment + envelope-printing core but apply a `FindQuery`
    predicate (find) or a table sort (sort) before printing — refactor the
    walk into `enumerate_folder_entries(folder, mount_root) -> Vec<LocalEntry>`
    (returns the stat-enriched rows) plus one small `print_entries(rows,
    output_json)`; `ls` = enumerate + print, `find` = enumerate + filter +
    print, `sort` = enumerate + sort + print.
  - The walk calls `blob_store.export_to_path(entry.hash, …)`
    (main.rs:2553-2558); the listing must **never materialize blobs** — it
    only checks `folder.has_local(hash)` (syncweb_folder.rs:116) to set
    `State`. (`folder` carries the blob store, so no extra
    `manager`/`node` param.)
  - **Disk is consulted only to enrich:** for entries where `has_local` is
    true, `std::fs::metadata(mount_root.join(entry.path))` overlays the real
    file `size` (byte count differs from blob metadata) and `modified` time on
    the row when present. This is a per-entry `stat`, **not** a directory
    scan. If the stat fails (file vanished between check and read), fall back
    to the doc metadata silently.
  - Columns (human): `Path`, `Size`, `Modified`, `State` (`local` if the blob
    is in the blob store, `remote` otherwise). `Modified` is available only for
    local rows (from the disk stat); the Rust doc index (`EntryLike`) carries no
    mtime, so remote rows print `-` for `Modified` — unlike Python's Syncthing
    `db/file` metadata, which does include it.
  - JSON: `{folder, path, entries: [{path, size, hash, local, modified?}]}`
    (plan 08 single-object envelope; bare array today at main.rs:4322-4327).
  - **Peer availability is deliberately absent.** Doc entries carry only
    `path/hash/size` (`EntryLike`, folder/public_subscription.rs:11) and the
    CLI has no client-side per-blob peer-count IPC (the daemon's `EnrichSort`
    peer map is always empty, ipc.rs:1467 — `sort --enrich` already degrades
    gracefully to metadata fields). Surfacing `peers`/% seeded needs a new IPC
    surface — out of this plan's scope; **filed as plan 09** (`network peers`).
- File: syncweb-cli/src/main.rs.

### 2. Python-style path→folder resolution: `resolve_selector_to_folder`

- Add `fn resolve_selector_to_folder(ctx, selector: &Path) -> Result<(NamespaceId, PathBuf, RelativePath)>`
  — the resolved namespace id, its mount root, and the selector's relative
  remainder (the mount root is needed for enrichment because
  `SyncwebFolder.path()` is `None`, syncweb_folder.rs:282-284). The tuple
  keeps both listing modes representable: embedded callers turn the id into a
  `SyncwebFolder` via `FolderManager::get(namespace_id)`;
  daemon-connected callers send `ListEntries { folder_selection: <id> }`
  without ever needing a local `SyncwebFolder`:
  1. **Namespace id:** `manager.resolve(selector)` (manager.rs:276) →
     wholesale listing, prefix empty. (Covers the `stats files` path and the
     existing `resolve_namespace` callers.)
  2. **Mount-path prefix match** (Python `path2fid`, ls.py:9-20): canonicalize
     the selector (`fs::canonicalize` where it exists on disk; otherwise the
     resolved absolute path), find the registered folder whose mount root is
     the longest ancestor prefix, return that folder + the relative remainder.
     Compare against `realpath`-ed roots so `..`/symlink tricks don't
     misresolve.
  3. **Neither** → a clear error (see Risks for the behavior flip):
     ```
     Error: <path> is not inside of a Syncweb folder —
     run `syncweb ls --local-only <path>` to list the disk directly
     ```
     (Message mirrors Python's `"X is not inside of a Syncweb folder"`
     ls.py:13 — where Python logs and continues, we **error and exit**; the
     Python client walks multiple folders and keeps going, the CLI is
     single-selector. See Risks.)
- **Registry (a core change):** add a `folder_mounts(namespace_id TEXT
  PRIMARY KEY, path TEXT NOT NULL, updated_at)` table to
  `syncweb-core/src/storage/node_db.rs` (alongside `folder_status_reports`,
  node_db.rs:210/555), upserted by the CLI in **both** embedded and daemon
  paths from the folder's actual mount dir at `create`/`join` (there is no
  `accept` verb — see plan 07); a `load_folder_mounts()` helper for the
  resolver. Registry rows are advisory — the resolver filters by live
  `FolderManager` membership at call time, so stale rows never resolve to a
  departed folder. In daemon-connected runs the resolver additionally seeds
  from `FolderList` paths (ipc.rs:662-665) so a `syncweb ls <path>` sees paths
  the daemon knows but the local registry lacks.
- **Daemon-connected listing:** the resolver's `print_folder_entries` caller
  picks the entry source by mode — embedded opens `open_node(data_dir)` and
  calls `folder.list_entries()` + `has_local` + the per-entry stat itself;
  daemon-connected sends the new `ListEntries` IPC (scope guard) and renders
  the response rows directly (the daemon computes `local`/enriched size/mtime
  there, since the CLI has no blob store in that mode). Selector resolution
  follows the same split: embedded uses `manager.resolve`/`resolve_namespace` +
  the mount registry; daemon-connected uses the existing
  `resolve_namespace_via_daemon` (main.rs:672, which matches via
  `IpcCommand::ListFolders` — path equality, namespace prefix, or the
  sole-folder fallback) and `FolderList` `path` fields as the mount roots for
  the longest-prefix step. Do **not** open a second embedded node while a daemon
  runs (store/endpoint conflict).
- **Removes yesterday's single-folder hack:** the old draft's "on a
  single-folder node every selector resolves to the sole folder" caveat is
  obsolete — `ls <folder-dir>` now resolves on multi-folder nodes via the
  mount registry, and unrelated local paths no longer leak into remote
  listings.

### 3. Flags

- Extend `LocalPathArgs` (commands.rs:347-358; today only
  `path`/`sort`/`threads`):
  - `--remote-only`: show only `State == remote` rows.
  - `--local-only`: force the current `ParallelScanner` disk path for the
    selector (works on **any** path, even outside a folder; bypasses step 2's
    error). This is today's default behavior preserved under an explicit flag.
    `--threads` is meaningful only here (metadata path never parallel-scans).
  - `--path-prefix` / `--path-glob` (hidden `--glob` alias): filter the entry
    list via the existing `ContentFilter`, using plan 03's renamed spelling
    from day one (`--glob → --path-glob`, plan 03 step 5). The
    depth/ext/size/type vocabulary lands later with plan 03's shared
    `ContentFilterArgs`, not as one-off flags.
  - `--no-enrich`: skip even the per-entry `stat` (pure metadata listing).
  - `--sort <by>`: on a resolved folder, sorts the metadata table
    (name/size/modified/state) instead of early-dispatching to `handle_sort`
    (main.rs:4300) — see step 5. Note the value set is a new, smaller
    vocabulary (`name`/`size`/`modified`/`state`), not `handle_sort`'s
    `--by` set (niche/frecency/peers/…); the flag name is shared but the two
    modes accept different values, so document the split in the man page.
    **Validation rule to implement:** under `--local-only` the value must
    parse in `handle_sort`'s `--by` set (else that handler errors today);
    on a resolved folder it must be one of `name`/`size`/`modified`/`state`
    (reject the others with a clear message). Sorting by `modified` on remote
    rows is undefined — they print `-` — so `modified` falls back to doc
    `size` for remote rows and the man page says so.
- Files: syncweb-cli/src/main.rs, syncweb-cli/src/cli/commands.rs.

### 4. Guard the no-metadata state

- When the folder has no local doc entries yet (joined, subscribe not yet
  ingressed), print a clear nudge instead of an empty table:
  `"folder <ns> has no remote entries yet; files will appear as they sync —
  run \`syncweb download-all\` to fetch current content"`.

### 5. Port `find` and `sort` onto the same metadata path

- `handle_find` (main.rs:4336-4410): resolve via step 2; build the `FindQuery`
  exactly as today but run it over the **entry list** (pattern against the
  entry `path`, honor `--full-path`, size/time/depth/ext/type predicates from
  `EntryLike` metadata) instead of over scanned disk entries; same stat
  enrichment; `--local-only` keeps the disk scan (including outside-folder
  paths); `--remote-only` predicate applies too. Python parity: `cmd_find`
  searches `db/browse` children, find.py:224-230.
- `handle_sort` (main.rs:4449-4475): sort the metadata table (by doc size or
  stat-enriched size/time when local); `--enrich` degrades to metadata
  fields until the IPC peer map exists (already the case, ipc.rs:1467).
- `handle_ls --sort` (main.rs:4300 early dispatch): only when selection does
  **not** resolve to a folder — i.e. under `--local-only` — keep routing to
  `handle_sort`. On a resolved folder, sort the table in place.
- Files: syncweb-cli/src/main.rs.

### 6. Wire `--json`

- `ls`/`find`/`sort` all emit the step-1 envelope `{folder, path,
  entries:[{path, size, hash, local, modified?}]}`; `folder` = resolved
  namespace id, `path` = selector as typed. Stable shape covered by an
  assertion (plan 08's single-envelope contract; one object per command,
  arrays only under a named key).
- **Verification note:** `ls --json` today emits a **bare array of paths**
  (main.rs:4322-4327) and `find --json` does the same (main.rs:4425-4436).
  This plan switches both to the envelope: a deliberate, **breaking contract
  change** overriding index principle #2's "additive only" in this one case
  (plan 08's envelope). Flag it in the changelog; the revert path is in Risks.

## Tests (name the file + behavior)

- `syncweb-cli/tests/workflow_test.rs` — after `join --no-download` (plan 02
  flips bare `join` to eager, so the lazy state needs the explicit opt-out):
  1. `syncweb ls <folder-dir>` lists remote entries **before any download**
     (assert a remote filename appears; the disk holds nothing). Python-parity
     core.
  2. `syncweb ls <plain-local-dir>` **errors** with "not inside of a Syncweb
     folder" — the deliberate flip (this is the old scan result's replacement).
  3. `syncweb ls --local-only <folder-dir>` prints nothing (files not on disk)
     — disk path preserved.
  4. `syncweb ls --local-only <plain-local-dir>` lists disk files exactly as
     today (no-folder scan regression).
  5. `syncweb find '*.md' <folder-dir>` returns remote entries from the
     metadata index; `--remote-only` narrows to undownloaded rows.
  6. `syncweb sort --by size <folder-dir>` returns the metadata table sorted;
     `--threads 8` is accepted but has no effect outside `--local-only`.
- `syncweb-cli/src/main.rs` `#[cfg(test)]` unit tests:
  - `print_folder_entries` JSON shape (`{folder, path, entries:[…]}`) and the
    `local` flip on `folder.has_local`, including the enrichment overlay when a
    local file's disk size/mtime differs from doc metadata.
  - `resolve_selector_to_folder`: namespace id → whole-folder; a path nested
    under a mount root → folder+prefix; outside every root → the error; a
    stale registry row filtered out by live `FolderManager` membership.
- `syncweb-cli/tests/daemon_integration_test.rs` (optional but encouraged):
  with a daemon running, `syncweb ls <ns>` shows remote entries via the new
  `ListEntries` IPC when the local mount registry is empty (no `--remote` flag
  exists; daemon-connected is the default when a daemon is up).

## Risks / rollback

- **Behavior flip (accepting it):** `ls`/`find`/`sort` on a path outside any
  folder moves from "list the disk" to a clear error, Python-style; disk
  listing survives only as `--local-only`. All existing scripts that used
  plain `ls <dir>` on a plain directory must add `--local-only`. This is the
  point of the plan (Python parity), so document it in the changelog and man
  pages (plan 07). Rollback = make the step-2 case-3 error a warn-and-scan;
  `--local-only` code path is unchanged either way.
- Both `--remote-only` and `--local-only` set → error out (`State` would be
  contradictory).
- Remote listing can lag disk reality for un-synced folders; `State` is always
  derived from the blob store (`has_local`), never from doc-entry presence.
  Verify with the step-1 unit test.
- Registry staleness is handled by live-membership filtering (step 2), not by
  trusting rows.
- Peer availability must not be invented from doc-entry counts (deferred).

## Handoff notes

- Do **not** regress the `ls --sort`→`handle_sort` dispatch for non-folder
  selectors (main.rs:4300); only folder selections move to the table sort.
- `FolderManager.create`/`join` call sites in main.rs:
  `handle_create` (main.rs:2280), `handle_join` (main.rs:2442),
  daemon join-download (ipc.rs:1235) — registry upsert goes where the folder's
  mount dir is known in **each** path (embedded + daemon). (There is no
  `accept` verb in the CLI — see plan 07 — so no accept-site upsert.)
- Man pages `syncweb-ls.1`, `syncweb-find.1`, `syncweb-sort.1` and
  completions regenerate after the flag change (plan 07's drift loop);
  `docs/commands.md:477-478` rows become accurate again (see plan 07).
- `cargo test --workspace`, `clippy`, `fmt` at the end (index "definition of
  done").