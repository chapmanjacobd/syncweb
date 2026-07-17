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
