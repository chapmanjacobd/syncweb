# Plan 007: Absorb `automatic` into `watch`

## Decisions needed

| # | Decision | Options | Recommendation | Chosen |
|---|----------|---------|----------------|--------|
| D1 | Surface | `watch --filters <file>` + `watch --dry-run [--paths ...]` replace `automatic` | yes | |
| D2 | Keep top-level `automatic` as alias | keep / drop | keep (visible alias) | |
| D3 | The no-arg surprise (start daemon in background) | move to `start` / keep in alias / remove | move to `start` | |
| D4 | `watch --dry-run` prints accept/reject per path (today's `automatic --dry-run`) | yes / no | yes | |

## Context for executing agent

- Repo: `syncweb-cli`. `automatic` is a rules-based sync daemon plus a filter dry-run tool;
  `watch` is the folder-watching counterpart (daemon path registers the folder for watching via
  `IpcCommand::AddFolder`; embedded `--no-daemon` path runs a local watcher). They are two
  halves of the same "automation" feature and share a help category.
- Current state:
  - `AutomaticArgs` at `cli/commands.rs:617-627`: `--show-filters`, `--dry-run`, `--paths`,
    `--filters <file>` (defaults to `DATA_DIR/filters.toml`).
  - `WatchArgs` at `cli/commands.rs:661-671`: `path`, `--debounce-ms`, `--exclude`, `--once`.
  - `handle_automatic` at `main.rs:2502-2559`:
    - `--show-filters` prints the `FilterEngine` config.
    - `--dry-run` evaluates `--paths` with `FilterEngine::evaluate` and prints accept/reject.
    - No flags → calls `handle_start` with `bg: true` (`main.rs:2542-2558`) — a hidden
      daemon start. This is the surprise D3.
  - `handle_watch` at `main.rs:2190`: with a daemon it sends `IpcCommand::AddFolder`
    (`main.rs:2194-2222`); with `--no-daemon` it opens a node and runs a local watcher
    (`main.rs:2223+`, reads events and imports them).
- Grouped help: both under `Automation` (`cli/args.rs:76-79`).

## Goal

One automation command:

```sh
syncweb watch [PATH] [--filters filters.toml]     # watch folder, honoring rules
syncweb watch [PATH] --dry-run [--paths ...]      # evaluate rules without applying
syncweb watch --show-filters                       # print active filter config
```

The daemon-starting behavior of `automatic` moves to `start` (D3).

## Structural change (do first)

1. In `cli/commands.rs`: remove `Command::Automatic` (keep alias if D2=keep).
2. Extend `WatchArgs` with `--filters <PathBuf>`, `--show-filters`, `--dry-run`, and
   `--paths Vec<PathBuf>` (used only with `--dry-run`).
3. Update dispatch at `main.rs:225` (remove `Command::Automatic` arm).

## Execution steps

1. Fold `handle_automatic` (`main.rs:2502-2559`) into `handle_watch` (`main.rs:2190`):
   - `--show-filters` / `--dry-run` branches call the `FilterEngine` logic as today.
   - Default (watch) path gains: load `FilterEngine` from `--filters` (or `DATA_DIR/filters.toml`)
     and evaluate each filesystem event before importing, honoring accept/reject actions.
   - Confirm how `handle_watch` currently imports events so the filter integration point is clear.
2. Remove the `handle_start` call from the former `automatic` no-flag path (D3=move):
   add the documented equivalent to `start` if it does not already exist (e.g. `start` is
   already the bg daemon launcher; `automatic` was just a decorated `start`).
3. Update `help_categories!`: remove `Command::Automatic(_) => "automatic",` from
   `Automation` (`cli/args.rs:78`).
4. Docs: `docs/commands.md` mapping rows and examples for `automatic`/`watch`
   (`docs/commands.md:462` watch row, any `automatic` examples); README if referenced.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
rm -f man/syncweb-automatic.1           # if D2=drop
./target/debug/syncweb watch --help     # confirm --filters / --dry-run / --show-filters
git status
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/cli/args.rs`
- `syncweb-cli/src/main.rs`
- `completions/*`, `man/*` (regenerated)
- `docs/commands.md`

## Dependencies

None. Run after 006. Touches the `Automation` category that 011 may also restructure.
`watch` keeps its embedded `--no-daemon` foreground watcher loop: under plan 005 D8
(no foreground live-sync loops except `start`), `watch` is explicitly an exception like `start`
— it is a foreground watcher, not a live-sync loop.
