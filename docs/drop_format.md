# Syncweb Package Drop Format Design

## Motivation
The current [data package management design](./packages.md#1-collection--package-manifests) heavily relies on the Iroh P2P network (gossip and direct peer connections) to fetch manifests and blobs. While powerful, there are many scenarios where users need an offline or traditional file-transfer mechanism:
* **Air-gapped environments**: Transferring packages via USB drives.
* **Direct messaging/Email**: Sending a package as an attachment.
* **Traditional HTTP hosting**: Hosting a static package file on a standard web server without running an Iroh node.

The **Syncweb Drop Format** provides a single-file, self-contained bundle of a data package that can be easily "dropped" into another node.

## Format Specification
The drop format is essentially a **Zstandard-compressed Content Addressable aRchive (CAR)**, matching Iroh's native CAR export conventions.

* **Extension**: `.car.zst`
* **MIME Type**: `application/vnd.syncweb.package+zstd`
* **Underlying Structure**: 
  1. A root CID pointing to the JSON `PackageManifest`.
  2. All the file blobs referenced in the manifest's `files` array.
  3. The whole stream is compressed using `zstd` (which provides excellent compression ratios and decompression speeds).

### Why CAR?
Iroh natively speaks CAR for bulk blob export and import. Instead of inventing a custom tarball or zip format (which would require re-hashing all files on import), a CAR file maintains the BLAKE3 content-addressed nature of the data. When imported, Iroh can verify and ingest the blobs directly into its blob store with zero overhead.

## CLI UX

### 1. Exporting a Drop
A user can export a specific version (or the latest version) of a package into a drop file. 

```bash
# Export the latest version (defaults to my-dataset.car.zst)
syncweb package drop export ./my-dataset

# Export a specific version to a specific file
syncweb package drop export --version 1.2.0 ./my-dataset my-dataset_v1.2.0.car.zst

# Export a partial package using granular filtering
syncweb package drop export --filter "ext!=mp4" ./my-dataset my-dataset-lite.car.zst

# Export multiple packages at the same time (last argument is output directory)
syncweb package drop export ./pkg1 ./pkg2 ./exports/
```

### 2. Importing a Drop
A user can import a drop file. The system will extract the manifest, ingest the blobs, and register the package in the local iroh-docs namespace.

```bash
# Import a drop file
syncweb package drop import my-dataset.car.zst
```

## Implementation Details

### Export Pipeline (`drop_export`)
1. **Resolve Manifest**: Fetch the `PackageManifest` from the local iroh-docs for the given package/version.
2. **Filter & Collect Hashes**: Hook into the existing `FilterEngine` to allow granular filtering of files. Extract the `manifest_hash` and iterate over `manifest.files`, evaluating each file against the `FilterEngine`. Only collect blob hashes for accepted files to create a partial drop.
3. **Generate CAR Stream**: Use `iroh_blobs::export::export_car` (or similar) with the list of hashes. The root of the CAR should be the manifest blob.
4. **Compress**: Pipe the CAR stream through `zstd` encoder. Handle export stream errors explicitly for better UX.
5. **Write**: Write the compressed stream to the destination file.
6. **Multi-Export**: If multiple packages are specified, iterate through each package sequentially, executing the pipeline and saving each package as an individual `.car.zst` file in the destination directory to maintain isolated drops.

```rust
pub async fn export_drop(folder: &SyncwebFolder, out_path: &Path, filter_engine: Option<&FilterEngine>) -> Result<()> {
    // Note: ensure concurrency safety (e.g. acquire locks or snapshot) so changes to the package during export don't silently corrupt the CAR.
    let manifest = folder.load_manifest().await?;
    let mut hashes = vec![manifest.manifest_hash];
    
    for file_meta in &manifest.files {
        // Granular filtering via FilterEngine
        if let Some(engine) = filter_engine {
            // Pseudo-code for evaluation
            // if engine.evaluate_file(file_meta) == FilterAction::Reject { continue; }
        }
        hashes.push(file_meta.hash);
    }
    
    let file = tokio::fs::File::create(out_path).await?;
    let mut encoder = async_compression::tokio::write::ZstdEncoder::new(file);
    
    // iroh-blobs export to CAR stream
    folder.blob_store.export_car(hashes, &mut encoder).await
        .map_err(|e| anyhow::anyhow!("failed to stream package: {}", e))?;
    encoder.shutdown().await?;
    
    Ok(())
}
```

### Import Pipeline (`drop_import`)
1. **Decompress**: Open the `.car.zst` file and stream it through a `zstd` decoder. Map decoding and stream errors explicitly to user-friendly messages (e.g. "unexpected end of file").
2. **Granular Filtering (Optional)**: Wrap the decompressed stream with a filtering decoder based on the `FilterEngine` or local blocklists. This allows skipping massive media files or rejecting blobs from known malicious authors directly from the stream without buffering into memory.
3. **Ingest CAR**: Stream the decompressed (and potentially filtered) CAR directly into `iroh_blobs::import::import_car`. Iroh will automatically verify the BLAKE3 hashes and store the blobs.
4. **Extract Manifest**: The root CID of the CAR points to the `PackageManifest`. Read this blob, parse the JSON.
5. **Dependency Check**: Validate the `PackageManifest` dependencies. Prevent installation of the package if its required dependencies are not already present in the local node at the time of import.
6. **Update Docs**: Insert the `PackageManifest` into the local iroh-docs namespace (`/.iroh-package/manifest.json`), updating the package state.
7. **Atomic Swap (Optional)**: If the package is meant to be instantly materialized on disk, trigger the parallel export/checkout to the local filesystem.

```rust
pub async fn import_drop(node: &IrohNode, in_path: &Path, target_dir: &Path, filter_engine: Option<&FilterEngine>) -> Result<()> {
    let file = tokio::fs::File::open(in_path).await?;
    let mut decoder = async_compression::tokio::read::ZstdDecoder::new(file);
    
    // Hook in granular filtering on the stream here if applicable
    // let mut filtering_decoder = ...
    
    // Ingest blobs directly into iroh
    let roots = node.blobs().import_car(&mut decoder).await
        .map_err(|e| anyhow::anyhow!("corrupted archive or unexpected EOF: {}", e))?;
    let manifest_hash = roots.first().context("Empty drop file")?;
    
    // Read manifest
    let manifest_bytes = node.blobs().read_to_bytes(*manifest_hash).await?;
    let manifest: PackageManifest = serde_json::from_slice(&manifest_bytes)?;
    
    // Check missing dependencies
    // if !node.has_dependencies(&manifest.dependencies).await? {
    //     anyhow::bail!("Cannot install package: missing required dependencies in local node.");
    // }
    
    // Set up folder / docs
    let mut folder = SyncwebFolder::init_from_manifest(node, target_dir, manifest).await?;
    
    // Materialize files to disk
    folder.checkout_latest().await?;
    
    Ok(())
}
```

## Security & Integrity
* **Tamper-Proof**: Because the imported blobs are verified against their content hashes upon CAR ingestion, a malicious actor cannot alter a file in the `.car.zst` archive without changing its hash. 
* **Manifest Validation**: The manifest hash acts as the absolute source of truth. If the manifest dictates a file should have hash `X`, and the `.car.zst` provides a tampered file with hash `Y`, the package validation will fail.
* **Manifest Signatures**: The `PackageManifest` includes an Ed25519 signature from the package maintainer. During the import pipeline, the node verifies this signature against the maintainer's public key. *Note: Since modifying the manifest alters its hash, the signature must be generated over a deterministic, unsigned representation of the manifest.* If someone provides a modified manifest or swaps out blobs and updates the manifest hash, the signature verification will fail, preventing the installation of tampered drops.
* **Anti-DOS / Allocation Limits**: When parsing drop files from untrusted sources, it is important to prevent out-of-memory attacks. `iroh-blobs` CAR import handles varint length parsing securely (e.g., enforcing `MAX_ALLOC` limits per block). Because we pipe `async-compression` directly into `iroh-blobs`, we get these stream safety guarantees automatically without loading the entire archive into memory.
# Syncweb Drop Format Implementation Plan

## Overview
Implement the "drop format" (`.car.zst`) for `syncweb` data packages, allowing offline distribution and importing of package bundles via a Zstandard-compressed Content Addressable aRchive (CAR).

## Phase 1: Core Struct Updates
- [ ] Add `signature: Option<String>` (Ed25519 signature) to the `PackageManifest` struct to support offline verification.
- [ ] Add `public_key` field to the `PackageManifest` or ensure it's determinable from the maintainer field.
- [ ] Update manifest serialization/deserialization and hashing logic (ensure the signature covers the deterministic, unsigned version of the manifest so it doesn't invalidate its own hash).

## Phase 2: Export Pipeline
- [ ] Implement `drop_export` logic with explicit error mapping and concurrency safety (snapshotting or locks).
- [ ] Integrate with `iroh-blobs` CAR export functionality.
- [ ] Add `zstd` streaming compression (`async-compression` crate with `tokio` feature).
- [ ] Hook the export logic into `FilterEngine` to allow for granular filtering (exporting partial drops).
- [ ] Add support for multiple package exports simultaneously, outputting to a specified directory.
- [ ] CLI command: `syncweb package drop export [--version <v>] [--filter <rules>] <path>... [<output>]`

## Phase 3: Import Pipeline
- [ ] Implement `drop_import` logic with strict error mapping for better UX (e.g. decoding errors, consumed bytes mismatches).
- [ ] Add `zstd` streaming decompression.
- [ ] Wrap decompression in a stream filtering step (using `FilterEngine` or local blocklists) to skip malicious or undesired blobs on the fly.
- [ ] Stream decompressed output directly into `iroh-blobs` CAR import.
- [ ] Extract `PackageManifest` from imported blobs.
- [ ] Verify manifest signature against the maintainer's public key.
- [ ] Validate package dependencies and abort import if required dependency packages are not present locally.
- [ ] Upsert the validated `PackageManifest` into the local `iroh-docs` namespace.
- [ ] CLI command: `syncweb package drop import <file.car.zst>`

## Phase 4: Verification and Atomic Materialization
- [ ] Add tests to ensure that corrupted or maliciously modified `.car.zst` files correctly fail the import process.
- [ ] Automatically checkout (materialize) the package to the filesystem using the existing parallel checkout logic after successful import.
