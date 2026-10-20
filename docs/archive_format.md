# Syncweb Package Archive Format Design

## Motivation
The current [data package management design](./packages.md#1-collection-package-manifests) heavily relies on the Iroh P2P network (gossip and direct peer connections) to fetch manifests and blobs. While powerful, there are many scenarios where users need an offline or traditional file-transfer mechanism:
* Air-gapped environments: Transferring packages via USB drives.
* Direct messaging/Email: Sending a package as an attachment.
* Traditional HTTP hosting: Hosting a static package file on a standard web server without running an Iroh node.

The Syncweb Archive Format provides a single-file, self-contained bundle of a data package that can be easily "dropped" into another node.

The export pipeline is available through `syncweb package export`, and
imports are handled by `syncweb package import`. Export creates the
archive in a staging file and atomically renames it into place, so concurrent
exports cannot leave a partially written destination. Content blocks are
copied in bounded chunks and verified against their manifest hash while they
stream through the Zstandard encoder.

Use `syncweb package inspect` to verify and inspect an archive without changing
the local blob store or installing the package:

```bash
syncweb package inspect my-dataset.car.zst
syncweb --json package inspect my-dataset.car.zst
```

## Format Specification
The archive format is essentially a Zstandard-compressed Content Addressable aRchive (CAR), matching Iroh's native CAR export conventions.

* Extension: `.car.zst`
* MIME Type: `application/vnd.syncweb.package+zstd`
* Underlying Structure:
  1. A root CID pointing to the JSON `CollectionManifest`.
  2. A second root CID pointing to a JSON `DropIndex` describing the archive.
  3. The unique file blobs selected by the manifest and export mode.
  4. The whole stream is compressed using `zstd` (which provides excellent compression ratios and decompression speeds).

The second root is optional when reading legacy archives. New archives include
the index so an offline recipient can inspect roots, collection identity,
selected blocks, logical paths, export mode, and optional source-community
metadata before importing. Provider capability hints may also be included by
library callers.

### Why CAR?
Iroh natively speaks CAR for bulk blob export and import. Instead of inventing a custom tarball or zip format (which would require re-hashing all files on import), a CAR file maintains the BLAKE3 content-addressed nature of the data. When imported, Iroh can verify and ingest the blobs directly into its blob store with zero overhead.

## CLI UX

### 1. Exporting a Drop
A user can export a specific version (or the latest version) of a package into a archive file. 

```bash
# Export the latest version (defaults to my-dataset.car.zst)
syncweb package export ./my-dataset

# Export a published collection by ID through the running daemon
syncweb package export <collection-id>

# Export a specific version to a specific file
syncweb package export --version 1.2.0 ./my-dataset my-dataset_v1.2.0.car.zst

# Export a partial package using granular filtering
syncweb package export --filter "ext!=mp4" ./my-dataset my-dataset-lite.car.zst

# Export only blocks needed by a destination
syncweb package export --mode missing-blocks \
  --block-hash <BLAKE3_HASH> ./my-dataset missing.car.zst

# Export a bounded contribution bundle
syncweb package export --mode contribution --max-bytes 104857600 \
  --source-community classroom ./my-dataset contribution.car.zst

# Export a repair bundle for selected blocks
syncweb package export --mode repair-bundle \
  --block-hash <BLAKE3_HASH> ./my-dataset repair.car.zst

# Export multiple packages at the same time (last argument is output directory)
syncweb package export ./pkg1 ./pkg2 ./exports/
```

### 2. Importing a Drop
A user can import a archive file. The system extracts the manifest, ingests the blobs, and registers the package in the local iroh-docs namespace.

```bash
# Import a archive file
syncweb package import my-dataset.car.zst

# Inspect and verify it without importing
syncweb package inspect my-dataset.car.zst
```

### 3. Verifying a Drop

Drops can be verified before import with the streaming core API:

```rust
let result = syncweb_core::verify_archive("my-dataset.car.zst").await?;
```

Verification decompresses and parses the CAR incrementally. It checks the CAR
roots, manifest serialization and signature, optional index metadata, every
content block's BLAKE3 hash and size, and that the archive contains exactly the
blocks referenced by the manifest. Header, section, manifest, index, and
varint limits are checked before any attacker-controlled allocation.

Community-scoped callers can require an exact manifest scope instead of using
the unrestricted verifier:

```rust
let result = syncweb_core::verify_archive_for_community(
    "my-dataset.car.zst",
    Some(community_id),
).await?;
```

Use `None` with `verify_archive_for_community` to require an explicitly open
manifest. Archive imports provide the same policy through
`DropImportOptions::with_community` and `with_open_community`; any index
`source_community` value is also checked against the manifest identity.

## Implementation Details

### Export Pipeline (`archive_export`)
1. Resolve Manifest: Select the requested semver version or the newest
   manifest supplied by the package source.
2. Filter & Collect Entries: Evaluate each `CollectionEntry` with the existing
   `FilterEngine`. A filtered manifest is unsigned because its content changed.
3. Generate CAR Stream: Write a CAR v1 header, the manifest and self-describing
   `DropIndex` as root blocks, and each unique selected BLAKE3 blob using raw
   BLAKE3 CIDs.
4. Compress: Stream CAR bytes through an asynchronous Zstandard encoder while
   copying blob data in bounded chunks and checking its size and hash.
5. Write Atomically: Write to a unique staging file and rename it into place
   only after compression completes.
6. Multi-Export: For multiple package paths, export one isolated `.car.zst`
   file per package into the requested output directory.

### Import Pipeline (`archive_import`)
1. Decompress the `.car.zst` file through a streaming Zstandard decoder.
2. Validate the CAR header, root manifest, optional index, signatures, sizes,
   hashes, duplicate blocks, and dependencies before mutating the local blob store.
3. Optionally evaluate each manifest entry with `FilterEngine`; rejected
   blocks are consumed and verified but are not retained.
4. Stage accepted blocks in bounded temporary files, then ingest them into the
   local blob store after the complete archive passes validation.
5. Publish the validated manifest into a new local `iroh-docs` namespace.
6. Materialize accepted blobs through a staging directory and atomic rename.

Import is restart-safe: validation never exposes partial materialization, and
blob ingestion is content-addressed and idempotent. If an import is interrupted
after some blocks are stored, rerunning `package import` reuses those verified
blocks rather than creating duplicates; if it is interrupted before final
materialization or installation, the destination remains unchanged and the
same command can be retried. Export has the corresponding atomic behavior:
rerunning after an interruption replaces only a completed destination and
never treats a partial staging file as a valid archive.

```rust
let result = syncweb_core::import_archive(
    &node,
    "my-dataset.car.zst",
    "./my-dataset",
    None,
).await?;
```

## Security & Integrity
* Tamper-Proof: Because the imported blobs are verified against their content hashes upon CAR ingestion, a malicious actor cannot alter a file in the `.car.zst` archive without changing its hash. 
* Manifest Validation: The manifest hash acts as the absolute source of truth. If the manifest dictates a file should have hash `X`, and the `.car.zst` provides a tampered file with hash `Y`, the package validation fails.
* Manifest Signatures: The `PackageManifest` includes an Ed25519 signature from the package maintainer. During the import pipeline, the node verifies this signature against the maintainer's public key. *Note: Since modifying the manifest alters its hash, the signature is generated over a deterministic, unsigned representation of the manifest.* If someone provides a modified manifest or swaps out blobs and updates the manifest hash, the signature verification fails, preventing the installation of tampered drops.
* Anti-DOS / Allocation Limits: When parsing archive files from untrusted sources, `iroh-blobs` CAR import handles varint length parsing securely (e.g., enforcing `MAX_ALLOC` limits per block). Piping `async-compression` directly into `iroh-blobs` provides these stream safety guarantees automatically without loading the entire archive into memory.

## Manifest identity and signatures

Collection manifests store the Ed25519 signature and maintainer public key as lowercase hexadecimal strings. `content_id()` hashes the canonical manifest with the signature removed, while `blob_id()` hashes the serialized manifest blob; the latter is used for blob tickets and document references so signed manifests remain fetchable. Dependency requirements use semver ranges and are checked against the versions available locally.
