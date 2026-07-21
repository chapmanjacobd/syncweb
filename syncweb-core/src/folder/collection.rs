use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

use iroh_blobs::Hash;
use iroh_docs::{AuthorId, api::Doc};
use semver::Version;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{Result, SyncwebError},
    node::{blob_store::BlobStore, docs_engine::DocsEngine},
};

const SCHEMA_VERSION: u32 = 1;

/// A content-addressed file included in a collection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CollectionEntry {
    pub content_id: Hash,
    pub logical_path: PathBuf,
    pub size: u64,
    pub media_type: Option<String>,
    pub role: String,
    pub relationships: Vec<String>,
}

impl CollectionEntry {
    /// # Errors
    ///
    /// Returns an error when the logical path is absolute or escapes the collection root.
    pub fn new(content_id: Hash, path: impl Into<PathBuf>, size: u64) -> Result<Self> {
        let logical_path = path.into();
        validate_logical_path(&logical_path)?;
        Ok(Self {
            content_id,
            logical_path,
            size,
            media_type: None,
            role: "primary".to_owned(),
            relationships: Vec::new(),
        })
    }
}

/// Package-specific metadata layered on a general collection.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PackageProfile {
    pub name: String,
    pub dependencies: Vec<PackageDependency>,
}

impl PackageProfile {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            dependencies: Vec::new(),
        }
    }
}

/// A version constraint for another package collection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PackageDependency {
    pub collection_id: Uuid,
    pub version_requirement: String,
}

/// Immutable description of a collection version.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CollectionManifest {
    pub schema_version: u32,
    pub collection_id: Uuid,
    pub version: String,
    pub parent: Option<Hash>,
    #[serde(default)]
    pub changelog: Option<String>,
    pub entries: Vec<CollectionEntry>,
    pub package: Option<PackageProfile>,
}

impl CollectionManifest {
    #[must_use]
    pub fn new(collection_id: Uuid, version: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            collection_id,
            version: version.into(),
            parent: None,
            changelog: None,
            entries: Vec::new(),
            package: None,
        }
    }

    /// # Errors
    ///
    /// Returns an error when the schema, version, paths, or duplicate entries are invalid.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(SyncwebError::InvalidConfig(format!(
                "unsupported collection schema version {}",
                self.schema_version
            )));
        }
        Version::parse(&self.version)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid collection version: {error}")))?;
        let mut paths = BTreeSet::new();
        for entry in &self.entries {
            validate_logical_path(&entry.logical_path)?;
            if !paths.insert(&entry.logical_path) {
                return Err(SyncwebError::InvalidConfig(format!(
                    "duplicate collection path {}",
                    entry.logical_path.display()
                )));
            }
        }
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error when the manifest is invalid or cannot be serialized.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self)
            .map_err(|error| SyncwebError::operation("failed to serialize collection manifest", error))
    }

    /// # Errors
    ///
    /// Returns an error when the manifest cannot be decoded or is invalid.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let manifest: Self = serde_json::from_slice(bytes.as_ref())
            .map_err(|error| SyncwebError::operation("failed to deserialize collection manifest", error))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// # Errors
    ///
    /// Returns an error when the manifest cannot be serialized.
    pub fn content_id(&self) -> Result<Hash> {
        Ok(Hash::new(self.to_bytes()?))
    }
}

/// Mutable pointer to the current immutable manifest for a collection.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CollectionHead {
    pub collection_id: Uuid,
    pub manifest: Hash,
    pub sequence: u64,
}

/// Local record of a collection version installed on this device.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InstalledCollection {
    pub manifest: Hash,
    pub versions: BTreeMap<String, PathBuf>,
    pub current: String,
}

/// Local collection installation state.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CollectionState {
    pub installed: BTreeMap<Uuid, InstalledCollection>,
}

impl CollectionState {
    #[must_use]
    pub fn current(&self, collection_id: Uuid) -> Option<&InstalledCollection> {
        self.installed.get(&collection_id)
    }
}

