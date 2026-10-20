# Communal LAN swarming

Status: Proposed
Priority: High for large ad-hoc LANs
Owners: Sync, discovery, daemon, CLI, and test infrastructure

## Goal

Extend syncweb from a mostly provider-oriented transfer model into a
content-addressed communal swarm. Hundreds or thousands of intermittently
connected devices should be able to discover useful peers, exchange different
parts of the same content concurrently, keep a configurable number of
replicas alive, and rejoin after a LAN partition or offline CAR/USB exchange.

The existing Iroh blobs, iroh-docs reconciliation, CAR.zst import/export,
mDNS, UDP beacon, DHT discovery, relay fallback, membership checks, and
transfer queues are the foundation. This plan addresses the missing
coordination and policy layers; it does not replace those primitives.

## Design constraints

- Keep content addressing and verification based on the existing BLAKE3 blob
  hashes.
- Use mDNS and the UDP beacon for bootstrap and endpoint discovery, not as a
  bulk-data or per-blob announcement channel.
- Prefer local/direct LAN paths and bounded concurrency; retain relay fallback
  for peers that cannot connect directly.
- Treat provider information as advisory and expiring. A failed or stale
  provider must never make content appear valid.
- Keep community participation opt-in for named networks. The default open
  daemon may discover peers, but sharing, pinning, and contribution must remain
  explicit.
- Preserve CAR.zst as an interoperable offline boundary. Offline imports must
  be safe even when the source community is unknown or no longer reachable.
- Avoid one global tracker bottleneck and avoid one DHT write per blob.

## Current gaps

- Blob fetching generally selects one provider and requests a whole blob.
- The fallback path is sequential first-working-provider selection; it is not a
  piece scheduler.
- Discovery advertises endpoint addresses, not content inventories or
  capability summaries.
- Named networks provide membership and connection allowlists, but not an
  explicit ad-hoc community identity, role, moderation, or contribution model.
- There is no topology hierarchy, local tracker/cache-peer role, or live
  multi-process gate for very large LANs.
- There are no realistic 100- or 1000-node live swarm gates.

## Workstreams

### 1. Provider inventory and signed availability leases

Introduce a compact, expiring availability model that can be learned from
successful transfers, explicit announcements, and offline imports.

Likely code: `syncweb-core/src/sync/peer_tracker.rs`,
`syncweb-core/src/sync/partial_fetch.rs`,
`syncweb-core/src/daemon/ipc.rs`, `syncweb-core/src/node/discovery.rs`,
`syncweb-core/src/node/beacon_lookup.rs`, and new provider-registry modules.

Tests: signed lease validation, expiry and eviction, malformed inventory
input, provider-count reporting, and registry recovery after restart.

### 2. Multi-source and chunk-level transfer scheduling

Make one content request able to use multiple providers while retaining
integrity and bounded resource use.

Likely code: `syncweb-core/src/node/blob_store.rs`,
`syncweb-core/src/daemon/daemon.rs`, transfer queue modules, package/archive
fetch paths, and a new scheduler module.

Tests: one blob from multiple providers, provider churn, duplicate and
  corrupt segments, bandwidth/concurrency limits, cancellation, relay fallback,
  and compatibility with whole-blob peers.

### 3. Replication, pinning, and seeding policy

Turn “download” and “share” into explicit, observable policy rather than an
accidental consequence of local storage.

Likely code: blob pinning/GC, `syncweb-core/src/sync/partial_fetch.rs`,
daemon scheduling, config models, and `syncweb-cli/src/cli/commands.rs`.

Tests: target convergence, churn, quota enforcement, node departure,
simultaneous repair, protected pins, and deterministic status output.

### 4. LAN topology, local trackers, and discovery scaling

Keep discovery useful when broadcast domains and peer counts are large.

- [ ] Define an optional local rendezvous/cache-peer role that aggregates
  inventory summaries and helps peers bootstrap without becoming a required
  single point of failure.
- [ ] Specify partition merge and duplicate-community behavior, including
  backoff when two large LAN segments reconnect.

