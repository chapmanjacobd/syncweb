# Plan 010 — Package UX: merge `collection` into `package`, multi-path package creation

## Overview

`collection` (`init` / `add` / `versions`) and `package` (`export` / `import` / `search` /
`info` / `install` / `upgrade` / `remove` / `verify` / `list` / `versions` / `switch`) are two
halves of the *same* versioned-package feature, split across two top-level command groups. Today
a user must run five commands to go from a directory to an installed package:

```sh
syncweb create <dir>                                             # 1. folder (just to obtain a namespace)
syncweb collection init <pkg> --name example                     # 2. init manifest
syncweb collection add <pkg>                                     # 3. scan + hash files
syncweb publish collection <pkg> --namespace <ns> --sequence 1   # 4. publish a manifest ticket
syncweb package install <ticket>                                 # 5. install
```

Every release bump is three more commands (`collection versions` → `publish collection` →
`package upgrade`).

The second gap: a package can only be assembled from a **single** directory root. Users want to
turn *multiple* paths into one package, with the package contents laid out relative to their
common root.

## Goal

1. Merge `collection` into `package` so the versioned-package feature has one home.
2. Make package creation accept multiple source paths and rebase them against a common root.
3. Collapse the 5-command publish flow toward `package init` → `package publish` →
   `package install`.

## Path rebasing semantics (from user)

Compute the longest common ancestor directory of all inputs; each input's logical path is its
path relative to that ancestor. Examples:

| Inputs                                                        | Common root   | Archive contents                    |
|---------------------------------------------------------------|---------------|-------------------------------------|
| `/library/thingdata/` `/library/thing.txt`                    | `/library`    | `./thingdata/` `thing.txt`          |
| `/library/dir/thingdata/` `/library/dir/thing.txt`            | `/library/dir`| `./thingdata/` `thing.txt`          |
| `/library/dir1/thingdata/` `/library/dir2/thingdata/thing.txt`| `/library`    | `./dir1/thingdata/` `./dir2/thingdata/thing.txt` |

Only the selected subtrees are included — siblings at the common root that are not among the
inputs are excluded. A single input defaults to its parent directory as the root (logical path =
basename). An explicit `--root` overrides auto-detection.

## Current state

- `CollectionCommand { Init, Add, Versions }` — `syncweb-cli/src/cli/commands.rs:789-814`;
  handler `handle_collection` — `syncweb-cli/src/main.rs:2724`.
- `PackageCommand { Export, Import, Search, Info, Install, Upgrade, Remove, Verify, List,
  Versions, Switch }` — `commands.rs:816-880`; handler `handle_package` — `main.rs:2878`.
- `PublishCommand::Collection` → `handle_collection_publish` — `main.rs:2792`: loads the
  workspace manifest, re-hashes each entry at `path.join(entry.logical_path)`, stores
  manifest + head in a folder via `CollectionStore`, and announces via `PackageCatalog`
  gossip (`CATALOG_TOPIC`).
- `CollectionManifest { collection_id, version, parent, changelog, entries, package }` and
  `CollectionEntry { content_id, logical_path, size, ... }` — `syncweb-core/src/folder/collection.rs`.
- `scan_collection_entries(path)` — `main.rs:3692`: single-root `ParallelScanner` scan.
- `validate_logical_path` — `collection.rs:600`: requires non-empty, non-absolute, non-escaping
  relative paths.

## Decisions needed

| # | Decision | Options | Recommendation |
|---|----------|---------|----------------|
| D1 | Group shape | single `package` group holding init/add/version/publish/install/... | yes |
| D2 | Keep `collection` after merge | drop / keep alias | drop (no reverse compat needed) |
| D3 | Multi-path command shape | `package init <path...> --name X` scans in one shot; `package add <path...>` re-scans | yes |
| D4 | Root detection | longest common ancestor (auto) vs explicit-only | auto + `--root` override |
| D5 | `publish collection` → | `package publish <path...> --namespace <ns>` | yes |
| D6 | Single-file input root | parent dir (basename) | yes |
| D7 | Overlap handling | error on nested/duplicate inputs vs dedupe | error on conflicts; dedupe identical paths |

## Proposed changes

1. `commands.rs`: delete `CollectionCommand`; add its variants into `PackageCommand`
   (`Init { paths: Vec<PathBuf>, version, name }`, `Add { paths: Vec<PathBuf> }`,
   `Versions { path, version, changelog }`, `Publish { paths, namespace, sequence, bootstrap }`).
2. Move `PublishCommand::Collection` payload into `package publish`.
3. `main.rs`: fold `handle_collection` into `handle_package`; update dispatch and
   `help_categories!` (`Content` category drops `collection`).
4. Replace `scan_collection_entries(path)` with
   `scan_collection_entries(paths: &[PathBuf], root: Option<PathBuf>)`:
   - compute longest common ancestor of inputs when `root` is absent,
   - scan each input, map `relative_path` against the root,
   - validate every resulting logical path through `validate_logical_path`.
5. `handle_collection_publish`: re-hash against `root.join(entry.logical_path)` rather than a
   single `path.join(...)`.
6. Tests: `full_suite_test.rs` (`collection_versions_bump`, `package_archive_export_cli`,
   `package_import_search_install_upgrade_remove`) and `workflow/indexing.rs`
   (`publish_collection_with_sequence_and_bootstrap`) rewritten to the merged surface; add a
   multi-path + common-root rebasing test asserting the three examples above.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
rm -f man/syncweb-collection.1
./target/debug/syncweb package --help       # init/add/versions/publish/install/... in one group
./target/debug/syncweb collection --help    # gone
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/cli/args.rs`
- `syncweb-cli/src/main.rs`
- `syncweb-core/src/folder/collection.rs` (expose a path-rebasing helper if it should live in core)
- `syncweb-cli/tests/full_suite_test.rs`, `syncweb-cli/tests/workflow/indexing.rs`
- `completions/*`, `man/*`
- `docs/commands.md`, `docs/packages.md`

## Dependencies

- None. Overlaps with 013 (publish semantics) and 016 (search merge) only in naming; can land
  independently.
