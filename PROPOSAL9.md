# Proposal 9: Scoped Deployment Modes

**Status:** Proposed
**Priority:** High
**Depends on:** All public-sharing proposals

## Problem

"Private," "community," and "public archive" are deployment models, but making
them application-wide is too coarse. A single installation may contain private
credentials, a team folder, a public dataset, and one publicly shared file.

## Goals

- Configure deployment policy at network, folder, and file granularity.
- Make inherited exposure explicit and safe.
- Support private, community, and public-archive behavior in one node.
- Prevent a broad policy from silently publishing sensitive content.

## Deployment profiles

```text
Private
  invite-only discovery, encrypted/private metadata, capability-based access

Community
  searchable signed metadata, selected federation/indexers, optional profiles

PublicArchive
  stable public links, public metadata, pinning, mirrors, version history
```

The profile controls defaults; it does not replace per-object capabilities.

## Policy resolution

Policies are resolved in this order:

```text
application defaults
  -> network policy
  -> folder policy
  -> file policy
```

An explicit value at a more-specific scope overrides an inherited value.
Security-sensitive settings are monotonic: a child may restrict publication,
indexing, replication, or access, but cannot silently broaden a parent policy.
A file may be explicitly promoted to public visibility only with a confirmation
that records the override.

Separate fields should be inherited independently. For example, a public folder
may still keep one file out of full-text indexing, and a community network may
permit a private folder.

## Configuration example

```toml
[deployment]
default_profile = "Private"

[networks.research.deployment]
profile = "Community"
catalogs = ["research-index"]
allow_federation = true

[folders.research-data.deployment]
profile = "PublicArchive"
public_alias = "climate-hourly"
pin_duration = "365d"

[folders.research-data.files."raw/participants.csv".deployment]
profile = "Private"
indexing = "none"
replication = "disabled"
```

The exact configuration format may change, but the model must remain scope-aware.
Policies should also be representable through CLI flags and API objects.

## User-facing interface

```text
syncweb deployment show [file-or-folder-or-network]
syncweb deployment set <scope> --profile <profile>
syncweb deployment explain <file>
```

`explain` shows each effective value and the scope that supplied it.

## Implementation steps

1. Define profile fields separately from capabilities and transport settings.
2. Implement inheritance and monotonic security validation.
3. Add file, folder, and network persistence.
4. Add `show`, `set`, and `explain` commands.
5. Add publication/indexing/replication integration tests for mixed scopes.

## Acceptance criteria

- One node can safely host private, community, and public content simultaneously.
- A file-level restriction wins over automatic folder or network behavior.
- Public promotion requires an explicit operation and produces an audit event.
- Users can explain why a file is public, private, indexed, or replicated.

## Grounded UX example

Before publishing a mixed folder, the user can inspect effective policy:

```console
$ syncweb deployment explain \
    research-data/raw/participants.csv
SETTING       EFFECTIVE       SOURCE                         GUARD
visibility    private         file override                  restrictive
indexing      none            file override                  restrictive
replication   disabled        file override                  restrictive
gateway       enabled         folder: research-data          blocked by visibility

$ syncweb catalog publish research-data --dry-run
Would publish: 46 entries
Would skip:     raw/participants.csv (private at file scope)

$ syncweb deployment set file:summary.csv --profile PublicArchive
Promotion private -> public requires confirmation.
Type the logical path to continue: summary.csv
Promoted summary.csv; audit event audit_410...
```

Noninteractive promotion should require an explicit flag such as
`--confirm-public summary.csv`, not a generic `--yes`. Configuration errors that
would broaden access must fail closed and name the field and source scopes.

## Code patterns and library candidates

Represent every field as a resolved value with provenance:

```rust
struct Resolved<T> {
    value: T,
    source: PolicyScope,
    explicit: bool,
}

struct EffectivePolicy {
    visibility: Resolved<Visibility>,
    indexing: Resolved<Indexing>,
    replication: Resolved<Replication>,
    gateway: Resolved<GatewayAccess>,
}
```

- Implement `resolve(defaults, network, folder, file)` as a pure function over
  typed `PolicyPatch` values. Table-driven unit tests should enumerate every
  parent/child combination for security-sensitive fields.
- Use restrictive lattices where the domain supports one, for example
  `Public > Community > Private` with child inheritance computed by `min` unless
  an audited promotion is explicitly supplied.
- Parse TOML with `serde` and `toml`, retain source paths for diagnostics, and
  reject unknown security-related fields rather than silently ignoring typos.
- Return a `DecisionTrace` consumed by CLI, catalog, ingestion, gateway, and
  replication code. Individual subsystems must not reimplement inheritance.
- Write promotion events before publishing side effects, then bind the resulting
  audit ID to the catalog or gateway operation.

## Pros and cons

**Pros**

- Very high usefulness and safety: mixed private/community/public data is a core
  stated use case, and one explainable engine prevents policy drift.
- Pure resolution logic is comparatively easy to test exhaustively.
- Field-level provenance makes surprising behavior diagnosable for users and
  downstream subsystems.

**Cons**

- Medium foundational complexity that becomes high as each proposal adds policy
  fields and exceptions.
- Not every setting forms a simple restrictive ordering; overusing monotonic
  rules can prevent legitimate configurations or produce hidden interactions.
- File-level persistence can become large and unwieldy for collections with
  millions of entries, requiring sparse overrides and careful indexing.
