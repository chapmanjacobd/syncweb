use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use iroh::Endpoint;
use iroh_blobs::{BlobFormat, Hash, ticket::BlobTicket};
use iroh_docs::NamespaceId;
use serde::{Deserialize, Serialize};

use crate::{
    error::{Result, SyncwebError},
    folder::SyncwebFolder,
    fs::{ExportEntry, Exporter, ParallelScanner},
    node::{blob_store::BlobStore, docs_engine::DocsEngine},
};

const SCHEMA_VERSION: u32 = 1;
const SNAPSHOT_PIN_PREFIX: &str = "syncweb/snapshot/";

/// The content-addressed identifier of a snapshot.
pub type SnapshotId = Hash;

/// A file reference captured by a snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SnapshotEntry {
    pub path: PathBuf,
    pub hash: Hash,
    pub size: u64,
}

impl SnapshotEntry {
    /// # Errors
    ///
    /// Returns an error if `path` is not a safe relative path.
    pub fn new(path_value: impl Into<PathBuf>, hash: Hash, size: u64) -> Result<Self> {
        let path = path_value.into();
        validate_path(&path)?;
        Ok(Self { path, hash, size })
    }
}

/// A content-addressed snapshot of a folder or local directory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Snapshot {
    pub schema_version: u32,
    pub id: SnapshotId,
    pub namespace_id: Option<NamespaceId>,
    pub root_hash: Hash,
    pub created_at: u64,
    pub description: Option<String>,
    pub total_size: u64,
    pub file_count: u64,
    pub entries: Vec<SnapshotEntry>,
}

impl Snapshot {
    /// Build a snapshot from already-addressed content references.
    ///
    /// No file data is copied by this operation.
    #[must_use]
    pub fn new(
        namespace_id: Option<NamespaceId>,
        mut entries: Vec<SnapshotEntry>,
        description: Option<String>,
    ) -> Self {
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        let root_hash = entries_hash(&entries);
        let total_size = entries.iter().map(|entry| entry.size).sum();
        let file_count = u64::try_from(entries.len()).unwrap_or(u64::MAX);
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        Self {
            schema_version: SCHEMA_VERSION,
            id: root_hash,
            namespace_id,
            root_hash,
            created_at,
            description,
            total_size,
            file_count,
            entries,
        }
    }

    /// Validate the snapshot schema, paths, counts, and content root.
    ///
    /// # Errors
    ///
    /// Returns an error if any snapshot invariant is violated.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(SyncwebError::InvalidConfig(format!(
                "unsupported snapshot schema version {}",
                self.schema_version
            )));
        }
        let mut paths = BTreeSet::new();
        for entry in &self.entries {
            validate_path(&entry.path)?;
            if !paths.insert(&entry.path) {
                return Err(SyncwebError::InvalidConfig(format!(
                    "duplicate snapshot path {}",
                    entry.path.display()
                )));
            }
        }
        let expected_count = u64::try_from(self.entries.len()).unwrap_or(u64::MAX);
        if self.file_count != expected_count {
            return Err(SyncwebError::InvalidConfig(
                "snapshot file count is incorrect".to_owned(),
            ));
        }
        let expected_size: u64 = self.entries.iter().map(|entry| entry.size).sum();
        if self.total_size != expected_size {
            return Err(SyncwebError::InvalidConfig(
                "snapshot total size is incorrect".to_owned(),
            ));
        }
        if self.root_hash != entries_hash(&self.entries) || self.id != self.root_hash {
            return Err(SyncwebError::InvalidConfig(
                "snapshot content root is incorrect".to_owned(),
            ));
        }
        Ok(())
    }

    /// Serialize a validated snapshot manifest.
    ///
    /// # Errors
    ///
    /// Returns an error if validation or serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|error| SyncwebError::operation("failed to serialize snapshot", error))
    }

    /// Decode and validate a snapshot manifest.
    ///
    /// # Errors
    ///
    /// Returns an error if decoding or validation fails.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let snapshot: Self = serde_json::from_slice(bytes.as_ref())
            .map_err(|error| SyncwebError::operation("failed to deserialize snapshot", error))?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    /// Compare two snapshots by logical path and content hash.
    ///
    /// # Errors
    ///
    /// Returns an error if either snapshot is invalid.
    pub fn diff(&self, other: &Self) -> Result<SnapshotDiff> {
        self.validate()?;
        other.validate()?;
        let left = self
            .entries
            .iter()
            .map(|entry| (&entry.path, entry))
            .collect::<BTreeMap<_, _>>();
        let right = other
            .entries
            .iter()
            .map(|entry| (&entry.path, entry))
            .collect::<BTreeMap<_, _>>();
        let mut diff = SnapshotDiff::default();
        for (path, entry) in &right {
            match left.get(path) {
                None => diff.added.push((*entry).clone()),
                Some(previous) if *previous != *entry => diff.modified.push(((*previous).clone(), (*entry).clone())),
                Some(_) => {}
            }
        }
        for (path, entry) in &left {
            if !right.contains_key(path) {
                diff.removed.push((*entry).clone());
            }
        }
        Ok(diff)
    }
}

