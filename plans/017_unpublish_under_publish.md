# Plan 017 — Fold `unpublish` under `publish`

## Overview

`unpublish` is a sibling top-level command while `publish` is a group; the publish/unpublish pair
should live together. `unpublish <namespace> --blob <hash>` (remove a public blob pin) is the
natural inverse of `publish blob <namespace> <hash>`.

## Current state

- `Command::Unpublish(UnpublishArgs)` — `commands.rs:70`; `UnpublishArgs { namespace, blob }` —
  `commands.rs:781`.
- `PublishCommand { Folder, Blob, Collection, Catalog }` — `commands.rs:732`.
- `handle_unpublish` — `main.rs:2693`.
- `help_categories!` lists `publish`/`unpublish` under `Sharing & Publishing` — `args.rs:75-81`.

## Proposed changes

1. `commands.rs`: add `PublishCommand::Unpublish { namespace, blob }` (visible name `rm` or
   `unpublish`), move the `UnpublishArgs` fields onto it; delete top-level `Command::Unpublish`.
2. `main.rs`: move the `handle_unpublish` body into the `handle_publish` match arm.
3. `args.rs`: remove `unpublish` from `help_categories!` (the exhaustive `category_of` match
   will force this).
4. Tests: `workflow/indexing.rs::publish_blob_and_unpublish_round_trip` and any `cli_test`
   references updated to `publish unpublish` / `publish rm`.

## Decisions needed

| # | Decision | Options | Recommendation |
|---|----------|---------|----------------|
| D1 | Verb name | `publish rm` / `publish unpublish` | `publish unpublish` (keeps the familiar word) |
| D2 | Keep top-level `unpublish` alias | keep / drop | drop (no reverse compat needed) |

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
./target/debug/syncweb publish --help   # shows unpublish/rm
./target/debug/syncweb unpublish --help # gone
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`, `src/cli/args.rs`
- `syncweb-cli/src/main.rs`
- `syncweb-cli/tests/workflow/indexing.rs`, `syncweb-cli/tests/cli_test.rs`
- `docs/commands.md`

## Dependencies

- Coordinate with 010 (publish collection → package publish) and 013 (publish semantics) since
  all three touch the `publish` group.
