# Plan 08 — Progress, status, and a stable `--json` contract everywhere (Maya's "is it happening?" + Oli's headless loop)

Priority: MEDIUM · Status: Draft · Owner: `syncweb-cli`
Depends on: plan 01 (lazy `ls` adds a remote dimension), plan 02 (eager join
needs a progress summary) · Fulfills story: #5 (Maya, phone status), #6 (Oli,
headless server)

## Goal

Maya should be able to answer "is my sync still running / how far along?"
without grepping logs; Oli should trust `syncweb stats network` from cron/CI.
Today progress exists only in scattered surfaces, and `--json` is global but
**not shape-stable** — the actual gap Oli hit ("stats files" exists, but he had
to know it was `stats files`, and the JSON shape varies per command).

## Evidence (verified)

- `--json` is a **global** flag (syncweb-cli/src/cli/args.rs:247-248) whose help
  text itself reads *"Emit machine-readable JSON **where supported**"* — an
  implicit admission of inconsistency. Most handlers already branch on
  `ctx.output_json` (`handle_ls` main.rs:4322, `handle_find` :4425,
  `handle_sort` :4543, `handle_stat` :4691, `handle_devices` :4279,
  `handle_stats_network` :1861), but the **shapes differ**: `ls`/`find`/`sort`
  emit a bare JSON array of paths; `stat` emits a `StatOutput` object;
  `stats network` emits the raw `BandwidthStats` object; `status` emits the raw
  `StateFile` report. There is no stable envelope.
- `handle_status` (main.rs:600) already exposes `output_json` reporting (daemon
  pid/uptime/bandwidth/schedule) from the `StateFile` report — but folders +
  devices + network health are separate surfaces (`handle_folders`
  main.rs:4201, `handle_devices` main.rs:4274, `handle_networks` main.rs:4294).
  Maya's one-liner "where are my files" needs those joined (plan 05's `access`
  view is the aggregation; note `handle_devices` only prints this device's
  identity, not a peer list — see plan 07).
- `stats network` JSON field names are `total_upload`/`total_download`/
  `per_folder`/`per_peer`/`period_start` (syncweb-core/src/bandwidth_stats.rs:32-37),
  emitted at main.rs:1861-1862. `StatsNetworkArgs.period` exists and is
  "Retained for compatibility" (commands.rs:643-644).
- The daemon already emits `SyncEvent`s; `download_joined_folder` consumes them
  (main.rs:2527-2539). There is no live streaming surface exposed to the user.

## Scope guard

- **Presentation layer + new `--json` shapes.** No daemon protocol change.
- Keeps `--json` as the single machine contract; human output can wrap it.

## Steps

1. **Persistent progress identity** — standardize `syncweb stats network --json`
   so Oli can diff runs: document the `BandwidthStats` shape
   (`total_upload`, `total_download`, `per_folder`, `per_peer`, `period_start`)
   and add `--since`/`--period` consistent with the existing `period` field.
   (The field already exists and is "retained for compatibility"; make it real
   rather than a no-op.)
2. **`syncweb status --json` aggregates** folders + devices + networks into one
   envelope `{daemon, folders, devices, networks}` — the machine counterpart of
   Maya's happy line, upgrading `handle_status`'s current single-`StateFile`
   surface (main.rs:600-655). No new IPC: reuse the same
   `handle_folders`/`handle_devices`/`handle_networks` data paths in an
   aggregated branch. (`devices` will show only self-identity until the peer
   list from plan 07/05 lands; keep the key present and empty rather than
   omitting it.)
3. **Event feed for live progress** — `syncweb stats network --follow` /
   `--watch` that streams daemon sync events live using the existing
   `SyncEvent`/fetch-intent plumbing (main.rs:2527-2539); Oli gets a
   `cron`-safe `-1` run, Maya gets `--follow` so the phone shows "syncing …"
   without polling.
4. **Stabilize `--json` as a global contract**: audit every `print` command's
   JSON shape and normalize to a stable envelope (single object per command;
   arrays only inside a named key). Update the `--json` help text from "where
   supported" to a firm contract, and add a shape assertion per command (see
   Tests). List any command that emits no JSON branch and give it one.

## Tests

- `syncweb-cli/tests/workflow_test.rs`: `stats network --period 24h --json`
  outputs a JSON object with `total_upload`/`total_download`/`per_folder`/
  `per_peer` (assert shape, not exact numbers).
- `syncweb-cli/src/cli/output.rs` unit: `status --json` envelope contains
  `daemon`, `folders`, `devices`, `networks` keys even when the lists are empty.
- `--follow`: run `join --subscribe` + side-by-side `stats network --follow`
  against the same daemon; assert at least one `SyncEvent::{Started|Progress|
  Finished}` token observed in the stream within 30s (non-flaky: assert >=1
  event, not exact counts).

## Risks / rollback

- `--follow` streaming under `--json` must emit **one JSON object per line**
  (NDJSON) — document it; scripts that expect a single blob will break. Provide
  `--follow --json-lines` vs `--follow --json <envelope>` and default the
  generic `--json` to NDJSON since it's a stream.
- Rollback: `--follow` and aggregated `status --json` are additive; removing
  them restores today's surface with no daemon change.

## Handoff notes

- The original draft cited the "where supported" phrase to `docs/commands.md:186`
  and `README.md`; it is actually in the CLI's own `--json` help string
  (args.rs:248). Fix the flag help, not just the docs.
- `stats network --json` field names are `total_upload`/`total_download`
  (not `upload_total`/`download_total`); do not rename them — scripts may
  already depend on the current `BandwidthStats` serialization.
