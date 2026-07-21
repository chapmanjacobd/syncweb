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
}
