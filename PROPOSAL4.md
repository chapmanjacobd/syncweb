# Proposal 4: Stable Links, Resolvers, and Mirrors

**Status:** Proposed
**Priority:** High
**Depends on:** PROPOSAL2, PROPOSAL5, PROPOSAL9

## Problem

A direct blob ticket is useful for immediate transfer, but it is not a durable
public reference. It may point at one node, lack a stable name, and provide no
standard way to resolve a newer version or alternate mirror.

## Goals

- Provide stable immutable and mutable links.
- Allow readers to use HTTPS without giving up content verification.
- Resolve publishers, mirrors, versions, and provider sets.
- Keep private links capability-based and cryptographically non-escalating.

## Design

Support three references:

```text
syncweb://content/<content-id>       # immutable content
syncweb://collection/<collection-id>@<version>
syncweb://name/<publisher>/<alias>   # signed mutable head
```

An optional HTTP gateway resolves these references to a signed manifest, then
serves verified range requests. The gateway may use any authorized provider.

Mutable aliases contain a signed pointer to an immutable manifest. They never
rewrite the content addressed by an old link. Version pinning is always available.

Public collections should advertise multiple providers and mirrors. Private
references continue to carry read capabilities, expiration, and optional path
restrictions.

## Scoped configuration

- **Network:** resolver/indexer allowlist and whether metadata may be public.
- **Folder:** public alias, version-head policy, mirror policy.
- **File:** public/ private visibility, expiration, and whether direct links are
  permitted.

## User-facing interface

```text
syncweb link create <file-or-collection>
syncweb link resolve <url>
syncweb link revoke <link>
syncweb mirror add <collection> <provider>
syncweb gateway run
```

## Implementation steps

1. Define immutable content and manifest URL formats.
2. Define signed mutable-head records and revocation/tombstone behavior.
3. Implement local resolution and provider fallback.
4. Add an HTTP gateway with range requests and verification.
5. Add mirror registration and version-aware redirects.

## Acceptance criteria

- An immutable link remains verifiable after the publisher changes its collection.
- A mutable alias resolves to the latest signed version or a clear offline error.
- A gateway can serve content from a mirror without changing its content ID.
- Revoking a private link prevents new authorized fetches; cached content policy is
  clearly documented rather than implying remote deletion.

## Grounded UX example

Links should make mutability and authority visible:

```console
$ syncweb link create climate-hourly --alias alice/climate-hourly
Mutable:   syncweb://name/alice/climate-hourly
Pinned:    syncweb://collection/col_91c@b3:19ac...
HTTPS:     https://gw.example/s/alice/climate-hourly

$ syncweb link resolve syncweb://name/alice/climate-hourly
Alias signed by: node5abcd...
Sequence: 12
Manifest: b3:19ac...
Providers: 2 fresh, 1 stale

$ curl -L -H 'Range: bytes=0-1048575' \
    https://gw.example/content/b3:8e7a...
```

Opening a mutable URL should display the resolved immutable version and offer
"copy pinned link." If all providers are offline, resolution should still return
a verified manifest when cached and clearly distinguish `known but unavailable`
from `name not found`.

## Code patterns and library candidates

Use one parser and resolver pipeline for CLI, gateway, and future GUI callers:

```rust
enum SyncwebRef {
    Content(ContentId),
    Collection { id: CollectionId, version: ManifestHash },
    Name { publisher: PublisherId, alias: Alias },
}

#[async_trait::async_trait]
trait Resolver {
    async fn resolve(&self, reference: &SyncwebRef)
        -> Result<ResolvedReference>;
}
```

- Use the `url` crate for syntax, percent-decoding, and round trips, but enforce
  scheme-specific segment counts and lengths in `SyncwebRef::from_str`.
- Use `axum`, `tower-http`, and `http-body-util` for the gateway. Support
  conditional requests and single byte ranges first; reject malformed or
  abusive multi-range requests explicitly.
- Stream through Iroh's verified blob reader and do not advertise bytes as
  successful before the requested range is verified.
- Store signed name records with a monotonic sequence number. Accepting a lower
  sequence than the locally observed head must produce a rollback warning or
  error.
- Put capability material in a URL fragment where feasible so ordinary HTTP
  access logs and resolver requests do not receive it; scrub secrets from all
  diagnostics.

## Pros and cons

**Pros**

- High usefulness: stable aliases are shareable by humans, while pinned links
  preserve reproducibility and HTTPS broadens client compatibility.
- Resolver/provider separation allows mirrors without weakening content
  verification.
- A shared typed parser reduces inconsistent security behavior across surfaces.

**Cons**

- High complexity once mutable names, rollback protection, revocation,
  capability secrecy, HTTP caching, and range semantics are combined.
- Running a public gateway adds bandwidth, abuse, logging, and operational
  responsibilities absent from a local-only peer.
- Revocation cannot retract previously downloaded content, which may surprise
  users unless the UX is explicit.
