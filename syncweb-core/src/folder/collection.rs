use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use iroh_blobs::Hash;
use iroh_docs::{AuthorId, api::Doc};
use semver::{Version, VersionReq};
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

    #[must_use]
    pub fn with_dependency(mut self, dependency: PackageDependency) -> Self {
        self.dependencies.push(dependency);
        self
    }
}

/// A version constraint for another package collection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PackageDependency {
    pub collection_id: Uuid,
    pub version_requirement: String,
}

impl PackageDependency {
    #[must_use]
    pub fn new(collection_id: Uuid, version_requirement: impl Into<String>) -> Self {
        Self {
            collection_id,
            version_requirement: version_requirement.into(),
        }
    }

    /// # Errors
    ///
    /// Returns an error when the version requirement is not valid semver.
    pub fn validate(&self) -> Result<()> {
        if self.version_requirement.trim().is_empty() {
            return Err(SyncwebError::InvalidConfig(format!(
                "dependency {} has an empty version requirement",
                self.collection_id
            )));
        }
        VersionReq::parse(&self.version_requirement).map_err(|error| {
            SyncwebError::InvalidConfig(format!(
                "invalid dependency version requirement for {}: {error}",
                self.collection_id
            ))
        })?;
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error when the dependency requirement is not valid semver.
    pub fn accepts(&self, version: &Version) -> Result<bool> {
        self.validate()?;
        let requirement = VersionReq::parse(&self.version_requirement).map_err(|error| {
            SyncwebError::InvalidConfig(format!(
                "invalid dependency version requirement for {}: {error}",
                self.collection_id
            ))
        })?;
        Ok(requirement.matches(version))
    }
}

impl PackageProfile {
    /// # Errors
    ///
    /// Returns an error when the package name or one of its dependencies is invalid.
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(SyncwebError::InvalidConfig("package name cannot be empty".to_owned()));
        }
        for dependency in &self.dependencies {
            dependency.validate()?;
        }
        Ok(())
    }

    /// Check whether every dependency is available at an accepted version.
    ///
    /// The map contains the installed version for each collection ID.
    ///
    /// # Errors
    ///
    /// Returns an error when a dependency requirement is invalid.
    pub fn dependencies_satisfied(&self, available: &BTreeMap<Uuid, Version>) -> Result<bool> {
        self.validate()?;
        self.dependencies.iter().try_fold(true, |satisfied, dependency| {
            let dependency_satisfied = available
                .get(&dependency.collection_id)
                .map(|version| dependency.accepts(version))
                .transpose()?
                .unwrap_or(false);
            Ok(satisfied && dependency_satisfied)
        })
    }
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
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub public_key: Option<String>,
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
            signature: None,
            public_key: None,
        }
    }

    /// # Errors
    ///
    /// Returns an error when the schema, version, paths, dependencies, or
    /// duplicate entries are invalid.
    pub fn validate(&self) -> Result<()> {
        self.validate_content()?;
        validate_signature_metadata(self)?;
        Ok(())
    }

    fn validate_content(&self) -> Result<()> {
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
        if let Some(package) = &self.package {
            package.validate()?;
            if package
                .dependencies
                .iter()
                .any(|dependency| dependency.collection_id == self.collection_id)
            {
                return Err(SyncwebError::InvalidConfig(
                    "package cannot depend on its own collection".to_owned(),
                ));
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

    /// Serialize the manifest without its signature.
    ///
    /// The public key remains part of this representation so changing the
    /// signer changes the manifest identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest is invalid or cannot be serialized.
    pub fn unsigned_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        unsigned.validate_content()?;
        if let Some(public_key) = &unsigned.public_key {
            decode_public_key(public_key)?;
        }
        serde_json::to_vec(&unsigned)
            .map_err(|error| SyncwebError::operation("failed to serialize unsigned collection manifest", error))
    }

    /// Sign the manifest with an Ed25519 maintainer key.
    ///
    /// The signature is encoded as lowercase hexadecimal in the manifest.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest is invalid or cannot be serialized.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        unsigned.public_key = Some(hex::encode(signing_key.verifying_key().to_bytes()));
        let signature = signing_key.sign(&unsigned.unsigned_bytes()?);
        self.public_key = unsigned.public_key;
        self.signature = Some(hex::encode(signature.to_bytes()));
        Ok(())
    }

    /// Verify the manifest's embedded Ed25519 signature.
    ///
    /// Unsigned manifests have no signature to verify and return `Ok(())`;
    /// a partially signed manifest is rejected.
    ///
    /// # Errors
    ///
    /// Returns an error when signature metadata is malformed or verification fails.
    pub fn verify_signature(&self) -> Result<()> {
        self.validate()?;
        let (Some(signature_text), Some(public_key)) = (&self.signature, &self.public_key) else {
            if self.signature.is_none() && self.public_key.is_none() {
                return Ok(());
            }
            return Err(SyncwebError::InvalidConfig(
                "manifest signature and public key must be provided together".to_owned(),
            ));
        };
        let verifying_key = decode_public_key(public_key)?;
        let signature = decode_signature(signature_text)?;
        verifying_key
            .verify(&self.unsigned_bytes()?, &signature)
            .map_err(|error| SyncwebError::InvalidConfig(format!("manifest signature is invalid: {error}")))
    }

    /// Compare this manifest's semver version with another manifest.
    ///
    /// # Errors
    ///
    /// Returns an error when either manifest contains an invalid semver version.
    pub fn version_ordering(&self, other_manifest: &Self) -> Result<Ordering> {
        let current = Version::parse(&self.version)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid collection version: {error}")))?;
        let other_version = Version::parse(&other_manifest.version)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid collection version: {error}")))?;
        Ok(current.cmp(&other_version))
    }

    /// Return whether this manifest is a newer version than another manifest.
    ///
    /// # Errors
    ///
    /// Returns an error when either manifest contains an invalid semver version.
    pub fn is_upgrade_from(&self, other: &Self) -> Result<bool> {
        Ok(self.version_ordering(other)? == Ordering::Greater)
    }

    /// Check whether all package dependencies are available.
    ///
    /// # Errors
    ///
    /// Returns an error when package metadata or dependency requirements are invalid.
    pub fn dependencies_satisfied(&self, available: &BTreeMap<Uuid, Version>) -> Result<bool> {
        self.package
            .as_ref()
            .map_or(Ok(true), |package| package.dependencies_satisfied(available))
    }

    /// # Errors
    ///
    /// Returns an error when the manifest cannot be serialized.
    pub fn content_id(&self) -> Result<Hash> {
        Ok(Hash::new(&self.unsigned_bytes()?))
    }

    /// Return the content hash of the serialized manifest blob.
    ///
    /// This differs from [`Self::content_id`] for signed manifests because
    /// the signature is intentionally excluded from the logical manifest ID.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest is invalid or cannot be serialized.
    pub fn blob_id(&self) -> Result<Hash> {
        Ok(Hash::new(&self.to_bytes()?))
    }
}

