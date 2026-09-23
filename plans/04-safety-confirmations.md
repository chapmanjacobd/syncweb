# Plan 04 — Real safety prompts for destructive ops (`leave --delete-files`, `unshare --write`, `unshare --blob`)

Priority: HIGH · Status: Draft · Owner: `syncweb-cli`
Depends on: — · Fulfills story: #2 (Maya, "stop sharing / don't nuke") +
cross-cutting theme #5 (safety needs friction)

## Goal

The most destructive local-data operations — `leave --delete-files`,
`unshare --write`, and `unshare --blob` — must not run silently in
automation and must ask before they delete or revoke interactively,
matching the visual weight their consequences deserve.

## Evidence (verified in code)

- A reusable confirmation helper already exists: `confirm_destructive`
  (syncweb-cli/src/cli/output.rs:25-37). Its exact semantics today:
  - `output_json` set → returns `Ok(true)` (auto-approve).
  - stdin is not a TTY → returns `Ok(true)` (auto-approve — NOT a safe
    default).
  - otherwise → interactive `Confirm` dialog, default false.
  - So the current helper auto-approves in every non-interactive context. Wiring
    it into `leave`/`unshare` as-is would change nothing for the exact
    automation cases this plan cares about.
- It is wired into only 5 sites today:
  - `shutdown` (main.rs:480)
  - `snapshot delete` (main.rs:1174)
  - `package remove` (main.rs:3088)
  - `network leave` (main.rs:3892)
  - `network kick` (main.rs:3925)
- Not wired (silent destructive paths):
  - `leave --delete-files` → `handle_leave` (main.rs:4163) calls
    `manager.drop_when_ready(ns)` (main.rs:4181) then
    `FolderManager::delete_folder_files(path)` (main.rs:4185) with no prompt.
  - `unshare --write` / `unshare --blob` → `handle_unshare` (main.rs:2737) calls
    `folder.unpublish_blob(hash)` (main.rs:2759) or `remove_share(...)` +
    `unpin_all_content` (main.rs:2772-2774) with no prompt.
- The daemon path shares the same gap: both handlers fork at
  `daemon_client_or_start(...)` (main.rs:2742, 4167) and forward
  `IpcCommand::Unshare` / `IpcCommand::LeaveFolder { delete_files }` straight to
  the daemon without a CLI-side guard first.

## Scope guard

- CLI-side prompts + one helper behavior fix. No daemon/core changes.
- `--json` keeps its auto-approve (JSON is an explicit machine contract; a
  prompt would corrupt stdout). Non-TTY without `--json` becomes safe
  (abort) unless the caller opts in with `--yes`. This is the "interactive-only
  friction, scriptable-safe" contract, made real.

## Steps

### 1. Fix the helper so non-TTY is safe-by-default

- In `confirm_destructive` (output.rs:25-37), change the non-TTY branch from
  `return Ok(true)` to `return Ok(false)`:
  - `output_json` → `Ok(true)` (unchanged; explicit machine contract).
  - `!stdin.is_terminal()` → `Ok(false)` (changed; headless/cron aborts unless
    `--yes` is passed).
  - interactive → `Confirm::new()…default(false)` (unchanged).
- This changes behavior at the 5 existing sites (main.rs:480, 1174, 3088,
  3892, 3925): non-TTY invocations that previously auto-approved will now abort.
  That is the intended safety improvement, and is exactly what this plan wants
  for `leave`/`unshare`. Document the change in the changelog; scripts that
  relied on silent non-TTY execution must add `--yes` (step 4).

### 2. Wire the prompt into `leave`

- In `handle_leave` (main.rs:4163), when `command.delete_files` is set, call
  `confirm_destructive(&format!("permanently delete all local files for folder {selector}"), output_json || ctx.yes)?`
  before the `daemon_client_or_start` fork (main.rs:4167) so both the daemon
  and embedded branches get the same guard.
- Plain `leave` (no `--delete-files`) stays prompt-free.

### 3. Wire the prompt into `unshare`

- In `handle_unshare` (main.rs:2737), prompt before the
  `daemon_client_or_start` fork (main.rs:2742) when the operation removes write
  access (`--write`) or unpins a blob (`--blob`):
  - `"revoke write access to folder {selector}"`
  - `"remove the pin for shared blob {hash}"`
  Branch on which sub-flag is present; prompt only the affected capability, not
  both.
- Read-only `unshare` (no `--write`, no `--blob`) stays prompt-free. Note the
  code path for read-only unshare also calls `unpin_all_content` (main.rs:2772-2774),
  so it drops retention pins too; this is a deliberate scope choice — revoking a
  read ticket + pins is not a data loss ("blobs remain, just unpinned"), unlike
  `leave --delete-files`. If reviewers want pin-removal guarded too, add a third
  prompt string for the plain-unshare path in the same step.

### 4. Add a global `--yes` (skip-all-prompts) flag

