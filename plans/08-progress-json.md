# Plan 08 — Progress, status, and `--json` everywhere (Maya's "is it happening?" + Oli's headless loop)

Priority: MEDIUM · Status: Draft · Owner: `syncweb-cli`
Depends on: plan 01 (lazy `ls` adds a remote dimension), plan 02 (eager join
needs a progress summary) · Fulfills story: #5 (Maya, phone status), #6 (Oli,
headless server)

## Goal

Maya should be able to answer "is my sync still running / how far along?"
without grepping logs; Oli should trust `syncweb stats network` from cron/CI.
Today progress exists only in:
- `stats network` (Statistics, main.rs:1850) — bandwidth counters;
- `transfer info` (main.rs:1253) — durable jobs;
- `handle_stats_files` (main.rs:1934) — file metadata;
- and the eager-join summary planned in plan 02.
But there's **no unified progress/status surface** and `--json` is "where
supported" — the actual gap Oli hit ("stats files" exists, but he had to know
it was `stats files`, and pipe through `--json`).

## Evidence (verified)

- `syncweb-cli/src/cli/output.rs` already has JSON helpers everywhere
  (`print_status`, `confirm_destructive` json branch at output.rs:24-39,
  grouped output). The `--json` flag exists per-command but **not on all**:
  e.g. `syncweb stats` and `syncweb status` differ; `--json` is described as
  "where supported" in `docs/commands.md` (line 186) and
  `README.md` — a documented inconsistency (plan 07's drift target).
- `handle_status` (main.rs:600) already exposes `output_json` reporting:
  daemon pid/uptime/bandwidth/schedule — but reports **folders + devices +
  network health** separately (`handle_folders` main.rs:4201, `handle_devices`
  main.rs:4274, `handle_networks` main.rs:4294). Maya's one-liner "where are my
  files" needs those three joined (plan 05's `access` view is the aggregation).

## Scope guard

- **Presentation layer + new `--json` surfaces.** No daemon protocol change.
- Keeps `--json` as the single machine contract; human output can wrap it.

## Steps

1. **Persistent progress identity** — add `syncweb transfer info --json` +
   `syncweb stats network --json` shape so Oli can diff runs (add `--since`/
   `--period` consistent with existing `stats --period 24h`, main.rs:1843-1850;
   reuse `StatsNetworkArgs.period` which already exists and is "retained for
   compatibility" per commands.rs:643).
2. **`syncweb status --json` aggregates** folders + devices + networks into one
   envelope (`{daemon, folders, devices, networks}`) — the machine counterpart
   of Maya's happy line, and the obvious upgrade to `handle_status`'s
   current single-daemon surface (main.rs:600-657). No new IPC: reuse the same
   `handle_folders`/`handle_devices`/`handle_networks` data paths in an
   aggregated branch.
3. **Event feed for live progress** — `syncweb stats network --follow` /
   `--watch` that streams daemon sync events live using the existing
   `IpcCommand::Subscribe`/event stream plumbing (the daemon already emits
   `SyncEvent`s, used at main.rs:2496-2528 in join); Oli gets a `cron`-safe `-1`
   run, Maya gets `--follow` so the phone shows "syncing …" without polling.
4. **`--json` everywhere**: audit every `print` command that doesn't have a
   `--json` branch (grep `output_json`; list the stragglers —
   `handle_ls`, `handle_find`, `handle_sort`, `handle_stat`, `handle_devices`),
   give each a stable `{...}` envelope, and update the help text so `--json` is
   stated as a global contract, not "where supported".

## Tests

- `syncweb-cli/tests/workflow_test.rs`: `stats network --period 24h --json`
   outputs a JSON object with `upload_total`/`download_total`/
   `namespace`/`peer` (assert shape, not exact numbers).
- `syncweb-cli/src/cli/output.rs` unit: `status --json` envelope contains
   `daemon`, `folders`, `devices`, `networks` keys even when empty lists.
- `--follow`: run `join --subscribe` + side-by-side `stats network --follow`
   against the same daemon; assert at least one `SyncEvent::{Started|
   Progress|Finished}` token observed in the stream within 30s (non-flaky:
   assert >=1 event, not exact counts).

## Risks / rollback

- `--follow` streaming under `--json` must emit **one JSON object per line**
  (NDJSON) — document it; scripts that expect a single blob will break. Provide
  `--follow --json-lines` vs `--follow --json <envelope>` and default the
  generic `--json` to NDJSON since it's a stream.
- Rollback: `--follow` and aggregated `status --json` are additive; removing
  them restores today's surface with no daemon change.
