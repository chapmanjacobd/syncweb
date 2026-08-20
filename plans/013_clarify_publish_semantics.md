# Plan 013 — Clarify `publish` semantics (vs. just making a namespace)

## Overview

It is not clear what `publish` does versus `create`, which already prints a `ticket` and a
`share_url`. The mental model is muddy because four publish variants each "share" something
different, and `create` already emits shareable output.

## Current state

- `create <path>` → prints `namespace`, `ticket`, `share_url` (a writable doc ticket).
- `publish folder <path> [--namespace]` → public *read* ticket for a folder.
- `publish blob <namespace> <hash>` → unauthenticated blob ticket for a single hash.
- `publish collection <path> --namespace <ns>` → stores a collection manifest + head in a folder
  and announces it on the package-catalog gossip topic.
- `publish catalog <folder> --catalog <name>` → publishes folder metadata into a catalog
  (iroh-docs) for `indexing search`.
- `unpublish <namespace> --blob <hash>` → removes a public blob pin.

Handlers: `handle_publish` — `main.rs:2623`, `handle_unpublish` — `main.rs:2693`.

## Goal

Establish a clear model: **`create` provisions a local, private namespace; `publish` makes
something discoverable/fetchable by others.** Reduce the surface to distinct, self-explanatory
verbs and make `create` stop doing double-duty as a share command.

## Open questions / alternatives

| # | Question | Alternatives |
|---|----------|--------------|
| Q1 | Should `create` still print `share_url`? | (a) drop `share_url`/`ticket` from `create` output (share = `publish folder`); (b) keep for convenience; (c) keep but label clearly ("private, writable"). |
| Q2 | Rename `publish folder` vs `create` | `publish folder` could be `publish read-ticket` / `share`; `create` = namespace only. |
| Q3 | `publish collection` naming | move to `package publish` (see 010). |
| Q4 | `publish catalog` vs editorial `channel` | reconcile the two "catalog" concepts (see 015/016). |
| Q5 | `unpublish` location | fold under `publish` as `publish rm`/`publish unpublish` (see 017). |

## Recommendation

- Q1: (c) keep output but relabel, or (a) drop — decide with user; lean (a) to make `publish`
  the only share path.
- Q2: rename to `publish folder` staying, but document that it emits a *read-only* ticket while
  `create` emits a *writable* ticket.
- Q3/Q4/Q5: defer to 010 / 015+016 / 017 respectively.

## Proposed changes (this plan only)

1. `commands.rs` + `main.rs`: update `create`/`publish` help text and (per Q1) trim
   `share_url`/`ticket` from `create` output.
2. Add a `docs/commands.md` "sharing model" section: namespace (private, writable) vs
   publish (public, read-only) vs catalog (metadata).
3. Tests: `cli_test.rs::test_create_outputs_url` updated to the chosen Q1 behavior.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
./target/debug/syncweb create --help && ./target/debug/syncweb publish --help
```

## Files affected

- `syncweb-cli/src/cli/commands.rs`
- `syncweb-cli/src/main.rs`
- `syncweb-cli/tests/cli_test.rs`
- `docs/commands.md`

## Dependencies

- Coordinate with 010 (publish collection → package publish) and 017 (unpublish).
