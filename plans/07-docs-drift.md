# Plan 07 — Kill stale man pages, completions, and doc-listed-but-absent commands (docs drift is a bug)

Priority: MEDIUM · Status: Draft · Owner: `syncweb-cli`
Depends on: plan 06 (collapse guarantees the verb set is canonical) ·
Fulfills story: #6 (Oli  "which man page do I read?") + principle: docs drift is a bug

## Goal

Every shipped artifact must describe a real command. Today docs/man pages list
verbs that **do not exist** in the `Command` enum (`mirror`, `repl`, `accept`,
`drop`, `accept --folder`, `conflicts`, `pending`, `undelete`), and man pages
+ completions for a longer list were generated but several reference commands
that were removed. `syncweb mirror` has no implementation; `syncweb repl` is
documented in the README quick-start (README.md:33) but absent from
`commands.rs`. Docs drift is therefore a **user-facing bug**: it teaches users
flags/verbs that error at dispatch.

## Evidence (verified)

- **Man pages on disk reference unenumerated commands**: `man/syncweb-mirror.1`
  (28 lines, exists on disk) — but `Command::Mirror` is **not** a variant of
  the `Command` enum (grep: no `Mirror` in syncweb-cli/src/cli/commands.rs, no
  handler). `grep -c "Mirror" main.rs` returns the domain type
  (`MirrorPin`, `register_mirror` in indexing.rs:242), never a CLI verb.
- **`repl`**: `README.md:33` (`syncweb repl  # interactive REPL`) and
  `docs/commands.md` map row "| `repl` | `repl` | Interactive REPL" — but
  `Command::Repl` does not exist (commands.rs:5-125). Verified: `grep -n "Repl"`
  in commands.rs → no variant; `main.rs` `Command::...` dispatch never matches it.
- **grouped help claims included daemon-auxiliary verbs that are unimplemented**
  (see plan 04/05 revoke scope): `accept`/`drop` (peer accept/revoke) and
  `conflicts`/`pending`/`deleted`/`undelete` are listed in
  `docs/overview.md` / `docs/commands.md` but have **no variant** +
  `DOC-listed-but-absent` output of the workflow. Verified by grep across
  commands.rs + main.rs handlers.
- Man pages + completions regenerate from `Cli::command()` (main.rs:296-301,
  `handle_manpages`/`handle_completions`) — so the fix is: regenerate **after**
  a truth-verified verb set, not patch the `.1` files by hand (they're
  generated artifacts, and hand-editing creates the same drift).

## Scope guard

- **Docs + generated artifacts only.** No daemon/core changes. Removing docs
  for unimplemented verbs is a docs change; completing the verbs as real
  commands is out of scope for this plan (gated on plans 06 + backlog).

## Steps

1. **Audit**: run `syncweb completions bash/zsh/fish` + `syncweb manpages
   <dir>` into a scratch dir, then `diff` the generated list (subcommand names)
   against `man/` and `completions/` currently in the repo. Every orphan (a
   file whose command is not in `Command`) is a drift artifact.
2. **Converge**: either (a) implement the 2-3 doc-real verbs users actually hit
   (`repl` → mark DEPRECATED in docs; remove from quick-start since it has no
   implementation — Maya story #1, Oli #6), or (b) remove the doc references.
   Choose (b) for verbs with no implementation behind a ticket (accept/drop/
   conflicts/pending/undelete → fold the **revocation** half into plan 05, the
   **conflict** half into a future plan).
3. **Automate the fix** so it can't regress:
   - Add a `syncweb manpages --check` (or a `syncweb-cli/tests/docs_drift_test.rs`)
     that: regenerates man/completions into memory, diffs them against the repo
     copies, and fails if any committed generated file is stale **or** any
     registered command is missing its artifact. This is the "docs drift is a
     bug" principle made checkable.
   - Wire the check into CI (Makefile `docs-check` target if present).
4. Update `docs/commands.md`'s command-mapping table to exactly match the
   implemented verb set (plan 06's 12 verbs + aliases), deleting rows like
   `accept/drop/conflicts/pending/deleted/undelete/repl` and correcting the
   `| |` blank-cell rows that currently imply unimplemented content.

## Tests

- `syncweb-cli/tests/docs_drift_test.rs`:
  - regenerated `syncweb --help` grouped output has no subcommand that lacks a
    `Command` variant (exhaustive — reuse the categorized-help test).
  - every `man/*.1` file's command name exists in `Command` (rename orphans to
    nothing: presence is the bug).
  - `README.md` `repl` reference removed or `--help` shows `repl` (must match:
    if docs say "repl", the CLI must say "repl" — flip a boolean in the test to
    fail loudly on one side).
- Assert the generated completions for each shell parse in a fake
  `shellcheck`-style smoke test (fish in CI).

## Risks / rollback

- Removing `repl` from the quick-start could break a README screenshot/test
  that references it — check `MANUAL_TESTING_PLAN.md` and screenshots before
  removing, and keep an alias if any documented flow uses it.
- Rollback: `git revert` the docs + generated-file changes; the `--check` mode
  stays in the CLI but defaults to a no-op unless `--check` passed, so no
  release path regresses.
