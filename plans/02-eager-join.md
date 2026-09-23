# Plan 02 — Eager `join`: download + live-sync by default

Priority: HIGH · Status: Draft · Owner: `syncweb-cli`
Depends on: — · Fulfills story: #1 (Maya) — "I just want my Documents on my phone"

## Goal

Make `syncweb join <ticket> [path]` the happy path again. Today a first `join`
is coercive: it prints the namespace, persists nothing, downloads nothing, and
subscribes nothing — the user must already know the two hidden flags
(`--download-all`, `--subscribe`) before their phone has any files.

## Evidence (verified in code)

- `FolderJoin` (syncweb-cli/src/cli/commands.rs:304-344) declares
  `subscribe: bool` (commands.rs:319) and `download_all: bool`
  (commands.rs:338) with **no `default_value_t`** → clap gives them `false`.
  Default join = "track metadata only", the most confusing possible first-run
  behavior for a phone user.
- `handle_join` (main.rs:2442) wires the flags directly:
  - `config.set_subscribe(&namespace, command.subscribe, &filters)` (main.rs:2494)
  - `download_joined_folder(...)` only runs when `--download-all`
    (main.rs:2496-2500)
- The `subscribe`/`download_all` capabilities already exist and are idempotent:
  re-joining an already-tracked folder with `--subscribe` is a no-op path
  (main.rs:2456-2460). There's no reason the default can't exercise them.
- `download_all` composes with filters via `subscribe_filters_from` +
  `SubscribeFilters` (main.rs:2563-2572, consumed at main.rs:2496), so eager
  download needs no new filter work.

## Scope guard

- Flags + defaults only. **No** daemon/core protocol changes, **no** new schema.
- Keeps `create --import` behavior untouched (creating is still opt-in import).

## Steps

### 1. Flip the two defaults

- **Clap gotcha to avoid**: `#[arg(default_value_t = true, overrides_with = "no_subscribe")]`
  does **not** work as an opt-out — verified: when an arg with a default value
  is overridden, clap restores the default, so `--no-subscribe` alone still
  yields `subscribe == true`. Do not use `overrides_with` here.
- Instead, give both flags a default of `true` with **no** `overrides_with`,
  add plain companion `--no-*` bools, and compute the effective value in
  `handle_join`:
  - `subscribe: bool` → `#[arg(long, default_value_t = true)]` + new
    `no_subscribe: bool` → `#[arg(long, help = "Skip enabling live syncing on join")]`
  - `download_all: bool` → `#[arg(long, default_value_t = true)]` + new
    `no_download: bool` → `#[arg(long, help = "Join without downloading existing content")]`
  - Effective values (computed once at the top of `handle_join`, then used at
    main.rs:2457, 2471, 2473, 2494, 2496 — 2471 and 2473 are the two
    `IpcCommand::Join` fields sent to the daemon fork, `subscribe: command.subscribe`
    and `download: command.download_all`, so the **effective** values — not the
    paired raw flags — must be what the fork passes): `let subscribe = command.subscribe && !command.no_subscribe;`
    and `let download_all = command.download_all && !command.no_download;`.
  - Verified behavior: no flags → both `true`; `--no-subscribe` → subscribe
    `false`; `--no-download` → download_all `false`; both conflicting flags →
    `no_*` wins (deterministic).
- Both become **opt-out, not opt-in**:
  - `--no-subscribe` / `--no-download` are the only ways to keep the old lazy
    behavior.
