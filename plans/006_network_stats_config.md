# Plan 006 — Network, Stats & Config Coverage

## Overview

Network create/ls/join/leave/invite/kick and the basic stats/config paths are tested, but the
network inspection commands, per-peer/per-folder stats options, and `config schedule folder` are not.

## Target file

- `syncweb-cli/tests/cli_test.rs` (network + stats)
- `syncweb-cli/tests/workflow/basic_sync.rs` (network/stats workflow)

## Missing coverage

### `network`
- `events <network_id>` (`--limit`) — untested
- `health` (`--network`) — untested
- `create`: `--label`, `--invite-only` tested
- `ls <name>` inspect — tested
- `kick`, `join`, `leave`, `invite` — tested

### `stats`
- `network`: `--folder`, `--peer`, `--reset`, `--period` untested (only bare `stats network`)
- `files`: `--folder` tested; `--by` (`extension`/`size`/`all`/`time`), `--top-largest` untested
- `seeding`: `--folder` tested; ContentFilter options (`--hash`, `--path-prefix`, `--glob`) untested

### `config`
- `schedule set` tested (`--active`); `--bandwidth` + `--period` untested
- `schedule folder <name>` (`--active`, `--max-upload`, `--max-download`) — untested
- `show <section>` tested; `config` with no subcommand (print full TOML) untested

## Proposed test cases

1. `network_events_and_health`
   - Create a network, perform create/invite to generate events; `network events <id> --limit 5`
     lists recent events; `network health --network <name>` reports connectivity health.

2. `stats_network_filters_and_reset`
   - `stats network --folder <ns>`, `--peer <id>`, `--period <p>`; after data, `--reset` zeroes
     the counters (assert `total_download` returns to 0).

3. `stats_files_by_and_top_largest`
   - `stats files --folder <ns> --by size --top-largest 2`; assert largest files listed.

4. `stats_seeding_with_content_filter`
   - `stats seeding --folder <ns> --hash <h>` filters the report.

5. `config_schedule_bandwidth_and_period`
   - `config schedule set --active 08:00-18:00 --bandwidth 2M --period 08:00-18:00`; assert persisted.

6. `config_schedule_folder_override`
   - `config schedule folder <name> --active 09:00-17:00 --max-upload 500K --max-download 1M`;
     assert override present.

7. `config_no_subcommand_prints_full_toml`
   - Bare `config` prints the full configuration TOML.
