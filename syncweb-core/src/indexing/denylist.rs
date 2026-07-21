//! Local and federated content filtering for the indexing service.
//!
//! Denylists are deliberately evaluated before discovery or fetching. They
//! describe local policy only; accepting a record from a remote filter list
//! never makes that list authoritative for another node.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use iroh_blobs::Hash;
use iroh_docs::NamespaceId;
use serde::{Deserialize, Serialize};

use crate::{
    error::{Result, SyncwebError},
    indexing::CatalogRecord,
};

const FILTER_LIST_CONTEXT: &[u8] = b"syncweb/filter-list/v1\0";

/// The scope of a denylist rule.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DenylistRule {
    Device(String),
    File {
        namespace_id: Option<NamespaceId>,
        key: Vec<u8>,
    },
    Hash(Hash),
}

impl DenylistRule {
    #[must_use]
    pub fn device<D: ToString + ?Sized>(device: &D) -> Self {
        Self::Device(device.to_string())
    }

    #[must_use]
    pub fn file(key: impl AsRef<[u8]>) -> Self {
        Self::File {
            namespace_id: None,
            key: key.as_ref().to_vec(),
        }
    }

    #[must_use]
    pub fn folder_file(namespace_id: NamespaceId, key: impl AsRef<[u8]>) -> Self {
        Self::File {
            namespace_id: Some(namespace_id),
            key: key.as_ref().to_vec(),
        }
    }

    #[must_use]
    pub const fn hash(hash: Hash) -> Self {
        Self::Hash(hash)
    }
}

/// Information supplied by a discovery or fetch hook.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct FilterContext {
    pub device: Option<String>,
    pub namespace_id: Option<NamespaceId>,
    pub key: Option<Vec<u8>>,
    pub hash: Option<Hash>,
}

impl FilterContext {
    #[must_use]
    pub fn new(hash: Hash) -> Self {
        Self {
            hash: Some(hash),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn for_file(namespace_id: NamespaceId, key: impl AsRef<[u8]>, hash: Hash) -> Self {
        Self {
            namespace_id: Some(namespace_id),
            key: Some(key.as_ref().to_vec()),
            hash: Some(hash),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_device<D: ToString + ?Sized>(mut self, device: &D) -> Self {
        self.device = Some(device.to_string());
        self
    }
}

/// The rule that caused a filter decision.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DenyReason {
    Device(String),
    File(Vec<u8>),
    Hash(Hash),
}

/// A failed denylist check.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Denied {
    pub reason: DenyReason,
}

impl std::fmt::Display for Denied {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.reason {
            DenyReason::Device(device) => write!(formatter, "device is denylisted: {device}"),
            DenyReason::File(key) => write!(formatter, "file is denylisted: {}", String::from_utf8_lossy(key)),
            DenyReason::Hash(hash) => write!(formatter, "content hash is denylisted: {hash}"),
        }
    }
}

/// A local content denylist.
#[derive(Clone, Debug, Default)]
pub struct Denylist {
    devices: HashSet<String>,
    files: HashSet<(Option<NamespaceId>, Vec<u8>)>,
    hashes: HashSet<Hash>,
    federated: HashMap<NamespaceId, u64>,
}

impl Denylist {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, rule: DenylistRule) {
        match rule {
            DenylistRule::Device(device) => {
                self.devices.insert(device);
            }
            DenylistRule::File { namespace_id, key } => {
                self.files.insert((namespace_id, key));
            }
            DenylistRule::Hash(hash) => {
                self.hashes.insert(hash);
            }
        }
    }

    #[must_use]
    pub fn remove(&mut self, rule: &DenylistRule) -> bool {
        match rule {
            DenylistRule::Device(device) => self.devices.remove(device),
            DenylistRule::File { namespace_id, key } => self.files.remove(&(*namespace_id, key.clone())),
            DenylistRule::Hash(hash) => self.hashes.remove(hash),
        }
    }

    pub fn block_device<D: ToString + ?Sized>(&mut self, device: &D) {
        self.devices.insert(device.to_string());
    }

    pub fn unblock_device<D: ToString + ?Sized>(&mut self, device: &D) -> bool {
        self.devices.remove(&device.to_string())
    }

    pub fn block_file(&mut self, key: impl AsRef<[u8]>) {
        self.files.insert((None, key.as_ref().to_vec()));
    }

