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
  is `list_shares`, syncweb-core/src/storage/node_db.rs:1432). **Scope check:**
  the share list carries only `(namespace, access, ticket)` — no pins and no
  blob tickets, in both the embedded path (main.rs:2720-2732) and the daemon
  path (`handle_share_list`, ipc.rs:1707-1730). So a `Pinned?` column has no
  share-table source; pins live in the blob store
  (`blob_store.list_pins(public_blob_pin prefix)`, node/blob_store.rs:238).
- Folder mode (`sendreceive`/`receiveonly`/`sendonly`) is shown in `folders`
  (`handle_folders`, main.rs:4201-4272) — the "what can that side do" signal —
  but is **not correlated** with the share list. **Scope check:** the daemon
  `FolderList` response uses `FolderStatusReport` (state.rs:80), which has no
  `mode` field; only the embedded (`--no-daemon`) `handle_folders` path prints
  mode. The `access` `Mode` column therefore needs mode added to the daemon
  `FolderStatusReport` (small core/IPC addition, contradicts "no daemon/core
  changes") **or** `access` must gather mode from the embedded path. Decide in
  step 1; do not silently ship an empty `Mode` column.
- Network membership lives in `network list` / `handle_networks`
  (main.rs:4294 → `handle_status_networks`, main.rs:4119); `network_manager`
  has `invite`/`kick`/`list` (syncweb-core/src/net/network_manager.rs).
- **Gap:** there is no "who has *joined* a folder" (inbound peer) list anywhere
  in the CLI or `node_db`. `handle_devices` (main.rs:4274-4292) only prints
  **this device's own** iroh/Syncthing identities — it does **not** list known
  peers. (`docs/commands.md:476` claims `devices` = "List known peers +
  connection status", which is doc-drift; see plan 07.)

## Scope guard

- Read/aggregate + a small `unshare`-driven revoke. Prefer reusing existing IPC
  (`share --list`, `folders`, `networks`).
- **One permitted IPC addition:** the `Mode` column needs `mode` on the daemon
  `FolderStatusReport` (state.rs:80). Two options, pick one in step 1: (a) add a
  `mode: String` field to `FolderStatusReport` populated by the daemon's folder
  registry (small, additive, backward-compatible `#[serde(default)]`), or (b)
  run `access`'s folder-mode gather through the embedded path. Option (a) is
  recommended — it is the only place `access` needs data the daemon doesn't
  already serve, and it also fixes the `handle_folders` daemon path, which today
  silently omits mode.
- The "who joined" (inbound peer) column is **deferred to plan 09**
  (`syncweb network peers`: a new read-only core/IPC peer-list surface, filed
  in 09). Until then, `access` reports outbound shares + network members, and
  leaves the `Devices` column empty with a clear "inbound peers not yet
  surfaced" note.
- `Pinned?` is populated from the blob store's pin list (`blob_store.list_pins`
  with the `public_blob_pin` prefix), which requires a node handle. If gathering
  it per-row adds cost/complexity, defer it to the same follow-up as inbound
  peers and drop the column rather than printing "false" guesses.

## Steps

1. Add `syncweb access [<path-or-ns>]` that renders one table:
   - Columns: `Folder` · `Mode` (sendreceive/…; from the permitted
     `FolderStatusReport.mode` addition) · `Write?` (share capability) ·
     `Shared with` (outbound share rows: ticket URL + access) · `Networks` ·
     `Pinned?` (blob-store pin list; see scope guard).
   - Merge three existing sources: `FolderManager.list` (folders, via
     `handle_folders` data path), `share --list` entries (access + ticket), and
     `networks` membership. Do **not** assume the daemon `FolderList` already
     carries mode or that `share --list` carries pins — those two gaps are
     handled by the scope-guard decision, not by the merge.
2. Add `--json` shape: `{folder, mode, write, shares: [{access, url, pinned}],
   networks: [...]}`. If the `Pinned?` column is deferred (scope guard), drop
   `pinned` from the JSON too rather than emitting `false` — the shape must not
   guess.
3. Add `access --revoke <ns> [--read|--write]` sugar that dispatches to
   `handle_unshare` (main.rs:2737) with the appropriate flags — i.e. one verb to
   both see and revoke, with plan 04's confirmation + `--yes` running underneath.
   **`UnshareArgs` has no `--read` flag** (only `--write` and `--blob`;
   commands.rs:813-824 — there is no `--ticket`); map `--read` to the plain
   unshare path (no `--write`), which is the read-ticket revoke — and remember
   plain unshare also unpins
   (`unpin_all_content`, main.rs:2772-2774), so `access --revoke --read` has the
   same pinned-column effect plan 04's step 3 notes for read-only unshare.
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
- The `FolderStatusReport.mode` addition is the only core/IPC change; it is
  additive with a `#[serde(default)]` fallback, so older daemons round-trip
  (mode shows as empty/`sendreceive`-fallback rather than erroring).
- The missing inbound-peer list is a real limitation; guard against implying it
  works by printing the explicit "inbound peers not surfaced" note rather than
  an empty column.
- Rollback: `access` is a new verb; removing it restores the old views. Aliases
  (`syncweb who`, `syncweb shares`) stay documented as deprecated.

## Handoff notes

- Plan 06 (command collapse) must account for `access` — either fold it into
  `share` as `share --list`'s richer cousin, or keep it as one of the canonical
  verbs. Do not let plan 06 orphan it.
