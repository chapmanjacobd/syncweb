# Plan 10 — Complete the command collapse for the major release (re-home hidden commands)

Priority: HIGH (major-release surface) · Status: Draft · Owner: `syncweb-cli`
Depends on: 06 (done), 07 (done) · Reason: user is cutting a new major release and
wants the hidden legacy surface gone, not just hidden.

## Goal

Plan 06 hid 21 legacy `Command` variants behind `#[command(hide = true)]` and
left 17 visible. For the major release we **delete** the hidden top-level
variants and **re-home their functionality** under noun-group verbs (or rename
the verb), so the grouped help *is* the whole surface: 30 top-level verbs,
zero hidden. `--show-legacy` is removed (nothing is hidden anymore).

Not a rename for compat: this is the break. Legacy spellings (`syncweb create`,
`syncweb networks`, …) stop parsing; docs/tests/men move to the new homes.

## Design principles

1. Group by **noun/subject**, not by action. `folders`, `network`, `snapshot`,
   `package`, `link`, `indexing`, `stats`, `db`, `transfer`, `config` own their
   sub-operations. This is why `folders create` is fine but `start status` is
   not (the user rejected action-nesting like `start status` / `start stop`).
2. Keep **flat single-purpose action verbs flat** when nesting reads badly:
   `start`, `stop`, `status`, `reload`, `sync`, `ls`, `stat`, `find`, `search`,
   `sort`, `download`, `verify`, `share`, `access`, `watch`.
3. More than 12 top-level verbs is acceptable; every one must be distinct and
   read naturally. The 12-verb constraint in plan 06 is dropped.
4. No backward-compat aliases for re-homed commands. The only renames are
   `shutdown`→`stop` and `daemon-sync`→`sync`; whether those old spellings stay
   as invisible aliases is the one open compat question (see Open decisions).

## Target surface (30 top-level = 25 functional + 5 meta)

| Category | Verbs |
|----------|-------|
| Daemon | `start`, `stop`, `status`, `reload`, `sync`, `devices` |
| Folders | `folders` (subcommands: `create`, `join`, `leave`, `import`; bare = list) |
| Files | `ls`, `stat`, `find`, `search`, `sort`, `download`, `verify`, `transfer` |
| Sharing & Access | `share`, `access`, `link`, `package` |
| Network | `network` |
| Automation | `watch`, `snapshot` |
| Indexing | `indexing` |
| Statistics | `stats` |
| Maintenance | `db` |
| Configuration | `config` |
| Tooling | `version`, `completions`, `manpages`, `help` |

Net change vs today's 38 variants: 8 top-level variants are **deleted and
re-homed** as subcommands (`create`/`join`/`leave`/`import` → `folders`,
`networks` → `network`, `publish` → `indexing`, `unshare` → `access`,
`provider` → `share`), 2 are **renamed** (`shutdown`→`stop`, `daemon-sync`→`sync`),
and the remaining 11 hidden variants (`status`, `reload`, `search`, `sort`,
`stat`, `verify`, `transfer`, `access`, `package`, `link`, `db`) become visible
top-level verbs unchanged. 17 currently-visible verbs stay as-is.

## Command-by-command changes

All locations cite the current tree (post-plan-06).

### Re-homed → `folders` (commands.rs: `Command::Create` :23, `Join` :28, `Leave` :30, `Import` :51)

`folders` becomes a subcommand container like `config`/`snapshot`:
`Command::Folders { command: Option<FoldersCommand> }`; bare `folders` = list
(main.rs `handle_folders` :4507).

- `folders create <dir>` — `FolderCreate` args, main.rs `handle_create` :2387
- `folders join <ticket>` — `FolderJoin` args, main.rs `handle_join` :2553 (+
  `handle_join_existing` :2720)
- `folders leave <folder>` — `LeaveArgs`, main.rs `handle_leave` :4457
- `folders import <path>` — `ImportArgs`, main.rs `handle_import` :731

Subcommand arg structs keep their exact flags (no reshape of the
`FolderCreate`/`FolderJoin`/`LeaveArgs`/`ImportArgs` fields).

### Re-homed → `network`

Delete `Command::Networks` (commands.rs :17). `networks` currently renders
list + health via `handle_networks` :4606 → `handle_status_networks` :4413.
`network list` (NetworkCommand::List) shows the list only. Add a
`network status [name]` subcommand that calls `handle_status_networks` so the
health view survives. Delete `NetworkListArgs` + `handle_networks` + the
`Command::Networks` dispatch arm.

