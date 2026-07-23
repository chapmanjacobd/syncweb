use std::path::PathBuf;

use iroh_blobs::Hash;

use crate::{
    Result, SyncwebError,
    folder::SyncwebFolder,
    node::{blob_store::BlobStore, docs_engine::DocsEngine},
};

/// Details about a blob whose local bytes do not match its expected hash.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct CorruptionInfo {
    pub path: PathBuf,
    pub expected_hash: Hash,
    pub actual_hash: Hash,
}

/// Result of checking local content referenced by a folder.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct VerifyResult {
    pub total: u64,
    pub verified: u64,
    pub corrupted: Vec<CorruptionInfo>,
    pub missing: Vec<PathBuf>,
}

impl VerifyResult {
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.corrupted.is_empty() && self.missing.is_empty()
    }
}

/// Filter constraints for selective verification.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct VerifyFilter {
    pub hashes: Option<Vec<Hash>>,
    pub path: Option<PathBuf>,
    pub glob: Option<String>,
}

impl VerifyFilter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            hashes: None,
            path: None,
            glob: None,
        }
    }

    #[must_use]
    pub fn with_hashes(mut self, hashes: Vec<Hash>) -> Self {
        self.hashes = Some(hashes);
        self
    }

    #[must_use]
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Check whether an entry matches this filter.
    #[must_use]
    pub fn matches(&self, entry_key: &[u8], entry_hash: &Hash) -> bool {
        if let Some(ref hashes) = self.hashes
            && !hashes.contains(entry_hash)
        {
            return false;
        }
        if let Some(ref path) = self.path {
            let entry_path = String::from_utf8_lossy(entry_key);
            if entry_path != path.to_string_lossy() {
                return false;
            }
        }
        if let Some(ref glob) = self.glob {
            let entry_path = String::from_utf8_lossy(entry_key);
            if !glob_match(glob, &entry_path) {
                return false;
            }
        }
        true
    }
}

fn glob_match(pattern: &str, path: &str) -> bool {
    globset::Glob::new(pattern).is_ok_and(|glob| glob.compile_matcher().is_match(path))
}

/// Outcome of attempting to repair a single corrupt blob.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct RepairOutcome {
    pub path: PathBuf,
    pub hash: Hash,
    pub success: bool,
    pub error: Option<String>,
}

impl RepairOutcome {
    #[must_use]
    pub const fn new(path: PathBuf, hash: Hash, success: bool, error: Option<String>) -> Self {
        Self {
            path,
            hash,
            success,
            error,
        }
    }
}

/// Result of a repair attempt across corrupt blobs.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct RepairResult {
    pub attempted: u64,
    pub repaired: u64,
    pub failed: Vec<RepairOutcome>,
}

/// Result of a combined verify+repair operation.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[non_exhaustive]
pub struct FixedVerifyResult {
    pub verify: VerifyResult,
    pub repair: Option<RepairResult>,
}

impl FixedVerifyResult {
    #[must_use]
    pub const fn new(verify: VerifyResult, repair: Option<RepairResult>) -> Self {
        Self { verify, repair }
    }
}

/// Re-checks the BLAKE3 content hash of locally stored folder blobs.
#[derive(Clone)]
pub struct IntegrityChecker {
    blob_store: BlobStore,
    docs_engine: DocsEngine,
}

impl IntegrityChecker {
    #[must_use]
    pub const fn new(blob_store: BlobStore, docs_engine: DocsEngine) -> Self {
        Self {
            blob_store,
            docs_engine,
        }
    }

    #[must_use]
    pub const fn blob_store(&self) -> &BlobStore {
        &self.blob_store
    }

    #[must_use]
    pub const fn docs_engine(&self) -> &DocsEngine {
        &self.docs_engine
    }

    /// Verify every non-system entry in a folder.
    ///
    /// # Errors
    ///
    /// Returns an error if folder entries or local blobs cannot be read.
    pub async fn verify_folder(&self, folder: &SyncwebFolder) -> Result<VerifyResult> {
        let entries = self.docs_engine.list_latest(folder.doc()).await?;
        let mut result = VerifyResult::default();
        for entry in entries {
            if entry.key().starts_with(b"sys/") {
                continue;
            }
            result.total = result.total.saturating_add(1);
            let path = String::from_utf8(entry.key().to_vec())
                .map(PathBuf::from)
                .map_err(|error| SyncwebError::operation("folder entry path is not UTF-8", error))?;
            let expected = entry.content_hash();
            if !self.blob_store.has(expected).await? {
                result.missing.push(path);
                continue;
            }
            let bytes = self.blob_store.get(expected).await?;
            let actual = Hash::from_bytes(*blake3::hash(&bytes).as_bytes());
            if actual == expected {
                result.verified = result.verified.saturating_add(1);
            } else {
                result.corrupted.push(CorruptionInfo {
                    path,
                    expected_hash: expected,
                    actual_hash: actual,
                });
            }
        }
        Ok(result)
    }

    /// Verify folder entries matching the optional filter.
    ///
    /// When the filter is `None`, all non-system entries are verified.
    ///
    /// # Errors
    ///
    /// Returns an error if folder entries or local blobs cannot be read.
    pub async fn verify_folder_filtered(
        &self,
        folder: &SyncwebFolder,
        filter: Option<&VerifyFilter>,
    ) -> Result<VerifyResult> {
        let entries = self.docs_engine.list_latest(folder.doc()).await?;
        let mut result = VerifyResult::default();
        for entry in entries {
            if entry.key().starts_with(b"sys/") {
                continue;
            }
            let expected = entry.content_hash();
            if filter.is_some_and(|f| !f.matches(entry.key(), &expected)) {
                continue;
            }
            result.total = result.total.saturating_add(1);
            let path = String::from_utf8(entry.key().to_vec())
                .map(PathBuf::from)
                .map_err(|error| SyncwebError::operation("folder entry path is not UTF-8", error))?;
            if !self.blob_store.has(expected).await? {
                result.missing.push(path);
                continue;
            }
            let bytes = self.blob_store.get(expected).await?;
            let actual = Hash::from_bytes(*blake3::hash(&bytes).as_bytes());
            if actual == expected {
                result.verified = result.verified.saturating_add(1);
            } else {
                result.corrupted.push(CorruptionInfo {
                    path,
                    expected_hash: expected,
                    actual_hash: actual,
                });
            }
        }
        Ok(result)
    }
}
