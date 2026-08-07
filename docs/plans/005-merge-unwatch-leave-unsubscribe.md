# Plan 005: Merge `unwatch`/`leave`/`unsubscribe`

Decisions D1-D6 resolved (code landed 2026-08-07); extended by decisions D7-D9 for the
`subscribe-changes` / live-sync model (decided 2026-08-07), whose config wiring is specified in
the "Affordance model" section and still to be implemented.

## Decisions needed

| # | Decision | Options | Recommendation | Chosen |
|---|----------|---------|----------------|--------|
| D1 | Single command name | `leave` / `forget` | `leave` (existing, documented) | `leave` |
| D2 | Model for "stop live syncing" | A: systemd-style `stop` / `disable` / `disable --now`; B: config-only per-folder `subscribe-changes` (runtime syncing = daemon running ∧ tracked ∧ enabled) | B (matches daemon reality) | B |
| D3 | Data-deletion flag | `--delete-files` / `--delete` | `--delete-files` (unambiguous) | `--delete-files` |
| D4 | Keep `unwatch` / `unsubscribe` as visible aliases | keep both / keep none | keep both | keep none |
| D5 | `--delete-files` keeps the destructive confirmation | yes / no | yes | no |
| D6 | Rename `IpcCommand::RemoveFolder` → `LeaveFolder` | yes / no | yes | yes |
| D7 | `join` default for `subscribe-changes` | on / off | off (membership ≠ live sync; join = track, `--subscribe` opts in) | off |
| D8 | Foreground live-sync loops (`join`/`subscribe` live loops) | keep / remove | remove — the daemon owns live sync via `subscribe-changes` config; only `start` runs in the foreground | remove |
| D9 | `daemon-sync <ns>` on a folder that is not live (untracked or `subscribe-changes` off) | start supervision anyway / warn + no-op | warn + no-op | warn + no-op |

Vocabulary rule (applies everywhere, CLI and IPC): the words purge, remove, and
detach must not appear as a command, flag, or IPC variant name in the merged surface.
`leave` covers membership, `--delete-files` covers deleting data, and live syncing is config
(`subscribe-changes`), never a verb. Plan 006 makes the mirror decision on the join side.

## Context for executing agent

- Repo: `syncweb-cli`. Three verbs for "stop doing something with a folder" at three
  granularities. Users cannot tell them apart from the help text, and the names (`unwatch`,
  `leave`, `unsubscribe`) all describe the same broad idea.
- Current state (all use `FolderSelector`, `cli/commands.rs:494-498`):
  - `Command::Unwatch` at `cli/commands.rs:19-20`; `handle_unwatch` at `main.rs:662-696`:
    resolves the folder to a namespace, then sends `IpcCommand::RemoveFolder`
    (`ipc.rs:76` → `handle_remove_folder` at `ipc.rs:841`), which cancels the session, drops
    the namespace from the docs engine, and untracks the folder. No confirmation.
  - `Command::Leave` at `cli/commands.rs:25-26`; `handle_leave` at `main.rs:3932-3963`:
    `confirm_destructive("leave this folder")`, sends `IpcCommand::LeaveFolder`
    (`ipc.rs:155` → `handle_leave_folder` at `ipc.rs:1758`), or in the embedded path
    `FolderManager::drop` + `cancel_session`. Behaviorally identical to Unwatch except for the
    confirmation.
  - `Command::Unsubscribe` at `cli/commands.rs:27-28`; `handle_unsubscribe` at `main.rs:3966-3999`:
    `confirm_destructive("unsubscribe from this folder")`, sends `IpcCommand::Unsubscribe`
    (`ipc.rs:152` → `handle_unsubscribe_command` at `ipc.rs:1715`), or `cancel_session` in the
    embedded path. This only cancels the live loop; the folder stays tracked.
- Key fact: the live loop is not a persisted state. Every daemon cycle, `run_cycle`
  (`daemon.rs:535-563`) calls `start_supervision` for every tracked folder. A transient
  "stop the live loop" is undone on the next cycle, so there is no meaningful "unsubscribed
  but still a member" state today. This is why D2 favors the config-only model.
