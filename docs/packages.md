# Packages, Policies, and Living Folders

## Scoped Policy Modes (Integrates Public Read-Only Folders)

### Concept
Rather than using broad abstract profiles (like "Community" or "PublicArchive"), policy is defined through explicit, grounded configuration levers (e.g., `visibility`, `searchable`, `pinning`). A single installation may contain private credentials, a team folder, a public dataset, and one publicly shared file.
This design configures deployment policy at network, folder, and file granularity. Inherited exposure is explicit and safe.

Policies are resolved in this order:
`application defaults -> network policy -> folder policy -> file policy`

An explicit value at a more-specific scope overrides an inherited value. Security-sensitive settings are monotonic: a child may restrict publication, indexing, replication, or access, but cannot silently broaden a parent policy.

### Explicit Policy Levers (Iroh-Native Options)
- `access`: 
  - `"capability"`: Strict access control. Requires an iroh-docs `DocTicket` or `NamespaceSecret` to discover and fetch.
  - `"public_ticket"`: Generates an iroh-blobs `BlobTicket`. Anyone with this ticket can fetch the blob without authentication.
- `encryption`: 
  - `"plaintext"`: Standard BLAKE3 hashing. Data is stored locally in plaintext and served over encrypted QUIC tunnels.
  - `"encrypted"`: Local payloads are encrypted before being hashed into the blob store (e.g. for untrusted mirrors).
- `searchable`: `true` (announces signed metadata to the DHT/gossip topic for discovery by peers/indexers) or `false`.
- `pinning`: `true` (prevents garbage collection of publicly shared blobs) or `false`.
- `replication`: `"disabled"` (do not replicate further), `"enabled"` (standard).

Iroh-blobs natively supports public tickets for unauthenticated reads. This is exposed by setting `access = "public_ticket"`.

### Content Pinning (GC Prevention)
iroh-blobs has garbage collection that removes unreferenced blobs. For public folders, we must pin blobs to prevent GC from deleting them (`pinning = true`):

```rust
impl SyncwebFolder {
    /// Pin all blobs in this folder (prevent GC)
    async fn pin_for_sharing(&self) -> Result<()> {
        let blobs = self.blob_store.blobs().await?;
        for blob in blobs {
            // Tag with a permanent tag to prevent GC
            self.blob_store.tag(
                format!("public/{}", self.namespace_id),
                blob.hash,
                blob.format,
            ).await?;
        }
        Ok(())
    }

    /// Unpin when stopping sharing
    async fn unpin(&self) -> Result<()> {
        self.blob_store.untag(format!("public/{}", self.namespace_id)).await?;
        Ok(())
    }
}
```

### Configuration Example

```toml
[policy]
access = "capability"
encryption = "plaintext"
searchable = false
pinning = false

[networks.research.policy]
searchable = true
catalogs = ["research-index"]

[folders.research-data.policy]
access = "public_ticket"
pinning = true
public_alias = "climate-hourly"
pin_duration = "365d"

[folders.research-data.files."raw/credentials.json".policy]
access = "capability"
encryption = "encrypted"
searchable = false
replication = "disabled"
```

### Public Folder Implementation

```rust
// Creating a public folder
async fn publish_folder(&self, folder_id: &NamespaceId) -> Result<BlobTicket> {
    // 1. Ensure folder is SendOnly or SendReceive (has namespace key)
    let folder = self.folders.get(folder_id)?;
    ensure!(folder.sync_mode.can_publish());

    // 2. Get the root hash of all blobs in this folder
    let root_hash = self.get_folder_root_hash(folder).await?;

    // 3. Create a public blob ticket
    let addr = self.node.endpoint().addr();
    let ticket = BlobTicket::new(addr, root_hash, BlobFormat::HashSeq);

    // 4. Announce on public gossip topic
    let topic = TopicId::from_bytes(blake3::hash(b"syncweb/public-folders"));
    self.node.gossip().publish(topic, PublicFolderAnnouncement {
        namespace_id: *folder_id,
        label: folder.local_path.file_name().unwrap().to_string_lossy().to_string(),
        ticket: ticket.clone(),
        version: folder.version.clone(),
        created_at: Timestamp::now(),
    }).await?;

    Ok(ticket)
}

// Subscribing to public folder (no auth needed)
async fn subscribe_public(&self, ticket: BlobTicket) -> Result<NamespaceId> {
    // 1. Create local blob store
    let blob_store = BlobStore::persistent(self.config.data_dir.join("blobs"))?;

    // 2. Start fetching blobs lazily from ticket
    let hash = ticket.hash();
    let format = ticket.format();

    // 3. Create doc entry for this folder
    let namespace_id = self.create_public_folder_entry(hash, format).await?;

    // 4. Subscribe to doc updates (gossip)
    self.node.gossip().subscribe(topic).await?;

    Ok(namespace_id)
}
```

