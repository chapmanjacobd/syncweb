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
