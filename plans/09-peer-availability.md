# Plan 09 — Peer availability + inbound peers: the one IPC surface the other plans deferred to nobody

Priority: LOW · Status: Draft · Owner: `syncweb-core` (one additive read-only IPC) + `syncweb-cli` (consume)

## Why this plan exists

Three earlier plans each defer the **same** reader-facing surface — per-blob
peer availability and the inbound-peer ("who joined") list — and each points at
a destination that does not deliver it:

- **Plan 01:109-113** — `ls`/`find`/`sort` "Peer availability is deliberately
  absent… Surfacing `peers`/% seeded needs a new IPC surface — out of this
  plan's scope; **tracked in plans 03/08**." Plan 03 is the unified-filter
  vocabulary and plan 08 is `stats network` bandwidth JSON — neither adds the
  peer surface. Dangling pointer.
- **Plan 05:56-58** — the `access` inbound-peer column: "should be filed as
  its own plan (or folded into the plan-07 `devices` doc-drift fix)." The
  plan-07 fold is docs-only (docs/commands.md:476 drift; 07:118-122 requires
  **no** IPC), so it can never surface inbound peers. The "own plan" was never
  filed. Dangling pointer.
- **Plan 08:60** — `devices` "will show only self-identity until the peer
  surface" lands; the surface never lands.

This plan is that surface. It is deliberately **additive and read-only**:
exactly one new core/IPC command, consumed by the three reading sites above,
so none of them has to invent a half-surface to stay in scope.

## Goal

Give the CLI a read-only view of (a) which **blobs** each folder can get from
which **peers**, and (b) which **peers joined** each folder — the two things
`ls`/`find`/`sort` `% seeded`/`peers`, `access`'s `Devices` column, and
`devices`' peer list all currently punt. Land it once, in core+IPC, and un-punt
plans 01/05/08's deferral lines.

## Evidence (verified in code)

- `devices` today prints **self-identity only** — iroh node id +
  syncthing device id (main.rs:4274-4290 "handle_devices"). No peer list.
- The daemon's per-blob peer-count map **exists but is always empty at the
  client**: `EnrichSort` carries a `peers` map that the CLI can't populate
  (`ipc.rs:1467` — `sort --enrich` already degrades gracefully to metadata
  fields; prior noted ipc.rs:1467).
- There is **no inbound-peer ("who joined") IPC**: the daemon's peer list
  (`handle_devices`) is outbound share/named-network data only
  (commands.rs:813-824 `UnshareArgs`, main.rs:2737 `handle_unshare`); folder
  status reports carry `mode`/path but no peer set (state.rs:80, plan 05 step
  1 adds `mode` but not peers).
- Python is likewise outbound-only today (`syncweb/syncthing.py:483-518`
  `devices` REST), so nothing to port; this is the CLI's first inbound view.

### Scope guard

- **Permissions exactly one core/IPC addition:** a read-only
  `IpcCommand::PeerAvailability { folder_selection }` →
  `{folder, peers: [{device_id, name?, connection}] ,
  per_blob: [{path, hash, peer_count, peers: [device_id], % seeded}]}`
  (single-envelope shape per plan 08 step 1's dict contract). Additive +
  `#[serde(default)]` on the response, so older daemons round-trip.
- **No mutating surface.** No `kick`, no `accept`, no `drop` — this renders
  what's there. Revoke/accept remain plan 04/05's verbs, and the plan-07
  `devices`/`shutdown` doc-drift rows still stay doc-only (no IPC; 07:118-122).
- The inbound-peer column in plan 05 and `% seeded` in plan 01 stay printing
  today's honest "not yet surfaced" state until this plan lands; they must not
  invent values (plan 00 index, principle 5).

## Steps

1. **Core IPC (read-only): `IpcCommand::PeerAvailability`** — daemon aggregates
   from its device registry (`networks`, `share --list`-adjacent data)
   + the blob store's per-blob peer-interest count (the count `EnrichSort`
   would carry). No new state; reuse existing aggregation in `handle_devices`
   + the (empty today) `EnrichSort` peer map at ipc.rs:1467.
2. **CLI surface: `syncweb network peers [<namespace-or-path>] [--json]`** —
   hidden-arg alias on the grouped `network` family (plan 06 verbs; commands.rs
   category table), never a new top-level verb. `--json` always available.
3. **Consumers get the column they deferred:**
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

- **Import serialization cost:** per-blob peer aggregation is O(blobs × peers)
  on the daemon; keep the count cheap (reuse the existing empty-`EnrichSort`
  map path) and page only what the CLI asks for. Rollback = drop the verb;
  core change is additive and `#[serde(default)]`.
- **Peer counts race** (new join mid-listen) — document that `% seeded` is a
  point-in-time snapshot, not a stream.
- Inbound-peer identity is Syncthing device IDs only until inbound-device
  acceptance (plan 07 devices) adds names; `name` is `Option` and empty-typed.

## Handoff notes

- Delete nothing. Plan 01/05/08's deferral lines now point here; their honest
  "not yet surfaced" states remain until this lands.
- Files: `syncweb-core/src/daemon/ipc.rs` (new command), `syncweb-cli/src/main.rs`
  (`handle_network_peers`), `syncweb-cli/src/cli/commands.rs` (grouped
  category), `docs/commands.md`.