### CLI Commands

```bash
# Show policy for a file, folder, or network
syncweb policy show [file-or-folder-or-network]

# Generate a public ticket for a folder (pins content)
syncweb policy set audio/ --access public_ticket --pinning true
# Output: iroh-blob://<ticket>  (shareable URL)

# Explain why a file has its effective policy settings
syncweb policy explain audio/raw/participants.csv

# Subscribe to public folder (no auth, read-only)
syncweb subscribe iroh-blob://<ticket>
# Creates local read-only folder, lazy-fetches on access

# List known public folders (from gossip)
syncweb public list

# Get version info for public folder
syncweb public info <ticket>

# Revert access to capability-only
syncweb policy set audio/ --access capability --pinning false
```

Noninteractive promotion to public should require an explicit flag such as `--confirm-public summary.csv`. Configuration errors that would broaden access must fail closed and name the field and source scopes.

### Code Implementation Patterns
```rust
struct Resolved<T> {
    value: T,
    source: PolicyScope,
    explicit: bool,
}

struct EffectivePolicy {
    access: Resolved<AccessMode>,
    encryption: Resolved<EncryptionMode>,
    indexing: Resolved<Indexing>,
    replication: Resolved<Replication>,
    gateway: Resolved<GatewayAccess>,
}
```

Implement `resolve(defaults, network, folder, file)` as a pure function over typed `PolicyPatch` values. Table-driven unit tests should enumerate every parent/child combination for security-sensitive fields. Use restrictive lattices where the domain supports one, for example access `"public_ticket"` > `"capability"` with child inheritance computed by `min` unless an audited promotion is explicitly supplied. Write promotion events before publishing side effects, then bind the resulting audit ID to the catalog or gateway operation.

### Use Cases
- Public datasets - Share large datasets via single URL (`access="public_ticket"`)
- Software distribution - Verified binary distribution with range requests
- Data packages - Versioned datasets with update tracking
- Read-only mirrors - One-way sync for backups/archives

## Living Folders (Mutable Heads)

### 1. The Living Folder Model
Rather than requiring users to manually manage "versions" (e.g., `v0.1.0`, `v0.2.0`) and trigger explicit upgrade commands, `syncweb` treats all shared collections as Living Folders. 
This provides the seamless background-sync experience of traditional Syncthing or Dropbox.

### 2. Signed Mutable Pointers
To achieve living folders over immutable BLAKE3 blobs, we use Signed Mutable Pointers (conceptually from PROPOSALS 2 & 4).
* The Pointer: A cryptographic signed record containing a monotonically increasing sequence number and a `ManifestHash`.
* The URI: Users share a mutable link: `syncweb://name/<publisher_pubkey>/<folder_alias>`
* Resolution: When a client resolves this URI, they fetch the latest signed pointer from the publisher or the DHT, extract the `ManifestHash`, and sync the underlying blobs.

### 3. Sync & Publish Modes
Folders can operate in one of two modes depending on how frequently the publisher wants to push changes:

Auto-publish (Default / "Syncthing" mode):
When the publisher adds, modifies, or deletes files in their local folder:
1. The local iroh-docs namespace automatically updates.
2. A new `ManifestHash` is generated.
3. The publisher automatically signs a new pointer with `sequence + 1` pointing to the new `ManifestHash`.
4. Subscribers following the `syncweb://name/...` pointer detect the new sequence number via Gossip/DHT and begin syncing the delta automatically.

