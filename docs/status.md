# Product status

This is the current product contract for the alpha CLI. A behavior listed as
supported is covered by the repository's normal test suite or by the
focused `make test-network` CI gate. A behavior listed as opt-in or
planned must not be presented as a default guarantee.

## Supported and CI-backed

| Area | Current behavior |
|------|------------------|
| Folder synchronization | Two-node folder creation, joining, listing, bidirectional sync, send-only/receive-only modes, and persistence across node restarts. |
| Networks | Network creation, invitations, joining, membership persistence, folder membership, owner checks, kick/leave behavior, and signed membership documents. |
| Connectivity | Iroh relay-backed local scenarios, blob transfer, daemon-to-daemon sync, and the Syncthing relay protocol through an in-process mock relay. |
| Daemon and CLI | Daemon start/stop/reload, IPC routing, watched-folder sync, subscriptions, transfer operations, and command-level workflow coverage. |
| Health and status | `status` combines daemon, folder, device, network, and durable-transfer state. `status --json` keeps those sections in one stable object, including when the daemon is stopped. `network peers` and `stats seeding` report bounded, expiring per-blob provider observations and distinguish unknown counts. |
| Content operations | Lazy fetching, partial-fetch policy tests, archive/package lifecycle tests, indexing, search, filtering, sorting, statistics, snapshots, and integrity verification. |
| Backup and restore | `backup` captures configuration, identity, databases, and a manifest; `--include-blobs` opts into copying the local blob store. `restore` verifies a backup into a new data directory, requires explicit `--yes` confirmation for identity material, and leaves incomplete or incompatible backups untouched. The existing `db backup` form remains supported. |
| JSON automation | Global `--json` is supported by the command handlers; list results use named object envelopes, and `stats network --follow` emits documented NDJSON records. |
| Package distribution | The common flow is `package add`/`bump` -> `package publish` -> `package info` or `package install` -> `package upgrade`; archive export/import remains available for offline transfer. |
| Shared filters | `ls`, `find`, `sort`, `download`, and `verify` share the `--ext`, `--size`, `--depth`, `--type`, `--modified-*`, and `--remote-only` vocabulary, with compatibility aliases for older spellings. |
| Conflict safety | Last-writer-wins materialization keeps compact text diffs when beneficial and full conflict copies otherwise; the CRDT remains the source of truth. |
| Conflict inspection | `conflicts list` and `conflicts show` expose divergent variants, deterministic winners, timestamps, authors, hashes, sizes, and local availability in table or JSON output. |

The named network gate is deliberately narrower than the full suite:

```sh
make test-network
```

## Opt-in or environment-dependent

- External Syncthing relay reachability and two-process relay interoperability
  require `SYNCWEB_INTEROP=1` and are not a default CI guarantee.
- DHT/global discovery, real-world NAT combinations, and third-party relay
  availability are not reproduced by the in-process test world.
- Performance numbers in `docs/testing.md` are targets, not compatibility or
  throughput guarantees; they require specified fixtures and hardware before
  becoming release gates.

## Planned or explicitly incomplete

- The daemon is still manually managed: `status` reports health and progress,
  but it does not auto-start or supervise the daemon. Follow-up work is
  tracked in [plans/ux-and-operations.md](../plans/ux-and-operations.md).
- Communal LAN swarming, including provider inventories, multi-source
  transfers, replication policy, and large-LAN validation, is planned rather
  than a current product guarantee. See
  [plans/communal-swarming.md](../plans/communal-swarming.md).
- Design documents may describe future commands, but generated help, manpages,
  and supported command guides must be generated from the authoritative
  `syncweb-cli/src/cli/commands.rs` surface and must not present future names
  as executable commands.

When behavior changes, update this file, the relevant command help, and the
nearest executable test in the same change. This keeps the user-facing
contract tied to code instead of to the longer-term design documents.
