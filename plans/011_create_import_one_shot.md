# Plan 011 — One-shot folder setup (`create` + `import`) and namespace ceremony

## Overview

Every "stand up a synced folder" flow in the test suite is two commands:

```sh
syncweb create <dir>          # namespace + ticket + share_url
syncweb import <dir>          # scan + hash + add entries
```

The workflow tests never `create` a non-empty directory and get its contents indexed — `create`
only provisions the namespace/doc; content ingestion is always a separate `import` (see
`workflow/basic_sync.rs` `write_import_find_stat`, `multiple_files_workflow`; the mirror test in
`workflow/indexing.rs` needs `network create` → `create --network` → `import` → `mirror`).

A related pain: many commands (`publish collection`, `publish blob`, `publish catalog`,
`stats files`, `stats seeding`, `verify`, `daemon-sync`, `leave`) accept a *namespace ID* that a
user can only obtain by running `create` and copy-pasting the ID.

## Goal

Model the CLI on git's plumbing/porcelain split: keep the low-level verbs (`create`, `import`,
`join`, `download`) for scripting, but make the common one-shot flows single commands, and let
folder *paths* be accepted anywhere a namespace ID is expected.

## Current state

- `FolderCreate { path, mode, relay_fallback, network }` — `commands.rs:259`; handler
  `handle_create` provisions a namespace/doc/ticket but does not scan content.
- `ImportArgs { path, folder, threads, enrich }` — `commands.rs:536`; `handle_import` scans into
  an existing folder.
- `resolve_folder` / `resolve_namespace_via_daemon` (`main.rs`) already translate a path →
  namespace in several places, but not uniformly.

## Open questions / alternatives

| # | Question | Alternatives |
|---|----------|--------------|
| Q1 | Should `create` scan an already-populated directory? | (a) `create` gains `--import` / `--scan` and ingests a non-empty dir in one shot; (b) leave `create` as pure namespace provisioning; (c) `create` always scans, no flag. |
| Q2 | Should `import` auto-create the folder when the path isn't a known namespace? | (a) yes (`import` becomes create-or-import, like `git init` + `git add`); (b) keep explicit. |
| Q3 | Add a porcelain verb? | (a) `syncweb add <paths...>` = create-or-reuse + import; (b) rely only on Q1/Q2 flags. |
| Q4 | Path selectors everywhere? | (a) accept a folder path anywhere a namespace ID is accepted (reuse `resolve_folder`); (b) keep IDs only. |
| Q5 | Symmetry for receiving | `join` already live-syncs via `--subscribe`; should `join` also accept `--prefix/--glob/--max-*` materialization in one step? |

## Recommendation

- Q1/Q2: make `create` ingest existing content (one-shot), and make `import` on an unknown path
  create the folder. Keep `--no-import` escape hatch if needed.
- Q3: defer a new verb; flags on existing verbs are lower risk.
- Q4: yes — uniform path-or-namespace selectors, surfaced clearly in help.
- Q5: out of scope for this plan; note as follow-up.

## Proposed changes

1. `commands.rs`: add `--import` (or `--scan`) to `FolderCreate`; ensure `import`'s path-or-id
   resolution is shared.
2. `main.rs`: in `handle_create`, after provisioning, optionally run the importer path
   (`ParallelImporter`) over `path` when the directory is non-empty (and `--import` set, per Q1).
3. Standardize selector parsing: a single `resolve_folder_selector(&manager, &str)` used by
   `publish`, `stats`, `verify`, `leave`, `daemon-sync`.
4. Tests: add a `create_then_import_is_one_command` workflow test and a
   `path_selector_resolves_like_namespace` test.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
./target/debug/syncweb create --help   # shows --import
./target/debug/syncweb stats files ~/my-folder   # path selector works
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/main.rs`
- `syncweb-cli/tests/workflow/*`, `syncweb-cli/tests/cli_test.rs`
- `docs/commands.md`

## Dependencies

- Overlaps with 013 (publish/namespace clarity) on the "selectors" theme; coordinate but can
  land independently.