/// The path-level changes between two snapshots.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct SnapshotDiff {
    pub added: Vec<SnapshotEntry>,
    pub removed: Vec<SnapshotEntry>,
    pub modified: Vec<(SnapshotEntry, SnapshotEntry)>,
}

/// Persists snapshot manifests and pins all content referenced by them.
#[derive(Clone)]
pub struct SnapshotStore {
    blobs: BlobStore,
    docs: Option<DocsEngine>,
}

impl SnapshotStore {
    #[must_use]
    pub const fn new(blobs: BlobStore) -> Self {
        Self { blobs, docs: None }
    }

    #[must_use]
    pub const fn with_docs(blobs: BlobStore, docs: DocsEngine) -> Self {
        Self {
            blobs,
            docs: Some(docs),
        }
    }

    /// Create and persist a snapshot from existing content references.
    ///
    /// # Errors
    ///
    /// Returns an error if a referenced blob is unavailable or the manifest cannot be stored.
    pub async fn create_snapshot(
        &self,
        namespace_id: Option<NamespaceId>,
        entries: Vec<SnapshotEntry>,
        description: Option<String>,
    ) -> Result<Snapshot> {
        let snapshot = Snapshot::new(namespace_id, entries, description);
        self.store(&snapshot).await?;
        Ok(snapshot)
    }

    /// Create a snapshot from a local directory.
    ///
    /// Files are imported into the content-addressed store once; the snapshot
    /// itself only stores references to those blobs.
    ///
    /// # Errors
    ///
    /// Returns an error if scanning, importing, or manifest storage fails.
    pub async fn create_from_path(
        &self,
        source_value: impl AsRef<Path>,
        threads: usize,
        description: Option<String>,
    ) -> Result<Snapshot> {
        let source_path = source_value.as_ref();
        let entries = ParallelScanner::new(source_path, Vec::<String>::new(), threads).scan()?;
        let mut snapshot_entries = Vec::with_capacity(entries.len());
        for entry in entries {
            let hash = self.blobs.add_file(&entry.path).await?;
            let expected = Hash::from_bytes(*entry.hash.as_bytes());
            if hash != expected {
                return Err(SyncwebError::InvalidConfig(format!(
                    "file changed while creating snapshot: {}",
                    entry.path.display()
                )));
            }
            snapshot_entries.push(SnapshotEntry::new(entry.relative_path, hash, entry.size)?);
        }
        self.create_snapshot(None, snapshot_entries, description).await
    }

    /// Create a snapshot of the latest entries in a synchronized folder.
    ///
    /// # Errors
    ///
    /// Returns an error if entries cannot be read or referenced blobs are missing.
    pub async fn create_for_folder(&self, folder: &SyncwebFolder, description: Option<String>) -> Result<Snapshot> {
        let docs = self
            .docs
            .as_ref()
            .ok_or_else(|| SyncwebError::InvalidConfig("snapshot store has no document engine".to_owned()))?;
        let entries = docs.list_latest(folder.doc()).await?;
        let snapshot_entries = entries
            .into_iter()
            .filter(|entry| !entry.key().starts_with(b"sys/"))
            .map(|entry| {
                let path = String::from_utf8(entry.key().to_vec())
                    .map_err(|error| SyncwebError::operation("folder entry path is not UTF-8", error))?;
                SnapshotEntry::new(path, entry.content_hash(), entry.content_len())
            })
            .collect::<Result<Vec<_>>>()?;
        self.create_snapshot(Some(folder.namespace_id()), snapshot_entries, description)
            .await
    }

