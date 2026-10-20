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

## Independent capabilities

The invitation does not use one overloaded "member" permission. Each of these
permissions is checked independently:

* `discovery`: learn community peers and bootstrap topics;
* `read`: fetch community metadata and content;
* `write`: publish documents or collection updates;
* `pin`: retain content under the community's replication policy;
* `relay`: offer or consume relay assistance; and
* `contribute`: upload content or bandwidth to the community.

The core API includes `read_only()` and `all()` presets, but callers should
construct the smallest capability set needed for each invitation. A read-only
invitation therefore cannot be promoted into a writer by merely discovering a
peer or receiving a provider lease.
