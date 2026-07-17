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
