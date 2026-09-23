# Plan 02 — Eager `join`: download + live-sync by default

Priority: HIGH · Status: Draft · Owner: `syncweb-cli`
Depends on: — · Fulfills story: #1 (Maya) — "I just want my Documents on my phone"

## Goal

Make `syncweb join <ticket> [path]` the happy path again. Today a first `join`
is coercive: it prints the namespace, persists nothing, downloads nothing, and
subscribes nothing — the user must already know the two hidden flags
(`--download-all`, `--subscribe`) before their phone has any files)Skip-comment: keep in file? – see note at top.

## Evidence (verified in code)

- `FolderJoin` (`syncweb-cli/src/cli/commands.rs:303-344`) declares
  `subscribe: bool` and `download_all: bool` with **no `default_value_t`** →
  clap gives them `false`. Default join = "track metadata only", the most
  confusing possible first-run behavior for a phone user.
- `handle_join` (main.rs:2442) wires the flags directly:
  - `config.set_subscribe(&namespace, command.subscribe, &filters)` (main.rs:2494)
  - `folder.list_entries()` → `download_joined_folder(...)` only when
    `--download-all` (main.rs:2496-2510)
- The `subscribe`/`download_all` capabilities already exist and are
  idempotent: re-joining an already-tracked folder with `--subscribe` is a no-op
  path (main.rs:2449-2460). There's no reason the default can't exercise them.
- `join --download-all` uses `subscribe_filters_from` + `ContentFilter`
  (main.rs:2501-2510) so filters compose with eager download with no new work.

## Scope guard

- Flags + defaults only. **No** daemon/core protocol changes, **no** new schema.
- Keeps `create --import` behavior untouched (creating is still opt-in import).

## Steps

### 1. Flip the two defaults

- `subscribe: bool` → `#[arg(long, default_value_t = true)]` and
- `download_all: bool` → `#[arg(long, default_value_t = true)]`.
- Both become **opt-out, not opt-in**:
  - `--no-subscribe` / `--no-download` (clap `overrides_with`) — the only way to
    keep the old lazy behavior.
- Keep `--subscribe` / `--download-all` as accepted no-ops for backward compat
  (existing scripts that pass them still work).
- Files: `syncweb-cli/src/cli/commands.rs` (the two `#[arg]`), and `main.rs`
  `handle_join` stays the same code — it just now runs with `true` by default.
- Result: first `join` prints namespace, accepts, subscribes for live sync, and
  downloads all current content — in one command, no flags learned.

### 2. Keep the eager run visible

- When the joined folder has content and `--download-all` is now default-true,
  print a one-line progress summary at the end of join:
  `joined <ns> — live sync on; downloaded <n> files (<size>)`.
  Reuse existing progress-state affordances from plan 08 (progress/`--json`) —
  this plan only adds the final summary line, not a spinner.

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
  (this exercises the grouped help formatting, existing tests at args.rs:204).

## Risks / rollback

- **Big folders on join**: phone now downloads everything by default. This is
  intentional for the happy path (Maya). Power users get the escape hatch
  (`--no-download`) and plan 01's lazy listing. A `--max-size`/`--min-size`
  filter on join already exists (`ContentFilter`) so `join --no-download
  --glob 'no/movies'` stays available via subscribe filters.
- Rollback: revert the two `default_value_t = true` lines. Everything else is
  unchanged.
</content>
