# Communal membership and invitations

Communities use a stable random identity rather than a display name as their
protocol boundary. A name is only presentation metadata; two communities may
share a label without sharing membership or content authority.

## Invitation format

`syncweb-core::net::CommunityInvitation` is the portable invitation format.
It serializes as bounded JSON and can be carried in any of these forms:

* `syncweb://community/<hex-json>` for command-line copy/paste and QR codes;
* the raw JSON bytes in a file or CAR metadata block; or
* an application-specific short code that decodes to the same bytes.

The signed fields are:

| Field | Meaning |
|-------|---------|
| `version` | Wire-format version. |
| `community_id` | 32-byte random community identity. |
| `name` | Human-readable label; never used for authorization. |
| `issuer` | Ed25519 public key that issued the invitation. |
| `subject` | Optional device-bound recipient. |
| `capabilities` | Independent permissions listed below. |
| `issued_at` / `expires_at` | Freshness window; expiry is optional for offline exchange. |
| `nonce` | 16-byte uniqueness value preventing identical invitations. |
| `signature` | Ed25519 signature over the versioned invitation context and all other fields. |

Consumers must verify the signature, compare the community identity to the
local scope, reject future-dated or expired invitations, and enforce the
optional subject before admitting a device. The issuer does not need to be
online after an invitation has been distributed; a later signed revocation
record can be propagated through the community's normal document or CAR
exchange path.

`CommunityAdmissionPolicy` implements that ownerless flow. Each node chooses
the issuer keys it trusts, validates invitations against the stable community
identity, and accepts signed `CommunityRevocation` records from those issuers.
Revocations can target a device for all of its invitations or a single
invitation nonce, so admission continues to work during an offline partition
without silently treating a display name as authority.

## Community-scoped content

Collection manifests can carry the same stable community identity as an
invitation. A scoped manifest includes `community_id` in its signed,
content-addressed bytes, and `validate_for_community` requires an exact match:
an unscoped node rejects a named-community manifest, while a named community
rejects an unscoped manifest. Scoped manifests must also carry a valid
Ed25519 signature.

`CommunityContentTicket` wraps an ordinary Iroh blob ticket with a community
identity, issuer signature, and optional expiry. It serializes as
`syncweb://content/<hex-json>` or bounded JSON suitable for a CAR metadata
block. Consumers must validate the issuer signature, expiry, and exact
community before passing the embedded blob ticket to a fetch operation.
Provider leases apply the same exact-scope rule to provider observations, so
manifests, content tickets, and provider availability cannot silently cross
community boundaries.

## Independent capabilities

The invitation does not use one overloaded "member" permission. Each of these
permissions is checked independently:

* `discovery`: learn community peers and bootstrap topics;
* `read`: fetch community metadata and content;
* `write`: publish documents or collection updates;
* `pin`: retain content under the community's replication policy;
* `relay`: offer or consume relay assistance; and
* `contribute`: upload content or bandwidth to the community.

Invitations may also carry an operator-visible role (`member`, `contributor`,
`relay`, `moderator`, or `owner`). Capabilities remain independently signed
permissions, and invitation validation rejects a role that grants a capability
outside its role's ceiling.

The core API includes `read_only()` and `all()` presets, but callers should
construct the smallest capability set needed for each invitation. A read-only
invitation therefore cannot be promoted into a writer by merely discovering a
peer or receiving a provider lease.

## Moderation hooks

`CommunityModerationPolicy` provides local enforcement hooks for communal
transfers. Operators can denylist node identities, cap individual object
sizes, and apply per-node byte budgets over a fixed time window. Accepted and
rejected decisions are retained as a bounded `CommunityAuditEvent` stream for
operator-visible status and audit output; the policy never makes a remote
denylist authoritative for another node.
