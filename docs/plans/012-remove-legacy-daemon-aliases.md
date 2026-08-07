# Plan 012: Remove legacy `daemon-*` aliases and stale artifacts

## Decisions needed

| # | Decision | Options | Recommendation | Chosen |
|---|----------|---------|----------------|--------|
| D1 | Which aliases to remove | `daemon` (start), `daemon-shutdown` (shutdown), `daemon-reload` (reload) | remove all three | |
| D2 | Delete stale generated artifacts | delete man pages for removed commands | yes | |

## Context for executing agent

- Repo: `syncweb-cli`. Deprecated `daemon-*` aliases from the syncweb-py era are still visible
  in help and are documented as such in `docs/commands.md:410-416`.
- Current state:
  - `Command::Start` has `alias = "daemon"` (`cli/commands.rs:9-10`).
  - `Command::Shutdown` has `alias = "daemon-shutdown"` (`cli/commands.rs:11-12`).
  - `Command::Reload` has `alias = "daemon-reload"` (`cli/commands.rs:15-16`).
  - `Command::DaemonSync` (`cli/commands.rs:17-18`) has no alias but the name itself is the
    legacy style (kept; see 013).
- Stale generated man pages in `man/` for commands that no longer exist (regenerating `make
  manpage` only writes current commands, so these are leftovers):
  `syncweb-daemon.1`, `syncweb-daemon-add.1`, `syncweb-daemon-remove.1`, `syncweb-daemon-reload.1`,
  `syncweb-daemon-shutdown.1`, `syncweb-daemon-sync.1`, `syncweb-accept.1`, `syncweb-drop.1`,
  `syncweb-backup.1`, `syncweb-restore.1`, `syncweb-snapshots.1`, `syncweb-repl.1`,
  `syncweb-report.1`.
- `docs/commands.md:410-416` mapping table still lists `daemon`, `daemon-shutdown`,
  `daemon-reload`, `daemon-sync`, `daemon-add`, `daemon-remove`.

## Goal

Help and generated artifacts show only the real command surface. `syncweb start`,
`syncweb shutdown`, `syncweb reload` are the only names.

## Structural change (do first)

1. In `cli/commands.rs`: remove the `alias = "daemon"`, `alias = "daemon-shutdown"`,
   `alias = "daemon-reload"` attributes (`cli/commands.rs:9-16`).
2. Regenerate `completions/*` and `man/*`.

## Execution steps

1. Apply the alias removals (step above).
2. Delete stale man pages (D2 list above) after `make manpage` so regenerated current pages
   stay and stale ones go. Confirm `git status` under `man/` shows exactly the current
   command set (compare against `./target/debug/syncweb help`).
3. Update `docs/commands.md:410-416` mapping table: drop the legacy rows (or mark them
   "removed").
4. Grep the repo for other `daemon-` references (`rg "daemon-shutdown|daemon-reload|daemon-sync|daemon-add|daemon-remove"`).

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
./target/debug/syncweb help        # no daemon-* aliases in Daemon category
ls man/ | rg 'syncweb-(daemon|accept|drop|backup|restore|snapshots|repl|report)'
# above should print nothing
git status
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `completions/*`, `man/*` (regenerated + stale removed)
- `docs/commands.md`

## Dependencies

Run after 011 (all structural merges done, so the generated artifact set is final).
