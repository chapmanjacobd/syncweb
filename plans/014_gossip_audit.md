# Plan 014 — Gossip audit: interrogate each topic, merge/remove dead-in-the-water features

## Overview

syncweb uses eight gossip topics, each with its own producer(s), optional daemon listener,
and consumer. Several of these overlap in intent (signed signals about content/providers) or may
be effectively useless for small networks because gossip is ephemeral (both peers must be online
during the broadcast). This plan is an *interrogation* first: produce a decision table, then
merge/remove as directed. De-duplicating the common subscribe→split→publish code pattern is
secondary to deciding which features survive.

## Topic inventory

Seed constants live in `syncweb-core/src/constants.rs`:

| Topic | Seed | Producer (CLI) | Consumer / listener | Persistence |
|-------|------|----------------|---------------------|-------------|
| Package catalog | `CATALOG_TOPIC` | `package`/`publish collection` announce (`main.rs:2851`) | `package search` (ephemeral query) | ephemeral |
| Reports | `REPORT_TOPIC` | `moderation report --broadcast` (`indexing.rs:1402`) | daemon `listen_for_reports` → `save_content_reports` (`daemon.rs:1578`) | persisted on receipt |
| Link revocation | `REVOCATION_TOPIC` | `link revoke --broadcast` (`indexing.rs:321`) | daemon `listen_for_revocations` → `save_links` (`daemon.rs:1530`) | persisted on receipt |
| Trust stream | `TRUST_STREAM_TOPIC` | `trust stream publish` (`indexing.rs:1065`) | `trust stream subscribe` / reputation store | ticket-based + stream |
| Resilience/leases | `RESILIENCE_TOPIC` | mirror (`resilience.rs`) | `ResilienceService` lease announcements | ephemeral |
| Attestation | `ATTESTATION_TOPIC` | `attest create --broadcast` (`indexing.rs:1912`) | daemon `listen_for_attestations` → `save_attestations` (`daemon.rs:1501`); `attest verify` | persisted on receipt |
| Network | `NETWORK_TOPIC_PREFIX` per network | `network` membership/folder gossip (`network_manager.rs:348`, `daemon.rs:1282`) | network membership listeners | ephemeral + membership doc |
| Channel | `CHANNEL_TOPIC_PREFIX` per channel | editorial `package search --channel` | `Channel` (gossip or catalog backend, `editorial/channel.rs`) | ephemeral or docs |

## Open questions / interrogation targets

| # | Question | Notes |
|---|----------|-------|
| Q1 | trust stream vs attestation: are `trust stream publish` (provider trust signal) and `attest create --broadcast` (content claim) meaningfully distinct, or two flavors of "signed signal over gossip"? | Both sign a claim with Ed25519 and broadcast. Could merge into one signed-signal channel keyed by subject type. |
| Q2 | network vs resilience: network gossip = membership + folder discovery; resilience = provider leases for mirroring. Are these two overlapping "who-has-what" protocols? | Mirroring currently takes a `--network` OR a provider ID; leases and membership may collapse into one discovery layer. |
| Q3 | revocation usefulness: `link revoke --broadcast` only reaches peers online at broadcast time; the listener persists it, but is it *enforced* at `link resolve`? | Verify enforcement. If revocations are only ephemerally propagated, fold revocation into a persistent doc (catalog/trust doc) instead. |
| Q4 | moderation report broadcast: is `moderation report --broadcast` (REPORT_TOPIC) consumed meaningfully, or is local `moderation hide` the real enforcement? | `listen_for_reports` persists incoming reports; check whether they affect fetch/hide decisions. |
| Q5 | attest verify: with no online peers it returns empty (`workflow/indexing.rs::attest_verify_with_timeout`). Useful for small networks, or should verification read the persisted local index? | Ties to 012 Q3. |
| Q6 | ephemeral vs persistent: for a small-network, delay-tolerant tool, should signals default to persistent docs (catalogs) rather than ephemeral gossip? | Channels already offer both backends (`ChannelBackend`). Extend that pattern to the other topics. |

## Deliverable

A decision table (topic → keep/merge/remove → replacement) agreed with the user, then follow-up
implementation plans. Likely outcomes:

- Merge `TRUST_STREAM_TOPIC` + `ATTESTATION_TOPIC` + `REPORT_TOPIC` into one signed-signal
  topic (or persistent doc), with a `kind` discriminator.
- Make link revocation persistent (fold into the catalog/trust doc) and drop `REVOCATION_TOPIC`.
- Collapse resilience-lease discovery into network membership (or vice-versa).
- Keep `CATALOG_TOPIC` (package search) and per-network/per-channel topics pending 015/016.

## Code de-duplication (secondary, still do it)

Regardless of which topics survive, factor the repeated
`gossip.subscribe(id, boot)` → `GossipService::split(topic)` →
`TopicChannel::<T>::new(Arc, id, sender)` dance (repeated at `indexing.rs:321/1065/1125/1208/
1402/1912/1931`, `daemon.rs:1506/1535/1583`, `resilience.rs:2023`) into:

- `TopicChannel::open(gossip, topic_id, bootstrap) -> Result<TopicChannel<T>>`, and
- a shared `spawn_topic_listener(gossip, topic_id, shutdown, |msg| ...)` helper in
  `syncweb-core/src/gossip/` to replace `spawn_attestation_listener` / `spawn_report_listener` /
  `spawn_revocation_listener` (`daemon.rs:1197-1230`).

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
```

No behavioral verification until decisions land; the audit deliverable is the decision table.

## Files affected

- `syncweb-core/src/constants.rs`, `src/gossip/*`, `src/daemon/daemon.rs`
- `syncweb-core/src/indexing/*` (reputation, resilience, denylist, wot)
- `syncweb-cli/src/cli/indexing.rs`
- `docs/*` (trust/indexing/architecture)

## Dependencies

- Feeds 012 (metadata vs attestation), 015 (folder vs catalog), 016 (search merge).
