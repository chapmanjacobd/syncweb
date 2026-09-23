# Plan 06 — Collapse 37 top-level commands into ~12 verbs (aliases keep scripts green)

Priority: MEDIUM · Status: Draft · Owner: `syncweb-cli`
Depends on: — · Fulfills story: #3 (Ari, surface sprawl), #6 (Oli, "which man page do I read")

## Goal

`syncweb --help` lists **37** top-level commands across **12** categories.
Grouped help helps, but it doesn't answer "which verb do I learn?" — Ari and Oli
each face a wall of nearly-identical surface verbs (`ls` vs `find` vs `search`
vs `sort`, `share` vs `publish` vs `package` vs `link` vs `provider`). Collapse
the functional surface to ~12 stable verbs, keep every legacy spelling as an
invisible alias, and make grouped help + man pages + completions regenerate from
one source.

## Evidence (verified in code)

- `help_categories!` macro (syncweb-cli/src/cli/args.rs:29-111) is the real
  surface: **12** categories ("Daemon", "Folders", "Files", "Automation",
  "Sharing & Publishing", "Content", "Network", "Statistics", "Configuration",
  "Maintenance", "Indexing", "Tooling"). `print_grouped_help` (args.rs:161-198)
  renders them.
- `Command` enum (syncweb-cli/src/cli/commands.rs:6-125) is the authority — 37
  variants. The macro's exhaustiveness match (`category_of`, args.rs:41-45) +
  `#[cfg(test)] all_subcommands_are_categorized` (args.rs:205-234) guarantees no
  orphan — this test **is the safety net** for any collapse: every new verb must
  be either a `Command` variant or a redirected alias, or the test fails.
- Man pages + shell completions are generated from the same `Cli::command()`
  (`Manpages`/`Completions` variants, commands.rs:110-119; generation in
  main.rs), so a collapse at `commands.rs` + alias table deterministically
  regenerates all four surfaces.
- Overlapping verbs (verified dispatch):
  - `ls`(LocalPathArgs) / `find`(FindArgs) / `search`(SearchArgs) / `sort`(SortArgs):
    different arg structs, different filter dialects → plan 03 merges the
    vocabulary, this plan merges the **surface**.
  - `share`/`publish`/`package`/`link`/`provider` + plan 05's `access`: distinct
    capabilities that all mean "get content to someone else" or "control access".

## Scope guard

- **CLI surface + help/completions/man only.** No daemon/core protocol changes.
- Every collapse is a **verb + alias**, never a rename: legacy scripts keep
  working byte-for-byte because aliases resolve to the same handler.

## Steps

1. **Canonical verbs (12 functional + 5 meta).** The functional surface:

   | canonical | absorbs |
   |-----------|---------|
   | `start`   | `start`, `shutdown` (alias `stop`), `status`, `reload`, `daemon-sync` |
   | `folders` | `folders`, `create`, `join`, `leave`, `import` |
   | `ls`      | `ls`, `stat` |
   | `find`    | `find`, `search`, `sort` |
   | `download`| `download`, `verify`, `transfer` |
   | `share`   | `share`, `unshare`, `access` (plan 05), `publish`, `package`, `link`, `provider` |
   | `snapshot`| `snapshot` |
   | `network` | `network`, `networks` |
   | `watch`   | `watch` |
   | `stats`   | `stats`, `db` |
   | `indexing`| `indexing` |
   | `config`  | `config` |

   Meta/utility commands stay top-level and are **not** counted in the 12:
   `version`, `devices`, `completions`, `manpages`, `help`.
2. Implement as clap **aliases on the retained verbs** (`visible_alias` for the
   legacy spellings → old scripts and muscle memory both work; new help shows
   only the canonical verbs, with the full legacy listing behind
   `--show-legacy`). Keep `all_subcommands_are_categorized` green by listing
   alias→category in the macro.
3. `stats file` and `db` stay reachable (power surfaces used by ops/backup
   agents; plan 08 keeps them scriptable) — they live under `stats`, not as
   their own top-level verbs.
4. Regenerate man pages + completions; delete the orphaned generation paths that
   a collapse would otherwise leave behind (see plan 07 for the doc-drift loop).

## Tests

- `syncweb-cli/src/cli/args.rs` `#[cfg(test)] all_subcommands_are_categorized`
  still passes (proves no orphaned/UNCATEGORIZED verb after collapse).
- `syncweb-cli/tests/cli_test.rs`: legacy spellings (`syncweb create`,
  `syncweb unshare --write`, `syncweb sort`) each run to the same handler as
  their canonical verb via alias dispatch (assert identical `--json` output for
  the canonical + legacy spelling).
- `syncweb-cli/src/cli/args.rs` unit: `print_grouped_help` shows exactly the 12
  functional verbs (plus the 5 meta) in the top block; `completions`/man pages
  list aliases but not removed top-level verbs.

## Risks / rollback

- Scripts relying on `syncweb <legacy>` subcommand **positional** matching must
  keep working: keep **all** legacy names as top-level aliases in addition to
  their canonical homes, so no positional call site breaks.
- Plan 05's `access` must not be orphaned — it is folded under `share` here; if
  plan 05 lands after this collapse, add `access` as a `share` subcommand/alias
  in the same step.
- Rollback: revert the alias table (commands.rs) + macro rows. No core change
  to revert.

## Handoff notes

- The category count in evidence is **12**, not 9 (the earlier draft mis-stated
  it). Re-verify `help_categories!` if you add/remove a category.
- `shutdown` is the real verb name; `stop` is only an alias, not a `Command`
  variant.
