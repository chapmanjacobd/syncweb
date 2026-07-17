# Proposal 2: Collections and Generalized Manifests

**Status:** Proposed
**Priority:** High
**Depends on:** Existing blob/doc model

## Problem

The current package model is useful for datasets and software releases, but general
file sharing needs a broader abstraction. A knowledge platform also needs stable
content identity independent of local paths, package names, or one physical folder.

## Goals

- Generalize `PackageManifest` into a reusable collection model.
- Preserve immutable content identity while supporting mutable version heads.
- Represent folders, datasets, media libraries, archives, and knowledge bases.
- Permit virtual collections assembled from content-addressed entries.

## Design

Introduce:

```text
CollectionManifest {
  collection_id
  title, description, type, tags
  entries[]
  version, parent_version
  publisher, license, provenance
  created_at
  signature
}

CollectionEntry {
  content_id
  logical_path
  name
  size, media_type
  role                  # primary, preview, transcript, metadata, etc.
  relationships[]
}
```

`logical_path` is presentation metadata, not the content identity. The same blob
can occur in several collections without duplication. A mutable collection name
resolves to a signed immutable manifest/version.

Packages become a collection type with dependency and atomic-install semantics.
The existing package workflow should remain available as a convenience profile,
not as the base data model.

## Scoped configuration

- **Network:** permitted collection types and default metadata schema.
- **Folder:** collection identity, default publication/version policy.
- **File:** logical name, role, metadata, and collection membership.

One file may belong to multiple virtual collections, subject to each collection's
access policy.

## User-facing interface

```text
syncweb collection init <path> --name <name> --type <type>
syncweb collection add <collection> <path-or-content-id>
syncweb collection publish <collection>
syncweb collection versions <collection>
syncweb collection diff <collection> <version-a> <version-b>
```

Existing `syncweb package` commands map to collection commands internally.

## Implementation steps

1. Define canonical manifest serialization and signature rules.
2. Migrate package manifests to the generalized schema.
3. Add immutable version manifests and signed mutable heads.
4. Add virtual collection membership and relationship fields.
5. Update catalog, resolver, and install flows to consume manifests.

## Acceptance criteria

- A single content hash can be referenced by multiple collections.
- A collection can be versioned without copying unchanged blobs.
- Package install/upgrade behavior remains available through the generalized model.
- A consumer can inspect a manifest without fetching all entry content.
