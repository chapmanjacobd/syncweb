# Plan 01 — Expose lazy metadata browsing: real remote-listing for joined folders

Priority: HIGH · Status: Draft · Owner: `syncweb-cli`
Depends on: — · Replaces story: #6 (Oli) and the "lazy fetch is hostile" theme

## Goal

Give users a real, documented way to **see a joined folder's contents before
those files exist on disk**. Today `ls` is a local-disk scan, so a joined
(but not yet downloaded) folder prints nothing — the exact opposite of the
promised "doc metadata only — no blob download needed" story in
`docs/commands.md:35`.

## Evidence (verified in code)

- `handle_ls` (syncweb-cli/src/main.rs:4298) always scans the **local
  filesystem** via `ParallelScanner`/`FindEngine`. It never consults the
  folder's doc entries.
- The lazy metadata source of truth **already exists**: joined folders expose
  `folder.list_entries()` (doc entries, no blob fetch) — used today only by
  `join --download-all` (main.rs:2546-2547) and `download` materialization.
- `FolderManager` is available at the same call site as `handle_ls`; the
  namespace-resolution helpers already exist:
  `manager.resolve_namespace(&selector)` (main.rs:4180) and `manager.get(ns)`
  (main.rs:2774). Enabling folder-awareness requires no new capabilities.
- `FindQuery`/`ContentFilter` filter plumbing already supports
  path/glob/ext/size/depth filters (syncweb-cli/src/cli/filter.rs), so filters
  can be reused rather than reinvented.

## Scope guard

- This plan changes **`ls` only**; `find` and `sort` keep their local-disk
  behavior for now (they get remoted in `01`'s follow-ups or a separate plan
  if the approach proves out). No daemon protocol changes; no core changes.

## Steps

### 1. Detach lazy listing into `handle_ls_remote`

- Extract the doc-metadata table path from the `join --download-all` flow into a
  reusable function:
  - Signature: `fn print_remote_entries(manager: &FolderManager, folder: &Folder, filter: &ContentFilter, output_json: bool) -> Result<()>`, modeled on the internal pattern at main.rs:2465-2546.
  - Columns (human): `Path`, `Size`, `Available` (peers/`%` seeded), `State`
    (`local` if blob present in blob_store, `remote` otherwise).
  - JSON: array of `{path, size, hash, available_on: [node_ids], local}`.
- File: syncweb-cli/src/main.rs.

### 2. Make `ls` folder-aware

- In `handle_ls`, before falling back to the local scan:
  1. Try `manager.resolve(&command.path)` (FolderManager), or resolve the
     namespace if the path is a managed folder's mount point.
  2. If it resolves: call `print_remote_entries` (the doc path).
  3. If it doesn't resolve, keep the current local scan (unchanged behavior for
     ordinary directories).
- Add flags consistent with `find`:
  - `--remote-only` (only show entries not yet on disk)
  - `--local-only` (show only the current disk scan; equivalent to today's
    default — this preserves scripts that rely on `ls` = disk)
  - reuse the existing filter flags (glob/ext/size/depth) to keep the flag
    vocabulary consistent (see plan 06).
- File: syncweb-cli/src/main.rs, syncweb-cli/src/cli/commands.rs.

### 3. Guard against the state where lazy metadata doesn't exist

- When the folder is joined but has no local doc entries yet (`list_entries`
  returns zero after subscribe has not ingressed), print a **clear nudge**:
  `"folder ... has no remote entries yet; files will appear as they sync — run
  \`syncweb download-all\` to fetch current content"` instead of printing an
  empty table header.

### 4. Wire `--json`

- The `--json` path must emit `{folder, path, entries: [...]}` and must be
  stable + `--json`-covered by an assertion (mirrors the
  `all_subcommands_are_categorized` test pattern in args.rs:204).

## Tests (name the file + behavior)

- `syncweb-cli/tests/workflow_test.rs` — after join **without** download:
  1. `syncweb ls <folder-dir>` lists remote entries (assert a remote filename
     appears).
  2. `syncweb ls --local-only <folder-dir>` prints nothing (files not on disk).
  3. `syncweb ls <plain-local-dir>` still lists disk files (no regression).
- `syncweb-cli/src/cli/indexing.rs` `#[cfg(test)]` — a unit test on
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
</content>
