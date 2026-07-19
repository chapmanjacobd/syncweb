# Proposal 8: Trust, Governance, and Moderation

**Status:** Proposed
**Priority:** Medium
**Depends on:** PROPOSAL1, PROPOSAL2, PROPOSAL4, PROPOSAL6

## Problem

Content hashes prove integrity, not accuracy, authorship, legality, or quality.
Public and community catalogs also require mechanisms for spam, abuse, takedown,
and conflicting claims.

## Goals

- Separate cryptographic integrity from contextual trust.
- Make publisher identity, license, and provenance visible.
- Support local and community moderation without requiring one global authority.
- Provide clear behavior for removal and revocation.
- Optionally support a web of trust for decentralized identity verification.

## Design

Add signed attestations for publisher metadata, licenses, provenance, and curation.
Trust policies are local or network-scoped; avoid a universal reputation score.
Support an optional web of trust where users can cryptographically delegate trust or endorse other publishers.

An indexer or network may maintain moderation records:

```text
ModerationRecord {
  subject_id
  action              # hide, warn, quarantine, restore
  reason
  scope
  expires_at
  moderator
  signature
}
```

Moderation hides or de-prioritizes records in an index; it does not rewrite or
delete immutable content on other nodes. Private-network administrators may add
stronger access revocation.

## Scoped configuration

- **Network:** trusted publishers, moderators, licenses, and discovery policy.
- **Folder:** default license, publication review, and moderation behavior.
- **File:** explicit license, provenance, warning, or publication hold.

More-specific restrictions override broader publication defaults.

## User-facing interface

```text
syncweb trust show <content-or-publisher>
syncweb trust delegate <publisher>
syncweb attest <content> --license <license>
syncweb report <record> --reason <reason>
syncweb moderation ls
syncweb moderation hide <record>
```

## Implementation steps

1. Add license, publisher, and provenance fields to manifests.
2. Define signed attestations and trust-policy evaluation, including optional web of trust endorsements.
3. Add catalog moderation records and audit history.
4. Add revocation/tombstone behavior to resolvers and indexers.
5. Document the distinction between hiding, revoking access, and deleting local
   copies.

## Acceptance criteria

- Search results expose publisher, license, provenance, and verification state.
- A network can hide or quarantine a record without corrupting its content hash.
- Moderation decisions are signed, scoped, auditable, and optionally expiring.
- Private access revocation does not claim to erase already downloaded content.
- Users can optionally establish and verify a web of trust by signing other identities.

## Grounded UX example

Trust output should state facts and local policy decisions separately:

```console
$ syncweb trust show cat_7f3a...
Integrity:    valid signature, content hash verified
Publisher:    node5abc... (trusted by network "research")
License:      CC-BY-4.0 (publisher attestation)
Provenance:   derived from b3:901e...
Moderation:   warning from research-index: incomplete attribution

$ syncweb report cat_7f3a... --reason spam
Submitted report rep_12d... to research-index

$ syncweb moderation hide cat_7f3a... --scope local --for 30d
Hidden from local search until 2026-08-16
The content was not deleted from local storage or other peers.
```

Warnings should include who made the decision, its scope, and an audit link.
`--show-hidden` should require an explicit flag and retain warnings rather than
making records disappear in a way users cannot diagnose.

## Code patterns and library candidates

Evaluate trust as composable policy, not a score embedded in content:

```rust
struct TrustContext<'a> {
    subject: &'a Verified<CatalogRecord>,
    attestations: &'a [Verified<Attestation>],
    moderation: &'a [Verified<ModerationRecord>],
    policy: &'a TrustPolicy,
}

enum DiscoveryDecision {
    Show,
    Warn(Vec<Reason>),
    Hide(Vec<Reason>),
    Quarantine(Vec<Reason>),
}
```

- Reuse the canonical signed-record envelope and Ed25519 identity from manifests.
  Domain-separate signatures by record type and schema version.
- Express initial policy as typed TOML structs with `serde`, not a general
  scripting language. The engine should return a decision trace for
  `trust show` and PROPOSAL9's `deployment explain`.
- Store moderation records append-only in SQLite and derive the active view by
  scope, expiry, and latest signed action.
- Use SPDX license identifiers and optionally the `spdx` crate for syntax and
  compatibility checks. Display unknown license text without treating it as
  validated.
- Rate-limit report submission at the catalog API with `tower` middleware and
  keep reports distinct from moderator actions.

## Pros and cons

**Pros**

- High usefulness for any shared catalog: integrity alone cannot answer whether
  content is appropriately licensed, attributable, or acceptable in a network.
- Scoped, signed decisions avoid imposing a single global authority.
- Explainable decisions align with the project's broader policy model.

**Cons**

- High social and operational complexity even if the code is moderate:
  communities need moderators, appeals, retention rules, and abuse handling.
- Conflicting attestations and moderation decisions cannot always be resolved
  technically and may confuse users.
- License identifiers and signatures prove what was asserted, not that the
  assertion is legally or factually correct.
