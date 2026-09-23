# Plan 05 — One access dashboard: who can read/write each folder (and revoke)

Priority: HIGH · Status: Draft · Owner: `syncweb-cli`
Depends on: plan 04 (revocation gets its prompts + `--yes`) · Fulfills story: #2 (Maya)

## Goal

Maya should be able to answer "who currently has access to my Documents, and
can they write?" from **one command**, and revoke in place — the spreadsheet
she keeps mentally today (paste URL in chat → lose track → can't find the
revoke) becomes a table.

## Evidence (verified in code)

The data needed is **partially** present and split across four views, and one
piece is missing:

- Capabilities per folder live in `NodeDatabase` share records — surfaced by
  `share --list [<path>]` (`handle_share_list`, main.rs:2696-2734; the DB side
  is `list_shares`, syncweb-core/src/storage/node_db.rs:1432): shows ns +
  read/write ticket, blob tickets, persisted pins.
- Folder mode (`sendreceive`/`receiveonly`/`sendonly`) is shown in `folders`
  (`handle_folders`, main.rs:4201-4272) — the "what can that side do" signal —
  but is **not correlated** with the share list.
- Network membership lives in `network list` / `handle_networks`
  (main.rs:4294 → `handle_status_networks`, main.rs:4119); `network_manager`
  has `invite`/`kick`/`list` (syncweb-core/src/net/network_manager.rs).
- **Gap:** there is no "who has *joined* a folder" (inbound peer) list anywhere
  in the CLI or `node_db`. `handle_devices` (main.rs:4274-4292) only prints
  **this device's own** iroh/Syncthing identities — it does **not** list known
  peers. (`docs/commands.md:476` claims `devices` = "List known peers +
  connection status", which is doc-drift; see plan 07.)

## Scope guard

- Read/aggregate + a small `unshare`-driven revoke. No daemon/core changes.
  Reuses existing IPC (`share --list`, `folders`, `networks`).
- The "who joined" (inbound peer) column is **deferred**: exposing it requires a
  new core/IPC peer-list surface, which is out of scope here and should be filed
  as its own plan (or folded into the plan-07 `devices` doc-drift fix). Until
  then, `access` reports outbound shares + network members, and leaves the
  `Devices` column empty with a clear "inbound peers not yet surfaced" note.

## Steps

1. Add `syncweb access [<path-or-ns>]` that renders one table:
   - Columns: `Folder` · `Mode` (sendreceive/…) · `Write?` (share capability) ·
     `Shared with` (outbound share rows: ticket URL + access) · `Networks` ·
     `Pinned?` (persisted pins).
   - Merge three existing sources: `FolderManager.list` (folders, via
     `handle_folders` data path), `share --list` entries (write flags + pins),
     and `networks` membership. All three already run through the daemon/IPC
     path, so this is a presentation-layer join.
2. Add `--json` shape: `{folder, mode, write, shares: [{access, url, pinned}],
   networks: [...]}`.
3. Add `access --revoke <ns> [--read|--write]` sugar that dispatches to
   `handle_unshare` (main.rs:2737) with the appropriate flags — i.e. one verb to
   both see and revoke, with plan 04's confirmation + `--yes` running underneath.
4. Fulfils Maya's asymmetric-access story end-to-end: invite a phone with a
   **read-only** share, then when you later want to hand out edit rights,
   `access documents` shows the phone's share row without `Write`, and you issue
   a separate `--write` ticket rather than mutating the old one.

## Tests

- `syncweb-cli/tests/workflow_test.rs`: create folder + `share --write` →
  `syncweb access` lists the folder once, with the share row under `Shared with`
  and `Write: true`; `access --revoke <ns> --write --yes` then flips the
  capability and `access --json` reflects it.
- `syncweb-cli/src/cli/output.rs` unit: table markers `Write?`/`Mode` present in
  both human + JSON renderings.

## Risks / rollback

- Aggregating the three sources adds an n+1 IPC fan-out on large share/network
  lists; cap the `Shared with` cell at `N + "and M more"` (default 12) and keep
  a `--full` escape hatch rather than printing hundreds of rows.
- The missing inbound-peer list is a real limitation; guard against implying it
  works by printing the explicit "inbound peers not surfaced" note rather than
  an empty column.
- Rollback: `access` is a new verb; removing it restores the old views. Aliases
  (`syncweb who`, `syncweb shares`) stay documented as deprecated.

## Handoff notes

- Plan 06 (command collapse) must account for `access` — either fold it into
  `share` as `share --list`'s richer cousin, or keep it as one of the canonical
  verbs. Do not let plan 06 orphan it.
