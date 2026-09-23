# Plan 07 — Kill stale man pages, completions, and doc-listed-but-absent commands (docs drift is a bug)

Priority: MEDIUM · Status: Draft · Owner: `syncweb-cli`
Depends on: — (re-run after plan 06, which changes the canonical verb set) ·
Fulfills story: #5 (Sam — "docs list `accept`/`drop` that don't exist; docs
drift is itself a UX trap") + cross-cutting theme #4 (docs drift is a bug)

## Goal

Every shipped artifact must describe a real command. Today docs/man pages list
verbs that **do not exist** in the `Command` enum (`mirror`, `repl`, `accept`,
`drop`, `conflicts`, `pending`, `deleted`, `undelete`, `policy`, `public list`),
and several documented commands do something different from what the docs
claim. Docs drift is a **user-facing bug**: it teaches users flags/verbs that
error at dispatch or behave differently than promised.

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
- **Full inventory of stale references (grep for the orphan names):**
  - `docs/commands.md`: `accept`/`drop` (:473-474), `devices`/`ls`/`find`
    behavior drift (:476-478), `repl` (:491), `conflicts` (:540),
    `deleted`/`undelete` (:605-609), `conflicts --resolve --auto-resolve`
    + `pending` example block (:687-693).
  - `docs/overview.md`: `accept`/`drop`/`repl` command list (:52-53),
    `conflicts` UX (:145), `devices`/`accept` (:266).
  - `docs/phases.md`: `repl` (:14), `accept`/`drop` (:19, :21).
  - `docs/testing.md`: functional-parity list including `accept`/`drop`/`repl`
    (:108) and `conflicts` risk row (:88).
  - `docs/offline-conflict.md`: `pending` (:45-46), `conflicts` examples
    (:158-172).
  - `docs/indexing.md`: `mirror` subcommands (:136, :148).
  - `syncweb-cli/README.md`: `repl` (:30).
  - `MANUAL_TESTING_PLAN.md`: `drop` row (:112), `mirror` rows (:353-356).
  - `man/syncweb-mirror.1` (orphan man page).
  - **Additional absent commands (first-audit miss, re-verified):** `policy`
    (`docs/commands.md:493`), `public list` (:494), and the `export`
    walkthrough (:621-622 `syncweb export …`) are **not** `Command` variants
    either — treat like `repl` (remove the rows). Top-level `export` is stale
    in two more places too: `docs/commands.md:827` ("parallel is default for
    ls, import, export") and `MANUAL_TESTING_PLAN.md:201-202` (`syncweb export
    …`) — `export` only exists as `package export` (commands.rs:873), so rewrite
    those to `syncweb package export` or delete them. Also
    `docs/overview.md:204` references an "undelete" feature, and
    `MANUAL_TESTING_PLAN.md` has `conflicts` rows (:429-433) and `pending` rows
    (:444-446) — pick all of these up in the same sweep as the other absent
    verbs.
  - **Flag-level drift (docs advertise flags the commands don't take):**
    `download --limit` (:564) / `download --size` (:567),
    `folders --limit-upload` (:642), `devices --peer-limit` (:643),
    `devices --bep` (:684), `find --glob` (:658) / `find --min-size` (:659),
    and `search --bootstrap`/`--timeout-ms` (:479). Correct or delete the
    flags. The `find` examples at :132 (`find --glob '/*.mp3'`) and :138
    (`find --type f --ext mp3 --min-size 10MB`) use the same two unwired
    spellings — fix those blocks in the same sweep. Note plan 03 makes `download --size`/`--ext` and `find`'s size/ext
    real — re-check those two rows *after* plan 03, not now; the
    `folders`/`devices`/`search` flags stay unwired and should be deleted or
    marked unsupported.
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
     remove the verb rows (docs/commands.md:473-474, docs/overview.md:52-53,
     docs/phases.md:19,21, docs/testing.md:108).
   - `conflicts`/`pending`/`deleted`/`undelete` → fold the conflict/resolution
     half into a future plan; remove the verb rows + example blocks
     (docs/commands.md:540, 605-609, 687-693; docs/offline-conflict.md:45-46,
     158-172; docs/overview.md:145).
   - `repl` → remove from docs and the CLI README quick-start (no
     implementation): docs/commands.md:491, docs/overview.md:53,
     docs/phases.md:14, docs/testing.md:108, syncweb-cli/README.md:30.
   - `policy` / `public list` / `export` → `policy` and `public list` have no
      implementation behind any plan, so remove their rows (docs/commands.md:493-494)
      **and the `syncweb public list` row at MANUAL_TESTING_PLAN.md:234** (the
      sweep missed it in the first audit).
      Top-level `export` (docs/commands.md:621-622, :827; MANUAL_TESTING_PLAN.md:201-202)
      is a stale spelling — the real command is `package export` — so rewrite the
      references to `syncweb package export` rather than dropping the (implemented)
      feature.
   - `conflicts`/`pending`/`deleted`/`undelete` rows in
      MANUAL_TESTING_PLAN.md (:429-433, :444-446) and the `docs/overview.md:204`
      "undelete" mention → remove alongside their docs/commands.md rows above.
   - `mirror` → remove `man/syncweb-mirror.1` and the docs/indexing.md
      references (:136, :148) and the MANUAL_TESTING_PLAN.md mirror section
      (:353-356); `mirror` is core-only (`register_mirror`), never a CLI verb.
   - `devices`/`ls`/`find` behavior drift → the `ls`/`find` rows
     (docs/commands.md:477-478) become **accurate** when plan 01 implements
     metadata-first `ls`/`find`/`sort` — keep them, and add their new
     `--local-only`/`--remote-only`/`--path-glob`/`--no-enrich` flags to the
     flag tables; only `devices` (docs/commands.md:476) has no implementing
     plan, so correct that row to match `handle_devices` or delete it.
   - Flag drift (see Evidence): correct the `find`/`download` rows after plan 03
     (they become real), and delete or mark unsupported the unwired
     `folders`/`devices`/`search` flags.
3. **Automate the fix** so it can't regress:
   - Add a `syncweb manpages --check` (or a `syncweb-cli/tests/docs_drift_test.rs`)
     that regenerates man/completions into memory, diffs them against the repo
     copies, and fails if any committed generated file is stale **or** any
     registered command is missing its artifact.
   - Wire the check into CI (Makefile `docs-check` target if present).
4. Update `docs/commands.md`'s command-mapping table to exactly match the
    implemented verb set (plan 06's verbs + aliases), deleting the
    `accept/drop/conflicts/pending/deleted/undelete/repl/policy/public list`
    rows (the `export` references are prose/walkthrough, not mapping-table rows;
    those are handled in step 2), keeping the `ls`/`find` rows as-is (accurate
    after plan 01's metadata-first port), correcting only the `devices`
    description and the flag-drift rows from Evidence. Also remove the
    `syncweb drop` row, the top-level `export` rows, and the
    `conflicts`/`pending` rows in MANUAL_TESTING_PLAN.md (:112, :201-202,
    :429-433, :444-446).

## Tests

- `syncweb-cli/tests/docs_drift_test.rs`:
  - regenerated `syncweb --help` grouped output has no subcommand that lacks a
    `Command` variant (exhaustive — reuse `all_subcommands_are_categorized`).
  - every `man/*.1` file's command name exists in `Command` (presence of an
    orphan file is the bug).
  - the `repl` references and the `Command` enum agree: if docs/READMEs say
    "repl", the CLI must say "repl" (assert one way and fail loudly on
    mismatch — currently `docs/commands.md:491` and `syncweb-cli/README.md:30`
    say it and the CLI doesn't, so removal is the fix). Note `docs/README.md`
    itself has no `repl` reference today.
- Assert the generated completions for each shell parse (smoke test in CI, e.g.
  fish).

## Risks / rollback

- Removing `repl`/`mirror`/`drop` from the quick-start and manual-testing docs
  could break documented flows that reference them — grep `MANUAL_TESTING_PLAN.md`
  (:112 drop, :353-356 mirror), `docs/phases.md`, `docs/testing.md:108`, and
  `syncweb-cli/README.md:30` before removing; keep an alias if any documented
  flow uses it.
- Rollback: `git revert` the docs + generated-file changes; the `--check` mode
  stays in the CLI but defaults to a no-op unless `--check` is passed, so no
  release path regresses.
