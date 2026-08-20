# Plan 009 — Workflow DSL Coverage (workflow_test.rs helpers)

## Overview

The workflow test harness (`syncweb-cli/tests/workflow/mod.rs`) defines a small DSL of `Device`
helpers. Three of them are declared but never exercised by any test — they carry
`#[expect(dead_code, reason = "part of DSL public API")]` markers:

- `Device::config_show()` (`workflow/mod.rs:160`)
- `Device::config_set(key, value)` (`workflow/mod.rs:165`)
- `Device::data_dir()` (`workflow/mod.rs:213`)

`tests/workflow/basic_sync.rs` currently covers: create, folders, ls, find, import, stat, verify,
config set/show (via raw `run_ok`, not the helpers), version, devices, network create/list/invite/
leave, snapshot create/list, stats network, db check, join.

## Goal

Remove the dead-code markers by wiring the three unused helpers into real workflow tests, and extend
the workflow DSL where it makes sense.

## Target file

- `syncweb-cli/tests/workflow/basic_sync.rs` (new tests using the helpers)
- `syncweb-cli/tests/workflow/mod.rs` (remove `#[expect(dead_code)]` once used)

## Proposed test cases

1. `config_set_and_show_via_helpers`
   - Use `device.config_set("bep.enabled", "true")` then `device.config_show()`;
     assert output contains the `bep` section. Drop the raw `run_ok` in the existing
     `config_round_trip` test in favor of the helpers, then remove the dead-code markers.

2. `data_dir_helper_exposes_isolated_dir`
   - Use `device.data_dir()` to assert the device's data directory exists and is
     `<world.root()>/data-<name>`; assert it's distinct between two devices.

3. `snapshot_delete_and_restore_workflow` (extends content coverage)
   - Create a folder, snapshot it, then exercise restore/delete through the DSL if helpers are added.

4. `multi_device_sync_workflow`
   - Wire the unused `devices()` accessor (`world.devices()`, `workflow/mod.rs:248`) into a test
     asserting both devices are present.