    /// Restore a snapshot to a local directory, removing files not present in it.
    ///
    /// # Errors
    ///
    /// Returns an error if referenced content is unavailable or materialization fails.
    pub async fn restore_to_path(
        &self,
        snapshot: &Snapshot,
        destination_value: impl AsRef<Path>,
    ) -> Result<Vec<PathBuf>> {
        snapshot.validate()?;
        self.verify_blobs(snapshot).await?;
        let destination = destination_value.as_ref();
        fs::create_dir_all(destination)?;
        let expected = snapshot
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<BTreeSet<_>>();
        for entry in ParallelScanner::new(destination, Vec::<String>::new(), 1).scan()? {
            if !expected.contains(&entry.relative_path) {
                fs::remove_file(entry.path)?;
            }
        }
        let exports = snapshot
            .entries
            .iter()
            .map(|entry| ExportEntry::new(entry.path.clone(), entry.hash, entry.size))
            .collect::<Vec<_>>();
        Exporter::new(self.blobs.clone(), destination)
            .export_all_verified(&exports)
            .await
    }

    /// Restore a snapshot by updating a synchronized folder's document entries.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot belongs to another folder, content is missing,
    /// or document updates fail.
    pub async fn restore_for_folder(&self, folder: &SyncwebFolder, snapshot: &Snapshot) -> Result<()> {
        snapshot.validate()?;
        if snapshot.namespace_id != Some(folder.namespace_id()) {
            return Err(SyncwebError::InvalidConfig(
                "snapshot belongs to a different folder".to_owned(),
            ));
        }
        self.verify_blobs(snapshot).await?;
        let docs = self
            .docs
            .as_ref()
            .ok_or_else(|| SyncwebError::InvalidConfig("snapshot store has no document engine".to_owned()))?;
        let expected = snapshot
            .entries
            .iter()
            .map(|entry| entry.path.as_os_str().as_encoded_bytes().to_vec())
            .collect::<BTreeSet<_>>();
        for entry in docs.list_latest(folder.doc()).await? {
            if !entry.key().starts_with(b"sys/") && !expected.contains(entry.key()) {
                folder.delete_entry(entry.key()).await?;
            }
        }
        for entry in &snapshot.entries {
            folder
                .set_blob_ref(entry.path.as_os_str().as_encoded_bytes(), entry.hash, entry.size)
                .await?;
        }
        Ok(())
    }

    /// List all locally pinned snapshots.
    ///
    /// # Errors
    ///
    /// Returns an error if snapshot pins or manifests cannot be read.
    pub async fn list(&self) -> Result<Vec<Snapshot>> {
        let mut snapshots = Vec::new();
        for (name, hash) in self.blobs.list_pins(SNAPSHOT_PIN_PREFIX).await? {
            if !name.ends_with("/manifest") {
                continue;
            }
            let snapshot = self.load_manifest(hash).await?;
            snapshots.push(snapshot);
        }
        snapshots.sort_by(|left, right| right.created_at.cmp(&left.created_at).then(left.id.cmp(&right.id)));
        Ok(snapshots)
    }

    /// Load a snapshot by its content-addressed identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot is not pinned or cannot be decoded.
    pub async fn load(&self, id: SnapshotId) -> Result<Snapshot> {
        let prefix = format!("{SNAPSHOT_PIN_PREFIX}{id}/manifest");
        let hash = self
            .blobs
            .list_pins(prefix.as_bytes())
            .await?
            .into_iter()
            .find_map(|(name, hash)| (name == prefix).then_some(hash))
            .ok_or_else(|| SyncwebError::InvalidConfig(format!("snapshot not found: {id}")))?;
        self.load_manifest(hash).await
    }