- Vocabulary today: `RemoveFolder`, `LeaveFolder`, and `Unsubscribe` overlap as IPC names, and
  `unwatch`/`leave` are already behavioral duplicates. `Unsubscribe` also handles `blob:`
  public-blob subscriptions (`ipc.rs:1716-1737`), a separate feature that must stay reachable.
- All three duplicate the namespace-resolution helper pattern (see `handle_unwatch` at
  `main.rs:667-691` and the `resolve_namespace` helper used by leave/unsubscribe).
- Grouped help: all under `Folders` (`cli/args.rs:57-65`).

## Goal

One membership verb and one explicit deletion flag. `leave` is the inverse of `join` (plan
006): it ends membership. Deleting the local copy is never implied — it requires
`--delete-files`.

```sh
syncweb leave FOLDER                 # end membership; stop syncing; files untouched
syncweb leave FOLDER --delete-files  # end membership, then delete the local files
```

Live syncing is not a third verb. Under D2=B it is per-folder config (`subscribe-changes`):
the daemon only auto-supervises tracked folders that have it enabled, and whether a folder is
syncing right now is a runtime fact (daemon running ∧ tracked ∧ enabled), which is what
`run_cycle` actually does. Under D7/D8 there is no foreground live-sync loop at all —
`join`/`subscribe` never block on a sync session, the daemon owns live sync, and `leave`
unconditionally unsubscribes.

## Affordance model for `subscribe-changes` / live syncing (D2=B, D7-D9)

The full surface (replaces the old foreground `subscribe` loop):

| Intent | Command |
|---|---|
| Track a new folder (no live sync) | `syncweb join <ticket>` |
| Track + enable live syncing | `syncweb join <ticket> --subscribe [filters]` |
| Re-enable live syncing on a tracked folder | `syncweb join <folder> --subscribe` ≡ `config set subscribe-changes <folder> on` + daemon apply (idempotent) |
| Disable live syncing, stay a member | `config set subscribe-changes <folder> off` |
| Inspect the setting | `config show subscribe` (section listing per-folder enabled + filters) |
| End membership (always unsubscribes) | `syncweb leave FOLDER [--delete-files]` |
| Immediate background sync of a live folder | `daemon-sync [ns]` — skips folders that are not live (D9) |

Invariants:
- `join --subscribe` is idempotent. Whether the folder is new or already tracked, it
  resolves to the same operations: set the per-folder `subscribe-changes` flag (plus filters),
  then nudge the daemon to apply it now (reuse the `reload`/`TriggerSync` channel). A user who
  ran plain `join` earlier and later wants live sync runs exactly the same command as a fresh
  user who wants it from the start.
- `leave` always unsubscribes. Leaving ends membership, which implies live syncing stops;
  there is no `--unsubscribe` flag and no "left but still syncing" state. The folder's
  `subscribe-changes` entry is removed with the folder.
- No foreground live-sync loop (D8). The `subscribe` command and the interactive loop in
  `handle_join` are removed. The daemon's `start_supervision`/supervisor is the only live-sync
  machinery. `start` (daemon) and `watch` (folder watcher, plan 007) remain foreground
  long-running commands; everything else is request/response.
- Config, never a verb (vocabulary). The setting is the noun `subscribe-changes`; there is
  no `subscribe`/`unsubscribe` command. `IpcCommand::Subscribe` is dropped (no consumers);
  `IpcCommand::Unsubscribe` stays, restricted to `blob:` (plan 005, already landed).

Config shape: a new `Config.subscribe: SubscribeConfig` section
(`storage/config.rs`), a map of folder namespace → `{ enabled: bool, filters: SubscribeFilters }`
where `SubscribeFilters` is the six `join` filter flags (`--ingest-only`, `--ignore-self`,
`--prefix`, `--sync-prefix`, `--glob`, `--max-count`, `--max-size`). `config set` gains a
`subscribe-changes <namespace> on|off` key form (the folder is resolved like any other folder
selector). Default for a freshly joined folder is `enabled: off` (D7).