Manual publish ("Git-style" explicit mode):
Useful for long-running edits where the publisher doesn't want to sync broken state to subscribers.
1. `publish_mode = "manual"` is set on the folder policy.
2. Local filesystem changes are indexed locally (staging), generating new immutable blobs, but the Signed Mutable Pointer is NOT updated.
3. Subscribers continue to see and sync the previous stable version.
4. When ready, the publisher explicitly runs `syncweb publish <folder>`. This atomically advances the mutable pointer, pushing the batch of changes to subscribers all at once.

### 4. Discovery via Ephemeral Gossip
There is no persistent global catalog. A folder is only discoverable if a node is actively seeding it.
* Topic: Publishers broadcast a `FolderAnnouncement` over the `syncweb/discovery` gossip topic.
* Announcement: Contains the folder's descriptive metadata (JSON) and the current Mutable Head pointer.
* Search: Clients can listen to the gossip topic to populate a local, ephemeral search index of available public folders.

## Why not APT/Debian packaging?

| dapt (APT-based) | syncweb (iroh-based) |
|-------------------|----------------------------|
| `.deb` packages | iroh-blobs content-addressed blobs |
| APT `Packages` indices | iroh-docs entries |
| GPG signing | Ed25519 identity (built into iroh protocol) |
| APT version comparison | BLAKE3 manifest hash + sequence number |
| HTTP repository mirrors | Gossip announcements + P2P blob transfer |
| rsync `--compare-dest` delta sync | Bao tree range requests (more granular) |
| `postinst`/`postrm` scripts | Native lazy fetch + atomic symlink swap |
| `dpkg-deb --build` | No build step -- files are the package |
| Platform: Debian/Ubuntu only | Platform: any OS with Rust |
| Central repository server | P2P -- any peer can serve |

### 1. Collection & Package Manifests

The package manifest is generalized into a `CollectionManifest`, which replaces dapt's `product.toml` + `.dapt-release.txt` + APT `Packages` index, but also supports virtual collections and datasets.
Stored in `iroh-blobs`, with their hashes and mutable heads published through `iroh-docs`.

```rust
/// Generalized Collection Manifest
#[derive(Serialize, Deserialize, Clone, Debug)]
struct CollectionManifestV1 {
    schema: SchemaVersion,
    collection_id: CollectionId,
    version: VersionId,
    parent: Option<ManifestHash>,
    entries: Vec<CollectionEntry>,
    publisher: PublicKey,
    signature: Signature,
}

/// A file within a collection
#[derive(Serialize, Deserialize, Clone, Debug)]
struct CollectionEntry {
    content_id: Hash,
    logical_path: String,
    name: String,
    size: u64,
    media_type: Option<String>,
    role: String,
    relationships: Vec<String>,
}

/// A mutable head pointing to the latest version
struct CollectionHead {
    collection_id: CollectionId,
    manifest: ManifestHash,
    sequence: u64,
    signature: Signature,
}
```

Manifest storage: Manifests are stored in `iroh-blobs`. Their hashes and mutable heads (`CollectionHead`) are published through `iroh-docs`. The manifest's own BLAKE3 hash serves as the version identifier -- content-addressed, tamper-proof, verifiable.

Lineage tracking: The `parent` field creates a linked list of versions. Each version points to its predecessor, forming a lineage chain.

Package Profile: Packages are just collections with dependencies. An adapter layer converts traditional package workflows into the generalized model.

### 2. Publishing Workflow

Full data package lifecycle, replacing dapt's `init-repo` → `new-product` → `release` → `refresh-repo`:

