# Plan 06 — Collapse 35+ top-level commands into ~12 verbs (aliases keep scripts green)

Priority: MEDIUM · Status: Draft · Owner: `syncweb-cli`
Depends on: — · Fulfills story: #3 (Ari, surface sprawl), #6 (Oli, "which man page do I read")

## Goal

`syncweb --help` lists **35+** top-level commands across 9 groups. Grouped help
helps, but it doesn't answer "which verb do I learn?" — Ari and Oli each face a
wall of nearly-identical surface verbs (`ls` vs `find` vs `search` vs `sort`,
`share` vs `publish` vs `package` vs `link` vs `provider`). Collapse the surface
to ~12 stable verbs, keep every legacy spelling as an invisible alias, and make
grouped help + man pages + completions regenerate from one source.

## Evidence (verified in code)

- `help_categories!` macro (syncweb-cli/src/cli/args.rs:29-111) is the real
  surface: 9 groups ("Daemon", "Folders", "Files", "Automation",
  "Sharing & Publishing", "Content", "Network", "Statistics",
  "Configuration", "Maintenance", "Indexing", "Tooling") — 12 headings with 35+
  command rows; `print_grouped_help` (args.rs:161-198) renders them.
- `Command` enum (syncweb-cli/src/cli/commands.rs:5-125) is the authority; the
  macro's exhaustiveness match (args.rs:49-111) + `#[cfg(test)]
  all_subcommands_are_categorized` (args.rs:204-233) guarantees no orphan — this
  test **is the safety net** for any collapse: every new verb must be either a
  `Command` variant or a redirected alias, or the test fails.
- Man pages + shell completions are generated from the same `Cli::command()`
  (main.rs:301 manpages, `Command::Completions`/`Manpages`), so a collapse at
  `commands.rs` + alias table deterministically regenerates all four surfaces.
- Overlapping verbs (verified dispatch):
  - `ls`(LocalPathArgs) / `find`(FindArgs) / `search`(SearchArgs) / `sort`(SortArgs):
    different arg structs, different filter dialects → plan 03 merges the
    vocabulary, this plan merges the **surface**.
  - `share`/`publish`/`package`/`link`/`provider` (plan 03 theme C): distinct
    capabilities that all mean "get content to someone else"; see grouping below.

## Scope guard

- **CLI surface + help/completions/man only.** No daemon/core protocol changes.
- Every collapse is a **verb + alias**, never a rename: legacy scripts keep
  working byte-for-byte because aliases resolve to the same handler.

## Steps

1. **Canonical verb list (12):**
   - `start / stop / status / reload` (Daemon)
   - `folders / join / leave` (lifecycle)
   - `ls / find / download / verify` (content access — the four story-drivers)
   - `share / network / config` (control plane)
   Then make everything else a subcommand of one of these **or** a documented
   alias:

   | legacy top-level | resolves to |
   |------------------|-------------|
   | `create` / `snapshots` / `import` / `export` | `folders`
   | `stat` / `devices` / `sort` (when no filter) | `ls`
   | `sort` (with filters) / `search` / `find` | `find`
   | `unshare` / `unshare --write` | `share --unshare`
   | `publish` / `package` / `link` / `provider` / `share --blob` | `share`
   | `networks` / `network` | `network`
   | `config` / `completions` / `manpages` | `config`
   | `stats` / `db` / `verify` | (stay; see below)
2. Implement as clap **aliases on the retained verbs** (`visible_alias` for the
   legacy spellings → old scripts and muscle memory both work; new help shows
   only the 12 verbs, with a `--help`-full listing behind `--show-legacy`).
   Keep `all_subcommands_are_categorized` green by listing alias→category in the
   macro.
3. `stats file` and `db` stay top-level (power surfaces used by ops/backup
   agents, plan 08 keeps them scriptable).
4. Regenerate man pages + completions; delete the orphaned generation paths that
   a collapse would otherwise leave behind (see plan 07 for the doc-drift loop).

## Tests

- `syncweb-cli/src/cli/args.rs` `#[cfg(test)] all_subcommands_are_categorized`
  still passes (proves no orphaned/UNCATEGORIZED verb after collapse).
- `syncweb-cli/tests/cli_test.rs`: legacy spellings (`syncweb create`,
  `syncweb unshare --write`, `syncweb find --glob x`) each run to the same
  handler as their new verb via alias dispatch (assert identical `--json` output
  for the canonical + legacy spelling).
- `syncweb-cli/src/cli/args.rs` unit: `print_grouped_help` shows exactly 12
  verbs in the top block; `completions`/man pages list aliases but not new verbs.

## Risks / rollback

- Scripts relying on `syncweb <legacy>` subcommand **positional** matching
  (e.g. `syncweb create ./x` used to be `create`; now `join`?) — mitigate by
  keeping **all** legacy names as top-level aliases in addition to the
  subcommand homes, so no positional call site breaks.
- Rollback: revert the alias table (commands.rs) + macro rows. No core change
  to revert.
