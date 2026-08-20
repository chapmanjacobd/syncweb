# Plan 001 — Daemon & Lifecycle Coverage

## Overview

The daemon lifecycle commands (`start`, `shutdown`, `status`, `reload`, `daemon-sync`) and the
global CLI options are only partially exercised. `daemon_integration_test.rs` covers the happy path
(`start --bg --no-relay`, `status`, `shutdown --force`, `reload`, bare `daemon-sync`) but none of the
`start` tuning flags, the `daemon-sync --namespace` selector, or the global `--network` option are
tested.

## Target file

- `syncweb-cli/tests/daemon_integration_test.rs` (daemon-backed)
- `syncweb-cli/tests/cli_test.rs` / `full_suite_test.rs` (embedded / local)

## Missing coverage

### Global options
| Option | Status |
|--------|--------|
| `--verbose` | tested |
| `--json` | tested |
| `--no-daemon` / `--embedded` | tested |
| `--data-dir` | tested |
| `--network <name>` | **untested** |

### `start` options (only `--bg` and `--no-relay` tested)
- `--media-only` (run only the media HTTP server and exit)
- `--log-file <path>` (write daemon logs to a file; assert file is created)
- `--max-threads <N>`
- `--sync-interval <secs>`
- `--no-mdns`
- `--no-beacon`
- `--beacon-port <u16>`
- `--discovery-interface <iface>`
- `--media-listen <socketaddr>`

### `daemon-sync`
- `--namespace <ns>` (sync a single live folder only)

## Proposed test cases

1. `global_network_flag_scopes_data_dir`
   - `--data-dir <dir> --network home create` produces a separate `home/` subtree;
     `effective_data_dir` (`args.rs:15`) is exercised. Assert folder appears under `<data_dir>/home`.

2. `start_with_log_file_writes_log`
   - `start --bg --log-file <path>`; assert the log file exists and is non-empty after the daemon
     is ready.

3. `start_media_only_exits`
   - `start --media-only`; assert it starts the media server and exits (does not leave a daemon).

4. `start_discovery_and_media_tuning_flags_accepted`
   - Exercise `--max-threads`, `--sync-interval`, `--no-mdns`, `--no-beacon`, `--beacon-port`,
     `--media-listen` and assert `status` reports the daemon running with the tuned config.

5. `daemon_sync_scoped_to_namespace`
   - After daemon is running and a folder is created, run `daemon-sync --namespace <ns>` and assert
     success.

6. `global_network_option_is_listed_in_help`
   - `--help` contains `--network`.
