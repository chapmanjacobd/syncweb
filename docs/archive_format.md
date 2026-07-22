# Syncweb Package Archive Format Design

## Motivation
The current [data package management design](./packages.md#1-collection--package-manifests) heavily relies on the Iroh P2P network (gossip and direct peer connections) to fetch manifests and blobs. While powerful, there are many scenarios where users need an offline or traditional file-transfer mechanism:
* Air-gapped environments: Transferring packages via USB drives.
* Direct messaging/Email: Sending a package as an attachment.
* Traditional HTTP hosting: Hosting a static package file on a standard web server without running an Iroh node.

The Syncweb Archive Format provides a single-file, self-contained bundle of a data package that can be easily "dropped" into another node.

The export pipeline is available through `syncweb package export`, and
imports are handled by `syncweb package import`. Export creates the
archive in a staging file and atomically renames it into place, so concurrent
exports cannot leave a partially written destination. Content blocks are
copied in bounded chunks and verified against their manifest hash while they
are streamed through the Zstandard encoder.

## Format Specification
The archive format is essentially a Zstandard-compressed Content Addressable aRchive (CAR), matching Iroh's native CAR export conventions.

* Extension: `.car.zst`
* MIME Type: `application/vnd.syncweb.package+zstd`
* Underlying Structure: 
  1. A root CID pointing to the JSON `CollectionManifest`.
  2. All the file blobs referenced in the manifest's `entries` array.
  3. The whole stream is compressed using `zstd` (which provides excellent compression ratios and decompression speeds).

### Why CAR?
Iroh natively speaks CAR for bulk blob export and import. Instead of inventing a custom tarball or zip format (which would require re-hashing all files on import), a CAR file maintains the BLAKE3 content-addressed nature of the data. When imported, Iroh can verify and ingest the blobs directly into its blob store with zero overhead.

## CLI UX

### 1. Exporting a Drop
A user can export a specific version (or the latest version) of a package into a archive file. 

```bash
# Export the latest version (defaults to my-dataset.car.zst)
syncweb package export ./my-dataset

# Export a specific version to a specific file
syncweb package export --version 1.2.0 ./my-dataset my-dataset_v1.2.0.car.zst

# Export a partial package using granular filtering
syncweb package export --filter "ext!=mp4" ./my-dataset my-dataset-lite.car.zst

# Export multiple packages at the same time (last argument is output directory)
syncweb package export ./pkg1 ./pkg2 ./exports/
```

### 2. Importing a Drop
A user can import a archive file. The system will extract the manifest, ingest the blobs, and register the package in the local iroh-docs namespace.

```bash
# Import a archive file
syncweb package import my-dataset.car.zst
```

### 3. Verifying a Drop

Drops can be verified before import with the streaming core API:

```rust
let result = syncweb_core::verify_archive("my-dataset.car.zst").await?;
```

Verification decompresses and parses the CAR incrementally. It checks the CAR
root, manifest serialization and signature, every content block's BLAKE3 hash
and size, and that the archive contains exactly the blocks referenced by the
manifest. Header, section, manifest, and varint limits are checked before any
attacker-controlled allocation.

## Implementation Details

### Export Pipeline (`archive_export`)
1. Resolve Manifest: Select the requested semver version or the newest
   manifest supplied by the package source.
2. Filter & Collect Entries: Evaluate each `CollectionEntry` with the existing
   `FilterEngine`. A filtered manifest is unsigned because its content changed.
3. Generate CAR Stream: Write a CAR v1 header, the manifest as the root block,
   and each unique referenced BLAKE3 blob using raw BLAKE3 CIDs.
4. Compress: Stream CAR bytes through an asynchronous Zstandard encoder while
   copying blob data in bounded chunks and checking its size and hash.
5. Write Atomically: Write to a unique staging file and rename it into place
   only after compression completes.
6. Multi-Export: For multiple package paths, export one isolated `.car.zst`
   file per package into the requested output directory.

### Import Pipeline (`archive_import`)
1. Decompress the `.car.zst` file through a streaming Zstandard decoder.
2. Validate the CAR header, root manifest, signatures, sizes, hashes,
   duplicate blocks, and dependencies before mutating the local blob store.
3. Optionally evaluate each manifest entry with `FilterEngine`; rejected
   blocks are consumed and verified but are not retained.
4. Stage accepted blocks in bounded temporary files, then ingest them into the
   local blob store after the complete archive passes validation.
5. Publish the validated manifest into a new local `iroh-docs` namespace.
6. Materialize accepted blobs through a staging directory and atomic rename.

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
* Manifest Validation: The manifest hash acts as the absolute source of truth. If the manifest dictates a file should have hash `X`, and the `.car.zst` provides a tampered file with hash `Y`, the package validation will fail.
* Manifest Signatures: The `PackageManifest` includes an Ed25519 signature from the package maintainer. During the import pipeline, the node verifies this signature against the maintainer's public key. *Note: Since modifying the manifest alters its hash, the signature must be generated over a deterministic, unsigned representation of the manifest.* If someone provides a modified manifest or swaps out blobs and updates the manifest hash, the signature verification will fail, preventing the installation of tampered drops.
* Anti-DOS / Allocation Limits: When parsing archive files from untrusted sources, it is important to prevent out-of-memory attacks. `iroh-blobs` CAR import handles varint length parsing securely (e.g., enforcing `MAX_ALLOC` limits per block). Because we pipe `async-compression` directly into `iroh-blobs`, we get these stream safety guarantees automatically without loading the entire archive into memory.

## Manifest identity and signatures

Collection manifests store the Ed25519 signature and maintainer public key as lowercase hexadecimal strings. `content_id()` hashes the canonical manifest with the signature removed, while `blob_id()` hashes the serialized manifest blob; the latter is used for blob tickets and document references so signed manifests remain fetchable. Dependency requirements use semver ranges and are checked against the versions available locally.
# Syncweb Archive Format Implementation Plan

## Overview
Implement the "archive format" (`.car.zst`) for `syncweb` data packages, allowing offline distribution and importing of package bundles via a Zstandard-compressed Content Addressable aRchive (CAR).

## Phase 1: Core Struct Updates
- [ ] Add `signature: Option<String>` (Ed25519 signature) to the `PackageManifest` struct to support offline verification.
- [ ] Add `public_key` field to the `PackageManifest` or ensure it's determinable from the maintainer field.
- [ ] Update manifest serialization/deserialization and hashing logic (ensure the signature covers the deterministic, unsigned version of the manifest so it doesn't invalidate its own hash).

## Phase 2: Export Pipeline
- [ ] Implement `archive_export` logic with explicit error mapping and concurrency safety (snapshotting or locks).
- [ ] Integrate with `iroh-blobs` CAR export functionality.
- [ ] Add `zstd` streaming compression (`async-compression` crate with `tokio` feature).
- [ ] Hook the export logic into `FilterEngine` to allow for granular filtering (exporting partial archives).
- [ ] Add support for multiple package exports simultaneously, outputting to a specified directory.
- [ ] CLI command: `syncweb package export [--version <v>] [--filter <rules>] <path>... [<output>]`

## Phase 3: Import Pipeline
- [x] Implement `archive_import` with strict CAR, decompression, and hash validation.
- [x] Add streaming Zstandard decompression with bounded staging writes.
- [x] Apply optional `FilterEngine` rules while consuming rejected blocks.
- [x] Ingest verified blocks into the local blob store.
- [x] Extract and validate the collection manifest.
- [x] Verify manifest signatures against the embedded maintainer public key.
- [x] Validate package dependencies before import.
- [x] Publish the validated manifest into a new local `iroh-docs` namespace.
- [x] CLI command: `syncweb package import <file.car.zst>`

## Phase 4: Verification and Atomic Materialization
- [x] Reject corrupted or maliciously modified `.car.zst` files before blob ingestion.
- [x] Materialize the package through a staging directory after successful import.