fn validate_signature_metadata(manifest: &CollectionManifest) -> Result<()> {
    match (&manifest.signature, &manifest.public_key) {
        (None, None) => Ok(()),
        (Some(signature), Some(public_key)) => {
            decode_signature(signature)?;
            decode_public_key(public_key)?;
            Ok(())
        }
        _ => Err(SyncwebError::InvalidConfig(
            "manifest signature and public key must be provided together".to_owned(),
        )),
    }
}

fn decode_signature(encoded: &str) -> Result<Signature> {
    let bytes = hex::decode(encoded)
        .map_err(|error| SyncwebError::InvalidConfig(format!("invalid manifest signature encoding: {error}")))?;
    Signature::from_slice(&bytes)
        .map_err(|error| SyncwebError::InvalidConfig(format!("invalid manifest signature: {error}")))
}

fn decode_public_key(encoded: &str) -> Result<VerifyingKey> {
    let decoded_bytes = hex::decode(encoded)
        .map_err(|error| SyncwebError::InvalidConfig(format!("invalid manifest public key encoding: {error}")))?;
    let key_bytes: [u8; 32] = decoded_bytes.try_into().map_err(|error: Vec<u8>| {
        SyncwebError::InvalidConfig(format!("manifest public key must be 32 bytes, got {}", error.len()))
    })?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|error| SyncwebError::InvalidConfig(format!("invalid manifest public key: {error}")))
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
