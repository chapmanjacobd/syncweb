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

## Grounded UX example

A researcher can publish the same immutable CSV in a release package and in a
virtual reading list without storing it twice:

```console
$ syncweb collection init ./climate --name climate-hourly --type dataset
Created col_climate with 47 local entries

$ syncweb collection add col_reading-list \
    syncweb://content/b3:8e7a... --as data/hourly.csv --role primary
Added existing content (no blob copied)

$ syncweb collection publish col_climate
Published climate-hourly@1.0.0 (manifest b3:19ac...)

$ syncweb collection diff col_climate 1.0.0 1.1.0
A  docs/methodology.md       18 KiB
M  data/hourly.csv           b3:8e7a... -> b3:3d11...
=  45 unchanged entries      reused
Transfer required: 84 MiB of 2.3 GiB
```

`collection inspect --metadata-only` should fetch and verify only the manifest.
Materialization should reject `../`, absolute paths, duplicate normalized paths,
and platform-reserved names before any bytes are written.

## Code patterns and library candidates

Separate immutable manifests from mutable local drafting state:

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct CollectionManifestV1 {
    schema: SchemaVersion,
    collection_id: CollectionId,
    version: VersionId,
    parent: Option<ManifestHash>,
    entries: Vec<CollectionEntry>,
    publisher: PublicKey,
    signature: Signature,
}

struct CollectionHead {
    collection_id: CollectionId,
    manifest: ManifestHash,
    sequence: u64,
    signature: Signature,
}
```

- Sort entries by normalized logical path before encoding, reject duplicates,
  and use a versioned canonical representation. `serde` is useful for the Rust
  model, while deterministic DAG-CBOR (`ciborium` with additional canonical
  validation) or a carefully specified binary encoding is preferable for IDs
  and signatures.
- Use `semver` only for the package profile. General collections should use an
  opaque monotonically ordered version or manifest hash rather than forcing all
  users into semantic versioning.
- Put manifests in `iroh-blobs`; publish their hashes and mutable heads through
  `iroh-docs`. This follows the conversion plan's blob/document split and keeps
  large entry lists out of mutable records.
- Use `camino::Utf8PathBuf` or a dedicated `LogicalPath` newtype and validate at
  parse time. Never pass a manifest path directly to `PathBuf::join`.
- Implement `PackageManifest` conversion as an adapter and retain fixtures for
  old package manifests to prevent a flag-day migration.

## Pros and cons

**Pros**

- Very high usefulness: one manifest abstraction supports folders, datasets,
  packages, media, and virtual collections while retaining blob deduplication.
- Establishes stable content and version semantics required by nearly every
  other proposal.
- Metadata-only inspection and immutable history improve auditability and
  offline distribution.

**Cons**

- High foundational complexity: canonical encoding, signatures, path safety,
  schema evolution, and migration mistakes are difficult to reverse.
- Generalization can obscure package-specific behavior unless profiles and
  adapters remain explicit.
- Very large manifests may eventually require paging or tree-shaped manifests,
  adding another compatibility concern.