Likely code: `syncweb-core/src/node/beacon_lookup.rs`,
`syncweb-core/src/node/discovery.rs`, `syncweb-core/src/node/iroh_node.rs`,
topic-tracker integration, and new topology/peer-sampling modules.

Tests: packet/message budgets, peer-sampling convergence, isolated
  partitions, local tracker loss, interface selection, and mixed LAN/relay
  paths.

### 5. Ad-hoc communal membership and trust

Provide a first-class community boundary for temporary, shared collections
without weakening named-network access controls.

- [ ] Add admission and revocation behavior for communities that have no
  always-online owner.
- [ ] Define how signed manifests, provider leases, and content tickets bind
  to a community without requiring a central service.
- [ ] Add moderation hooks: denylisted nodes, rate limits, maximum object size,
  and operator-visible audit events.
Likely code: network membership models, ticket/invitation handling,
connection allowlists, discovery topic scoping, and CLI network commands.

Tests: invitation join, capability denial, expiry/revocation, conflicting
  community IDs, malicious announcements, and restart persistence.

### 6. CAR/USB inventory reconciliation and enrollment

Make offline exchange a first-class way to add content and providers to a
swarm, not only a one-time archive restore.

- [ ] Add a CAR index containing roots, blocks, collection metadata, source
  community identity, and optional provider/capability hints.
- [ ] Add export modes for a collection, missing blocks, repair bundles, and a
  bounded contribution bundle.
- [ ] Support resumable, inspectable imports and exports with clear duplicate
  and conflicting-root behavior.

Likely code: `syncweb-core/src/archive`, package lifecycle modules,
`docs/archive_format.md` implementation, daemon IPC, and archive CLI commands.

Tests: interrupted import/export, duplicate blocks, unknown community,
partial bundle, conflicting manifest, corrupt CAR, and automatic provider
publication.

### 7. CLI, daemon operations, and observability

Make communal participation understandable and controllable.

- [ ] Add explicit commands or options for community create/join, pin,
  unpin, seed, pause contribution, and set resource budgets.
- [ ] Provide machine-readable events and metrics for discovery, leases,
  segment transfers, repairs, and rejected requests.
Likely code: daemon lifecycle, IPC, `syncweb-cli/src/cli/commands.rs`,
config, status rendering, and logging/metrics.

Tests: JSON schema stability, command permission checks, daemon restart,
budget enforcement, and end-to-end status transitions.

### 8. Scale simulation and LAN acceptance gates

Validate control-plane and data-plane behavior before claiming large-LAN
support.

- [ ] Add live multi-process tests for at least 10, 50, and 100 nodes.

Likely code: integration test harnesses, simulator modules, fixtures, and
`docs/testing.md`.

## Delivery order

1. Measure and instrument: provider observations, status fields, and a
   deterministic simulator skeleton.
2. Advertise and schedule: signed provider leases, bounded peer sampling,
   multi-source transfers, and whole-blob compatibility.
3. Keep content alive: replication budgets, pin/GC policy, repair, and
   contribution controls.
4. Scale discovery: local tracker/cache roles, inventory reconciliation,
   topology hints, and partition handling.
5. Close the offline loop: CAR indexes, import enrollment, repair bundles,
   and USB-oriented CLI workflows.
6. Harden the product: communal admission, moderation, daemon supervision,
   UX, metrics, and scale acceptance gates.

## Completion criteria

This plan is complete when all of the following are demonstrated:

- A single large content object can be assembled from multiple eligible
  providers, with integrity verification and bounded resource use.
- Provider information is authenticated, expiring, observable, and useful to
  partial-fetch decisions without requiring a central tracker.
- Replica targets converge under churn and do not create repair storms.
- A 1000-node simulation stays within documented control-plane, memory, and
  convergence budgets.
- A verified CAR/USB import can merge missing content, enroll the local node as
  a provider when permitted, and recover after interruption.
- Open, named-private, and offline-imported communities have documented
  admission and capability boundaries.
- The CLI and daemon make downloading, pinning, seeding, contribution limits,
  and swarm health understandable in both human and JSON output.
