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

## Design

Add signed attestations for publisher metadata, licenses, provenance, and curation.
Trust policies are local or network-scoped; avoid a universal reputation score.

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
syncweb attest <content> --license <license>
syncweb report <record> --reason <reason>
syncweb moderation ls
syncweb moderation hide <record>
```

## Implementation steps

1. Add license, publisher, and provenance fields to manifests.
2. Define signed attestations and trust-policy evaluation.
3. Add catalog moderation records and audit history.
4. Add revocation/tombstone behavior to resolvers and indexers.
5. Document the distinction between hiding, revoking access, and deleting local
   copies.

## Acceptance criteria

- Search results expose publisher, license, provenance, and verification state.
- A network can hide or quarantine a record without corrupting its content hash.
- Moderation decisions are signed, scoped, auditable, and optionally expiring.
- Private access revocation does not claim to erase already downloaded content.
