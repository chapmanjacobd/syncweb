# Proposal 10: Delivery Plan and Vertical Slice

**Status:** Proposed
**Priority:** High
**Depends on:** PROPOSAL1 through PROPOSAL9

## Problem

The full plan contains many valuable features, but implementing discovery,
packages, CRDTs, DHT tracking, BEP compatibility, knowledge objects, and
moderation simultaneously creates a large integration risk.

## Goals

- Deliver a useful general-sharing workflow early.
- Validate the catalog/data-plane boundary before adding federation.
- Keep advanced features compatible with the first manifest and policy formats.

## Milestones

### Milestone 1: Verified sharing foundation

- generalized collection manifest;
- immutable content IDs and signed metadata;
- local catalog and full metadata search;
- publish, browse, fetch, and verify;
- scoped private/public policy evaluation.

### Milestone 2: Public and community sharing

- stable resolver and HTTP gateway;
- provider leases and mirror fallback;
- queues, slots, quotas, and saved searches;
- community catalog API and moderation records.

### Milestone 3: Knowledge workflows

- derivative manifests and local full-text search;
- annotations, citations, provenance, and collection relationships;
- network-scoped collaborative workspaces.

### Milestone 4: Federation and resilience

- catalog federation;
- DHT/gossip bootstrap improvements;
- replication policies and multi-provider fetching;
- optional semantic indexes.

### Later compatibility work

Defer BEP interoperability, advanced CRDT behavior, and highly optimized peer
caches until the core collection/catalog workflow is stable. They should integrate
through the same manifest, provider, and policy interfaces rather than define
them.

## End-to-end acceptance test

1. A publisher creates a collection containing public and private files.
2. The public files receive signed metadata and a stable alias.
3. A community catalog indexes the public metadata.
4. A reader searches, previews, queues, and fetches a file.
5. A mirror takes over when the publisher is offline.
6. The reader verifies the content and its provenance.
7. A private file remains undiscoverable and unreplicated.
8. A permitted user adds an annotation anchored to the fetched content.

The same test should be run with the public file configured at network, folder,
and file scopes to verify policy inheritance and overrides.

## Exit criteria

- The end-to-end workflow works without raw ticket manipulation.
- Every retrieved content item is verified against its manifest.
- Scope resolution is deterministic, explainable, and covered by tests.
- Public records can be indexed and removed without mutating immutable content.
- The first release has a clear path to add federation without changing user
  identity or content identity.

## Grounded vertical-slice demo

The first releasable demo should fit in one local integration test with two
nodes and no federation:

```console
# Publisher
$ syncweb collection init ./demo --name field-notes --type documents
$ syncweb deployment set folder:./demo --profile PublicArchive
$ syncweb deployment set file:./demo/private.txt --profile Private
$ syncweb collection publish ./demo --catalog local
Published field-notes@1; 3 public entries, 1 private entry skipped

# Reader
$ syncweb search field-notes --catalog http://publisher.test/catalog
$ syncweb browse field-notes
$ syncweb queue add field-notes --include notes.pdf
$ syncweb transfers --wait
Complete; manifest, publisher signature, and 1 blob verified
```

The test should then stop the publisher, start a configured mirror, repeat the
fetch from a clean reader store, and assert that `private.txt` is absent from the
catalog, mirror pins, and gateway responses. Fixed keys, clocks, and local Iroh
endpoints make the scenario reproducible.

## Implementation patterns and library candidates

- Build a library crate plus a thin `clap` binary so integration tests invoke
  typed command services without parsing terminal output.
- Use `tempfile` for isolated stores, `assert_cmd` and `predicates` for a small
  number of true CLI contract tests, and `insta` only for intentionally stable
  human-readable output.
- Introduce traits for `Clock`, catalog transport, and provider resolution.
  Production adapters use Iroh/HTTP; deterministic fakes cover expiry, offline
  publishers, and mirror fallback.
- Gate milestone features with modules and explicit configuration, not divergent
  manifest formats. Old clients should safely ignore optional fields or reject a
  schema version they cannot interpret.
- Track a milestone dependency rule: Milestone 1 may define interfaces needed by
  later work, but it must not require federation, semantic search, generalized
  CRDTs, or BEP compatibility to ship.

## Pros and cons

**Pros**

- Very high usefulness: the vertical slice validates real user value and the
  riskiest boundaries—manifest, policy, catalog, transfer, and verification—
  before broadening scope.
- Milestones create explicit deferral points for attractive but nonessential
  features.
- A deterministic two-node test becomes a durable compatibility contract.

**Cons**

- The first milestone is still medium-to-high complexity because manifests,
  identity, policy, local search, and transfer must all interoperate.
- Designing extension points too early can over-generalize the foundation;
  designing none can force schema changes later.
- Mirror failover in the first full end-to-end test adds work, but omitting it
  would leave the stable-link claim largely unvalidated.