    pub fn unblock_file(&mut self, key: impl AsRef<[u8]>) -> bool {
        self.files.remove(&(None, key.as_ref().to_vec()))
    }

    pub fn block_folder_file(&mut self, namespace_id: NamespaceId, key: impl AsRef<[u8]>) {
        self.files.insert((Some(namespace_id), key.as_ref().to_vec()));
    }

    pub fn unblock_folder_file(&mut self, namespace_id: NamespaceId, key: impl AsRef<[u8]>) -> bool {
        self.files.remove(&(Some(namespace_id), key.as_ref().to_vec()))
    }

    pub fn block_hash(&mut self, hash: Hash) {
        self.hashes.insert(hash);
    }

    pub fn unblock_hash(&mut self, hash: &Hash) -> bool {
        self.hashes.remove(hash)
    }

    #[must_use]
    pub fn is_device_blocked<D: ToString + ?Sized>(&self, device: &D) -> bool {
        self.devices.contains(&device.to_string())
    }

    #[must_use]
    pub fn is_file_blocked(&self, namespace_id: Option<NamespaceId>, key: impl AsRef<[u8]>) -> bool {
        let key_bytes = key.as_ref();
        self.files.contains(&(None, key_bytes.to_vec()))
            || namespace_id.is_some_and(|namespace| self.files.contains(&(Some(namespace), key_bytes.to_vec())))
    }

    #[must_use]
    pub fn is_hash_blocked(&self, hash: &Hash) -> bool {
        self.hashes.contains(hash)
    }

    /// Return whether any rule blocks the supplied context.
    #[must_use]
    pub fn is_blocked(&self, context: &FilterContext) -> bool {
        self.matching_rule(context).is_some()
    }

    /// Return the first matching local or federated rule.
    #[must_use]
    pub fn matching_rule(&self, context: &FilterContext) -> Option<DenyReason> {
        if let Some(device) = context.device.as_deref()
            && self.devices.contains(device)
        {
            return Some(DenyReason::Device(device.to_owned()));
        }
        if let Some(key) = context.key.as_deref()
            && self.is_file_blocked(context.namespace_id, key)
        {
            return Some(DenyReason::File(key.to_vec()));
        }
        if let Some(hash) = context.hash
            && self.hashes.contains(&hash)
        {
            return Some(DenyReason::Hash(hash));
        }
        None
    }

    /// Validate a fetch request against this denylist.
    ///
    /// # Errors
    ///
    /// Returns `InvalidConfig` when a rule blocks the request.
    pub fn check_fetch(&self, context: &FilterContext) -> Result<()> {
        if let Some(reason) = self.matching_rule(context) {
            return Err(SyncwebError::InvalidConfig(Denied { reason }.to_string()));
        }
        Ok(())
    }

    /// Validate a discovery record before it is shown or imported.
    ///
    /// # Errors
    ///
    /// Returns `InvalidConfig` when a rule blocks the record.
    pub fn check_discovery(&self, record: &CatalogRecord) -> Result<()> {
        self.check_fetch(&FilterContext::for_file(
            record.folder_namespace_id,
            &record.key,
            record.hash,
        ))
    }

    /// Apply a newer federated filter list.
    ///
    /// Lists are monotonic per namespace. Entries from an older list are
    /// ignored, so a delayed update cannot remove local filtering rules.
    ///
    /// # Errors
    ///
    /// Returns an error if the list is malformed or unsigned.
    pub fn sync_filter_list(&mut self, list: &FilterList) -> Result<bool> {
        list.verify_signature()?;
        let current = self.federated.get(&list.namespace_id).copied().unwrap_or(0);
        if list.sequence <= current {
            return Ok(false);
        }
        for rule in &list.entries {
            self.add(rule.clone());
        }
        self.federated.insert(list.namespace_id, list.sequence);
        Ok(true)
    }

    /// Subscribe to a federated filter namespace and apply its first list.
    ///
    /// # Errors
    ///
    /// Returns an error if the list is malformed or unsigned.
    pub fn subscribe_filter_list(&mut self, list: &FilterList) -> Result<bool> {
        self.sync_filter_list(list)
    }

    /// Alias for [`Self::subscribe_filter_list`].
    ///
    /// # Errors
    ///
    /// Returns an error if the list is malformed or unsigned.
    pub fn subscribe(&mut self, list: &FilterList) -> Result<bool> {
        self.subscribe_filter_list(list)
    }

