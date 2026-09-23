# Plan 01 — Expose lazy metadata browsing: real remote-listing for joined folders

Priority: HIGH · Status: Draft · Owner: `syncweb-cli`
Depends on: — · Serves story #4 (Ari: browse a huge remote library before
fetching) and cross-cutting theme #2 ("lazy fetch surprises everyone") —
the lazy-listing escape hatch that makes eager `join` (plan 02) safe

## Goal

Give users a real, documented way to **see a joined folder's contents before
those files exist on disk**. Today `ls` is a local-disk scan, so a joined (but
not yet downloaded) folder prints nothing — the exact opposite of the promised
"doc metadata only — no blob download needed" story in `docs/commands.md:35`
and the "List doc entries (lazy)" row at `docs/commands.md:477`.

## Evidence (verified in code)

- `handle_ls` (syncweb-cli/src/main.rs:4298) always scans the **local
  filesystem** via `ParallelScanner` (main.rs:4317). It never consults the
  folder's doc entries.
- The lazy metadata source of truth **already exists**: joined folders expose
  `folder.list_entries()` (doc entries, no blob fetch) — used today only inside
  `download_joined_folder` (main.rs:2546) during `join --download-all` (and by
  the daemon's join-download handler, ipc.rs:1235).
- `FolderManager` is available in the same module as `handle_ls`; the
  namespace-resolution helpers already exist and require no new capabilities:
  - `manager.resolve(&path)` (syncweb-core/src/folder/manager.rs:276) — resolves
    a path selector to a `SyncwebFolder`; already used by `stats files`
    (main.rs:1954).
  - `manager.resolve_namespace(&str)` (manager.rs:287) — used at
    main.rs:2602/2653/2755/2876.
  - `manager.get(namespace)` — used at main.rs:2654/2758/2773.
- Filter plumbing already exists to reuse rather than reinvent:
  - `FindQuery` (syncweb-core/src/search.rs:29) supports glob/exact/regex
    pattern, extensions, size, depth, time, and file-type constraints; consumed
    by `handle_find` (main.rs:4338-4410).
  - `ContentFilter` (syncweb-cli/src/cli/filter.rs:9) is the download/verify
    filter group (hash/path-prefix/glob). Plan 03 unifies the filter vocabulary
    into one shared `ContentFilterArgs`; this plan reuses only the existing
    `ContentFilter` (path-prefix/glob) and defers the depth/ext/size predicates
    to plan 03 rather than adding one-off flags.

## Scope guard

- This plan changes **`ls` only**; `find` and `sort` keep their local-disk
  behavior for now (they get remoted as a follow-up once this approach proves
  out — see plan 03). No daemon protocol changes; no core changes.

## Steps

### 1. Detach lazy listing into `print_remote_entries`

- Extract the doc-metadata table path from the `join --download-all` flow into a
  reusable function:
  - Signature: `fn print_remote_entries(folder: &SyncwebFolder, filter: &ContentFilter, output_json: bool) -> Result<()>`, modeled on the entry walk at main.rs:2546-2560 **minus its disk-write step** — the walk calls `blob_store.export_to_path(entry.hash, …)` (main.rs:2553-2558); the listing must never materialize blobs, only check `folder.has_local(hash)`. (`folder` already carries the blob store, so `folder.has_local(hash)` (syncweb_folder.rs:116) decides `State`; no separate `manager`/`node` param is needed.)
  - Columns (human): `Path`, `Size`, `State` (`local` if the blob is present in
    the blob store, `remote` otherwise).
  - JSON: `{folder, path, entries: [{path, size, hash, local}]}` (see step 4 —
    one envelope per command, consistent with plan 08).
  - **Peer availability is deliberately absent.** Doc entries carry only
    `path/hash/size` (`EntryLike`, folder/public_subscription.rs:11) and the CLI
    has no client-side per-blob peer-count IPC today (the daemon's
    `EnrichSort` peer map is always empty, ipc.rs:1467). Surfacing `peers`/%
    seeded would need a new IPC surface — out of this plan's "no daemon/core
    changes" scope. Track it as a follow-up (plans 03/08).
  - File: syncweb-cli/src/main.rs.

### 2. Make `ls` folder-aware

- In `handle_ls`, before falling back to the local scan:
  1. Try `manager.resolve(&command.path)` (manager.rs:276) or
     `manager.resolve_namespace(selector)`.
  2. If it resolves to a folder: call `print_remote_entries` (the doc path).
  3. If it doesn't resolve, keep the current local scan (unchanged behavior for
     ordinary directories).
  - **Selector-resolution caveat:** `resolve_namespace` (manager.rs:287-297)
    matches only a namespace ID or, when there is exactly **one** managed
    folder, the sole folder — it does **not** match an arbitrary mount-point
    path, and `SyncwebFolder` does not track its mount path today
    (`path()` returns `None`, syncweb_folder.rs:282-284). So `ls <folder-dir>`
    resolves only when `<folder-dir>` is a namespace ID or the node has a
    single folder. Mapping an arbitrary managed path → folder requires folder
    path tracking, which is a core change out of scope here; note it as a
    follow-up rather than claiming path resolution works.
    **Consequence on single-folder nodes:** on a node with exactly one managed
    folder, *every* `ls <path>` first resolves to that folder's remote listing
    — including unrelated local paths. That is the intended "lazy wins" default;
    `--local-only` below is the documented escape hatch.