- Keep `--subscribe` / `--download-all` as accepted no-ops for backward compat
  (existing scripts that pass them still work; they're default-true).
- Out-of-date message to update while there: the bail at main.rs:2458
  ("re-enable live syncing with `join <folder> --subscribe`") fires only on the
  `--no-subscribe` path now, but bare `join <folder>` already re-subscribes — so
  the suggested command is wrong; drop the `--subscribe` from the message.
- Files: `syncweb-cli/src/cli/commands.rs` (the two `#[arg]` default changes +
  the two new companion fields). `main.rs` `handle_join` gains the two
  effective-value lines above; the two `IpcCommand::Join` fields (main.rs:2471
  `subscribe:`, :2473 `download:`) must bind to those effective locals
  (`subscribe`/`download_all`) instead of the raw `command.*` fields. Nothing
  else in the handler changes.
- Result: first `join` prints namespace, accepts, subscribes for live sync, and
  downloads all current content — in one command, no flags learned.

### 2. Keep the eager run visible

- When the joined folder has content and `download_all` is now default-true,
  print a one-line progress summary at the end of join:
  `joined <ns> — live sync on; downloaded <n> files (<size>)`. Print the
  `— live sync on;` fragment only when the effective `subscribe` is true
  (`--no-subscribe` drops it), and print the summary itself only when the
  effective `download_all` ran (omit it under `--no-download` — the existing
  human branch at main.rs:2508-2510 must switch from `command.download_all` to
  the effective value so `--no-download` doesn't print `downloaded: 0 files`).
  Reuse existing progress-state affordances from plan 08 (progress/`--json`) —
  this plan only adds the final summary line, not a spinner. The existing
  `--json` branch (main.rs:2501-2505) already emits
  `{"status":"joined", ..., "downloaded": n}`; if the human line shows
  `(<size>)`, add `size` to the JSON too — `download_joined_folder` currently
  returns only a file **count**, so summing entry sizes needs a small change to
  that helper (it already walks `folder.list_entries()`, main.rs:2546-2560).

### 3. Preserve the lazy escape hatch in docs

- Document `join --no-download --no-subscribe` as the "I just want to look
  around" mode (this is exactly plan 01's story: join a huge folder, browse
  metadata lazily, fetch selectively later).

## Tests

- `syncweb-cli/tests/workflow_test.rs`:
  - `join <ticket>` with **no flags** → assert files on disk (download happened)
    AND `config` shows `subscribe: true` for the namespace.
  - `join <ticket> --no-subscribe` → assert `subscribe` not enabled.
  - `join <ticket> --no-download` → assert no files on disk (lazy preserved).
  - regression: `join` of an already-tracked folder stays idempotent.
- Unit (`syncweb-cli/src/cli/args.rs` or `commands.rs` `#[cfg(test)]`):
  parse-level assertion that `FolderJoin` parses with `subscribe`/`download_all`
  `true` by default, and that the effective-value computation
  (`subscribe && !no_subscribe`, `download_all && !no_download`) yields `false`
  for each `--no-*` flag and deterministic "no-wins" when both spellings are
  passed. Note: bool flags never render `[default: true]` in the grouped help —
  `spec_tail` (args.rs:142-144) skips args whose action takes no values
  (`ArgAction::SetTrue`), so the grouped-help formatting test cannot assert a
  `[default:]` tail on these flags.
- **Existing tests become download-bearing:** every current non-TTY `join`
  invocation now downloads by default — including the `workflow` helpers
  `join`/`join_with_options` (tests/workflow/mod.rs:106-116, used across
  `basic_sync.rs` and friends) and the daemon-mode join at
  daemon_integration_test.rs:426 (`join --subscribe --ingest-only`, which
  currently stays lazy; the existing `--download-all` joins at
  daemon_integration_test.rs:1049/:1191 — the `syncweb()` calls start at
  :1049/:1187 — are unaffected). They assert on folder tracking / capability,
  not on absence of files, so they keep passing — but any lazy-focused
  scenario (plan 01's tests)
  must pin `--no-download`, and the suite gets slower. Note this in the PR
  description rather than modifying every helper.

## Risks / rollback

- **Big folders on join**: phone now downloads everything by default. This is
  intentional for the happy path (Maya). Power users get the escape hatch
  (`--no-download`) and plan 01's lazy listing. A size/path filter on join
  already exists (`--max-size`, `--sync-prefix`, `--glob` on `FolderJoin`), so
  `join --no-download --glob 'no/movies'` stays available via subscribe filters.
- Rollback: revert the two `default_value_t = true` changes, remove the two
  companion fields, and drop the two effective-value lines in `handle_join`.
  Everything else is unchanged.

## Handoff notes

- The `mode` default (`receiveonly`, commands.rs:309) is out of scope and
  unchanged; only `subscribe` and `download_all` defaults move.
- Keep `handle_join_existing` (main.rs:2574) untouched — it already handles the
  already-tracked idempotent path.