```bash
# 1. Initialize a folder as a data package
# 1. Initialize a folder as a collection (e.g. dataset)
syncweb collection init ./climate --name climate-hourly --type dataset
# Creates local drafting state

# 2. Add files to the collection
syncweb collection add col_climate ./data/observations.csv
# OR add from existing content hash (virtual collections):
# syncweb collection add col_reading-list syncweb://content/b3:8e7a... --as data/hourly.csv

# 3. Publish (creates immutable version + updates mutable head)
syncweb collection publish col_climate
# Output: Published climate-hourly@1.0.0 (manifest b3:19ac...)

# 4. Diff versions
syncweb collection diff col_climate 1.0.0 1.1.0
# Output: syncweb://package/<node-ticket>/<namespace-id>?v=0.2.0
```

Implementation:

```rust
impl SyncwebFolder {
    /// Initialize a folder as a data package
    async fn package_init(
        &mut self,
        name: &str,
        maintainer: &str,
        description: &str,
    ) -> Result<PackageManifest> {
        let manifest = PackageManifest {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            seq: 1,
            maintainer: maintainer.to_string(),
            description: description.to_string(),
            tags: vec![],
            dependencies: vec![],
            files: vec![],
            manifest_hash: Hash::EMPTY, // computed after first add
            created_at: Timestamp::now(),
            changelog: "Initial version".to_string(),
            parent_hash: None,
        };

        // Store manifest as doc entry
        self.store_manifest(&manifest).await?;
        Ok(manifest)
    }

    /// Add files to the package manifest
    async fn package_add(&mut self, paths: &[PathBuf]) -> Result<usize> {
        let mut manifest = self.load_manifest().await?;
        let mut added = 0;

        for path in paths {
            // Hash the file with BLAKE3
            let data = tokio::fs::read(path).await?;
            let hash = blake3::hash(&data);

            // Add to blob store
            let tag = self.blob_store.add_bytes(data).await?;

            // Add to manifest
            manifest.files.push(PackageFileEntry {
                path: path.strip_prefix(&self.local_path)?.to_path_buf(),
                hash: hash.into(),
                size: tag.size,
                mime_type: None,
            });
            added += 1;
        }

        // Recompute manifest hash
        manifest.manifest_hash = self.hash_manifest(&manifest)?;
        self.store_manifest(&manifest).await?;
        Ok(added)
    }

    /// Bump package version
    async fn package_bump(
        &mut self,
        bump_type: BumpType,
        changelog: &str,
    ) -> Result<PackageManifest> {
        let mut manifest = self.load_manifest().await?;
        let old_hash = manifest.manifest_hash;

        // Bump version
        manifest.version = match bump_type {
            BumpType::Major => bump_major(&manifest.version),
            BumpType::Minor => bump_minor(&manifest.version),
            BumpType::Patch => bump_patch(&manifest.version),
        };
        manifest.seq += 1;
        manifest.parent_hash = Some(old_hash);
        manifest.changelog = changelog.to_string();
        manifest.created_at = Timestamp::now();

        manifest.manifest_hash = self.hash_manifest(&manifest)?;
        self.store_manifest(&manifest).await?;
        Ok(manifest)
    }

    /// Publish package (pin blobs + announce on gossip)
    async fn package_publish(&self) -> Result<PackageTicket> {
        // Pin all package blobs (prevent GC)
        self.pin_for_sharing().await?;

        // Create ticket
        let root_hash = self.get_folder_root_hash().await?;
        let ticket = PackageTicket {
            node_addr: self.node.endpoint().addr(),
            namespace_id: self.namespace_id,
            root_hash,
            version: self.load_manifest().await?.version,
        };

        // Announce on package gossip topic
        let topic = TopicId::from_bytes(*b"syncweb/packages");
        let manifest = self.load_manifest().await?;
        self.node.gossip().publish(topic, PackageAnnouncement {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
            tags: manifest.tags.clone(),
            ticket: ticket.clone(),
            manifest_hash: manifest.manifest_hash,
            announced_at: Timestamp::now(),
        }).await?;

        Ok(ticket)
    }
}

enum BumpType { Major, Minor, Patch }
```

### 3. Package Discovery Catalog

Gossip-based package registry replaces dapt's APT `Packages` index and `Release` file.
Every publisher announces on `syncweb/packages`; consumers subscribe to discover available packages.