- Add `yes: bool` to `Cli` (syncweb-cli/src/cli/args.rs, `#[arg(long, global = true, help = "Assume yes to every destructive-operation prompt")]`).
- Plumb it through `CliContext` (add a `yes: bool` field, args.rs:7-13) so
  handlers can pass it — `CliContext` is constructed at three sites in
  main.rs (172, 253, 262), all must add the field. `confirm_destructive`'s signature becomes
  `confirm_destructive(operation: &str, assume_yes: bool)` where the caller
  passes `ctx.output_json || ctx.yes`. Update the existing call sites
  (main.rs:480 `shutdown` and 1174 `snapshot delete` use `ctx` directly; 3892
  `network leave` and 3925 `network kick` are inside `handle_network(ctx, …)`,
  main.rs:3854, so they can too — but `package remove` is different:
  `handle_package_remove(&packages, &collection_id, &version, output_json)`
  (main.rs:3082) takes only `output_json`, no `ctx`, so combine at its call
  site (main.rs:2982): `handle_package_remove(..., output_json || ctx.yes)`).
  Otherwise `--yes` would be ignored there. Treat `--yes` as the explicit
  "I know what I'm doing" automation escape hatch: it sets the same `Ok(true)`
  branch as `output_json`, but is available on plain-TTY automation too.
- Keep `--json`'s auto-approve; `--yes` is for automation that doesn't want JSON
  but still wants no interaction.

## Tests

- `syncweb-cli/tests/cli_test.rs` (harness runs non-TTY, no `--json`):
  - `leave --delete-files` → files not deleted (non-TTY aborts; default safe).
  - `leave --delete-files --yes` → files deleted.
  - `unshare --write` → capability not revoked.
  - `unshare --write --yes` → revoked.
  - `shutdown` under non-TTY → does not shut down (regression on the 5
    existing sites now aborting); `shutdown --yes` → shuts down.
- Existing tests that invoke destructive commands non-TTY will now abort and
  must add `--yes`. Enumerate them so none is missed (grep `shutdown|unshare|
  "network", "leave"|"network", "kick"`):
  - `daemon_integration_test.rs`: 26 `shutdown` invocations (lines 114,
    150, 167, 215, 234, 253, 280, 323, 361, 392, 430, 473, 511, 561, 599, 640,
    765, 793, 865, 892, 952, 989, 1075, 1132, 1212-1213) — the `syncweb()` test
    helper makes this a mechanical `--yes` addition per call. (Re-verified: the
    count is exactly 26, matching this list.)
  - `daemon_integration_test.rs:549` (inside
    `test_daemon_leave_delete_files_via_ipc`, :520): `leave --delete-files <ns>` —
    asserts the directory is deleted on success (:556-559); without `--yes` it
    now aborts and the assert flips. (The plain `leave` calls at :507/:750 are
    unaffected.) The workflow helper `leave_delete_files` (mod.rs:123) is
    currently `#[expect(dead_code)]` — not called, so nothing to migrate there.
  - `cli_test.rs`: `shutdown --force` (:778-779), `unshare --write` (:1003-1008),
    `network leave` (:1253-1254), `network kick` (:1225-1230 — this test expects
    the kick to *fail*; without `--yes` the non-TTY abort exits 0 and flips the
    assertion).
  - `workflow/basic_sync.rs`: network leave (:216) and `snapshot delete` (:286)
    both run non-TTY and now abort; the workflow helpers `network_leave`
    (mod.rs:209) and `snapshot_delete` (mod.rs:269-277) take no `--json`/
    `--yes`, so add `--yes` in the helper (or at these two call sites).
  - `full_suite_test.rs:923`: `network leave` (plain `Command`, non-TTY, no
    `--json`) — unaffected by the `--json` branch but hit by the non-TTY abort;
    add `--yes`. (The `package remove --json` at :694 auto-approves, unchanged.)
  - Verify by running the full suite — any test that hangs is a missed prompt;
  any test that now aborts (or asserts on the aborted exit) is a missed `--yes`.

## Risks / rollback

- Headless scripts that passed `leave --delete-files`/`shutdown` expecting
  silent execution now abort (non-TTY returns false). Mitigation: `--yes` is
  the documented automation escape hatch, and `--json` still auto-approves.
  This is a deliberate, breaking behavior change for safety — call it out in the
  changelog rather than hiding it.
- `--json` auto-approving destructive ops remains a footgun for "scriptable
  safety"; note it in docs, but do not change it in this plan (changing it would
  corrupt JSON output). A future `--json --yes` explicit combination is
  preferred over silent auto-approve.
- Rollback: revert step 1's one-line change (`Ok(false)` → `Ok(true)`); the new
  prompts and `--yes` flag are additive and harmless to leave in place.

## Handoff notes

- `confirm_destructive` returns `Result<bool>`; callers must `?`-propagate the
  `false` (abort) path as a clean no-op exit, not an error — follow the existing
  `if !confirm_destructive(...) { return Ok(()); }` pattern (main.rs:480).
