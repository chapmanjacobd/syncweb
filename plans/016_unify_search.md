# Plan 016 — Unify search (`indexing search` vs `package search --channel`)

## Overview

There are two content-discovery surfaces with different backends and different flags:

- `indexing search <query> [--limit]` — FTS over subscribed catalog docs
  (`cli/indexing.rs`, `IndexingDatabase` FTS).
- `package search [query] [--bootstrap] [--timeout-ms] [--channel]` — gossip announcement query
  over the package catalog (`main.rs:3379` `handle_package_search`), with `--channel` routing to
  editorial channels.

A user searching for "a package" must know which of the two commands to use and which backend
backs each.

## Goal

One search entry point that transparently queries the merged discovery backend, with
`--channel` / `--catalog` as *filters* rather than a command split.

## Current state

- `IndexingCommand::Search { query, limit }` — `commands.rs:930`.
- `PackageCommand::Search { query, bootstrap, timeout_ms, channel }` — `commands.rs:834`.
- `publish catalog` (iroh-docs) vs editorial `channel` (gossip or catalog backend,
  `editorial/channel.rs`) are the two *publishing* counterparts of this split.

## Open questions / alternatives

| # | Question | Alternatives |
|---|----------|--------------|
| Q1 | Entry point | (a) single top-level `search`; (b) keep under one group (`indexing search` absorbing `package search`); (c) `search <query>` defaulting to "everything", with `--kind package|catalog|channel`. |
| Q2 | Backend resolution | (a) always query the persisted index and gossip-query only when requested (`--channel`/`--bootstrap`); (b) always both and merge/dedupe results. |
| Q3 | Channel filtering | `--channel X` filters results by editorial channel regardless of backend. |
| Q4 | Relationship to 015 | if catalog becomes a folder-derived index, search reduces to "query the index" and channel is just an index attribute. |

## Recommendation

- Q1: (a) `search <query>` with `--kind` and `--channel` filters; keep `package search` and
  `indexing search` as aliases during transition, then drop.
- Q2: (b) query the index; gossip-query only on explicit `--channel`/`--bootstrap`.
- Implement after 015 lands (to avoid building search against two backends that are about to
  merge).

## Proposed changes

1. `commands.rs`: add top-level `Search { query, kind, channel, limit, timeout_ms, bootstrap }`;
   remove `PackageCommand::Search` and `IndexingCommand::Search` (or alias them).
2. `main.rs` + `cli/indexing.rs`: route `search` through a shared
   `search(query, kind, channel, ...)` that queries `IndexingDatabase` FTS and, when
   `--channel`/`--bootstrap` is set, the package-catalog/editorial gossip.
3. `help_categories!`: add `search` to an appropriate category; drop the two old entries.
4. Tests: `full_suite_test.rs::package_search_channel_and_bootstrap`,
   `workflow/indexing.rs::indexing_search_with_limit` rewritten to the unified command.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
./target/debug/syncweb search <q> --channel curated
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`, `src/cli/args.rs`
- `syncweb-cli/src/main.rs`, `src/cli/indexing.rs`
- `syncweb-cli/tests/*`
- `docs/commands.md`, `docs/indexing.md`, `docs/packages.md`

## Dependencies

- Run after 015 (folder/catalog unification) and coordinated with 014 (gossip audit).