Daemon behavior changes:
- `run_cycle` (`daemon.rs:535-563`): only `start_supervision` folders whose
  `subscribe-changes` is `enabled: true`, combined with the existing schedule gate
  (`schedule_manager.is_active` → `set_intent_active`, `daemon.rs:548-559`). Live-active =
  tracked ∧ enabled ∧ schedule-active.
- `run_trigger(namespace)` (`daemon.rs:565-578`, the `daemon-sync <ns>` path): if the
  namespace is not tracked, print a warning that it is not a syncweb folder and do nothing
  (D9); if it is tracked but `subscribe-changes` is off, warn that live syncing is disabled
  and do nothing. `daemon-sync` with no argument still runs `run_cycle`, which skips disabled
  folders per the gate above.
- `join --subscribe`/`config set subscribe-changes … on` must also nudge an already-running
  daemon to apply the change immediately (the `TriggerSync`/reload channel), so the enable
  feels live rather than waiting for the next cycle.

## Structural change (do first)

1. In `cli/commands.rs`: replace `Command::Unwatch`, `Command::Leave`, `Command::Unsubscribe`
   with a single `Command::Leave(LeaveArgs)` and no aliases (D4=drop).
2. Define `LeaveArgs`: `pub folder: String` (reuse `FolderSelector` semantics) plus
   `#[arg(long)] pub delete_files: bool`.
3. In `ipc.rs` (D6): merge `RemoveFolder` and `LeaveFolder` into one `LeaveFolder` variant
   carrying `delete_files: bool`; drop the folder case of `Unsubscribe` (keep the `blob:`
   branch — see step 4).

## Execution steps

1. Write `handle_leave(ctx, LeaveArgs)` in `main.rs` (replaces all three handlers):
   - Extract the shared namespace resolution (from `main.rs:667-691` / `resolve_namespace`)
     into one helper used by every path.
   - Sends `IpcCommand::LeaveFolder { namespace, delete_files }` via
     `daemon_client_or_start`; embedded path mirrors it: `cancel_session` + `manager.drop`,
     then delete the folder path if `delete_files`.
   - `confirm_destructive` is not used anywhere in this handler (D5=no): plain `leave`
     is non-destructive and `--delete-files` is an explicit opt-in flag.
   - New daemon-side work in `handle_leave_folder`: after `manager.drop`, delete the
     materialized files at the tracked `FolderEntry.path`. Refuse `.`/`/` and any path that
     is (or is a parent of) the data dir / workspace — only the folder's registered path may
     be deleted.
2. Update dispatch at `main.rs:207-208` (remove `Unsubscribe` and `Leave` arms, replace the
   `Unwatch` arm at `main.rs:328`).
3. Update `help_categories!`: `Folders` category now lists `leave`; remove the `unwatch` and
   `unsubscribe` entries (`cli/args.rs:61-62`).
4. Blob subscriptions: the CLI surface for unsubscribing from `blob:` subscriptions is
   dropped with the `unsubscribe` command (D4=drop); the daemon IPC path
   (`IpcCommand::Unsubscribe`, restricted to `blob:` namespaces) remains for internal use
   until a replacement surface lands (candidate: `snapshot`/`transfer` after plans 008/014).
5. D2=B config wiring (the follow-up this plan now fully specifies, see "Affordance model"):
   - `storage/config.rs`: add `Config.subscribe: SubscribeConfig` (`enabled` + filters per
     folder namespace); extend `Config::set` with the `subscribe-changes <namespace> on|off`
     key form; `config show subscribe` lists it.
   - `run_cycle` (`daemon.rs:535-563`): gate `start_supervision` on `subscribe-changes`
     enabled ∧ schedule-active; drop the unconditional supervision of every tracked folder.
   - `run_trigger` (`daemon.rs:565-578`): untracked → warn "not a syncweb folder", no-op;
     tracked-but-disabled → warn, no-op (D9).
   - `join --subscribe` and `config set subscribe-changes … on` nudge the daemon via the
     `TriggerSync`/reload channel so the change applies immediately (no waiting for a cycle).
   - `leave` removes the folder's `subscribe-changes` entry along with the folder (leave
     always unsubscribes).
   - Drop `IpcCommand::Subscribe` / `handle_subscribe_command` (no foreground consumers left,
     D8). `IpcCommand::Unsubscribe` stays, restricted to `blob:`.
