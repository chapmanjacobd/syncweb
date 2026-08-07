# Plan 009: Merge `stats`/`filestats`/`health`

## Decisions needed

| # | Decision | Options | Recommendation | Chosen |
|---|----------|---------|----------------|--------|
| D1 | Structure | `stats network` (bandwidth) / `stats files` (was filestats) / `stats seeding` (was health) | yes | |
| D2 | Keep `filestats` / `health` as visible aliases | keep / drop | keep | |
| D3 | Preserve existing flag sets under each subcommand | yes / consolidate | yes | |

## Context for executing agent

- Repo: `syncweb-cli`. Three "show statistics" commands with overlapping user intent but
  different data sources (bandwidth accounting, per-file stats, per-blob seeding).
- Current state:
  - `Command::Stats(StatsArgs)` at `cli/commands.rs:69`; `StatsArgs` at `cli/commands.rs:673-683`
    (`--folder`, `--peer`, `--reset`, `--period`). `handle_stats` at `main.rs:1976` — bandwidth.
  - `Command::FileStats(FileStatsArgs)` at `cli/commands.rs:70`; `FileStatsArgs` at
    `cli/commands.rs:685-697` (`folder`, `--by extension|size|all|time`, `--top-largest`).
    `handle_filestats` at `main.rs:2056`.
  - `Command::Health(HealthArgs)` at `cli/commands.rs:56`; `HealthArgs` at `cli/commands.rs:580-586`
    (`path`, flattened `ContentFilter`). `handle_health` at `main.rs:1683` — seeding status per blob.
- Grouped help: `stats`/`filestats` under `Statistics` (`cli/args.rs:99-102`), `health` under
  `Files` (`cli/args.rs:66-75`).

## Goal

```sh
syncweb stats network [--folder F] [--peer N] [--reset]   # was stats
syncweb stats files FOLDER [--by ...] [--top-largest N]   # was filestats
syncweb stats seeding PATH [filters]                      # was health
```

## Structural change (do first)

1. In `cli/commands.rs`: convert `Command::Stats(StatsArgs)` into a group
   `Stats { #[command(subcommand)] command: StatsCommand }` with `Network(StatsArgs)`,
   `Files(FileStatsArgs)`, `Seeding(HealthArgs)`.
2. Remove `Command::FileStats` and `Command::Health` variants; add aliases `filestats`/`health`
   on the group if D2=keep (same clap-alias caveat as 008: verify the alias can route to the
   right subcommand; otherwise keep thin forwarding variants).
3. Update dispatch at `main.rs:214` (`Stat` stays), `main.rs:215`/`220` (health/filestats arms).

## Execution steps

1. Add `handle_stats(ctx, StatsCommand)` in `main.rs` that routes to the three existing handler
   bodies (`handle_stats` at 1976, `handle_filestats` at 2056, `handle_health` at 1683).
   Keep the existing functions as the three sub-handlers.
2. Update the dispatch arms in `execute_cli` (`main.rs:214-221`).
3. Update `help_categories!`: `Statistics` category lists `stats` (drop `filestats`);
   remove `health` from `Files` (`cli/args.rs:74`).
4. Docs: `docs/commands.md` mapping rows for `stats`, `filestats`, `health` and examples at
   `docs/commands.md:517-518`, `docs/commands.md:476`.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
rm -f man/syncweb-filestats.1 man/syncweb-health.1     # if D2=drop
./target/debug/syncweb stats --help     # confirm network|files|seeding
./target/debug/syncweb health --help    # works if D2=keep
git status
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/cli/args.rs`
- `syncweb-cli/src/main.rs`
- `completions/*`, `man/*` (regenerated)
- `docs/commands.md`

## Dependencies

None. Run after 008. Uses `ContentFilter` from `cli/filter.rs` (unchanged).