- Add flags:
  - `--remote-only` (only show entries not yet on disk)
  - `--local-only` (show only the current disk scan; equivalent to today's
    default — this preserves scripts that rely on `ls` = disk)
  - Filter flags on `ls` are **new** (there are none today — `LocalPathArgs`
    has only `path`/`sort`/`threads`, commands.rs:347-358). This plan adds
    `--path-prefix`/`--path-glob` (with hidden `--glob` alias), wired into
    `print_remote_entries` via the existing `ContentFilter` and using plan 03's
    renamed spelling from day one (`--glob` → `--path-glob`, step 5 of plan 03);
    the depth/ext/size/type vocabulary lands later with plan 03's shared
    `ContentFilterArgs`, not as one-off flags here.
- Files: syncweb-cli/src/main.rs, syncweb-cli/src/cli/commands.rs
  (`LocalPathArgs`, currently commands.rs:347-358).

### 3. Guard against the state where lazy metadata doesn't exist

- When the folder is joined but has no local doc entries yet (`list_entries`
  returns zero after subscribe has not ingressed), print a **clear nudge**:
  `"folder ... has no remote entries yet; files will appear as they sync — run
  \`syncweb download-all\` to fetch current content"` instead of printing an
  empty table header.

### 4. Wire `--json`

- The `--json` path emits `{folder, path, entries: [...]}` (the step-1 shape;
  `entries[i] = {path, size, hash, local}`) and must be stable +
  `--json`-covered by an assertion. Human output renders the same data as a
  table. Follows plan 08's single-envelope contract (one object per command;
  arrays only under a named key).
- `folder` = the resolved namespace id; `path` = the selector as typed.
- **Verification note:** `ls --json` today emits a **bare array of paths**
  (`—json` branch at main.rs:4322-4327) whenever `handle_ls` runs. For managed
  folders, this plan switches that branch to the envelope — a deliberate,
  **breaking contract change** (not additive), and the exact case index
  principle #2 ("additive only, must not regress") is intentionally overridden
  here with plan 08's envelope. Flag it in the changelog; the revert path is
  the note in Risks.

## Tests (name the file + behavior)

- `syncweb-cli/tests/workflow_test.rs` — after `join --no-download` (plan 02
  flips bare `join` to eager, so the lazy state requires the explicit opt-out):
  1. `syncweb ls <folder-dir>` lists remote entries (assert a remote filename
     appears).
  2. `syncweb ls --local-only <folder-dir>` prints nothing (files not on disk).
  3. `syncweb ls <plain-local-dir>` still lists disk files — **only on a node
     whose manager has zero or ≥2 managed folders** (a single-folder node
     resolves every path to its sole folder by design; see the step-2 caveat).
     Use the two-folder fixture in the workflow suite, not the happy-path world.
- `syncweb-cli/src/main.rs` `#[cfg(test)]` — a unit test on
  `print_remote_entries` JSON shape (`{folder, path, entries:[{path, size, hash,
  local}]}`), asserting `local` flips on `folder.has_local`.

## Risks / rollback

- If remote listing grows stale vs. disk (doc entries lag real files), the
  `State` column must show `local`/`remote` based on the blob store
  (`folder.has_local`), not on doc-entry presence; verify with the test in step
  2. Peer availability is deferred (no client-side IPC surface today — see step
  1) and must not be invented from doc-entry counts.
- Rollback: revert the `handle_ls` folder-awareness branch; `--local-only` and
  plain `ls` behavior are unchanged, so scripts keep working.

## Handoff notes

- Must not regress `syncweb ls --sort ...` (main.rs:4300 dispatch to
  `handle_sort`). Keep that path intact.
- Man page `syncweb-ls.1` and completions regenerate after the flag change
  (see plan 07 for the doc-drift loop).