```rust
/// Announcement broadcast on the packages gossip topic
#[derive(Serialize, Deserialize, Clone, Debug)]
struct PackageAnnouncement {
    /// Package name
    name: String,
    /// Latest version
    version: String,
    /// Description
    description: String,
    /// Searchable tags
    tags: Vec<String>,
    /// Ticket to fetch this package
    ticket: PackageTicket,
    /// Manifest hash (for integrity)
    manifest_hash: Hash,
    /// When announced
    announced_at: Timestamp,
}

/// A package ticket (like dapt's APT repository URL, but P2P)
#[derive(Serialize, Deserialize, Clone, Debug)]
struct PackageTicket {
    /// Publisher's node address
    node_addr: EndpointAddr,
    /// Namespace containing the package
    namespace_id: NamespaceId,
    /// Root hash of the package content
    root_hash: Hash,
    /// Version string
    version: String,
}

impl PackageTicket {
    /// Format as shareable URL
    fn to_url(&self) -> String {
        format!(
            "syncweb://package/{}?v={}",
            self.node_addr.node_id(),
            self.version
        )
    }

    /// Parse from URL
    fn from_url(url: &str) -> Result<Self>;
}
```

CLI:

```bash
# Search available packages (queries gossip + local cache)
syncweb package search "climate"
# Output:
# NAME              VERSION  TAGS          DESCRIPTION
# climate-hourly    1.2.0    weather data  Hourly climate observations
# climate-daily     2.0.1    weather data  Daily climate summaries

# Get detailed info about a package
syncweb package info climate-hourly
# Output:
# Package: climate-hourly
# Version: 1.2.0 (seq 12)
# Maintainer: alice@example.com
# Description: Hourly climate observations
# Tags: weather, climate, hourly
# Files: 47 files, 2.3 GiB
# Published: 2026-07-16T10:30:00Z
# Publisher: node5abcd...
# Lineage: v1.0.0 → v1.1.0 → v1.2.0

# Browse by tag
syncweb package search --tag weather

# List all announced packages (no filter)
syncweb package search --all
```

### 4. Install/Remove State Management

Local state file tracks installed packages. Replaces dapt's dpkg status database.

```rust
/// Local state for installed packages
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
struct PackageState {
    /// Installed packages indexed by name
    installed: HashMap<String, InstalledPackage>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct InstalledPackage {
    /// Package name
    name: String,
    /// Installed version
    version: String,
    /// Package namespace ID
    namespace_id: NamespaceId,
    /// Local install path
    local_path: PathBuf,
    /// When installed
    installed_at: Timestamp,
    /// Manifest hash at install time
    manifest_hash: Hash,
    /// Installed file paths with hashes (for verification)
    files: Vec<PackageFileEntry>,
}

impl PackageState {
    /// Load from disk
    fn load(path: &Path) -> Result<Self>;

    /// Save to disk
    fn save(&self, path: &Path) -> Result<()>;

    /// Get state file path
    fn state_path(data_dir: &Path) -> PathBuf {
        data_dir.join("packages/state.json")
    }
}
```

CLI:

```bash
# Install a package from a ticket
syncweb package install syncweb://package/<node-id>?v=1.2.0 /path/to/install
# Fetches blobs, verifies manifest, stages files, atomic symlink swap
# Output:
# Fetching 47 files (2.3 GiB)...
# Verifying manifest hash... OK
# Installing to /path/to/install/1.2.0/
# Linking current → 1.2.0
# Done -- climate-hourly 1.2.0 installed

# Upgrade to latest version
syncweb package upgrade climate-hourly
# Queries publisher for latest, fetches delta, swaps symlink

# Remove a package
syncweb package remove climate-hourly
# Removes files + state entry

# List installed packages
syncweb package list
# Output:
# NAME              VERSION  INSTALLED      SIZE
# climate-hourly    1.2.0    2026-07-16     2.3 GiB
# earthquake-daily  3.0.0    2026-07-15     890 MiB
```

### 5. Integrity Verification

Per-package manifest with file-level BLAKE3 hashes. iroh-blobs provides blob-level integrity;
the manifest provides file-level integrity within a package. Replaces dapt's `dpkg-deb --verify`
and APT's `Expected-SHA256` checksums.