### Re-homed → `indexing`

Delete `Command::Publish` (commands.rs :72). `indexing` (already visible,
`IndexingCommand` in commands.rs :968+) gains `IndexingCommand::Publish(PublishCatalogArgs)`.
`handle_publish` :2769 is already a one-line shim to
`cli::indexing::handle_catalog_publish` (indexing.rs :65); move that arm into
`handle_indexing`. Delete `PublishCommand` enum + `handle_publish`.

### Re-homed → `access` (absorbs `unshare`)

Delete `Command::Unshare` (commands.rs :79). `unshare` has two jobs: revoke a
folder/write share (`unshare <path> --write`) and revoke a blob
(`unshare --blob <hash>`). `access --revoke` (main.rs `handle_access` :2956)
already revokes folder shares in place. Add a `--blob <hash>` flag to
`AccessArgs` (commands.rs :801) so `access --revoke <ns> --blob <hash>`
replaces `unshare --blob`. The `--read` default stays prompt-free; blob revoke
reuses plan 04's confirmation path already in `handle_access`.
Delete `UnshareArgs` + `handle_unshare` :2886.

### Re-homed → `share` (absorbs `provider`)

Delete `Command::Provider` (commands.rs :108). `provider add <collection>
<provider-ticket>` is a single subcommand (`ProviderCommand`, commands.rs :1022,
handled by `cli::indexing::handle_provider`). Convert `share` from
`Share(ShareArgs)` (commands.rs :77) to a container that keeps the positional
share path + flags and adds subcommands:
`ShareArgs` gains `#[command(subcommand)] command: Option<ShareCommand>` with
`args_conflicts_with_subcommands = true`, where `ShareCommand` = `List` (replaces
the `--list` flag; main.rs `handle_share_list` :2845) and
`Provider { collection, provider }` (indexing.rs `handle_provider`).
Delete `ProviderCommand` + its dispatch arm.

### Renamed (flat)

- `Shutdown(ShutdownArgs)` (commands.rs :11) → `Stop(ShutdownArgs)`; `about`
  becomes "Stop the local syncweb daemon". `handle_shutdown` :490 unchanged.
  Invisible alias `shutdown` optional (Open decision 1).
- `DaemonSync(DaemonSyncArgs)` (commands.rs :21) → `Sync(DaemonSyncArgs)`;
  `about` becomes "Ask the local daemon to trigger synchronization".
  `handle_daemon_sync` :675 unchanged. Invisible alias `daemon-sync` optional.

### Un-hidden to visible flat verbs

Remove `hide = true` from: `Status` (:13), `Reload` (:19), `Search` (:43),
`Sort` (:45), `Stat` (:47), `Verify` (:70), `Transfer` (:58), `Access` (:81),
`Package` (:83), `Link` (:103), `Db` (:93). Dispatch arms, handlers, and arg
structs are unchanged. `stats`, `watch`, `snapshot`, `indexing`, `config`,
`network`, `share`, `download`, `ls`, `find`, `start`, `folders`, `devices`,
`version`, `completions`, `manpages`, `help` already-visible stay as-is.

## Implementation steps

1. **commands.rs**: delete re-homed variants (`Create`, `Join`, `Leave`,
   `Import`, `Networks`, `Publish`, `Unshare`, `Provider`); rename `Shutdown`→
   `Stop`, `DaemonSync`→`Sync`; remove `hide = true` from the 11 un-hidden
   verbs; convert `folders` and `share` to subcommand containers; add
   `FoldersCommand`, `ShareCommand`; add `AccessArgs.blob`, `NetworkCommand::
   Status`, `IndexingCommand::Publish`. Remove now-unused arg structs/enums
   (`NetworkListArgs`, `PublishCommand`, `UnshareArgs`, `ProviderCommand`).
2. **args.rs**: rewrite `help_categories!` for the 11 new categories (the macro
   keeps its exhaustive `category_of` over `Command`). Remove the
   `show_legacy` path from `print_grouped_help`/`build_grouped_help` and the
   `--show-legacy` handling in main.rs (main.rs `top_level_help_exit_code` :124,
   `main` :111, `handle_help` :152). Update unit tests (drop the
   `show_legacy`/legacy-visibility tests; keep `all_subcommands_are_categorized`).
