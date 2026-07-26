use std::collections::HashSet;

use iroh_blobs::Hash;
use iroh_docs::NamespaceId;

use crate::{Result, storage::node_db::NodeDatabase};

/// Manages sync session checkpointing for crash recovery.
///
/// Tracks which entries have been processed, failed, or skipped during a sync
/// session so that interrupted syncs can resume without re-downloading
/// already-transferred blobs.
#[derive(Clone, Debug)]
pub struct SyncCheckpoint {
    namespace_id: NamespaceId,
    session_id: String,
    database: NodeDatabase,
}

/// Progress for a single entry within a sync session.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EntryProgress {
    pub entry_key: Vec<u8>,
    pub hash: Hash,
    pub size: u64,
    pub status: String,
    pub retries: u32,
    pub error_message: Option<String>,
}

/// Aggregate progress for a sync session.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct CheckpointProgress {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub pending: usize,
    pub bytes_transferred: u64,
    pub bytes_total: Option<u64>,
    pub percentage: f64,
}

impl SyncCheckpoint {
    /// Create a new checkpoint manager for the given namespace.
    #[must_use]
    pub fn new(database: &NodeDatabase, namespace_id: NamespaceId) -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        Self {
            namespace_id,
            session_id,
            database: database.clone(),
        }
    }

    /// The current session ID.
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// The namespace ID for this checkpoint.
    #[must_use]
    pub const fn namespace_id(&self) -> NamespaceId {
        self.namespace_id
    }

    /// Create a new sync session in the database and return the checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn create_session(database: &NodeDatabase, namespace_id: NamespaceId) -> Result<Self> {
        let cp = Self::new(database, namespace_id);
        database.create_sync_checkpoint(&cp.namespace_id.to_string(), &cp.session_id)?;
        Ok(cp)
    }

    /// Mark an entry as completed (downloaded successfully).
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn mark_completed(&self, entry_key: &[u8], hash: Hash, size: u64) -> Result<()> {
        self.database
            .upsert_sync_entry(&crate::storage::node_db::SyncEntryParams {
                namespace_id: &self.namespace_id.to_string(),
                session_id: &self.session_id,
                entry_key,
                hash: &hash.to_string().into_bytes(),
                size,
                status: "completed",
                retries: 0,
                error_message: None,
            })
    }

    /// Mark an entry as failed with error message.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn mark_failed(&self, entry_key: &[u8], error: &str) -> Result<()> {
        self.database
            .upsert_sync_entry(&crate::storage::node_db::SyncEntryParams {
                namespace_id: &self.namespace_id.to_string(),
                session_id: &self.session_id,
                entry_key,
                hash: &[],
                size: 0,
                status: "failed",
                retries: 0,
                error_message: Some(error),
            })
    }

    /// Mark an entry as skipped (already present locally or deleted).
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn mark_skipped(&self, entry_key: &[u8], hash: Hash, size: u64) -> Result<()> {
        self.database
            .upsert_sync_entry(&crate::storage::node_db::SyncEntryParams {
                namespace_id: &self.namespace_id.to_string(),
                session_id: &self.session_id,
                entry_key,
                hash: &hash.to_string().into_bytes(),
                size,
                status: "skipped",
                retries: 0,
                error_message: None,
            })
    }

    /// Get all completed entries for this session (for resume filtering).
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn completed_entries(&self) -> Result<Vec<EntryProgress>> {
        self.database
            .list_sync_entries(&self.namespace_id.to_string(), &self.session_id, "completed")
    }

    /// Get all failed entries for this session (for retry).
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn failed_entries(&self) -> Result<Vec<EntryProgress>> {
        self.database
            .list_sync_entries(&self.namespace_id.to_string(), &self.session_id, "failed")
    }

    /// Get all completed entry keys as a `HashSet` for fast lookup.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn completed_keys(&self) -> Result<HashSet<Vec<u8>>> {
        let entries = self.completed_entries()?;
        Ok(entries.into_iter().map(|e| e.entry_key).collect())
    }

    /// Get overall progress.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn progress(&self) -> Result<CheckpointProgress> {
        self.database
            .get_checkpoint_progress(&self.namespace_id.to_string(), &self.session_id)
    }

    /// Mark session as completed.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn complete(&self) -> Result<()> {
        self.database
            .update_checkpoint_status(&self.namespace_id.to_string(), &self.session_id, "completed")
    }

    /// Mark session as incomplete (has leftovers for next resume).
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub const fn mark_incomplete(&self) -> Result<()> {
        // Leave the status as 'running' so it's picked up on resume
        Ok(())
    }

    /// Delete the checkpoint and its entries.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn delete(&self) -> Result<()> {
        self.database
            .delete_sync_checkpoint(&self.namespace_id.to_string(), &self.session_id)
    }

    /// Load the most recent unfinished checkpoint for a folder.
    ///
    /// Returns `None` if no unfinished checkpoint exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn resume(database: &NodeDatabase, namespace_id: NamespaceId) -> Result<Option<Self>> {
        let ns = namespace_id.to_string();
        Ok(database.find_unfinished_checkpoint(&ns)?.map(|sid| Self {
            namespace_id,
            session_id: sid,
            database: database.clone(),
        }))
    }
}
