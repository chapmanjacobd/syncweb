# Communal topology policy

Communal discovery has two separate concerns: finding a useful peer and
deciding whether two community scopes may exchange state. The topology policy
keeps both bounded and leaves transport availability as a fallback problem.

## Optional cache-peer role

`syncweb_core::RendezvousCache` is an optional local rendezvous/cache-peer
role. A node may enable it when a LAN has many intermittently connected
devices. It stores short-lived `InventorySummary` records keyed by provider
and community, then returns a deterministic bounded sample of provider IDs for
bootstrap.

The cache is deliberately disposable:

* summaries are scoped to either the open daemon or one `CommunityId`;
* records have an observation and expiry time;
* record counts, root counts, and content counts are bounded;
* stale updates cannot replace newer observations; and
* direct discovery, gossip, DHT, and relay paths remain valid when the cache
  is unavailable.

The cache contains advisory availability information only. It does not grant
membership, authorize a fetch, or make content valid. Provider leases and
content verification remain authoritative.

## Partition merge policy

`community_merge_decision` applies the community identity rule when two LAN
segments reconnect:

* two open scopes may merge;
* two equal explicit community IDs may merge, even if their display names
  differ; and
* an open scope and a named scope, or two different named IDs, stay separate.

`PartitionMergePolicy` supplies a deterministic exponential reconnect delay
with pair-specific jitter. The delay is bounded by the configured maximum and
is symmetric for the two partition IDs. This spreads inventory reconciliation
after a large segment reconnects without making the result depend on wall-clock
randomness, which also keeps simulations and recovery behavior reproducible.
