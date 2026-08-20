# Plan 005 — Content Coverage (snapshot / transfer / collection / package)

## Overview

`snapshot create`/`list`, `collection init`/`add`, and `package export` are tested. The entire
`transfer` subsystem, `snapshot restore/diff/delete`, `collection versions`, and nearly all of
`package` are not.

## Target file

- `syncweb-cli/tests/full_suite_test.rs` (package/collection workflow precedent)
- `syncweb-cli/tests/workflow/basic_sync.rs` (snapshot workflow)
- New `syncweb-cli/tests/transfer_test.rs` (transfer subsystem)

## Missing coverage

### `transfer` (entire subcommand untested)
- `info` (`--namespace`, `--state`, `--sort`, `--group-by`, `--limit`)
- `remaining`
- `root <id> <path>` (`--min-free`, `--disabled`)
- `enqueue --namespace --path --hash <size>`
- `allocate` (`--dry-run`, `--namespace`, `--path-prefix`, `--min-size`, `--max-size`)
- `materialize` (`--namespace`)
- `pause` / `resume` / `cancel` / `retry <id>`

### `snapshot`
- `restore <path> <snapshot>` — untested
- `diff <path> <first> <second>` — untested
- `delete <path> <snapshot>` — untested
- `create`: `--description`, `--threads` untested

### `collection`
- `versions --version --changelog` — untested (only `init`/`add`)

### `package`
- `import`, `search` (`--bootstrap`, `--timeout-ms`, `--channel`), `info` (`--hash`, `--node-id`),
  `install`, `upgrade`, `remove`, `verify`, `list`, `versions`, `switch` — all untested
- `export`: `--version`, `--filter` (`--filter` tested)

## Proposed test cases

1. `snapshot_restore_diff_delete_round_trip`
   - Create snapshot v1, modify file, create v2; `snapshot diff <path> v1 v2` shows the change;
     `snapshot restore` materializes; `snapshot delete` releases pins.

2. `snapshot_create_with_description`
   - `snapshot create --description "tag" --threads 1`; assert description recorded.

3. `transfer_root_and_enqueue`
   - `transfer root <id> <dir> --min-free 0`; `transfer enqueue --namespace <ns> --path p --hash <h> <size>`;
     `transfer info` lists the job; `transfer allocate --dry-run`; `transfer pause/resume/cancel/retry <id>`;
     `transfer remaining`.

4. `collection_versions_bump`
   - `collection init --version 1.0.0 --name x`; add files; `collection versions --version 2.0.0 --changelog "x"`;
     assert new version.

5. `package_import_search_install_upgrade_remove`
   - `package export` (existing) then `package import <archive>`; `package search <query>`;
     `package install <ticket>`; `package list`; `package versions <coll>`;
     `package switch <coll> <version>`; `package remove <coll> <version>`; `package verify <coll>`.

6. `package_info_from_ticket_and_hash`
   - `package info <ticket>`; `package info --hash <h> --node-id <id>`.

7. `package_search_channel_and_bootstrap`
   - `package search <q> --channel <c> --bootstrap <node> --timeout-ms <n>`.
