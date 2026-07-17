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

## Grounded UX example

Availability should report evidence rather than one misleading peer count:

```console
$ syncweb health climate-hourly
CONTENT       LOCAL  LEASES        CHECKED       ASSESSMENT
b3:8e7a...    yes    3 (2 fresh)   2 reachable   healthy
b3:3d11...    no     1 fresh       unreachable   at risk

$ syncweb seed b3:3d11... --for 30d --max-storage 20GiB
Pinned 84 MiB until 2026-08-16; lease renews every 20m

$ syncweb replicate climate-hourly --min-providers 3 --dry-run
Would request 1 additional replica for 6 entries (412 MiB)
No action: raw/participants.csv (replication disabled by file policy)
```

`health --explain` should include lease age, last successful transfer or probe,
and whether providers are distinct identities—not imply geographic or operator
independence that the node cannot verify.

## Code patterns and library candidates

Keep claims, observations, and probe results as separate types:

```rust
struct Availability {
    advertised: Vec<Verified<ProviderLease>>,
    observed: Vec<TransferObservation>,
    probed: Vec<LivenessResult>,
}

struct ReplicationDecision {
    content: ContentId,
    desired: u16,
    eligible_providers: Vec<NodeId>,
    reason: DecisionReason,
}
```

- Store leases in SQLite indexed by `(content_id, provider, expires_at)` and
  delete or ignore them by expiry in queries. Use the signed lease timestamp for
  claims and the local monotonic clock for probe throttling.
- Schedule renewal and probes with `tokio_util::time::DelayQueue` or a bounded
  periodic worker; add jitter so many nodes do not renew simultaneously.
- Reuse the transfer scheduler from PROPOSAL3 for replication. Replication jobs
  need the same quotas, cancellation, and policy checks as user downloads.
- Score providers with a small, explainable tuple—valid lease, recent success,
  latency bucket, failures—rather than a difficult-to-debug global numeric
  reputation.
- Use rendezvous hashing to spread replicas consistently when a trusted set of
  candidate seeders is known, but treat it as placement preference rather than
  proof that a replica exists.

## Pros and cons

**Pros**

- High usefulness for public archives and intermittently connected publishers:
  mirrors make stable links meaningfully durable.
- Explicit evidence and expiry are much more honest than the existing
  observation-only peer cache.
- Reusing queue and policy infrastructure limits duplication.

**Cons**

- Medium-to-high complexity for leases, probes, quotas, pin lifecycle, provider
  selection, and reconciliation after failures.
- Provider identity does not prove failure-domain independence, so a requested
  replica count can overstate resilience.
- Automatic replication consumes storage and bandwidth and can amplify unwanted
  content; conservative opt-in defaults are essential.
