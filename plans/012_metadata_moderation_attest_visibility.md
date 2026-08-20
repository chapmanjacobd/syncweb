# Plan 012 — Metadata / moderation / attestation read-write symmetry

## Overview

Three *writers* populate signed records about content, but there is no symmetric *reader* under
the same command. All reads funnel through `trust show --content`:

- `indexing meta add <hash> <key> <value>` writes signed metadata; no `indexing meta list`.
- `attest create <hash> --license/--provenance/--derivative` writes attestations; `attest verify`
  only checks the *network* (returns empty with no peers), not local state; no `attest list`.
- `moderation hide` / `moderation report` write moderation records; `moderation ls` exists.

This is confirmed by `workflow/indexing.rs`: `indexing_meta_add_persists_metadata` and
`attest_report_and_moderation_state_persist` both assert persistence by reading back through
`trust show <hash> --content`, not through the writer's own group.

## Goal

Give each writer a symmetric reader (or promote a single canonical read surface), so a user who
writes metadata/attestation/moderation can inspect it without knowing to look under `trust`.

## Current state

- `MetaCommand::Add` — `commands.rs:950`; no `List`.
- `AttestCommand { Create, Verify }` — `commands.rs:1128`; `Verify` is network-only
  (`workflow/indexing.rs::attest_verify_with_timeout` shows empty result with no peers).
- `ModerationCommand { List, Hide, Report }` — `commands.rs:1152`.
- `TrustCommand::Show { subject, content }` — `commands.rs:1013`; aggregates trust, moderation,
  attestations, and metadata for a content hash when `--content` is set.
- Underlying store: `IndexingDatabase` (attestations, content reports, metadata,
  provider-trust records) — `syncweb-core/src/indexing.rs`.

## Open questions / alternatives

| # | Question | Alternatives |
|---|----------|--------------|
| Q1 | Read surface | (a) per-domain `list`: `indexing meta list`, `attest list`, `moderation ls <hash>`; (b) a single canonical `inspect`/`trust show` promoted as the one read command; (c) both. |
| Q2 | Is `indexing meta add` redundant with `attest create`? | (a) fold metadata into attestation (`attest create --meta key=value`); (b) keep separate (metadata is arbitrary key/value, attestation is a discrete claim). |
| Q3 | `attest verify` semantics | (a) also report *local* attestations by default; (b) keep network-only, add `attest list` for local. |
| Q4 | `trust show` subject confusion | `--content` toggles publisher-vs-content subject; consider `trust show provider X` vs `trust show content H` subcommands. |

## Recommendation

- Q1: (c) add the per-domain `list` commands (cheap, symmetric) AND document `trust show` as the
  aggregate. No behavior removed.
- Q2: keep separate for now; revisit after 014 (gossip/trust audit) — the "is metadata just an
  attestation?" question belongs there.
- Q3: (a) add `attest list <hash>`; leave `verify` as-is (network collection).
- Q4: split into `trust show provider` / `trust show content` (cleaner than a `--content` flag).

## Proposed changes

1. `commands.rs`: add `MetaCommand::List { hash }`, `AttestCommand::List { hash }`,
   `ModerationCommand::List` already accepts an optional `content` filter (verify it reads
   content-scoped records). Optional Q4: restructure `TrustCommand::Show`.
2. `cli/indexing.rs`: implement the three list handlers reading `IndexingDatabase`.
3. Tests: `indexing_meta_list`, `attest_list_local`, `moderation_list_content_scoped` in
   `workflow/indexing.rs`; assert they return what `trust show --content` returns.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
./target/debug/syncweb indexing meta list <hash>
./target/debug/syncweb attest list <hash>
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/cli/indexing.rs`
- `syncweb-cli/tests/workflow/indexing.rs`
- `docs/commands.md`, `docs/indexing.md`

## Dependencies

- Q2 depends on 014 (trust/gossip audit); implement the rest independently.
