# syncweb UX Plan Index

Status: ACTIVE — drafted 2026-09-22, grounded in `syncweb-cli/src/main.rs`
(4697 lines), `syncweb-cli/src/cli/args.rs`, `syncweb-cli/src/cli/commands.rs`,
`syncweb-cli/src/cli/indexing.rs`, `syncweb-cli/src/cli/filter.rs`, and
`docs/commands.md`.

Source of truth for stories/voids: `docs/user-stories.md`.

## Reading order for reviewers/handoff agents

Plan 01 (metadata-first `ls`/`find`/`sort`) is done — `ls`/`find`/`sort`
read the doc metadata index (`list_entries()`), the filesystem only enriches a
row, a `folder_mounts` registry plus the `ListEntries` IPC resolve paths in
both embedded and daemon modes, and `--local-only` preserves today's disk
scans.

Plan 02 (eager `join`) is done — bare `join` is metadata-only: it tracks the
folder but live sync is opt-in via `join --subscribe` (one-line summary with
count + size, `size` in `--json`), and bulk download is opt-in via
`join --download-existing` (alias `--download`) so a big folder can't fill
your disk by accident.

Plan 03 (one content-filter vocabulary) is done — `ls`/`find`/`sort`/`download`/
`verify` share a flattened `ContentFilterArgs` group (`--ext`, `--size`,
`--depth`, `--type`, `--modified-*`, `--remote-only`) in
`syncweb-cli/src/cli/filter.rs`, with hidden aliases (`--extension`, `--levels`,
`--sizes`, `-S`, `--changed-*`) for old spellings, `--path-glob` replacing the
`--glob` spelling on the content surface, and client-side selection on
`download`/`verify` (find/sort parity is pinned by a workflow test). Safety and
command-collapse follow; docs drift last.

Plan 04 (safety confirmations) is done — destructive CLI ops are safe by
default: `confirm_destructive` now aborts when stdin is not a TTY, `--yes` is a
global opt-in and the only auto-approve path (`--json` no longer bypasses the
prompt), and prompts are wired into `leave --delete-files`,
`unshare --write`/`--blob`, plus the existing `shutdown`, `snapshot delete`,
`package remove`, `network leave`/`kick`, and `link revoke` sites. Read-only
`unshare` and plain `leave` stay prompt-free. Tests that previously relied on
silent non-TTY execution now pass `--yes`.

Plan 05 (access dashboard) is done — `syncweb access [<path-or-ns>]` merges
folder mode, outbound share tickets, and network membership into one table
(plus `--json`), and `access --revoke <ns> [--read|--write]` revokes in place
reusing plan 04's confirmation. The one permitted core/IPC change landed:
`FolderStatusReport.mode` (additive, `#[serde(default)]`), which also fixes the
daemon `folders` path that previously omitted mode. Pin status and inbound-peer
lists are deferred to plan 09 and called out in `access` output rather than
guessed.

Plan 06 (command collapse) is done — the visible surface is now 12 functional
verbs (`start`, `folders`, `ls`, `find`, `download`, `share`, `snapshot`,
`network`, `watch`, `stats`, `indexing`, `config`) plus 5 meta commands
(`version`, `devices`, `completions`, `manpages`, `help`). Every legacy spelling
(`create`, `join`, `sort`, `db`, `access`, …) is a retained `Command` variant
marked `#[command(hide = true)]` — fully functional, just dropped from grouped
help, man pages, and clap's own help. `stop` is a pure alias of `shutdown`.
`--show-legacy` re-lists hidden verbs in grouped help; all legacy man pages for
hidden verbs were removed and grouped-help/man/completions regenerate from
`Cli::command()`. No core/IPC changes. This hide phase is the transition to
plan 10's deletion for the major release.

Plan 07 (docs drift) is done — stale man pages/completions and
doc-listed-but-absent commands (`mirror`, `repl`, `accept`, `drop`, `conflicts`,
`pending`, `deleted`, `undelete`, `policy`, `public list`) were removed from the
surfaces, so docs match reality ahead of the major release.

Plan 10 (command re-home) is done — for the major release the hidden legacy
surface is deleted and re-homed: `create`/`join`/`leave`/`import` → `folders`,
`networks` → `network status`, `publish` → `indexing publish`, `unshare` →
`access --revoke` (+ `--blob`), `provider` → `share provider add`,
`shutdown` → `stop` (alias `shutdown`), `daemon-sync` → `sync`. The remaining
hidden verbs became visible; `--show-legacy` was removed. The surface is now 30
top-level verbs (25 functional + 5 meta), `folders`/`share` are subcommand
containers, and man/completions/docs/tests were regenerated. See
[10-command-rehome.md](10-command-rehome.md).

## Plans

| #  | Plan | Priority | Goal | Depends on |
|----|------|----------|------|------------|
| 08 | [08-progress-json.md](08-progress-json.md) | MEDIUM | Progress/status surfaces: `stats network`, persistent transfer + event feed, `--json` everywhere | 02 |
| 09 | [09-peer-availability.md](09-peer-availability.md) | LOW | Read-only per-folder peer surface: `network peers` (per-blob avail + inbound-peer list) that plans 05/08 defer to | 05, 08 |
| 10 | [10-command-rehome.md](10-command-rehome.md) | HIGH | Major release: delete hidden legacy commands, re-home under noun-group verbs, 30-verb surface | 06, 07 |

Notes on dependencies:

- Plan 08's dependency on plan 02 (eager `join`) is satisfied.
- Plan 05 landed `access` as a canonical top-level verb; plan 06 kept it as a
  retained (hidden) `Command` variant folded under `share` when collapsing the
  surface — still fully functional.
- Plan 08's universal `--json` contract builds on plan 01's `ListEntries` IPC
  (the listing the filters run over) and plan 03's shared
  `ContentFilterArgs`; the work should land once, in the order 03 → 08.
- Plan 01's `ListEntries` IPC is now additive and stable (daemon-computed
  `local`/enriched size/mtime); plan 03 filters in the daemon before shipping
  rows.

## Working principles for all plans

1. Verify before you write. Every claim below cites a `file:line`. Agents
   must re-read the cited code and update line numbers if code has moved.
2. Preserve machine-friendliness. `--json` output must not regress, and new
   JSON surfaces must be added where missing. Additive only.
3. Aliases, not renames. Don't break existing scripts: new verbs are aliases
   of existing commands kept for compat.
4. Tests alongside. Each plan names the test file and the behavior to assert.
5. No docs-only claims. If a feature is planned, it must either (a) land in
   the CLI or (b) be removed from docs. Drift is a bug (see 07).
6. Server operators can opt out. Schedules, anonymous layers, and
   confirmations must be configurable so `--no-daemon`/`--json`/headless flows
   stay non-interactive.

## Definition of done (all plans)

- `cargo test --workspace` passes.
- `cargo clippy --workspace --all-targets` clean (repo pins `#![deny(clippy::all)]`).
- `cargo fmt --check` clean.
- Grouped help + auto man pages + completions regenerate without producing
  orphaned/unimplemented commands.
- `MANUAL_TESTING_PLAN.md` relevant sections updated.
