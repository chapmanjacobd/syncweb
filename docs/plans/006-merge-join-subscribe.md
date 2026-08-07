# Plan 006: Fold `subscribe` into `join` as `--subscribe` (no live loop)

Superseded by 005 D7-D9 (decided 2026-08-07): the original plan ("merge `join`/`subscribe`,
`join` re-enters the live loop") is replaced by the model below. There is no foreground
live-sync loop at all (005 D8). `subscribe` the command is dropped, not folded; live syncing
is the persisted `subscribe-changes` config (005 D2=B), enabled by
`join --subscribe` or `config set subscribe-changes … on`.

## Decisions needed

| # | Decision | Options | Recommendation | Chosen |
|---|----------|---------|----------------|--------|
| D1 | Fate of `subscribe` command | A: fold into `join` as re-enter-loop; B: drop entirely (config key only) | B (D8: no loops to re-enter) | B |
| D2 | `join` accepts a folder selector | only with `--subscribe` (idempotent re-enable) / never | only with `--subscribe` | only with `--subscribe` |
| D3 | Keep top-level `subscribe` as alias | keep / drop | drop (there is no underlying verb) | drop |
| D4 | Six filter flags (`--ingest-only`, `--ignore-self`, `--prefix`, `--sync-prefix`, `--glob`, `--max-count`, `--max-size`) | stay on `join --subscribe` and persist / drop | stay on `join --subscribe`, persist into `SubscribeFilters` | stay + persist |
| D5 | `join --once` | keep / drop | drop (nothing loops anymore) | drop |

## Context for executing agent

- Repo: `syncweb-cli`. Today `subscribe <folder> [six filter flags]` re-declares the same six
  flags as `join` and both register an `ActiveSession` in a foreground process
  (`handle_subscribe` at `main.rs:2562`, `handle_join` at `main.rs:2411`, same
  `SubscribeParams` block at `main.rs:2452-2479`). 005 D8 removes these foreground loops: the
  daemon's `start_supervision` (via `subscribe-changes`) is the only live-sync machinery.
- 005 (landed) already dropped `unwatch`/`unsubscribe` and merged `leave`; the vocabulary rule
  (no `purge`/`remove`/`detach`) and the `join`/`leave` membership ontology apply here.
- `JoinArgs`/`FolderJoin` at `cli/commands.rs:283-310` (`ticket`, `path`, `--mode`,
  `--relay-fallback`, `--network`, `--once`, six filter flags). `SubscribeArgs` at
  `cli/commands.rs:738-754`.
- Grouped help: both under `Folders` (`cli/args.rs:57-65`).

## Goal

```sh
syncweb join <ticket>                 # track a new folder; subscribe-changes: off (005 D7)
syncweb join <ticket> --subscribe     # track + enable live syncing (persisted), then exit
syncweb join <folder> --subscribe     # idempotent: enable live syncing on an existing folder
config set subscribe-changes <folder> on|off   # the canonical toggle (005)
```

No command enters a foreground sync loop. `join` always returns after persisting state.

## Structural change (do first)

1. In `cli/commands.rs`: remove `Command::Subscribe` (D1=B, D3=drop) and `Command::Join`'s
   live-loop entry point; add `#[arg(long)] subscribe: bool` to `FolderJoin` and drop `--once`
   (D5).
2. Widen `FolderJoin.ticket`: keep it as a required target string, but when `--subscribe` is
   set it may be either a ticket (new folder) or a folder selector (existing folder, D2). When
   `--subscribe` is absent it must be a ticket (creating a folder); the folder-selector form
   without `--subscribe` errors ("folder already tracked — re-enable live syncing with
   `join <folder> --subscribe`").
3. Factor the six filter fields into a shared `#[derive(clap::Args)] struct SubscribeFilters`
   and flatten into `FolderJoin`; serialize the same struct into the persisted
   `SubscribeFilters` config (005).

## Execution steps

1. Extract the `SubscribeParams` construction (`main.rs:2452-2479` / `main.rs:2567-2593`)
   into `build_subscribe_params(&filters)`; the daemon supervisor consumes
   `SubscribeFilters` from config instead of a foreground session.
2. Rewrite `handle_join`:
   - ticket + `--subscribe` → create/track, write `subscribe-changes = on` + filters, nudge
     the daemon (`TriggerSync`/reload), exit.
   - ticket only → create/track, `subscribe-changes = off` (D7 default), exit.
   - folder selector + `--subscribe` → resolve the folder, write `subscribe-changes = on` +
     filters, nudge the daemon, exit (idempotent, 005).
   - Remove the `--once` branch and the live loop (`main.rs:2480-2498`).
3. Remove `handle_subscribe` (`main.rs:2562`) and `IpcCommand::Subscribe`
   (`ipc.rs` `handle_subscribe_command`); delete the `Command::Subscribe` dispatch arm
   (`main.rs:207-208`).
4. Update `help_categories!`: `Folders` lists `join` (with `--subscribe`) and `leave`; remove
   `subscribe` (`cli/args.rs:61-65`).
5. Docs: `docs/commands.md` mapping rows and subscribe examples (`docs/commands.md:489-495`);
   README if referenced.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
rm -f man/syncweb-subscribe.1
./target/debug/syncweb join --help      # confirm --subscribe, filters; no --once
./target/debug/syncweb subscribe --help # gone
./target/debug/syncweb help             # Folders: create, join, leave, folders
git status
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/cli/args.rs`
- `syncweb-cli/src/main.rs`
- `syncweb-core/src/daemon/ipc.rs`
- `syncweb-core/src/storage/config.rs` (`SubscribeFilters`)
- `completions/*`, `man/*` (regenerated)
- `docs/commands.md`

## Dependencies

Run after 005 (the `leave`/`subscribe-changes` surface this plan's `join --subscribe` completes).
Shares the config work and `run_cycle` gating with 005 D2=B. `watch` (007) is unaffected — it
keeps its embedded `--no-daemon` foreground watcher loop.
