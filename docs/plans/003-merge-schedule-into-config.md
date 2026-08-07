# Plan 003: Merge `schedule` into `config`

## Decisions needed

| # | Decision | Options | Recommendation | Chosen |
|---|----------|---------|----------------|--------|
| D1 | Keep `schedule` as a visible top-level alias | keep / drop | drop | |
| D2 | Subcommand names under `config` | `config schedule set|folder` / `config set schedule.*` | `config schedule set|folder` | |

## Context for executing agent

- Repo: `syncweb-cli`. Schedules are configuration for the sync scheduler, so `schedule`
  duplicates `config`'s job. `docs/commands.md` already lists `schedule` under configuration.
- Current state:
  - `Command::Schedule { command }` at `cli/commands.rs:74-78`, dispatched at `main.rs:336-338`
    inside `execute_auxiliary_command` (`Command::Schedule { command: schedule } => handle_schedule(...)`).
  - `ScheduleCommand` at `cli/commands.rs:711-736`: `Set { --active, --bandwidth, --period }`
    and `Folder { name, --active, --max-upload, --max-download }`.
  - `ConfigCommand` at `cli/commands.rs:164-170`: `Set { key, value }` and `Show { section }`.
  - `Command::Config { command }` at `cli/commands.rs:33-37`, dispatch in
    `execute_auxiliary_command` (`main.rs:339-341`) and `handle_config` around `main.rs:361-410`.
- Grouped help: `schedule` is under `Configuration` (`cli/args.rs:103-106`).

## Goal

One configuration surface: `syncweb config schedule set --active ...` and
`syncweb config schedule folder media --active ...`. `schedule` remains an alias if D1=keep.

## Structural change (do first)

1. In `cli/commands.rs`: remove `Command::Schedule`; add a `Schedule(ScheduleCommand)` variant
   to `ConfigCommand` (`cli/commands.rs:164-170`). Keep `ScheduleCommand` as-is.
2. Add `#[command(subcommand)]` behavior so `config schedule` dispatches to the same handler.
3. If D1=keep: the alias can be implemented as a clap alias on `Config`, but aliases only work
   for the command name, not the nested group. Simplest back-compat: keep a hidden
   `Command::Schedule` that forwards to `Command::Config` handling (see steps).
4. Dispatch `config schedule` in `handle_config` (`main.rs:361`): add a `Schedule(ScheduleCommand)`
   arm to `ConfigCommand` that calls the existing schedule handler logic.

## Execution steps

1. Re-structure `ConfigCommand` to `{ Show { section }, Set { key, value }, Schedule(ScheduleCommand) }`.
2. Move `handle_schedule`'s body (referenced from `main.rs:336`) to run under the `Config`
   dispatch; keep the function but call it from `handle_config`.
3. If D1=keep: in `main.rs`, make `Command::Schedule` forward to the config path
   (`Command::Schedule { command } => handle_config_schedule(...)`) instead of being removed
   outright. If D1=drop: delete the variant, its dispatch arm, and `man/syncweb-schedule.1`.
4. Update `help_categories!`: `Configuration` keeps `Command::Config { .. }`; remove
   `Command::Schedule { .. }` (`cli/args.rs:105`).
5. Docs: `docs/commands.md:479-481` (`syncweb schedule ...` examples) and the mapping table.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
rm -f man/syncweb-schedule.1            # only if D1=drop
./target/debug/syncweb config schedule set --active "22:00-06:00"
./target/debug/syncweb schedule --help   # works if D1=keep
git status
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/cli/args.rs`
- `syncweb-cli/src/main.rs`
- `completions/*`, `man/*` (regenerated)
- `docs/commands.md`

## Dependencies

None. Run after 001/002 (same files); before 011 which may flatten `config` further.
