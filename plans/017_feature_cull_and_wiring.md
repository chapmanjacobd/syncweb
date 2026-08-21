# Plan 017 — Missing wiring + feature cull (post-014 cleanup)

## Overview

Plan 014 merged the three signed-signal topics into `SIGNAL_TOPIC`, removed dead
network/revocation gossip, wired the resilience lease listener, and de-duplicated
the gossip plumbing. It also left several *wiring* gaps open and did not decide
the fate of the weakest features. This plan (a) captures every remaining wiring
gap the 014 audit surfaced, (b) proposes culling the features whose designs were
never finished, and (c) lands the channel-backend default switch.

## What attestations / reports / trust signals are for

Before cutting, the record (re-education, verified against source):

| Feature | What it is | Consumer today | Verdict |
|---------|-----------|----------------|---------|
| Attestations (`attest create`/`verify`/`list`) | Signed claims about a content hash for WoT provenance/rights: `--license`, `--provenance`, `--derivative` (`wot.rs:966`). | `attest verify` (now reads the persisted index, 014), `attest list`, surfaced in `trust show` and WoT metadata search. | KEEP — concrete purpose, working consumer. |
| Reports (`moderation report --broadcast`) | Signed community flags ("content X is abusive") — `ReportRecord` → `content_reports_v2`. | None. Never consulted by any fetch/hide decision; the intended denylist consumer has zero call sites (`denylist.rs:245-263`). | REMOVE — fully inert. |
| Trust signals (`trust stream publish`/`subscribe`) | Signed provider-quality observations ("provider P failed to serve H") feeding `ProviderReputationStore` → mirror/replication provider ranking (`reputation.rs:556`). | Local reputation is also fed by direct fetch observations (`record_failure_at`/`record_success`); the gossip stream requires a manual `trust stream subscribe` (250 ms drain or `file://` ticket). | CULL gossip sharing; keep the local reputation store + `trust provider vouch/distrust`. Re-implement sharing when the design is concrete. |

## Missing wiring inventory (from the 014 audit)

| # | Gap | Current state | Proposal |
|---|-----|---------------|----------|
| W1 | Network membership doc has no writer | `spawn_membership_listeners` reads `sys/network/members` (`daemon.rs:1410`) but nothing writes it; `network_doc_namespace()` (`membership_doc.rs:114`) is dead code; CLI `network create` never sets `doc_ticket`. | Wire the writer: the owner signs a `SignedMemberList` on create/invite/kick, and the doc ticket flows through `NetworkTicket` so removal detection works. This is the one docs-based wiring already designed. |
| W2 | No lease producer on `RESILIENCE_TOPIC` | 014 wired the daemon listener (persist incoming) + mirror DB hydration; nothing announces leases. | Announce newly created/valid leases at their creation sites (CLI `download_blob` `cli/indexing.rs:527`, `provider add`); optionally a daemon periodic re-announce. Leases stay on gossip (time-bound events). |
| W3 | Denylist never enforced | `Denylist::check_fetch`/`check_discovery`/`is_blocked` and their `DenylistService` wrappers have zero call sites outside `denylist.rs`; `filter add`/`filter subscribe` only store rules. | Wire `check_discovery` into catalog/channel search and `check_fetch` into the fetch path — or document as storage-only and defer. |
| W4 | Moderation only filters metadata display | `moderation_decision` has exactly one production caller (`trust show` display, `cli/indexing.rs:653`); hides filter `search_metadata` output, not blob fetch/serve. | Decide: enforce at fetch/serve, or document display-only. Tied to W3. |
| W5 | Channel default backend | `Channel::new`/`ChannelBackend::default()` were Gossip; `package search --channel` fallback created a gossip channel implicitly. | DONE in this plan — default is now `Catalog`; the deliberate gossip-search fallback is explicit `with_backend(Gossip)`. |
| W6 | 014 leftovers | `revocation_topic_id()` is a deprecated dead stub (`links.rs:1336`); `SignedGossipMessage for PrivateLink` is now unused; `SignedSignal::kind()` is unused. | Remove/reconcile during the cull. |

## Proposed changes

### 1. Feature cull (confirm before implementing)
- Remove reports: `ReportRecord` (`indexing.rs:3022`), `content_reports_v2` table + `save_content_reports`/`load_content_reports`, `SignedSignal::Report`, daemon report persistence in `IncomingSignals` (`daemon.rs`), CLI `moderation report` + `handle_moderation_report`, and the `moderation_hide_with_reason_and_report_broadcast` workflow test.
- Cull trust-stream gossip: drop `SignedSignal::Trust`, the `trust stream` CLI (`handle_trust_stream`, `receive_trust_signals`), and daemon trust-signal persistence; keep `ProviderTrustRecord`/`ProviderTrustSignal` local semantics and `trust provider vouch/distrust` (local-only, no `--broadcast`). Keep `ProviderReputationStore` fed by direct fetch observations.
- If both go, `SIGNAL_TOPIC` carries only attestations — either collapse `SignedSignal` to attestations or keep the enum for future signal kinds.

### 2. Wiring (implement after cull decisions)
- W1: writer for the membership doc + ticket propagation (owner signs member list; invite/kick/leave update it).
- W2: a `lease_announce` helper plus call sites at lease creation; keeps `RESILIENCE_TOPIC` gossip.
- W3/W4: enforce denylist + moderation at fetch/discovery, or document them as storage/display-only (decide; both are currently inert bookkeeping).

### 3. Landed in this plan
- `ChannelBackend` default → `Catalog` (`editorial/channel.rs`); `Channel::new` → catalog backend.
- `package search --channel` gossip fallback is now explicitly `ChannelBackend::Gossip` (`daemon/ipc.rs:2551`), so the default change does not silently redirect the fallback.
- Re-export `ChannelBackend` from `editorial.rs`.

## Verification

```sh
make check && make lint
cargo test -q --all-targets --all-features
```

## Files affected

- `syncweb-core/src/indexing/signals.rs`, `indexing.rs`, `indexing/reputation.rs`, `indexing/denylist.rs`, `indexing/links.rs`
- `syncweb-core/src/daemon/daemon.rs`, `daemon/ipc.rs`
- `syncweb-core/src/net/membership_doc.rs`, `net/network_manager.rs`
- `syncweb-core/src/editorial/channel.rs` (done), `editorial.rs`
- `syncweb-cli/src/cli/indexing.rs`, `src/main.rs`, `src/cli/commands.rs`
- tests (`workflow/indexing.rs`, `full_suite_test.rs`) + `docs/*` (indexing, security-model, commands)

## Dependencies

- Runs after 014; coordinates with 015 (namespace provisioning / `DocsEngine::create_or_open_namespace`) for W1 and 016 (search unify) for W3/W5.