/// Persists immutable manifests as pinned blobs and mutable heads in iroh-docs.
#[derive(Clone)]
pub struct CollectionStore {
    doc: Doc,
    author: AuthorId,
    blobs: BlobStore,
    docs: DocsEngine,
}

impl CollectionStore {
    #[must_use]
    pub const fn new(doc: Doc, author: AuthorId, blobs: BlobStore, docs: DocsEngine) -> Self {
        Self {
            doc,
            author,
            blobs,
            docs,
        }
    }

    /// # Errors
    ///
    /// Returns an error if manifest storage or head publication fails.
    pub async fn publish(&self, manifest: &CollectionManifest, sequence: u64) -> Result<CollectionHead> {
        manifest.validate()?;
        if sequence == 0 {
            return Err(SyncwebError::InvalidConfig(
                "collection sequence must be greater than zero".to_owned(),
            ));
        }
        if let Some(previous) = self.head(manifest.collection_id).await?
            && sequence <= previous.sequence
        {
            return Err(SyncwebError::InvalidConfig(format!(
                "collection sequence {sequence} must be greater than the current sequence {}",
                previous.sequence
            )));
        }
        let bytes = manifest.to_bytes()?;
        for entry in &manifest.entries {
            if !self.blobs.has(entry.content_id).await? {
                return Err(SyncwebError::InvalidConfig(format!(
                    "collection content is missing from the blob store: {}",
                    entry.logical_path.display()
                )));
            }
            self.blobs
                .pin(
                    content_pin_name(manifest.collection_id, entry.content_id),
                    entry.content_id,
                )
                .await?;
        }
        let hash = self.blobs.add_bytes(&bytes).await?;
        self.blobs.pin(manifest_pin_name(hash), hash).await?;
        self.docs
            .set_blob(
                &self.doc,
                self.author,
                manifest_key(manifest.collection_id, &manifest.version),
                hash,
                u64::try_from(bytes.len()).map_err(|error| SyncwebError::operation("manifest is too large", error))?,
            )
            .await?;
        let head = CollectionHead {
            collection_id: manifest.collection_id,
            manifest: hash,
            sequence,
        };
        let head_bytes = serde_json::to_vec(&head)
            .map_err(|error| SyncwebError::operation("failed to serialize collection head", error))?;
        self.docs
            .set(&self.doc, self.author, head_key(manifest.collection_id), head_bytes)
            .await?;
        Ok(head)
    }

    /// Read the mutable collection head stored in the document.
    ///
    /// # Errors
    ///
    /// Returns an error if the head cannot be read or decoded.
    pub async fn head(&self, collection_id: Uuid) -> Result<Option<CollectionHead>> {
        let Some(entry) = self.docs.get_any(&self.doc, head_key(collection_id)).await? else {
            return Ok(None);
        };
        let bytes = self.blobs.get(entry.content_hash()).await?;
        let head: CollectionHead = serde_json::from_slice(&bytes)
            .map_err(|error| SyncwebError::operation("failed to deserialize collection head", error))?;
        if head.collection_id != collection_id {
            return Err(SyncwebError::InvalidConfig(
                "collection head ID does not match its key".to_owned(),
            ));
        }
        Ok(Some(head))
    }

    /// # Errors
    ///
    /// Returns an error if the manifest blob cannot be read or decoded.
    pub async fn load(&self, hash: Hash) -> Result<CollectionManifest> {
        CollectionManifest::from_bytes(self.blobs.get(hash).await?)
    }
}

fn validate_logical_path(path: &Path) -> Result<()> {
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
            "collection path must be a non-empty relative path: {}",
            path.display()
        )));
    }
    Ok(())
}

fn manifest_key(collection_id: Uuid, version: &str) -> String {
    format!("collections/{collection_id}/manifests/{version}")
}

fn head_key(collection_id: Uuid) -> String {
    format!("collections/{collection_id}/head")
}

fn manifest_pin_name(hash: Hash) -> String {
    format!("syncweb/collection-manifest/{hash}")
}

fn content_pin_name(collection_id: Uuid, hash: Hash) -> String {
    format!("syncweb/collection/{collection_id}/{hash}")
}