```rust
impl SyncwebFolder {
    /// Verify installed package integrity against manifest
    async fn package_verify(&self) -> Result<VerifyResult> {
        let manifest = self.load_manifest().await?;
        let mut verified = 0u64;
        let mut failed = Vec::new();

        for entry in &manifest.files {
            let local_path = self.local_path.join(&entry.path);
            match tokio::fs::read(&local_path).await {
                Ok(data) => {
                    let hash = blake3::hash(&data);
                    if Hash::from(hash) == entry.hash {
                        verified += 1;
                    } else {
                        failed.push(VerifyFailure {
                            path: entry.path.clone(),
                            expected: entry.hash,
                            actual: Hash::from(hash),
                        });
                    }
                }
                Err(_) => {
                    failed.push(VerifyFailure {
                        path: entry.path.clone(),
                        expected: entry.hash,
                        actual: Hash::EMPTY,
                    });
                }
            }
        }

        Ok(VerifyResult { verified, failed })
    }
}

struct VerifyResult {
    verified: u64,
    failed: Vec<VerifyFailure>,
}

struct VerifyFailure {
    path: PathBuf,
    expected: Hash,
    actual: Hash,
}
```

CLI:

```bash
# Verify installed package integrity
syncweb package verify climate-hourly
# Output: OK -- 47 files verified, all hashes match

# Verify with verbose output
syncweb package verify climate-hourly --verbose
# Output:
# climate-hourly 1.2.0 -- verifying 47 files...
#   data/observations.csv   OK (sha3: a1b2c3...)
#   data/metadata.json      OK (sha3: d4e5f6...)
#   ...
# OK -- 47 files verified, all hashes match

# Verify all installed packages
syncweb package verify --all
```

### 6. Multi-version Coexistence

Optional side-by-side version directories. Replaces dapt's `/var/lib/dapt/store/<product>/<version>/`
layout. Content-addressed blob storage means identical files between versions share underlying
storage -- no duplication.

```text
~/.local/share/syncweb/packages/
  climate-hourly/
    0.1.0/
      data/observations.csv
      data/metadata.json
    1.0.0/
      data/observations.csv      # same blob as 0.1.0 if unchanged
      data/metadata.json
      data/extra.csv
    1.2.0/
      data/observations.csv      # only changed bytes re-fetched
      data/metadata.json
      data/extra.csv
    current -> 1.2.0/            # active version symlink
```

```rust
impl PackageState {
    /// Get all installed versions of a package
    fn versions(&self, name: &str) -> Vec<&InstalledPackage>;

    /// Get the active (symlinked) version
    fn active_version(&self, name: &str) -> Option<&InstalledPackage>;

    /// Switch active version
    fn switch_version(&mut self, name: &str, version: &str) -> Result<()>;
}
```

CLI:

```bash
# List installed versions
syncweb package versions climate-hourly
# Output:
# VERSION  INSTALLED      SIZE     STATUS
# 0.1.0    2026-07-01     1.8 GiB
# 1.0.0    2026-07-10     2.1 GiB
# 1.2.0    2026-07-16     2.3 GiB  (current)

# Switch active version
syncweb package switch climate-hourly 1.0.0
# Symlink swap: current → 1.0.0/
# Instant -- no data movement needed
```

### 7. Atomic Upgrades

Same principle as dapt's `mv -Tf` symlink swap, but with content-addressed storage providing
additional safety guarantees.

Upgrade sequence:

1. Fetch: Download new version's blobs from publisher (or peers)
2. Stage: Write files to temporary directory (`/tmp/syncweb-stage-<hash>/`)
3. Verify: Check every file's BLAKE3 hash against the new manifest
4. Swap: Atomic `rename()` of staging dir to `<name>/<new-version>/`
5. Link: Atomic `rename()` of `current` symlink to new version dir
6. Cleanup: Delete old version directory (if not kept for coexistence)

If any step fails, the old version remains active. Rollback is instant -- just re-swap the symlink.

