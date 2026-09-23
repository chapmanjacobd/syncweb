# Plan 05 — One access dashboard: who can read/write each folder (and revoke)

Priority: HIGH · Status: Draft · Owner: `syncweb-cli`
Depends on: plan 04 (revocation gets its prompts) · Fulfills story: #2 (Maya)

## Goal

Maya should be able to answer "who currently has access to my Documents, and
can they write?" from **one command**, and revoke in place — the spreadsheet
she keeps mentally today (paste URL in chat → lose track → can't find the
revoke) becomes a table.

## Evidence (verified in code)

The data needed already exists but is split across four different views:

- Capabilities per folder live in `NodeDatabase.share_map` — surfaced by
  `share --list [<path>]` (`handle_share_list`, main.rs:2696-2734): shows ns +
  read/write ticket, blob tickets, persisted pins.
- **Who joined** (the peer side) lives in `node_db` device/share records:
  `handle_devices` (main.rs:4274-4291) lists devices + folders; network
  membership via `network list` (main.rs:3965).
- Folder mode (`sendreceive`/`receiveonly`/`sendonly`) is shown in `folders`
  (handle_folders, main.rs:4201-4268) — the “what can that side do” signal —
  but is **not correlated** with the share list.
- There is **no single query** that answers "give me every folder, its write
  capability, and the devices that hold it."

## Scope guard

- Read/aggregate + a small `unshare`-driven revoke. No daemon/core changes.
  Reuses existing IPC (`share --list` set + `devices` set + `networks ls`).

## Steps

1. Add `syncweb access [<path-or-ns>]` that renders one table:
   - Columns: `Folder` · `Mode` (sendreceive/…) · `Write?` (share capability) ·
     `Devices` (count + names) · `Networks` · `Reshare?` (persisted share rows).
   - Merge three existing sources: `FolderManager.list` (folders),
     `share --list` entries (write flags + pins), `devices`/`networks` (peer
     side). All three already run through the daemon/IPC path, so this is a
     presentation-layer join.
2. Add `--json` shape: `{folder, mode, write, devices: [...], networks: [...],
   shares: [{access, url}]}`.
3. Add `access --revoke <ns> [--read|--write]` sugar that dispatches to
   `handle_unshare` (main.rs:2737) with the appropriate flags — i.e. one verb to
   both see and revoke, with plan 04's confirmation running underneath.
4. Fulfils Maya's asymmetric-access story end-to-end: invite a phone with a
   **read-only** share, then when you later want to hand out edit rights,
   `access documents` shows the phone row without `Write`, and you issue a
   separate `--write` ticket rather than mutating the old one.

## Tests

- `syncweb-cli/tests/workflow_test.rs`: create folder + `share --write` +
  second device joins → `syncweb access` lists the folder once, with the
  joining device under `Devices` and `Write: true`; `access --revoke <ns>
  --write --yes` then flips the capability and `access --json` reflects it.
- `syncweb-cli/src/cli/output.rs` unit: table marker `Write?`/`Mode` columns
  present in both human + JSON renderings (guards against regression from the
  shared `confirm_destructive`-style skip logic in plan 04).

## Risks / rollback

- Aggregating the three sources adds a n+1 IPC fan-out on huge device lists;
  cap `Devices` cell at `N + "and M more"` (default 12) and keep a
  `--full-devices` escape hatch rather than printing thousands of rows.
- Rollback: `access` is a new verb; removing it restores the old four views.
  Aliases (`syncweb who`, `syncweb shares`) stay documented as deprecated.
</content>
