# Proposal 1: Catalog and Discovery

**Status:** Proposed
**Priority:** High
**Depends on:** PROPOSAL2, PROPOSAL4, PROPOSAL9

## Problem

Blobs, tickets, gossip, and DHT records provide transport and rendezvous, but they
do not provide a durable, searchable, or complete catalog. Gossip announcements
cannot support reliable global search by themselves, and a peer-availability cache
only describes what one node has observed.

## Goals

- Make files and collections discoverable without knowing a ticket or `NodeId`.
- Support private, community, and public catalogs using the same record format.
- Return verifiable metadata and provider hints before downloading content.
- Keep catalog operation independent from the blob transfer data plane.

## Design

Introduce a signed `CatalogRecord`:

```text
CatalogRecord {
  record_id
  content_id or collection_id
  name, aliases, description, tags
  media_type, size, language, duration
  publisher, license, provenance
  version_head, created_at, expires_at
  providers[]
  signature
}
```

Catalog records are immutable announcements. Updates create a new signed record;
removals create signed tombstones. Records have an expiry so stale availability
does not look authoritative.

An **indexer** stores and searches records. An indexer may be:

- local and private;
- hosted by a community;
- federated with other trusted indexers; or
- populated from public gossip/DHT announcements.

Search results must include the record signature and content or manifest hash.
Search is therefore an aid to discovery, not a substitute for content verification.

## User-facing interface

```text
syncweb search "query" [--catalog <name>]
syncweb browse <publisher-or-collection>
syncweb catalog publish <path>
syncweb catalog remove <record-id>
syncweb watch "query"
syncweb catalog servers
```

`watch` is the Soulseek-style saved-search workflow. It produces notifications
when a new signed record matches, without requiring continuous blob downloads.

## Scoped configuration

- **Network:** default catalog visibility, allowed indexers, query policy.
- **Folder:** collection metadata, publication state, searchable fields.
- **File:** opt out of indexing, override title/tags/license, or publish separately.

The effective policy follows PROPOSAL9. A file may be made more private than its
folder or network, but a broad network policy must not silently expose a file.

## Implementation steps

1. Define and sign `CatalogRecord`, provider references, expiry, and tombstones.
2. Build a local SQLite-backed index with exact, prefix, tag, and metadata search.
3. Add catalog import/export and record verification.
4. Add optional gossip ingestion and a simple HTTP catalog API.
5. Add federation only after local indexing and authorization are stable.

## Acceptance criteria

- A user can publish a collection, search it by metadata, verify the result, and
  fetch content without knowing the publisher's ticket in advance.
- Expired providers and tombstoned records are excluded from default results.
- Private folders do not appear in public or community indexes.
- Search results remain useful when the original publisher is offline but a
  listed provider has the content.