```rust
impl SyncwebFolder {
    /// Upgrade package atomically
    async fn package_upgrade(&mut self, name: &str) -> Result<UpgradeResult> {
        // 1. Get latest manifest from publisher
        let remote_manifest = self.fetch_latest_manifest(name).await?;
        let local_manifest = self.load_manifest().await?;

        if remote_manifest.seq <= local_manifest.seq {
            return Ok(UpgradeResult::AlreadyUpToDate);
        }

        // 2. Stage new version
        let staging_dir = self.create_staging_dir().await?;
        let fetched = self.fetch_and_stage(&remote_manifest, &staging_dir).await?;

        // 3. Verify staged files
        let verify_result = self.verify_staged(&remote_manifest, &staging_dir).await?;
        if !verify_result.failed.is_empty() {
            tokio::fs::remove_dir_all(&staging_dir).await?;
            return Err(Error::VerifyFailed(verify_result));
        }

        // 4. Atomic swap: staging → version dir
        let version_dir = self.local_path.join("packages")
            .join(&remote_manifest.name)
            .join(&remote_manifest.version);
        tokio::fs::rename(&staging_dir, &version_dir).await?;

        // 5. Atomic symlink swap: current → new version
        let current_link = self.local_path.join("packages")
            .join(&remote_manifest.name)
            .join("current");
        let new_target = PathBuf::from(&remote_manifest.version);
        // Remove old symlink, create new one atomically
        tokio::fs::remove_file(&current_link).await.ok();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&new_target, &current_link)?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&new_target, &current_link)?;

        // 6. Update state
        self.state.upgrade(name, &remote_manifest)?;

        // 7. Cleanup old version (optional, keep if --keep-versions)
        self.maybe_cleanup_old_versions(name).await?;

        Ok(UpgradeResult::Upgraded {
            from: local_manifest.version,
            to: remote_manifest.version,
            files_changed: fetched,
        })
    }
}

enum UpgradeResult {
    AlreadyUpToDate,
    Upgraded { from: String, to: String, files_changed: usize },
}
```

### 8. Delta Sync for Packages

iroh-blobs Bao trees enable efficient range-request delta sync for large files within a package.
When upgrading, only changed files (or changed byte ranges within files) are transferred.

```text
Example: Large CSV file (10 GB) with 500 MB of new rows appended

dapt (rsync):      Transfers 500 MB delta via --compare-dest
syncweb:    Transfers 500 MB delta via Bao tree range requests
                   (more granular: works at sub-file level, not just whole-file)

Example: 1000-file dataset with 10 files changed

dapt (rsync):      Transfers 10 changed files + metadata
syncweb:    Transfers 10 changed files + only changed byte ranges
                   within partially-modified files
```

Implementation: Transparent -- iroh-blobs handles delta sync automatically. When a blob is
re-added with the same path but different content, iroh-blobs stores only the new content.
The Bao tree structure enables range requests, so even within a single large file, only the
changed byte ranges need to be transferred.

```bash
# Upgrade with delta sync (default behavior)
syncweb package upgrade climate-hourly
# Output:
# Fetching delta for data/observations.csv (10.0 GiB, 500 MiB changed)...
# Fetching 3 new files...
# Total transfer: 523 MiB (instead of 10.5 GiB full)
```

### Package Ticket Format

```text
syncweb://package/<node-id>/<namespace-id>?v=<version>
syncweb://package/<node-id>/<namespace-id>              # latest version
syncweb://package/<node-id>/<namespace-id>?hash=<hash>  # specific manifest hash
```

CLI:

```bash
# Install from ticket
syncweb package install syncweb://package/abc123/def456?v=1.2.0 ./data

# Share package ticket
syncweb package publish ./my-dataset
# Output: syncweb://package/abc123/def456?v=0.2.0
```

### Use Cases
- Research datasets - Versioned, reproducible data packages with full lineage
- ML training data - Versioned datasets with delta sync for large files
- Software releases - Binary packages with integrity verification
- Configuration packages - Shared configs with version rollback
- Media libraries - Large file collections with incremental updates

---
