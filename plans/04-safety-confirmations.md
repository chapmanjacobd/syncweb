# Plan 04 — Real safety prompts for destructive ops (`leave --delete-files`, `unshare --write`, snapshot delete)

Priority: HIGH · Status: Draft · Owner: `syncweb-cli`
Depends on: — · Fulfills stories: #2 (Maya, "stop sharing / don't nuke"), #4 (Oli)

## Goal

The two most destructive local-data operations — `leave` with
`--delete-files` and `unshare --write` (and `unshare --blob` unpinning) — must
**always ask before they delete or revoke**, matching the visual weight their
consequences deserve. Today they run silently and immediately.

## Evidence (verified in code)

- A reusable confirmation helper **already exists**: `confirm_destructive`
  (syncweb-cli/src/main.rs import at :24, output.rs groups it) — prints
  "Are you sure you want to <operation>?" with a default of **false** and skips
  the prompt when `output_json` is set or stdin is not a TTY. It is wired into
  only 5 sites today:
  - `shutdown` (main.rs:480)
  - `snapshot delete` (main.rs:1174)
  - `package remove` (main.rs:3088)
  - `network leave` (main.rs:3892)
  - `network kick` (main.rs:3925)
- **Not wired** (silent destructive paths):
  - `leave --delete-files` → `handle_leave` (main.rs:4163) calls
    `manager.drop_when_ready(ns)` then `FolderManager::delete_folder_files(path)`
    (main.rs:4183-4185) with **no prompt**.
  - `unshare --write` / `unshare --blob` → `handle_unshare` (main.rs:2737) calls
    `folder.unpublish_blob(hash)` + removes the pin / revokes write access with
    **no prompt**.
  - `leave` on a **write-capable** shared folder (same risk surface as delete).
- The daemon path shares the same gap: `handle_leave` via
  `daemon_client_or_start` (main.rs:4167) forwards `LeaveFolder { delete_files }`
  straight to the daemon without the CLI-side guard first.

## Scope guard

- CLI-side prompts only. **No** daemon/core changes. `--json` and non-TTY flows
  keep the current behavior (prompt is auto-skipped, safe default `false` is
  enforced via `confirm_destructive`'s existing `output_json`/`is_terminal`
  branches) — this is exactly the "interactive-only friction, scriptable-safe"
  contract the tool already documents for the other 5 sites.

## Steps

1. **`leave`**: in `handle_leave`, when `command.delete_files` is set, call
   `confirm_destructive(&format!("permanently delete all local files for folder {selector}"), output_json)?` **before** `drop_when_ready`/`delete_folder_files`. Add `--yes` (see step 4) for the genuinely automated path.
2. **`unshare`**: in `handle_unshare`, prompt when the operation removes write
   access (`--write`) **or** unpins a blob (`--blob`):
   - `"revoke write access to folder {name}"`
   - `"remove the pin for shared blob {hash}"`
   Branch on which sub-flag is present; prompt only the affected capability, not
   both.
3. **Make the two runs symmetric** between embedded and daemon paths: extract the
   prompt into a small helper `async fn confirm_capability_change(desc, output_json)`
   called in `handle_leave`/`handle_unshare` **before** the
   `daemon_client_or_start` split (main.rs:2442-2449), so both the daemon and
   embedded branches get the same guard.
4. Add a global `--yes` (skip-all-prompts) convenience flag on `Cli`
   (syncweb-cli/src/cli/args.rs) that sets the same skip branch as `output_json`
   for `confirm_destructive`. Keep `--json`'s auto-skip; `--yes` is for plain
   TTY automation that explicitly wants no interaction.

## Tests

- `syncweb-cli/tests/cli_test.rs`:
  - `leave --delete-files` under a **non-TTY** stdin (test harness pipes
    `"n"`/EOF) → files **not** deleted (default false honored).
  - `leave --delete-files --yes` → files deleted.
  - `unshare --write` under non-TTY → capability **not** revoked.
  - `unshare --write --yes` → revoked.
- Keep existing workflow tests green: they already run `--no-daemon`/headless
  paths where `confirm_destructive`'s `is_terminal()` branch is false, so no
  existing test should need a `--yes` backfill; verify this by running the suite
  (if one test does hang on a prompt, add `--yes` to that invocation only).

## Risks / rollback

- Headless scripts that were passing `leave --delete-files` expecting silent
  execution now hang on a prompt. Mitigation: `--yes` is documented as the
  automation escape hatch, and `output_json`/non-TTY already skip. To be extra
  safe, `leave`'s prompt is only added for `--write`-capable folders and
  `--delete-files` — the two truly destructive combinations; plain `leave`
  (no delete) stays prompt-free.
- Rollback: remove the three `confirm_destructive` calls; the helper and `--yes`
  flag are additive and harmless to leave in place.
</content>
