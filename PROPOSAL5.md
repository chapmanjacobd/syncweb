# Proposal 5: Availability, Seeding, and Replication

**Status:** Proposed
**Priority:** High
**Depends on:** Existing peer tracker, PROPOSAL2, PROPOSAL4

## Problem

The current peer tracker records observations from local transfers. It cannot
reliably answer how many providers exist globally, whether a provider is still
online, or whether rare content will survive the publisher going offline.

## Goals

- Make provider availability explicit and time-bounded.
- Improve rare-content resilience through opt-in replication.
- Support multi-provider fetching and transparent fallback.
- Distinguish local observations from signed provider claims.

## Design

Add a `ProviderLease`:

```text
ProviderLease {
  content_id or collection_id
  provider
  capabilities       # range, preview, full, metadata
  expires_at
  observed_at
  signature
}
```

Providers renew leases while serving or pinning content. Consumers may perform
lightweight liveness checks before displaying availability. A replication policy
can request a minimum number of independent providers.

The fetch engine should select providers by availability, latency, reliability,
and policy. Where supported, different ranges or entries may be fetched from
different providers.

## Scoped configuration

- **Network:** default provider trust, replication budget, and relay policy.
- **Folder:** minimum provider count, pin duration, and preferred regions/peers.
- **File:** priority, required redundancy, and whether replication is allowed.

Sensitive files must never be replicated solely because a parent folder requests
general resilience.

## User-facing interface

```text
syncweb health <file-or-collection>
syncweb seed <file-or-collection> [--for <duration>]
syncweb replicate <collection> --min-providers <n>
syncweb providers <content-id>
```

## Implementation steps

1. Add signed provider leases and expiry handling.
2. Separate observed availability from advertised availability in the data model.
3. Add provider selection and fallback to the fetch engine.
4. Add pin and replication jobs with quotas.
5. Add health reporting with confidence and freshness indicators.

## Acceptance criteria

- Health output distinguishes local observation, provider advertisement, and
  successful liveness checks.
- A collection can be configured to maintain a minimum provider count.
- Fetching succeeds when the original publisher is offline but a valid mirror is
  available.
- Expired or revoked leases do not count toward redundancy.
