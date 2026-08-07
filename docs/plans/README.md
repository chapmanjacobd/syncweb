# CLI Simplification — Plan Index

This directory holds user decision artifacts: one plan per change to the `syncweb` CLI.
Each plan is self-contained so a future agent can execute it independently without reading
this index. Plans are ordered so structural changes (command-tree merges) run first and
cosmetic cleanup runs last.

## Execution order

| # | Plan | Kind | Top-level cmd removed | Risk |
|---|------|------|----------------------|------|
| 001 | [Merge `create`/`init`](001-merge-create-init.md) | structural | `init` | low |
| 002 | [Fold `provider` into `trust`](002-fold-provider-into-trust.md) | structural | `provider` | low |
| 003 | [Merge `schedule` into `config`](003-merge-schedule-into-config.md) | structural | `schedule` | low |
| 004 | [Merge `media` into `start`](004-merge-media-into-start.md) | structural | `media` | low |
| 005 | [Merge `unwatch`/`leave`/`unsubscribe`; `subscribe-changes` live-sync model](005-merge-unwatch-leave-unsubscribe.md) | structural | `unwatch`, `unsubscribe`, `subscribe` (with 006) | medium |
| 006 | [Fold `subscribe` into `join --subscribe`](006-merge-join-subscribe.md) | structural | `subscribe` | medium |
| 007 | [Absorb `automatic` into `watch`](007-absorb-automatic-into-watch.md) | structural | `automatic` | medium |
| 008 | [Unify `publish` commands](008-unify-publish-commands.md) | structural | `collection publish`, `indexing publish` (moved) | medium |
| 009 | [Merge `stats`/`filestats`/`health`](009-merge-stats-filestats-health.md) | structural | `filestats`, `health` | medium |
| 010 | [Collapse `ls`/`find`/`sort`](010-collapse-ls-find-sort.md) | structural | `find`, `sort` | high |
| 011 | [Flatten deep groups](011-flatten-deep-groups.md) | structural (UI) | — | medium |
| 012 | [Remove legacy `daemon-*` aliases](012-remove-legacy-daemon-aliases.md) | cleanup | — | low |
| 013 | [Trim alias clutter](013-trim-alias-clutter.md) | cleanup | — | low |
| 014 | [Normalize `snapshot` naming](014-normalize-snapshot-naming.md) | cleanup | — | low |

Run plans in numeric order. They all touch shared files (`commands.rs`, `args.rs`, `main.rs`),
so executing them in parallel will produce merge conflicts; a later plan may assume the
command tree state produced by earlier plans.

## Shared conventions for every executing agent

### Repo layout

- Workspace root: `/home/xk/github/xk/syncweb` (Cargo workspace: `syncweb-cli`, `syncweb-core`).
- CLI lives in `syncweb-cli/src`:
  - `cli/commands.rs` — `Command` enum (all top-level subcommands), all `*Args` structs, and
    nested `*Command` enums (group subcommands). This is the single source of truth for the CLI surface.
  - `cli/args.rs` — `Cli` (global flags: `--verbose`, `--json`, `--no-daemon`, `--data-dir`,
    `--network`), the `help_categories!` macro + `COMMAND_CATEGORIES` (grouped help), and
    `print_grouped_help`.
  - `main.rs` — dispatch `match` over `Command` (lines ~197-255 for main, ~263-355 for
    auxiliary commands) and every `handle_*` function.
  - `cli/indexing.rs` — handlers for `indexing`, `link`, `provider`, `trust`, `attest`,
    `moderation` groups.
  - `cli/filter.rs` — shared `ContentFilter` / `ProviderSelector` flatten structs.
- Generated artifacts (regenerate, do not hand-edit): `completions/syncweb.{bash,zsh,fish,elvish,ps1}`, `man/*`.

### Non-negotiables when changing a command

1. Update the `help_categories!` macro in `cli/args.rs` (`COMMAND_CATEGORIES`). It is an
   exhaustive `match` over `Command`; a new/renamed variant that is not categorized is a
   compile error. The test `all_subcommands_are_categorized` in `args.rs` enforces that
   every subcommand is listed exactly once.
2. Update the dispatch match in `main.rs` (`execute_cli` and `execute_auxiliary_command`).
3. Prefer clap aliases for backward compatibility. A renamed or merged command should keep
   its old name as a visible alias unless the plan explicitly says to drop it.
4. Do not add code comments unless the surrounding code already documents a section; mimic
   existing style. Preserve `--json` output handling in handlers.
5. Regenerate artifacts and update docs (below).

### Verify after any change

```sh
make check          # cargo check -q --all-targets --all-features
make lint           # cargo clippy --all-targets --all-features
make test           # cargo nextest (or: make test0)
cargo test -q --all-targets --all-features
make completions    # regenerate shell completions (requires debug build)
make manpage        # regenerate man pages
```

After regenerating man pages, delete stale files for removed commands (e.g.
`man/syncweb-init.1` after 001). Compare `git status` under `man/` and `completions/` against
the surviving command list.

### Docs to keep in sync

- `docs/commands.md` — the command mapping table (~lines 393-463) and the CLI examples
  (~lines 465-587).
- `docs/overview.md`, `docs/data-models.md`, `docs/phases.md` — command references used in prose.
- `README.md` — Quick Start (lines 29-34) mentions `create`, `join`, `folders`, `devices`.

### How a plan should be executed

1. Read the plan's Decisions needed section; confirm the user has filled the Decision
   record table (or choose the marked "recommended" option if not).
2. Read the referenced current code (`commands.rs`, `args.rs`, `main.rs`, `indexing.rs`).
3. Make the structural change first (enum/args/dispatch), then handlers, then docs/artifacts.
4. Run the verify commands above; fix fallout (e.g. broken tests referencing removed args).
5. Update `docs/plans/README.md` if the plan's scope affects the index.

### Markdown conventions for plan files

- "Recommendation" lines pick a default so the plan can proceed without a human answer.
- The Decision record table is the artifact the user fills in; treat empty cells as
  "use the recommendation".
