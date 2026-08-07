# Plan 014: Normalize `snapshot` naming

## Decisions needed

| # | Decision | Options | Recommendation | Chosen |
|---|----------|---------|----------------|--------|
| D1 | Add docs-era aliases | `backup` → snapshot create, `restore` → snapshot restore, `snapshots` → snapshot list | yes (aliases, not renames) | |
| D2 | Keep the `snapshot` group as the canonical name | keep group name `snapshot` / rename group to `backup` | keep `snapshot` | |
| D3 | Update docs to match real command surface | yes / no | yes | |

## Context for executing agent

- Repo: `syncweb-cli`. Docs and prose consistently call this feature `backup`/`restore`/
  `snapshots` while the actual CLI surface is `snapshot create|restore|list|diff|delete` —
  a naming drift that confuses users.
- Current state:
  - `Command::Snapshot { command }` at `cli/commands.rs:50-54`.
  - `SnapshotCommand` at `cli/commands.rs:588-607`: `Create(SnapshotCreateArgs)`, `Restore`,
    `List { path }`, `Diff`, `Delete`.
  - Handlers: `handle_snapshot` at `main.rs:1171`, `handle_snapshot_create` at `main.rs:1104`,
    `handle_snapshot_restore` at `main.rs:1139`.
  - Grouped help: under `Content` (`cli/args.rs:88-93`).
- Docs that use `backup`/`restore`/`snapshots`:
  - `docs/commands.md:449-451` (mapping table), `docs/commands.md:538-542` (examples).
  - `docs/data-models.md:260-278`.
  - `docs/phases.md:98-100`.
  - `docs/overview.md` (grep for `backup`).
- Note: stale man pages `syncweb-backup.1`, `syncweb-restore.1`, `syncweb-snapshots.1` exist in
  `man/` (see 012) — they predate the `snapshot` group and should be removed on regen regardless.

## Goal

The help surface and the documentation name the same thing. Add `backup`/`restore`/
`snapshots` as aliases (D1) and update docs to the canonical `snapshot` verbs (D3).

## Structural change (do first)

1. In `cli/commands.rs:588-607`: add clap aliases on the `SnapshotCommand` variants:
   - `Create` → alias `backup`
   - `Restore` → alias `restore`
   - `List` → alias `snapshots`
2. If D2=rename instead, rename the group and update `help_categories!` (`cli/args.rs:89`) and
   dispatch (`main.rs:217-219`).

## Execution steps

1. Add the aliases (step above). Verify clap allows per-variant aliases here (it does for
   subcommand variants).
2. Regenerate `completions/*` and `man/*`; remove stale `man/syncweb-backup.1`,
   `man/syncweb-restore.1`, `man/syncweb-snapshots.1`.
3. Update docs to the canonical surface:
   - `docs/commands.md:538-542` → `snapshot create/restore/list/diff/delete` examples.
   - `docs/commands.md:449-451` mapping table.
   - `docs/data-models.md:260-278`, `docs/phases.md:98-100`, `docs/overview.md` prose.
4. Grep for remaining `backup`/`snapshots`/`restore` CLI references:
   `rg -n "syncweb (backup|restore|snapshots)" docs/`.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
rm -f man/syncweb-backup.1 man/syncweb-restore.1 man/syncweb-snapshots.1
./target/debug/syncweb snapshot --help    # confirm backup/restore/snapshots aliases
./target/debug/syncweb backup PATH --description "x"   # alias works
rg -n "syncweb (backup|restore|snapshots)" docs/       # expect no hits
git status
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `completions/*`, `man/*` (regenerated + stale removed)
- `docs/commands.md`, `docs/data-models.md`, `docs/phases.md`, `docs/overview.md`

## Dependencies

Run after 012 (012 deletes the same stale man pages; avoid deleting them twice). Last plan
in the sequence.
