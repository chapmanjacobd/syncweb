# Plan 001: Merge `create`/`init`

## Decisions needed

| # | Decision | Options | Recommendation | Chosen |
|---|----------|---------|----------------|--------|
| D1 | Primary name of the merged command | `create` / `init` | `create` | |
| D2 | Move `--relay-fallback` and `--network` onto the merged command | yes / no | yes | |
| D3 | Print the `share_url` in default output | always / behind `--url` | always (matches `init` today) | |
| D4 | Keep `init` as a visible alias | keep / drop | keep | |

## Context for executing agent

- Repo: `syncweb-cli`. This is the lowest-risk merge; `create` and `init` are near-identical.
- Current state:
  - `Command::Create(FolderCreate)` at `cli/commands.rs:21`, `Command::Init(InitArgs)` at `cli/commands.rs:63`.
  - `FolderCreate` args at `cli/commands.rs:271-285` (`path`, `--mode`, `--relay-fallback`, `--network`).
  - `InitArgs` args at `cli/commands.rs:609-615` (`path`, `--mode` only).
  - `handle_create` at `main.rs:2332-2368`: prints `namespace` + `ticket`.
  - `handle_init` at `main.rs:2371-2408`: prints `path`, `namespace`, `ticket`, `share_url`.
  - Both send `IpcCommand::CreateFolder { path, mode }` to the daemon and both fall back to
    `FolderManager::create(SyncMode::from_str(&mode)?)`. The daemon path ignores
    `--relay-fallback` and `--network` in both (network is only applied in the embedded path).
- The grouped-help category entries are at `cli/args.rs:57-65` (`Folders`) and `cli/args.rs:80-87`
  (`Sharing & Publishing` — `Command::Init(_)` lives here).

## Goal

One folder-creation command: `syncweb create [PATH] [--mode M] [--relay-fallback] [--network N]`
that always prints the namespace, ticket, and share URL. Remove the separate `init` surface.

## Structural change (do first)

1. In `cli/commands.rs`: remove `Command::Init`; keep `Command::Create(FolderCreate)`.
2. Extend `FolderCreate` with `InitArgs`'s behavior. No new flag needed if D3=always;
   if D3=flag, add `#[arg(long)] pub url: bool`.
3. Add `#[command(alias = "init")]` to `Command::Create` if D4=keep.
4. Delete `InitArgs` struct (and its man page `man/syncweb-init.1` after regen).

## Execution steps

1. Merge args: move nothing structurally, but fold `InitArgs`'s semantics into `FolderCreate`.
2. Merge handlers: fold `handle_init`'s output block into `handle_create` so it prints
   `path`, `namespace`, `ticket`, and `share_url`. `InitResult::new(&command.path, folder.namespace_id(), ticket)`
   is used by `handle_init`; keep that call in the merged handler.
3. Update dispatch: remove `Command::Init(command) => handle_init(&ctx, command).await?,` at `main.rs:224`.
4. Update `help_categories!`: remove `Command::Init(_) => "init",` from the
   `Sharing & Publishing` category (`cli/args.rs:85`).
5. If D4=keep, verify `syncweb init PATH` still works via the alias.
6. Docs: `docs/commands.md:393-463` mapping table (drop `init` row, or note alias),
   `docs/commands.md:320-355` init section, `docs/commands.md:565-567` examples,
   `README.md` Quick Start if it mentions init (it does not today).

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
rm -f man/syncweb-init.1
./target/debug/syncweb help          # confirm no 'init' line in grouped help
./target/debug/syncweb create --help # confirm new flags
git status                           # review completions/ and man/ diffs
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/cli/args.rs`
- `syncweb-cli/src/main.rs`
- `completions/*`, `man/*` (regenerated)
- `docs/commands.md`

## Dependencies

None (first plan). Later plans (008, 010) may also touch the same files; run sequentially.
