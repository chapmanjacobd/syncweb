use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use iroh::Endpoint;
use iroh_blobs::{BlobFormat, Hash, ticket::BlobTicket};
use semver::Version;
#[cfg(not(unix))]
use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::{Result, SyncwebError},
    folder::{CollectionManifest, CollectionState, InstalledCollection},
};

const STATE_FILE: &str = "collections.json";

/// Installs collection package versions into versioned directories.
#[derive(Clone, Debug)]
pub struct PackageManager {
    root: PathBuf,
}

impl PackageManager {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// # Errors
    ///
    /// Returns an error if state cannot be read or decoded.
    pub fn state(&self) -> Result<CollectionState> {
        let path = self.root.join(STATE_FILE);
        if !path.exists() {
            return Ok(CollectionState::default());
        }
        let bytes =
            fs::read(&path).map_err(|error| SyncwebError::operation("failed to read collection state", error))?;
        serde_json::from_slice(&bytes)
            .map_err(|error| SyncwebError::operation("failed to decode collection state", error))
    }

    /// Return the currently selected version for each installed collection.
    ///
    /// # Errors
    ///
    /// Returns an error if an installed version is not valid semver.
    pub fn available_versions(&self) -> Result<BTreeMap<Uuid, Version>> {
        self.state()?
            .installed
            .into_iter()
            .map(|(collection_id, installed)| {
                let version = Version::parse(&installed.current).map_err(|error| {
                    SyncwebError::InvalidConfig(format!(
                        "installed collection {collection_id} has an invalid version: {error}"
                    ))
                })?;
                Ok((collection_id, version))
            })
            .collect()
    }

    /// Stage, verify, and atomically make a collection version current.
    ///
    /// # Errors
    ///
    /// Returns an error when source content does not match the manifest or installation fails.
    pub fn install(&self, manifest: &CollectionManifest, source: impl AsRef<Path>) -> Result<()> {
        manifest.validate()?;
        verify_directory(manifest, source.as_ref())?;
        let manifest_hash = manifest.blob_id()?;
        let collection_dir = self.root.join(manifest.collection_id.to_string());
        fs::create_dir_all(&collection_dir)
            .map_err(|error| SyncwebError::operation("failed to create package directory", error))?;
        let version_dir = collection_dir.join(&manifest.version);
        if version_dir.exists() {
            return Err(SyncwebError::InvalidConfig(format!(
                "collection version {} is already installed",
                manifest.version
            )));
        }

        let staging = collection_dir.join(format!(".stage-{}", Uuid::new_v4()));
        copy_manifest_entries(manifest, source.as_ref(), &staging)?;
        fs::rename(&staging, &version_dir)
            .map_err(|error| SyncwebError::operation("failed to finalize staged package", error))?;
        self.set_current(manifest.collection_id, &manifest.version)?;

        let mut state = self.state()?;
        let installed = state
            .installed
            .entry(manifest.collection_id)
            .or_insert_with(|| InstalledCollection {
                manifest: manifest_hash,
                versions: BTreeMap::default(),
                current: manifest.version.clone(),
            });
        installed.manifest = manifest_hash;
        installed.current.clone_from(&manifest.version);
        installed.versions.insert(manifest.version.clone(), version_dir);
        self.save_state(&state)
    }

    /// Fetch a manifest and all of its content blobs, then install it
    /// atomically.
    ///
    /// # Errors
    ///
    /// Returns an error if a ticket, manifest, content blob, or installation
    /// step is invalid.
    pub async fn install_from_ticket(
        &self,
        manifest_ticket: &BlobTicket,
        endpoint: &Endpoint,
        blobs: &crate::node::blob_store::BlobStore,
    ) -> Result<CollectionManifest> {
        if !blobs.has(manifest_ticket.hash()).await? {
            blobs.fetch(endpoint, manifest_ticket).await?;
        }
        let manifest_bytes = blobs.get(manifest_ticket.hash()).await?;
        let manifest = CollectionManifest::from_bytes(&manifest_bytes)?;
        if manifest.blob_id()? != manifest_ticket.hash() {
            return Err(SyncwebError::InvalidTicket(
                "manifest ticket hash does not match manifest content".to_owned(),
            ));
        }

        let source = self.root.join(format!(".source-{}", Uuid::new_v4()));
        fs::create_dir_all(&source)
            .map_err(|error| SyncwebError::operation("failed to create package source directory", error))?;
        for entry in &manifest.entries {
            if !blobs.has(entry.content_id).await? {
                let ticket = BlobTicket::new(manifest_ticket.addr().clone(), entry.content_id, BlobFormat::Raw);
                blobs.fetch(endpoint, &ticket).await?;
            }
            let bytes = blobs.get(entry.content_id).await?;
            let destination = source.join(&entry.logical_path);
            let parent = destination
                .parent()
                .ok_or_else(|| SyncwebError::InvalidConfig("package entry has no parent directory".to_owned()))?;
            fs::create_dir_all(parent)
                .map_err(|error| SyncwebError::operation("failed to create package source directory", error))?;
            fs::write(destination, bytes)
                .map_err(|error| SyncwebError::operation("failed to materialize package entry", error))?;
        }

        let install_result = self.install(&manifest, &source);
        let cleanup_result = fs::remove_dir_all(&source)
            .map_err(|error| SyncwebError::operation("failed to remove temporary package source", error));
        match (install_result, cleanup_result) {
            (Err(error), _) | (Ok(()), Err(error)) => Err(error),
            (Ok(()), Ok(())) => Ok(manifest),
        }
    }

