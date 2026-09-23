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

- `subscribe: bool` → `#[arg(long, default_value_t = true, overrides_with = "no_subscribe")]`
  and a new companion `no_subscribe: bool` → `#[arg(long, overrides_with = "subscribe")]`.
- `download_all: bool` → `#[arg(long, default_value_t = true, overrides_with = "no_download")]`
  and a new companion `no_download: bool` → `#[arg(long, overrides_with = "download_all")]`.
- Both become **opt-out, not opt-in**:
  - `--no-subscribe` / `--no-download` are the only ways to keep the old lazy
    behavior.
- Keep `--subscribe` / `--download-all` as accepted no-ops for backward compat
  (existing scripts that pass them still work; they're already default-true).
- Files: `syncweb-cli/src/cli/commands.rs` (the two `#[arg]` + the two new
  companion fields). `main.rs` `handle_join` stays the same code — it just now
  runs with `true` by default.
- Result: first `join` prints namespace, accepts, subscribes for live sync, and
  downloads all current content — in one command, no flags learned.

### 2. Keep the eager run visible

- When the joined folder has content and `download_all` is now default-true,
  print a one-line progress summary at the end of join:
  `joined <ns> — live sync on; downloaded <n> files (<size>)`.
  Reuse existing progress-state affordances from plan 08 (progress/`--json`) —
  this plan only adds the final summary line, not a spinner. The existing
  `--json` branch (main.rs:2501-2505) already emits `{"status":"joined", ...,
  "downloaded": n}`; add the file count there too.

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
- Unit: `args.rs` `spec_tail` — both flags now show `[default: true]` in help
  (this exercises the grouped help formatting; existing tests at args.rs:205).

## Risks / rollback

- **Big folders on join**: phone now downloads everything by default. This is
  intentional for the happy path (Maya). Power users get the escape hatch
  (`--no-download`) and plan 01's lazy listing. A size/path filter on join
  already exists (`--max-size`, `--sync-prefix`, `--glob` on `FolderJoin`), so
  `join --no-download --glob 'no/movies'` stays available via subscribe filters.
- Rollback: revert the two `default_value_t = true` lines + remove the two
  companion fields. Everything else is unchanged.

## Handoff notes

- The `mode` default (`receiveonly`, commands.rs:309) is out of scope and
  unchanged; only `subscribe` and `download_all` defaults move.
- Keep `handle_join_existing` (main.rs:2574) untouched — it already handles the
  already-tracked idempotent path.