    /// Alias for [`Self::sync_filter_list`].
    ///
    /// # Errors
    ///
    /// Returns an error if the list is malformed or unsigned.
    pub fn sync(&mut self, list: &FilterList) -> Result<bool> {
        self.sync_filter_list(list)
    }

    #[must_use]
    pub fn federated_sequence(&self, namespace_id: &NamespaceId) -> Option<u64> {
        self.federated.get(namespace_id).copied()
    }
}

/// Thread-safe denylist hooks suitable for sharing with fetch and discovery
/// tasks.
#[derive(Clone, Debug, Default)]
pub struct DenylistService {
    denylist: Arc<RwLock<Denylist>>,
}

impl DenylistService {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// # Errors
    ///
    /// Returns an error if the denylist lock is poisoned.
    pub fn snapshot(&self) -> Result<Denylist> {
        self.denylist
            .read()
            .map(|denylist| denylist.clone())
            .map_err(|error| SyncwebError::operation("denylist lock poisoned", error))
    }

    /// # Errors
    ///
    /// Returns an error if the denylist lock is poisoned.
    pub fn add(&self, rule: DenylistRule) -> Result<()> {
        self.denylist
            .write()
            .map_err(|error| SyncwebError::operation("denylist lock poisoned", error))?
            .add(rule);
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if the denylist lock is poisoned.
    pub fn remove(&self, rule: &DenylistRule) -> Result<bool> {
        Ok(self
            .denylist
            .write()
            .map_err(|error| SyncwebError::operation("denylist lock poisoned", error))?
            .remove(rule))
    }

    /// # Errors
    ///
    /// Returns an error if the denylist lock is poisoned or the context is blocked.
    pub fn check_fetch(&self, context: &FilterContext) -> Result<()> {
        self.denylist
            .read()
            .map_err(|error| SyncwebError::operation("denylist lock poisoned", error))?
            .check_fetch(context)
    }

    /// Return whether any local rule blocks the supplied context.
    /// # Errors
    ///
    /// Returns an error if the denylist lock is poisoned.
    pub fn is_blocked(&self, context: &FilterContext) -> Result<bool> {
        Ok(self
            .denylist
            .read()
            .map_err(|error| SyncwebError::operation("denylist lock poisoned", error))?
            .is_blocked(context))
    }

    /// # Errors
    ///
    /// Returns an error if the denylist lock is poisoned or the record is blocked.
    pub fn check_discovery(&self, record: &CatalogRecord) -> Result<()> {
        self.denylist
            .read()
            .map_err(|error| SyncwebError::operation("denylist lock poisoned", error))?
            .check_discovery(record)
    }

    /// # Errors
    ///
    /// Returns an error if the denylist lock is poisoned or the list is invalid.
    pub fn sync_filter_list(&self, list: &FilterList) -> Result<bool> {
        self.denylist
            .write()
            .map_err(|error| SyncwebError::operation("denylist lock poisoned", error))?
            .sync_filter_list(list)
    }

    /// Alias for [`Self::sync_filter_list`].
    ///
    /// # Errors
    ///
    /// Returns an error if the denylist lock is poisoned or the list is invalid.
    pub fn subscribe(&self, list: &FilterList) -> Result<bool> {
        self.sync_filter_list(list)
    }
}

/// A signed, monotonic federated filter-list update.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FilterList {
    pub namespace_id: NamespaceId,
    pub sequence: u64,
    pub publisher: String,
    pub entries: Vec<DenylistRule>,
    pub created_at: u64,
    pub signature: Option<String>,
}

impl FilterList {
    /// Create and sign a filter list.
    ///
    /// # Errors
    ///
    /// Returns an error when the sequence is zero or serialization/signing
    /// fails.
    pub fn new(
        namespace_id: NamespaceId,
        sequence: u64,
        entries: Vec<DenylistRule>,
        signing_key: &SigningKey,
    ) -> Result<Self> {
        let mut list = Self {
            namespace_id,
            sequence,
            publisher: hex::encode(signing_key.verifying_key().to_bytes()),
            entries,
            created_at: current_epoch_seconds(),
            signature: None,
        };
        list.sign(signing_key)?;
        Ok(list)
    }

