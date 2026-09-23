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

Plan 02 (eager `join`) is done — bare `join` downloads existing content
and enables live sync by default (one-line summary with count + size, `size`
in `--json`), with `--no-download` / `--no-subscribe` as the lazy opt-outs.
Start with `03-unified-filters.md`. Safety and command-collapse follow; docs
drift last.

## Plans

| #  | Plan | Priority | Goal | Depends on |
|----|------|----------|------|------------|
| 03 | [03-unified-filters.md](03-unified-filters.md) | HIGH | One filter vocabulary (`--ext`, `--size`, `--depth`, `--type`…) shared by `find`/`sort`/`download`/`verify`/`ls` | — |
| 04 | [04-safety-confirmations.md](04-safety-confirmations.md) | HIGH | Real (safe-by-default) prompts before `leave --delete-files`, `unshare --write`, `unshare --blob`; add `--yes` | — |
| 05 | [05-access-dashboard.md](05-access-dashboard.md) | HIGH | One view of who can read/write each folder + revoke (tickets, `share --list`, `networks`) | 04 |
| 06 | [06-command-collapse.md](06-command-collapse.md) | MEDIUM | Collapse 37 top-level commands into ~12 verbs with aliases; unify filters + `--json` | — |
| 07 | [07-docs-drift.md](07-docs-drift.md) | MEDIUM | Kill stale man pages/completions + doc-listed-but-absent commands (`mirror`, `repl`, `accept`, `drop`, `conflicts`, `pending`, `deleted`, `undelete`, `policy`, `public list`) | — |
| 08 | [08-progress-json.md](08-progress-json.md) | MEDIUM | Progress/status surfaces: `stats network`, persistent transfer + event feed, `--json` everywhere | 02 |
| 09 | [09-peer-availability.md](09-peer-availability.md) | LOW | Read-only per-folder peer surface: `network peers` (per-blob avail + inbound-peer list) that plans 05/08 defer to | 05, 08 |

Notes on dependencies:

- Plan 05 depends on plan 04 only for its `--revoke` path (which reuses plan 04's
  confirmation + `--yes`). Its read-only aggregation is independent of plan 04.
- Plan 08's dependency on plan 02 (eager `join`) is satisfied.
- Plan 05 introduces a new top-level verb `access`; plan 06 must account for
  it when collapsing the surface (either as a canonical verb or an alias).
- Plan 03's shared `ContentFilterArgs` builds on plan 01's `ListEntries` IPC
  (the listing the filters run over) and its `--path-prefix`/`--path-glob`
  spellings. Plans 03 and 08 both mention a universal `--json` contract; the
  work should land once, in the order 03 → 08.
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
