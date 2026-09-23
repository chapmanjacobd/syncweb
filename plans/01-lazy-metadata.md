# Plan 01 — Expose lazy metadata browsing: real remote-listing for joined folders

Priority: HIGH · Status: Draft · Owner: `syncweb-cli`
Depends on: — · Replaces story: #6 (Oli) and the "lazy fetch is hostile" theme

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
  `download_joined_folder` (main.rs:2546) during `join --download-all`.
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
    filter group (hash/path-prefix/glob). Plan 03 unifies these into one shared
    group; this plan only needs to *reuse* the depth/ext/size predicate, not
    unify the flags yet.

## Scope guard

- This plan changes **`ls` only**; `find` and `sort` keep their local-disk
  behavior for now (they get remoted as a follow-up once this approach proves
  out — see plan 03). No daemon protocol changes; no core changes.

## Steps

### 1. Detach lazy listing into `print_remote_entries`

- Extract the doc-metadata table path from the `join --download-all` flow into a
  reusable function:
  - Signature: `fn print_remote_entries(manager: &FolderManager, folder: &SyncwebFolder, filter: &ContentFilter, output_json: bool) -> Result<()>`, modeled on the entry walk at main.rs:2546-2560.
  - Columns (human): `Path`, `Size`, `Available` (peers/`%` seeded), `State`
    (`local` if blob present in the blob store, `remote` otherwise).
  - JSON: array of `{path, size, hash, available_on: [node_ids], local}`.
  - File: syncweb-cli/src/main.rs.

### 2. Make `ls` folder-aware

- In `handle_ls`, before falling back to the local scan:
  1. Try `manager.resolve(&command.path)` (manager.rs:276), or resolve the
     namespace if the path is a managed folder's mount point.
  2. If it resolves to a folder: call `print_remote_entries` (the doc path).
  3. If it doesn't resolve, keep the current local scan (unchanged behavior for
     ordinary directories).
- Add flags consistent with `find`:
  - `--remote-only` (only show entries not yet on disk)
  - `--local-only` (show only the current disk scan; equivalent to today's
    default — this preserves scripts that rely on `ls` = disk)
  - reuse the existing filter flags (depth/ext/size) to keep the flag
    vocabulary consistent (see plans 03 and 06).
- Files: syncweb-cli/src/main.rs, syncweb-cli/src/cli/commands.rs
  (`LocalPathArgs`, currently commands.rs:347-358).

### 3. Guard against the state where lazy metadata doesn't exist

- When the folder is joined but has no local doc entries yet (`list_entries`
  returns zero after subscribe has not ingressed), print a **clear nudge**:
  `"folder ... has no remote entries yet; files will appear as they sync — run
  \`syncweb download-all\` to fetch current content"` instead of printing an
  empty table header.

### 4. Wire `--json`

- The `--json` path must emit `{folder, path, entries: [...]}` and must be
  stable + `--json`-covered by an assertion (mirrors the
  `all_subcommands_are_categorized` test pattern in args.rs:205).

## Tests (name the file + behavior)

- `syncweb-cli/tests/workflow_test.rs` — after join **without** download:
  1. `syncweb ls <folder-dir>` lists remote entries (assert a remote filename
     appears).
  2. `syncweb ls --local-only <folder-dir>` prints nothing (files not on disk).
  3. `syncweb ls <plain-local-dir>` still lists disk files (no regression).
- `syncweb-cli/src/main.rs` `#[cfg(test)]` — a unit test on
  `print_remote_entries` JSON shape (Path/Size/Available/State).

## Risks / rollback

- If remote listing grows stale vs. disk (doc entries lag real files), the
  `State` column must show `local`/`remote` based on the blob store, not on
  doc-entry presence; verify with the test in step 2.
- Rollback: revert the `handle_ls` folder-awareness branch; `--local-only` and
  plain `ls` behavior are unchanged, so scripts keep working.

## Handoff notes

- Must not regress `syncweb ls --sort ...` (main.rs:4300 dispatch to
  `handle_sort`). Keep that path intact.
- Man page `syncweb-ls.1` and repl/completions regenerate after the flag change
  (see plan 07 for the doc-drift loop).