    /// Remove a snapshot and release its pins.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot cannot be found or pins cannot be removed.
    pub async fn delete(&self, id: SnapshotId) -> Result<()> {
        let manifest_name = manifest_pin_name(id);
        let manifest_hash = self
            .blobs
            .list_pins(manifest_name.as_bytes())
            .await?
            .into_iter()
            .find_map(|(name, hash)| (name == manifest_name).then_some(hash))
            .ok_or_else(|| SyncwebError::InvalidConfig(format!("snapshot not found: {id}")))?;
        let snapshot = self.load_manifest(manifest_hash).await?;
        self.blobs.unpin(manifest_name).await?;
        for entry in &snapshot.entries {
            self.blobs.unpin(content_pin_name(id, entry.hash)).await?;
        }
        Ok(())
    }

    /// Create a ticket for a snapshot manifest.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot is not locally available.
    pub async fn ticket(&self, endpoint: &Endpoint, snapshot: &Snapshot) -> Result<BlobTicket> {
        let hash = Hash::new(snapshot.to_bytes()?);
        if !self.blobs.has(hash).await? {
            return Err(SyncwebError::InvalidConfig(
                "snapshot manifest is not stored".to_owned(),
            ));
        }
        Ok(self.blobs.ticket(endpoint, hash))
    }

    /// Fetch a shared snapshot manifest and all referenced content blobs.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket, manifest, or any content blob cannot be fetched.
    pub async fn import_ticket(&self, endpoint: &Endpoint, ticket: &BlobTicket) -> Result<Snapshot> {
        if !self.blobs.has(ticket.hash()).await? {
            self.blobs.fetch(endpoint, ticket).await?;
        }
        let snapshot = self.load_manifest(ticket.hash()).await?;
        for entry in &snapshot.entries {
            if !self.blobs.has(entry.hash).await? {
                let content_ticket = BlobTicket::new(ticket.addr().clone(), entry.hash, BlobFormat::Raw);
                self.blobs.fetch(endpoint, &content_ticket).await?;
            }
        }
        self.store(&snapshot).await?;
        Ok(snapshot)
    }

    async fn store(&self, snapshot: &Snapshot) -> Result<()> {
        snapshot.validate()?;
        for entry in &snapshot.entries {
            if !self.blobs.has(entry.hash).await? {
                return Err(SyncwebError::InvalidConfig(format!(
                    "snapshot content is missing: {}",
                    entry.path.display()
                )));
            }
            self.blobs
                .pin(content_pin_name(snapshot.id, entry.hash), entry.hash)
                .await?;
        }
        let bytes = snapshot.to_bytes()?;
        let manifest_hash = self.blobs.add_bytes(&bytes).await?;
        self.blobs.pin(manifest_pin_name(snapshot.id), manifest_hash).await?;
        Ok(())
    }

    async fn load_manifest(&self, hash: Hash) -> Result<Snapshot> {
        let snapshot = Snapshot::from_bytes(self.blobs.get(hash).await?)?;
        if snapshot.id != snapshot.root_hash {
            return Err(SyncwebError::InvalidConfig(
                "snapshot ID does not match its root".to_owned(),
            ));
        }
        Ok(snapshot)
    }

    async fn verify_blobs(&self, snapshot: &Snapshot) -> Result<()> {
        for entry in &snapshot.entries {
            if !self.blobs.has(entry.hash).await? {
                return Err(SyncwebError::InvalidConfig(format!(
                    "snapshot content is unavailable: {}",
                    entry.path.display()
                )));
            }
        }
        Ok(())
    }
}

fn entries_hash(entries: &[SnapshotEntry]) -> Hash {
    let bytes = serde_json::to_vec(entries).unwrap_or_default();
    Hash::new(bytes)
}

fn validate_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(SyncwebError::InvalidConfig(format!(
            "snapshot path must be a non-empty relative path: {}",
            path.display()
        )));
    }
    Ok(())
}

fn manifest_pin_name(id: Hash) -> String {
    format!("{SNAPSHOT_PIN_PREFIX}{id}/manifest")
}

fn content_pin_name(id: Hash, hash: Hash) -> String {
    format!("{SNAPSHOT_PIN_PREFIX}{id}/blob/{hash}")
}
