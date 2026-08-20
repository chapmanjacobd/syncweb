# Plan 007 — Indexing, Trust & Attest, Moderation Coverage

## Overview

The persistence tests in `indexing_test.rs` cover most core operations, but several options and a
few subcommands are untested, especially the gossip/broadcast and expiry/scoping flags.

## Target file

- `syncweb-cli/tests/indexing_test.rs` (embedded, JSON assertions)

## Missing coverage

### `indexing`
- `search --limit` untested (bare `search` tested)
- `meta add --sequence` untested
- `filter add device` untested (only `hash` and `file` tested)
- `filter subscribe <source>` — untested

### `trust`
- `delegate`: `--expires`, `--scope`, `--sequence`, `--max-depth` untested
- `revoke-delegation <publisher>` (`--scope`) — untested
- `provider ban`: `--hash`, `--duration` untested
- `provider vouch` / `distrust`: `--scope` untested (`--reason`, `--broadcast` tested)
- `stream publish`: `--hash`, `--sequence` untested (`--provider`, `--signal` tested)

### `attest`
- `create`: `--provenance`, `--derivative`, `--sequence`, `--broadcast` untested (`--license` tested)
- `verify` (`--timeout`) — entire subcommand untested

### `moderation`
- `hide --reason` untested
- `report --broadcast` untested

## Proposed test cases

1. `indexing_search_with_limit`
   - `indexing search <q> --limit 5`; assert ≤ 5 results.

2. `indexing_meta_add_with_sequence`
   - `indexing meta add <hash> <key> <value> --sequence 7`; assert sequence recorded.

3. `indexing_filter_add_device_and_subscribe`
   - `indexing filter add device <id>`; `indexing filter subscribe <source>` (signed federated list).

4. `trust_delegate_with_scope_expiry_depth`
   - `trust delegate <pub> --scope <s> --expires <ts> --sequence 2 --max-depth 2`; assert delegation.

5. `trust_revoke_delegation`
   - Delegate then `trust revoke-delegation <pub> --scope <s>`; assert trust removed.

6. `trust_provider_ban_scoped_and_durable`
   - `trust provider ban <id> --hash <h> --duration <secs> --reason <r>`; assert scoped ban expires.

7. `trust_provider_vouch_and_distrust_with_scope`
   - `vouch --scope <s>` and `distrust --scope <s>`; assert scoped trust state.

8. `trust_stream_publish_with_hash_and_sequence`
   - `trust stream publish --provider <id> --signal <s> --hash <h> --sequence <n>`.

9. `attest_provenance_derivative_and_broadcast`
   - `attest create <hash> --provenance <t>`; `attest create <hash> --derivative <t>` (each conflicts
     with the others); `attest create <hash> --license MIT --broadcast`.

10. `attest_verify_with_timeout`
    - `attest verify <hash> --timeout <s>`; assert verification result.

11. `moderation_hide_with_reason_and_report_broadcast`
    - `moderation hide <record> --reason <r>`; `moderation report <record> --reason <r> --broadcast`.
