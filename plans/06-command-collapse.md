# Plan 06 — Collapse 37 top-level commands into ~12 verbs (aliases keep scripts green)

Priority: MEDIUM · Status: Draft · Owner: `syncweb-cli`
Depends on: — · Fulfills story: #3 (Dev, surface sprawl) + cross-cutting theme
#3 (35+ top-level commands → ~12 verbs)

## Goal

`syncweb --help` lists 37 top-level commands across 12 categories.
Grouped help helps, but it doesn't answer "which verb do I learn?" — Ari and Oli
each face a wall of nearly-identical surface verbs (`ls` vs `find` vs `search`
vs `sort`, `share` vs `publish` vs `package` vs `link` vs `provider`). Collapse
the visible functional surface to ~12 stable verbs, keep every legacy spelling
as a hidden variant (never a rename — scripts keep working), and make
grouped help + man pages + completions regenerate from one source.

## Evidence (verified in code)

- `help_categories!` macro (syncweb-cli/src/cli/args.rs:29-111) is the real
  surface: 12 categories ("Daemon", "Folders", "Files", "Automation",
  "Sharing & Publishing", "Content", "Network", "Statistics", "Configuration",
  "Maintenance", "Indexing", "Tooling"). `print_grouped_help` (args.rs:161-198)
  renders them.
- `Command` enum (syncweb-cli/src/cli/commands.rs:6-125) is the authority — 37
  variants. The macro's exhaustiveness match (`category_of`, args.rs:41-45) +
  `#[cfg(test)] all_subcommands_are_categorized` (args.rs:205-234) guarantees no
  orphan — this test is the safety net for any collapse: every new verb must
  be either a `Command` variant or a redirected alias, or the test fails.
- Man pages + shell completions are generated from the same `Cli::command()`
  (`Manpages`/`Completions` variants, commands.rs:110-119; generation in
  main.rs), so a collapse at `commands.rs` + alias table deterministically
  regenerates all four surfaces.
- Overlapping verbs (verified dispatch):
  - `ls`(LocalPathArgs) / `find`(FindArgs) / `search`(SearchArgs) / `sort`(SortArgs):
    different arg structs, different filter dialects → plan 03 merged the
    vocabulary, this plan merges the surface.
  - `share`/`publish`/`package`/`link`/`provider` + plan 05's `access`: distinct
    capabilities that all mean "get content to someone else" or "control access".

## Scope guard

- CLI surface + help/completions/man only. No daemon/core protocol changes.
- Every collapse keeps the legacy spelling functional: legacy `Command`
  variants are hidden, not deleted, and pure synonyms are clap aliases — so
  scripts keep working byte-for-byte.

## Steps

1. Canonical verbs (12 functional + 5 meta). The functional surface:

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

   Meta/utility commands stay top-level and are not counted in the 12:
   `version`, `devices`, `completions`, `manpages`, `help`.
2. Mechanism (keep every legacy spelling byte-for-byte working). Clap
   subcommand aliases cannot merge incompatible arg structs — aliasing
   `create` onto `folders` does not make `folders` accept `FolderCreate` args.
   So the collapse is implemented as:
   - Keep every legacy `Command` variant in `commands.rs` and mark the
     non-canonical ones `#[command(hide = true)]` so they stay fully functional
     but drop out of the grouped help. This is the "aliases, not renames" rule:
     `syncweb create ./docs`, `syncweb sort music/`, `syncweb unshare --write`
     all keep their exact handlers.
   - Add clap `alias`/`visible_alias` only for pure synonyms that are not
     already `Command` variants (e.g. `stop` → `shutdown`).
   - Do not use `visible_alias` for the hidden legacy verbs — a visible
     alias would re-add them to `--help`, defeating the collapse. Use `hide`.
     (The earlier draft said `visible_alias`; that contradicts "new help shows
     only the canonical verbs".)
   - Underspecification fix — do *not* delete legacy rows from
     `help_categories!`. The macro generates both `COMMAND_CATEGORIES`
     (display list) and `category_of` — an exhaustive `match command`
     over all 37 variants (args.rs:41-45, verified exhaustive today). Removing
     a legacy verb's row drops its `category_of` arm → non-exhaustive match →
     compile error, and `all_subcommands_are_categorized`'s reverse checks
     (args.rs:220-233 — it compares the counted set against *all* subcommands,
     hidden or not) fail. Instead: keep all 37 rows in the macro and drive
     visibility purely from display code —
     1. mark legacy verbs `#[command(hide = true)]` (stays parseable; clap
        `--help` omits them), and
     2. make `print_grouped_help` skip hidden subcommands
        (`if sc.is_hidden() { continue; }` at args.rs:174-183) so the grouped
        help shows only the 12 + 5 names. `--show-legacy` re-lists the hidden
        ones in that loop.
     With this, `all_subcommands_are_categorized` stays unchanged and
     green — every variant remains categorized (compile-checked) and the
     display list is a strict subset. If you prefer the smaller
     `COMMAND_CATEGORIES`, split the macro into a display list + separate
     exhaustive category match; do not claim "the macro lists only 17" without
     that split.
3. `stats files` and `db` stay reachable (power surfaces used by ops/backup
   agents; plan 08 keeps them scriptable) — they live under `stats`, not as
   their own top-level verbs. (The subcommand is `stats files`, not `stats file`.)
4. Regenerate man pages + completions from `Cli::command()` after the
   hide/alias changes; hidden verbs must not produce orphaned `.1` files that
   plan 07's `--check` would flag (man/completions for hidden subcommands are
   intentionally dropped or kept only under `--show-legacy`).

## Tests

- `syncweb-cli/src/cli/args.rs` `#[cfg(test)] all_subcommands_are_categorized`
  updated to assert every visible subcommand is categorized (hidden legacy
  verbs exempt) and still passes — proves no orphaned/UNCATEGORIZED visible
  verb after collapse.
- `syncweb-cli/tests/cli_test.rs`: legacy spellings still parse and run —
  `syncweb create <dir>`, `syncweb sort <dir>`, `syncweb unshare --write <ns>`
  each succeed with their existing output (they are retained `Command`
  variants, just hidden). Pure aliases dispatch to the same handler: assert
  `syncweb stop` and `syncweb shutdown` produce identical output.
- `syncweb-cli/src/cli/args.rs` unit: `print_grouped_help` shows exactly the 12
  functional verbs (plus the 5 meta) in the top block; hidden legacy verbs do
  not appear there, and `--show-legacy` lists them.

## Risks / rollback

- Scripts relying on `syncweb <legacy>` must keep working: keeping every legacy
  name as a `Command` variant (just `hide = true`) is stronger than an alias —
  no positional call site can break, and each legacy verb keeps its own arg
  struct. Only pure synonyms that are not already `Command` variants need a
  clap alias (e.g. `stop` → `shutdown`); `networks` is already a `Command`
  variant and must NOT be re-added as an alias (step 2's rule).
- Plan 05's `access` must not be orphaned — it is folded under `share` here; if
  plan 05 lands after this collapse, add `access` as a visible `share`
  subcommand/alias in the same step.
- Rollback: revert the `hide = true` annotations + macro rows. No core change
  to revert.

## Handoff notes

- The category count in evidence is 12, not 9 (the earlier draft mis-stated
  it). Re-verify `help_categories!` if you add/remove a category.
- `shutdown` is the real verb name; `stop` is only an alias, not a `Command`
  variant.
