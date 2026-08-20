# Plan 004 — Sharing & Publishing Coverage (publish / unpublish / mirror / provider / link)

## Overview

`publish folder` and `provider add` are tested; `unpublish`, the other `publish` targets, real
`mirror` runs, and most `link` options are not.

## Target file

- `syncweb-cli/tests/indexing_test.rs` (persistent, embedded, JSON assertions)
- `syncweb-cli/tests/cli_test.rs` (help / negative cases)

## Missing coverage

### `publish`
- `blob <namespace> <hash>` — untested
- `collection` (`--namespace`, `--sequence`, `--bootstrap`) — untested
- `folder` — tested (daemon IPC)
- `catalog` (`--catalog`, `--tag`) — `--catalog` tested; `--tag` untested

### `unpublish`
- `unpublish <namespace> --blob <hash>` — entire command untested

### `mirror`
- `mirror <provider>` / `mirror --network <name>` real run — untested (only `--help` + no-args fail)
- `--min-providers`, `--no-sharing`, `--dry-run`

### `provider`
- `add <collection> <provider>` — tested

### `link`
- `create`: `--name` tested; `--version`, `--sequence`, `--expires`, `--publish` untested
- `resolve`: `--version` untested
- `revoke`: `--broadcast` untested

## Proposed test cases

1. `publish_blob_and_unpublish_round_trip`
   - `publish blob <ns> <hash>`; capture ticket; `unpublish <ns> --blob <hash>`; assert pin removed.

2. `publish_collection_with_sequence_and_bootstrap`
   - `publish collection --namespace <ns> --sequence 3`; assert a ticket/head is produced.

3. `publish_catalog_with_tags`
   - `publish catalog <folder> --catalog <name> --tag <tag>`; assert publication confirmation.

4. `mirror_from_provider_and_network`
   - `mirror <provider-id>` and `mirror --network <name>`; assert blobs mirrored.
   - `mirror --dry-run` reports without fetching; `--no-sharing` skips lease announcements;
     `--min-providers <N>`.

5. `link_create_version_sequence_expires_publish`
   - `link create <hash> --name latest --version 2 --sequence 5 --publish <ns>`; assert mutable link.
   - `link create <hash> --expires <ts>` produces a private capability with expiry.

6. `link_resolve_with_version`
   - Resolve a versioned link with `--version <v>`.

7. `link_revoke_with_broadcast`
   - `link revoke <link> --broadcast`; assert revocation broadcast succeeds.

8. `link_help_lists_untested_options`
   - `link create --help` lists `--version`, `--sequence`, `--expires`, `--publish`.
