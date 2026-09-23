# Plan 09 — Peer availability + inbound peers: the one IPC surface the other plans deferred to nobody

Priority: LOW · Status: Draft · Owner: `syncweb-core` (one additive read-only IPC) + `syncweb-cli` (consume)

## Why this plan exists

Three earlier plans each defer the same reader-facing surface — per-blob
peer availability and the inbound-peer ("who joined") list — and all three
explicitly point here. This plan is the destination they name; it must
actually land, or those deferral lines become dangling:

- Plan 01:109-114 — `ls`/`find`/`sort` "Peer availability is deliberately
  absent… Surfacing `peers`/% seeded needs a new IPC surface — out of this
  plan's scope; filed as plan 09 (`network peers`)."
- Plan 05:56-59 — the `access` inbound-peer `Devices` column is
  "deferred to plan 09 (`syncweb network peers`: a new read-only
  core/IPC peer-list surface, filed in 09)."
- Plan 08:60-62 — `devices` "shows only self-identity until plan 09's
  `syncweb network peers` peer surface… lands; keep the key present and empty
  rather than omitting it."

Note: an earlier draft of this plan misquoted those three deferral lines as
pointing at plan 03 / plan 08 / the plan-07 doc fold; the plans as they stand
all name this one correctly, so no retargeting is needed — only the work below.

This plan is that surface. It is deliberately additive and read-only:
exactly one new core/IPC command, consumed by the three reading sites above,
so none of them has to invent a half-surface to stay in scope.

## Goal

Give the CLI a read-only view of (a) which blobs each folder can get from
which peers, and (b) which peers joined each folder — the two things
`ls`/`find`/`sort` `% seeded`/`peers`, `access`'s `Devices` column, and
`devices`' peer list all currently punt. Land it once, in core+IPC, and un-punt
plans 01/05/08's deferral lines.

## Evidence (verified in code)

- `devices` today prints self-identity only — iroh node id +
  syncthing device id (main.rs:4274-4292 `handle_devices`). No peer list.
- The daemon's per-blob peer-count map exists but is always empty at the
  client: `EnrichSort` carries a `peers` map that the CLI can't populate
  (`ipc.rs:1467` — `sort --enrich` already degrades gracefully to metadata
  fields; prior noted ipc.rs:1467).
- There is no inbound-peer ("who joined") IPC: the only share/network
  surfaces are outbound — the persisted share records (`share --list`,
  `handle_unshare`, main.rs:2737) and named-network membership — and folder
  status reports carry `mode`/path but no peer set (state.rs:80; plan 05 step
  1 adds `mode` but not peers).
- Python is likewise outbound-only today (`syncweb/syncthing.py:483-518`
  `devices` REST), so nothing to port; this is the CLI's first inbound view.

### Scope guard

- Permits exactly one core/IPC addition: a read-only
  `IpcCommand::PeerAvailability { folder_selection }` →
  `{folder, peers: [{device_id, name?, connection}] ,
  per_blob: [{path, hash, peer_count, peers: [device_id], % seeded}]}`
  (single-envelope shape per plan 08 step 1's dict contract). Additive +
  `#[serde(default)]` on the response, so older daemons round-trip.
- No mutating surface. No `kick`, no `accept`, no `drop` — this renders
  what's there. Revoke/accept remain plan 04/05's verbs, and the plan-07
  `devices`/`shutdown` doc-drift rows still stay doc-only (no IPC; 07:118-122).
- The inbound-peer column in plan 05 and `% seeded` in plan 01 stay printing
  today's honest "not yet surfaced" state until this plan lands; they must not
  invent values (plan 00 index, principle 5).

## Steps

1. Core IPC (read-only): `IpcCommand::PeerAvailability` — daemon aggregates
   from its device registry (`networks`, `share --list`-adjacent data)
   + the blob store's per-blob peer-interest count (the count `EnrichSort`
   would carry). No new state; reuse the daemon's existing network/membership
   aggregation (the data `handle_status_networks`, main.rs:4119, draws on
   through `network_manager`) + the (empty today) `EnrichSort` peer map at
   ipc.rs:1467. (`handle_devices` is CLI-side self-identity printing,
   main.rs:4274, and has no peer aggregation to reuse.)
2. CLI surface: `syncweb network peers [<namespace-or-path>] [--json]` —
   hidden-arg alias on the grouped `network` family (plan 06 verbs; commands.rs
   category table), never a new top-level verb. `--json` always available.
3. Consumers get the column they deferred:
   - plan 01 `ls`/`find`/`sort`: populates `% seeded`/`peers` when
     `--local-only` is absent, from `PeerAvailability.per_blob`.
   - plan 05 `access`: populates the inbound `Devices` column when the daemon
     answers; keeps the "inbound peers not yet surfaced" note when it can't.
   - plan 08 `devices`: replaces "self-identity only" with the real peer list
     (identity stays the first row).
4. Import the note rows: `syncweb network peers` is recorded in
   `docs/commands.md` + grouped help as the canonical peer view (the CLI has no
   `peer` noun of its own; the verb is `network`).

## Tests

- `syncweb-cli/tests/daemon_integration_test.rs`: after `join --subscribe`
  (plan 02), `network peers <ns> --json` returns `{folder, peers, per_blob}`
  with at least first-blob `% seeded` ≥ 0 and a `peers` array; assert shape,
  not exact numbers (peer counts depend on the network).
- `syncweb-cli/src/main.rs` `#[cfg(test)]`: `--json` envelope shape is stable
  and `peers` keys are present-but-empty when the daemon has no peers (no
  "false" guesses).
- Grouped help: `network peers` appears under the `network` category and not as
  a new tree level.

## Risks / rollback

- Import serialization cost: per-blob peer aggregation is O(blobs × peers)
  on the daemon; keep the count cheap (reuse the existing empty-`EnrichSort`
  map path) and page only what the CLI asks for. Rollback = drop the verb;
  core change is additive and `#[serde(default)]`.
- Peer counts race (new join mid-listen) — document that `% seeded` is a
  point-in-time snapshot, not a stream.
- Inbound-peer identity is Syncthing device IDs only until inbound-device
  acceptance (plan 07 devices) adds names; `name` is `Option` and empty-typed.

## Handoff notes

- Delete nothing. Plan 01/05/08's deferral lines now point here; their honest
  "not yet surfaced" states remain until this lands.
- Files: `syncweb-core/src/daemon/ipc.rs` (new command), `syncweb-cli/src/main.rs`
  (`handle_network_peers`), `syncweb-cli/src/cli/commands.rs` (grouped
  category), `docs/commands.md`.
