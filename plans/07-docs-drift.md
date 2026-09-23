# Plan 07 — Kill stale man pages, completions, and doc-listed-but-absent commands (docs drift is a bug)

Priority: MEDIUM · Status: Draft · Owner: `syncweb-cli`
Depends on: — (re-run after plan 06, which changes the canonical verb set) ·
Fulfills story: #6 (Oli, "which man page do I read?") + principle: docs drift is a bug

## Goal

Every shipped artifact must describe a real command. Today docs/man pages list
verbs that **do not exist** in the `Command` enum (`mirror`, `repl`, `accept`,
`drop`, `conflicts`, `pending`, `deleted`, `undelete`), and several documented
commands do something different from what the docs claim. Docs drift is a
**user-facing bug**: it teaches users flags/verbs that error at dispatch or
behave differently than promised.

## Evidence (verified)

- **Man page references an unenumerated command**: `man/syncweb-mirror.1`
  (exists on disk) — but `Command::Mirror` is **not** a variant of the `Command`
  enum (syncweb-cli/src/cli/commands.rs:6-125 has no `Mirror`). `mirror` in
  core is the domain type `MirrorPin`/`register_mirror`, never a CLI verb.
- **`repl`**: `docs/commands.md:491` (`| repl | repl | Interactive REPL |`) and
  `docs/overview.md:53` list it; `Command::Repl` does not exist.
- **`accept` / `drop`**: `docs/commands.md:473-474` and `docs/overview.md:52`
  list them; no `Command::Accept`/`Command::Drop`.
- **`conflicts` / `pending` / `deleted` / `undelete`**:
  `docs/commands.md:540`, `:605-609`, `:693`; no variants in `Command`.
- **Behavior drift (not just absence)** — documented but implemented differently:
  - `docs/commands.md:476` claims `devices` = "List known peers + connection
    status"; `handle_devices` (main.rs:4274-4292) only prints this device's own
    iroh/Syncthing identities.
  - `docs/commands.md:477` claims `ls` = "List doc entries (lazy)"; `handle_ls`
    (main.rs:4298) is a local-disk scan (see plan 01).
  - `docs/commands.md:478` claims `find` = "Search doc entries (with filters)";
    `handle_find` (main.rs:4336) is a local-disk scan (see plan 01/03).
- Man pages + completions regenerate from `Cli::command()` (generation via the
  `Manpages`/`Completions` variants, commands.rs:110-119) — so the fix is:
  regenerate **after** a truth-verified verb set, not patch the `.1` files by
  hand (they're generated artifacts, and hand-editing creates the same drift).

## Scope guard

- **Docs + generated artifacts only.** No daemon/core changes. Removing docs for
  unimplemented verbs is a docs change; completing the verbs as real commands is
  out of scope for this plan (gated on plan 06 + backlog).

## Steps

1. **Audit**: run `syncweb completions bash/zsh/fish` + `syncweb manpages <dir>`
   into a scratch dir, then `diff` the generated list (subcommand names) against
   `man/` and `completions/` currently in the repo. Every orphan (a file whose
   command is not in `Command`) is a drift artifact.
2. **Converge**: choose per drift item:
   - (a) implement the verb, or
   - (b) remove the doc reference.
   For verbs with no implementation behind a ticket, choose (b):
   - `accept`/`drop` → fold the **revocation** half into plan 05 (`access`),
     remove the verb rows.
   - `conflicts`/`pending`/`deleted`/`undelete` → fold the conflict/resolution
     half into a future plan; remove the verb rows + example blocks
     (docs/commands.md:540, 605-609, 687-693).
   - `repl` → remove from quick-start/docs (no implementation; Maya story #1,
     Oli #6).
   - `devices`/`ls`/`find` behavior drift → correct the docs rows to match the
     implemented behavior (or implement the promised behavior via plans 01/05).
3. **Automate the fix** so it can't regress:
   - Add a `syncweb manpages --check` (or a `syncweb-cli/tests/docs_drift_test.rs`)
     that regenerates man/completions into memory, diffs them against the repo
     copies, and fails if any committed generated file is stale **or** any
     registered command is missing its artifact.
   - Wire the check into CI (Makefile `docs-check` target if present).
4. Update `docs/commands.md`'s command-mapping table to exactly match the
   implemented verb set (plan 06's verbs + aliases), deleting the
   `accept/drop/conflicts/pending/deleted/undelete/repl` rows and correcting the
   `ls`/`find`/`devices` descriptions.

## Tests

- `syncweb-cli/tests/docs_drift_test.rs`:
  - regenerated `syncweb --help` grouped output has no subcommand that lacks a
    `Command` variant (exhaustive — reuse `all_subcommands_are_categorized`).
  - every `man/*.1` file's command name exists in `Command` (presence of an
    orphan file is the bug).
  - the docs/README `repl` reference and the `Command` enum agree: if docs say
    "repl", the CLI must say "repl" (assert one way and fail loudly on
    mismatch — currently docs say it and the CLI doesn't, so removal is the fix).
- Assert the generated completions for each shell parse (smoke test in CI, e.g.
  fish).

## Risks / rollback

- Removing `repl` from the quick-start could break a README screenshot/test that
  references it — check `MANUAL_TESTING_PLAN.md` and screenshots before
  removing; keep an alias if any documented flow uses it.
- Rollback: `git revert` the docs + generated-file changes; the `--check` mode
  stays in the CLI but defaults to a no-op unless `--check` is passed, so no
  release path regresses.
