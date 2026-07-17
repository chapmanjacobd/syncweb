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

## Grounded UX example

Alice publishes a public climate collection while keeping the raw participant
file private:

```console
$ syncweb catalog publish ./climate-hourly
Published collection climate-hourly@1.2.0
Catalog: local, research-index
Indexed: 46 entries
Skipped by policy: raw/participants.csv
Record: cat_7f3a...

$ syncweb search 'hourly temperature' --catalog research-index
ID          TITLE                    SIZE     PROVIDERS  TRUST
clm_91c...  Climate hourly 1.2.0     2.3 GiB  2 (fresh)  signed

$ syncweb search 'hourly temperature' --json |
    jq -r '.[0].collection_id' |
    xargs syncweb queue add
Queued 46 entries (2.3 GiB); metadata and signatures verified
```

The default table should communicate whether a result is merely indexed,
cryptographically valid, and currently fetchable. `syncweb search --explain
clm_91c...` should show which indexer returned the record, when its provider
leases were refreshed, and why local policy did or did not hide it.

## Code patterns and library candidates

Keep indexing behind a trait so local SQLite search can ship before an HTTP or
federated implementation:

```rust
#[async_trait::async_trait]
trait Catalog {
    async fn upsert(&self, record: Verified<CatalogRecord>) -> Result<()>;
    async fn tombstone(&self, record: Verified<CatalogTombstone>) -> Result<()>;
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchHit>>;
}
```

- Use `rusqlite` with bundled migrations for the first local index. SQLite FTS5
  can cover title, aliases, description, and tags without another service.
- Use `serde` plus one explicitly canonical encoding shared with PROPOSAL2.
  Sign the canonical bytes with the existing Iroh Ed25519 identity; do not sign
  a reserialized in-memory object.
- Use `axum` and `tower` for the optional catalog API, with request-size limits,
  pagination, timeouts, and authorization middleware.
- Model verification as `Verified<T>` so untrusted gossip or HTTP input cannot
  accidentally enter the searchable store through the same method as checked
  records.
- Store provider leases separately from descriptive records. Availability
  changes frequently and should not require republishing otherwise stable
  metadata.

## Pros and cons

**Pros**

- Very high usefulness: discovery removes the need to exchange raw tickets and
  enables the search/browse/queue loop expected from a sharing application.
- SQLite FTS5 provides a valuable local-first slice with modest operational
  complexity and no required server.
- Signed records keep search infrastructure outside the trusted transfer path.

**Cons**

- Medium initial complexity for canonical signing, expiry, tombstones, policy
  filtering, migrations, and useful ranking; federation raises this to high.
- Public indexing creates privacy and moderation obligations even though content
  transfer remains peer-to-peer.
- Search quality can consume substantial product effort without improving core
  synchronization correctness.