    /// # Errors
    ///
    /// Returns an error if the signer does not match the publisher or
    /// serialization fails.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<()> {
        if self.publisher != hex::encode(signing_key.verifying_key().to_bytes()) {
            return Err(SyncwebError::InvalidIdentity(
                "filter list signer does not match publisher".to_owned(),
            ));
        }
        self.signature = Some(hex::encode(signing_key.sign(&self.unsigned_bytes()?).to_bytes()));
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if the list, publisher, or signature is invalid.
    pub fn verify_signature(&self) -> Result<()> {
        self.validate()?;
        let signature = self
            .signature
            .as_deref()
            .ok_or_else(|| SyncwebError::InvalidConfig("filter list must be signed".to_owned()))?;
        let bytes = hex::decode(signature)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid filter list signature: {error}")))?;
        let parsed_signature = Signature::from_slice(&bytes)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid filter list signature: {error}")))?;
        let publisher_bytes = hex::decode(&self.publisher)
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid filter list publisher: {error}")))?;
        let publisher: [u8; 32] = publisher_bytes.try_into().map_err(|publisher_bytes_vec: Vec<u8>| {
            SyncwebError::InvalidIdentity(format!(
                "filter list publisher must be 32 bytes, got {}",
                publisher_bytes_vec.len()
            ))
        })?;
        let key = VerifyingKey::from_bytes(&publisher)
            .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid filter list publisher: {error}")))?;
        key.verify(&self.unsigned_bytes()?, &parsed_signature)
            .map_err(|error| SyncwebError::InvalidConfig(format!("filter list signature is invalid: {error}")))
    }

    /// # Errors
    ///
    /// Returns an error if the sequence or publisher is invalid.
    pub fn validate(&self) -> Result<()> {
        if self.sequence == 0 {
            return Err(SyncwebError::InvalidConfig(
                "filter list sequence must be greater than zero".to_owned(),
            ));
        }
        if self.publisher.trim().is_empty() {
            return Err(SyncwebError::InvalidIdentity(
                "filter list publisher cannot be empty".to_owned(),
            ));
        }
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if validation or serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|error| SyncwebError::operation("failed to serialize filter list", error))
    }

    /// # Errors
    ///
    /// Returns an error if deserialization or validation fails.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let list: Self = serde_json::from_slice(bytes.as_ref())
            .map_err(|error| SyncwebError::operation("failed to deserialize filter list", error))?;
        list.validate()?;
        Ok(list)
    }

    fn unsigned_bytes(&self) -> Result<Vec<u8>> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        let encoded = serde_json::to_vec(&unsigned)
            .map_err(|error| SyncwebError::operation("failed to serialize filter list", error))?;
        let capacity = FILTER_LIST_CONTEXT
            .len()
            .checked_add(encoded.len())
            .ok_or_else(|| SyncwebError::operation("filter list payload is too large", "capacity overflow"))?;
        let mut bytes = Vec::with_capacity(capacity);
        bytes.extend_from_slice(FILTER_LIST_CONTEXT);
        bytes.extend_from_slice(&encoded);
        Ok(bytes)
    }
}

fn current_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    #[test]
    fn device_file_and_hash_rules_block_fetches() -> anyhow::Result<()> {
        let namespace = NamespaceId::from([1_u8; 32]);
        let hash = Hash::from_bytes([2_u8; 32]);
        let mut denylist = Denylist::new();
        denylist.block_device("device-a");
        denylist.block_folder_file(namespace, b"private.txt");
        denylist.block_hash(hash);

        anyhow::ensure!(
            denylist
                .check_fetch(&FilterContext::new(Hash::from_bytes([3_u8; 32])).with_device("device-a"))
                .is_err()
        );
        anyhow::ensure!(
            denylist
                .check_fetch(&FilterContext::for_file(
                    namespace,
                    b"private.txt",
                    Hash::from_bytes([3_u8; 32])
                ))
                .is_err()
        );
        anyhow::ensure!(denylist.check_fetch(&FilterContext::new(hash)).is_err());
        Ok(())
    }

    #[test]
    fn federated_filter_lists_are_signed_and_monotonic() -> anyhow::Result<()> {
        let namespace = NamespaceId::from([4_u8; 32]);
        let list = FilterList::new(
            namespace,
            1,
            vec![DenylistRule::hash(Hash::from_bytes([5_u8; 32]))],
            &key(1),
        )?;
        let mut denylist = Denylist::new();
        anyhow::ensure!(denylist.subscribe_filter_list(&list.clone())?);
        anyhow::ensure!(!denylist.subscribe_filter_list(&list)?);
        anyhow::ensure!(denylist.is_hash_blocked(&Hash::from_bytes([5_u8; 32])));
        Ok(())
    }
}
