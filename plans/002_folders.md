# Plan 002 — Folders Coverage (create / join / leave)

## Overview

`create`, `join`, and `leave` are exercised end-to-end, but most of their option surface is untested.
`join` in particular has a large subscription/filter option set that only `--network`, `--subscribe`,
and `--ingest-only` are covered for.

## Target file

- `syncweb-cli/tests/workflow/basic_sync.rs` (embedded, multi-device)
- `syncweb-cli/tests/daemon_integration_test.rs` (daemon-routed)

## Missing coverage

### `create`
| Option | Status |
|--------|--------|
| `--mode` | tested (`sendonly`) |
| `--network` | tested |
| `--relay-fallback` | untested |

### `join`
| Option | Status |
|--------|--------|
| default `--mode receiveonly` | untested (join always uses default) |
| `--relay-fallback` | untested |
| `--network` | tested |
| `--subscribe` | tested (help + daemon IPC) |
| `--ingest-only` | tested (with subscribe) |
| `--ignore-self` | untested |
| `--prefix` | untested |
| `--sync-prefix` | untested |
| `--glob` | untested |
| `--max-count` | untested |
| `--max-size` | untested |

### `leave`
| Option | Status |
|--------|--------|
| default | tested |
| `--delete-files` | tested (daemon) |

## Proposed test cases

1. `join_with_mode_receiveonly`
   - Create a folder, have a second device join with `--mode receiveonly` (explicit), assert the
     joined folder is ReceiveOnly via `folders`.

2. `join_with_relay_fallback`
   - `join --relay-fallback`; assert success and that BEP relay fallback is enabled on the folder.

3. `create_with_relay_fallback`
   - `create --relay-fallback`; assert success.

4. `join_subscription_filters`
   - Two devices; creator writes files, joiner subscribes with each of:
     - `--ignore-self`
     - `--prefix <dir>`
     - `--sync-prefix <area>`
     - `--glob '/*.txt'`
     - `--max-count <N>` and `--max-size <N>`
   - Assert the joiner only receives entries matching the filter (e.g. `--max-count` limits entries).

5. `join_subscribe_help_lists_new_options`
   - `join --help` already asserted for most flags (`cli_test.rs`); extend to `--ignore-self`,
     `--prefix`, `--max-count`, `--max-size` if not present.

6. `leave_default_keeps_files`
   - `leave <ns>` (no flag) removes the folder but keeps local files on disk.