3. **main.rs**: update the `execute_cli` match (:182-229) — new variants,
   deleted arms; update `is_auxiliary_command` (:238) + `execute_auxiliary_command`
   (:253) for `Stop`/`Sync`; delete dead handlers (`handle_networks`,
   `handle_publish`, `handle_unshare`, `handle_create`/`join`/`leave`/`import`
   get re-wired under `handle_folders`); add `folders <sub>` dispatch +
   `network status` arm + `share` subcommand dispatch. Keep every moved handler
   body intact, only re-point the entry.
4. **cli/indexing.rs**: add `IndexingCommand::Publish` arm in `handle_indexing`
   (:25) calling `handle_catalog_publish` (:65); add `ShareCommand::Provider`
   path calling `handle_provider`; delete `ProviderCommand` import if unused.
5. **Tests**: rewrite the ~142 legacy-spelling call sites
   (`syncweb-cli/tests/{cli_test,daemon_integration_test,full_suite_test,interop_test}.rs`)
   to the new homes — `"folders","create"`, `"folders","join"`, `"folders",
   "leave"`, `"folders","import"`, `"network","list"`/`"network","status"`,
   `"indexing","publish"`, `"access","--revoke"`, `"share","provider","add"`,
   `"stop"`, `"sync"`. Add assertion: `--help` shows exactly the 30 verbs and
   none of the 8 deleted spellings parse.
6. **Regenerate artifacts**: `make manpage completions`; delete stale
   `man/syncweb-*.1` for deleted top-level verbs (create/join/leave/import/
   networks/publish/unshare/provider) and add ones for the new verbs
   (stop/sync). Subcommand pages (`folders create`, …) are not generated
   individually (consistent with existing `config`/`network` handling).
7. **Docs**: update `docs/commands.md` (drop moved/renamed top-level rows, add
   new subcommand rows), `MANUAL_TESTING_PLAN.md` sections that use
   `syncweb create`/`sort`/`db`/… to the new homes, and `plans/00-index.md`
   (mark this plan; note plan 07 done and 06's hide phase superseded).

## Tests

- `args.rs` unit: `grouped_help_lists_canonical_verbs` now asserts exactly the
  30-verb set; legacy-visibility tests are deleted (nothing hidden).
- `cli_test.rs`: new `help_lists_only_major_surface` (30 verbs present, the 8
  deleted spellings absent); `folders` bare-list vs `folders create` both work;
  `network list` == old `networks` output (minus health) and `network status`
  preserves health; `access --revoke --blob` replaces `unshare --blob`;
  `share provider add` == old `provider add`; `stop` == old `shutdown`
  behavior. Legacy `create`/`join`/`sort`/… tests re-spelled to new homes.
- `daemon_integration_test.rs` / `full_suite_test.rs` / `interop_test.rs`:
  mechanical re-spelling of the 142 call sites; behavior assertions unchanged.

## Open decisions (defaults chosen; confirm or override)

1. **Rename aliases**: keep `shutdown` and `daemon-sync` as invisible aliases
   of `stop`/`sync`, or drop them entirely? Default: drop `daemon-sync`, keep
   `shutdown` as an invisible alias (pure synonym, one attribute). Say the word
   to drop both.
2. **`verify`** stays flat (not folded under `download`). Alternative if fewer
   verbs wanted: `download verify <path>`.
3. **`provider`** folds under `share` as `share provider add` per plan 06.
   Alternative: keep a tiny flat `provider` verb.
4. **`networks` health view** moves to a new `network status [name]`.
   Alternative: fold health into `network list`.

## Risks / rollback

- This is the breaking change of the major release: every legacy spelling stops
  parsing. Rollback is re-adding the deleted variants (their handlers are only
  re-pointed, not deleted, so nothing is lost).
- `share`/`folders` become subcommand containers; the positional `share <path>`
  path must keep working via `args_conflicts_with_subcommands` + a defaulting
  subcommand field. Pin this with a test that `share .` and `share list` both
  parse.
- 142 test call sites is mechanical churn; keep behavior assertions identical
  so the churn stays greppable (`git diff` should show renames only).

## Definition of done

- `cargo test --workspace` passes.
- `cargo clippy --workspace --all-targets` clean.
- `cargo fmt --check` clean.
- `syncweb --help` shows exactly the 30-verb surface; none of the 8 deleted
  spellings parse.
- man pages + completions regenerate without orphaned/unimplemented commands.
- `docs/commands.md` + `MANUAL_TESTING_PLAN.md` updated; `plans/00-index.md`
  marks this plan.