6. Foreground-loop removal (D8, shares plan 006): remove `Command::Subscribe` and the
   interactive live loop in `handle_join`; drop `join --once`; the six filter flags move into
   the persisted `SubscribeFilters` config. `watch` keeps its embedded `--no-daemon` watcher
   loop (it is a foreground watcher like `start`, not a live-sync loop).
7. Docs: `docs/commands.md` mapping table rows for `unwatch`, `unsubscribe`, `leave`;
   `README.md` if referenced.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
make completions && make manpage
rm -f man/syncweb-unwatch.1 man/syncweb-unsubscribe.1
./target/debug/syncweb help            # confirm one 'leave' under Folders
./target/debug/syncweb leave --help    # confirm --delete-files; unwatch/unsubscribe gone
git status
```

## Implementation notes (2026-08-07)

- `cli/commands.rs`: single `Command::Leave(LeaveArgs { folder, delete_files })`, no aliases;
  `FolderSelector` removed. `cli/args.rs`: `unwatch`/`unsubscribe` dropped from help categories.
- `ipc.rs`: `RemoveFolder` merged into `LeaveFolder { namespace, delete_files }`
  (`#[serde(default)]`); `handle_leave_folder` cancels the session, drops the namespace, removes
  the folder entry, then `FolderManager::delete_folder_files(entry.path)` when `--delete-files`.
  `Unsubscribe` remains as a daemon-internal IPC restricted to `blob:` namespaces (D4=drop; no
  CLI surface). Non-blob namespaces are rejected with "use `leave` for folders".
- `main.rs`: merged `handle_leave(ctx, LeaveArgs)` (no `confirm_destructive`), shared
  `resolve_namespace_via_daemon` helper; `handle_unwatch`/`handle_unsubscribe` removed.
- `folder/manager.rs`: added `FolderManager::delete_folder_files` (refuses `/`, the current
  directory, and non-file/non-dir paths) and `drop_when_ready` (bounded retry while the live
  session replica is still closing, resolving the "replica is not closed" race that the strict
  `test_daemon_leave_untracks_via_ipc` surfaced after a `daemon-sync`).
- Tests: `test_daemon_leave_delete_files_via_ipc` and `test_daemon_leave_untracks_via_ipc`
  (strict) replace the old unwatch/unsubscribe integration tests; IPC tests updated for
  `LeaveFolder { namespace, delete_files }` and blob-only `Unsubscribe`.
- Still to implement (D7-D9, specified in "Affordance model" + execution step 5/6): the
  `Config.subscribe` section and `config set subscribe-changes … on|off`; the `run_cycle`
  supervision gate; the `run_trigger` warn+no-op for untracked/disabled folders; the removal
  of the `subscribe` command and the `join`/`subscribe` foreground live loops (shares plan
  006); `join --subscribe` idempotent enable. `watch` keeps its embedded `--no-daemon` loop.
- `docs/commands.md` has no `unwatch`/`unsubscribe` rows (only a `daemon-remove` legacy row,
  untouched), so no table changes were needed.

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/cli/args.rs`
- `syncweb-cli/src/main.rs`
- `syncweb-core/src/daemon/ipc.rs`
- `syncweb-core/src/daemon/daemon.rs` (`run_cycle` gate, `run_trigger` warn+no-op, D7-D9)
- `syncweb-core/src/storage/config.rs` (`Config.subscribe`, `Config::set` key form)
- `completions/*`, `man/*` (regenerated)
- `docs/commands.md`

## Dependencies

Decide together with 006 (`join`/`subscribe`) — both change the folder lifecycle verbs and
must share the vocabulary (D1/D6 here align with the `join`/`leave` naming there). D8 (no
foreground live-sync loops) removes the premise of 006's original "re-enter the loop" model;
006 is rewritten accordingly. D2=B (subscribe-changes) touches the daemon `run_cycle`
auto-supervision (`daemon.rs:535-563`) and per-folder persistence; wiring is specified here and
executed as a follow-up.