    /// # Errors
    ///
    /// Returns an error if the requested version is not installed or the active link cannot be switched.
    pub fn switch(&self, collection_id: Uuid, requested_version: impl AsRef<str>) -> Result<()> {
        let version = requested_version.as_ref();
        let mut state = self.state()?;
        let installed = state
            .installed
            .get_mut(&collection_id)
            .ok_or_else(|| SyncwebError::FolderNotFound(collection_id.to_string()))?;
        if !installed.versions.contains_key(version) {
            return Err(SyncwebError::InvalidConfig(format!(
                "collection version {version} is not installed"
            )));
        }
        self.set_current(collection_id, version)?;
        version.clone_into(&mut installed.current);
        self.save_state(&state)
    }

    /// # Errors
    ///
    /// Returns an error if an installed version cannot be deleted.
    pub fn remove(&self, collection_id: Uuid, requested_version: impl AsRef<str>) -> Result<()> {
        let version = requested_version.as_ref();
        let mut state = self.state()?;
        let installed = state
            .installed
            .get_mut(&collection_id)
            .ok_or_else(|| SyncwebError::FolderNotFound(collection_id.to_string()))?;
        if installed.current == version {
            return Err(SyncwebError::InvalidConfig(
                "switch away from the active collection version before removing it".to_owned(),
            ));
        }
        let path = installed
            .versions
            .remove(version)
            .ok_or_else(|| SyncwebError::InvalidConfig(format!("collection version {version} is not installed")))?;
        fs::remove_dir_all(path)
            .map_err(|error| SyncwebError::operation("failed to remove installed collection version", error))?;
        if installed.versions.is_empty() {
            state.installed.remove(&collection_id);
        }
        self.save_state(&state)
    }

    /// # Errors
    ///
    /// Returns an error when an installed version's content does not match its manifest.
    pub fn verify(&self, manifest: &CollectionManifest) -> Result<()> {
        let state = self.state()?;
        let installed = state
            .installed
            .get(&manifest.collection_id)
            .ok_or_else(|| SyncwebError::FolderNotFound(manifest.collection_id.to_string()))?;
        let path = installed.versions.get(&manifest.version).ok_or_else(|| {
            SyncwebError::InvalidConfig(format!("collection version {} is not installed", manifest.version))
        })?;
        verify_directory(manifest, path)
    }

    fn set_current(&self, collection_id: Uuid, version: &str) -> Result<()> {
        let collection_dir = self.root.join(collection_id.to_string());
        let current = collection_dir.join("current");
        let temporary = collection_dir.join(format!(".current-{}", Uuid::new_v4()));
        create_current_link(version, &temporary)?;
        fs::rename(&temporary, &current)
            .map_err(|error| SyncwebError::operation("failed to atomically switch collection version", error))
    }

    fn save_state(&self, state: &CollectionState) -> Result<()> {
        fs::create_dir_all(&self.root)
            .map_err(|error| SyncwebError::operation("failed to create package state directory", error))?;
        let temporary = self.root.join(format!(".{STATE_FILE}-{}", Uuid::new_v4()));
        let bytes = serde_json::to_vec_pretty(state)
            .map_err(|error| SyncwebError::operation("failed to serialize collection state", error))?;
        fs::write(&temporary, bytes)
            .map_err(|error| SyncwebError::operation("failed to write collection state", error))?;
        fs::rename(temporary, self.root.join(STATE_FILE))
            .map_err(|error| SyncwebError::operation("failed to atomically save collection state", error))
    }
}

fn verify_directory(manifest: &CollectionManifest, root: &Path) -> Result<()> {
    for entry in &manifest.entries {
        let path = root.join(&entry.logical_path);
        let bytes = fs::read(&path)
            .map_err(|error| SyncwebError::operation("failed to read package entry during verification", error))?;
        let actual_size =
            u64::try_from(bytes.len()).map_err(|error| SyncwebError::operation("package entry is too large", error))?;
        if actual_size != entry.size || Hash::new(&bytes) != entry.content_id {
            return Err(SyncwebError::InvalidConfig(format!(
                "package entry does not match manifest: {}",
                entry.logical_path.display()
            )));
        }
    }
    Ok(())
}

fn copy_manifest_entries(manifest: &CollectionManifest, source: &Path, staging: &Path) -> Result<()> {
    for entry in &manifest.entries {
        let destination = staging.join(&entry.logical_path);
        let parent = destination
            .parent()
            .ok_or_else(|| SyncwebError::InvalidConfig("package entry has no parent directory".to_owned()))?;
        fs::create_dir_all(parent)
            .map_err(|error| SyncwebError::operation("failed to create package staging directory", error))?;
        fs::copy(source.join(&entry.logical_path), destination)
            .map_err(|error| SyncwebError::operation("failed to stage package entry", error))?;
    }
    Ok(())
}

#[cfg(unix)]
fn create_current_link(version: &str, path: &Path) -> Result<()> {
    std::os::unix::fs::symlink(version, path)
        .map_err(|error| SyncwebError::operation("failed to create current collection link", error))
}

#[cfg(not(unix))]
fn create_current_link(version: &str, path: &Path) -> Result<()> {
    fs::write(path, Value::String(version.to_owned()))
        .map_err(|error| SyncwebError::operation("failed to create current collection marker", error))
}
