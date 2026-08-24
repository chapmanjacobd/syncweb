//! Durable transfer-job processing shared between the daemon and the
//! embedded CLI path (`transfer enqueue --now`).

use std::path::{Path, PathBuf};
use std::time::Duration;

use n0_future::StreamExt;

use crate::{
    allocation::{AllocationCandidate, StorageRoot, materialization_path},
    error::{Result, SyncwebError},
    folder::FolderManager,
    node::iroh_node::IrohNode,
    storage::node_db::{NodeDatabase, TransferJobRecord},
    sync::{FetchFilter, FetchStrategy, SyncEngine, SyncEvent},
};

const TRANSFER_TIMEOUT: Duration = Duration::from_mins(5);

/// Outcome counters from processing a batch of transfer jobs.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct TransferJobSummary {
    pub completed: u64,
    pub failed: u64,
}

impl TransferJobSummary {
    /// Create a summary from completion counters.
    #[must_use]
    pub const fn new(completed: u64, failed: u64) -> Self {
        Self { completed, failed }
    }
}

/// Fetch and materialize all queued transfer jobs, optionally scoped to a
/// single namespace.
///
/// # Errors
///
/// Returns an error if the queued jobs cannot be listed.
pub async fn process_transfer_jobs(
    node: &IrohNode,
    node_db: &NodeDatabase,
    namespace: Option<&str>,
) -> Result<TransferJobSummary> {
    let jobs = node_db.list_transfer_jobs(namespace, Some("queued"))?;
    let mut summary = TransferJobSummary::default();
    for job in jobs {
        match process_transfer_job(node, node_db, job).await {
            TransferJobOutcome::Completed => summary.completed = summary.completed.saturating_add(1),
            TransferJobOutcome::Failed => summary.failed = summary.failed.saturating_add(1),
            TransferJobOutcome::Skipped => {}
        }
    }
    Ok(summary)
}

#[derive(Clone, Copy)]
enum TransferJobOutcome {
    Completed,
    Failed,
    Skipped,
}

async fn process_transfer_job(node: &IrohNode, node_db: &NodeDatabase, job: TransferJobRecord) -> TransferJobOutcome {
    let Some(destination) = job.destination.as_ref() else {
        mark_transfer_job_failed(
            node_db,
            &job.id,
            &"job has no allocated destination",
            "missing transfer destination",
        );
        return TransferJobOutcome::Failed;
    };
    let hash = iroh_blobs::Hash::from(job.hash);
    if let Err(error) = validate_transfer_job(node_db, &job, destination, hash) {
        mark_transfer_job_failed(node_db, &job.id, &error, "transfer path validation error");
        return TransferJobOutcome::Failed;
    }

    let fetch_claimed = match node_db.transition_transfer_job_state(&job.id, "queued", "fetching", None) {
        Ok(claimed) => claimed,
        Err(error) => {
            tracing::error!(%error, job_id = %job.id, "failed to mark transfer job fetching");
            return TransferJobOutcome::Failed;
        }
    };
    if !fetch_claimed {
        return TransferJobOutcome::Skipped;
    }

    let has_blob = match node.blob_store().has(hash).await {
        Ok(has_blob) => has_blob,
        Err(error) => {
            mark_transfer_job_failed(node_db, &job.id, &error, "blob lookup error");
            return TransferJobOutcome::Failed;
        }
    };
    if !has_blob {
        let path = match String::from_utf8(job.entry_key.clone()) {
            Ok(path) => path,
            Err(error) => {
                mark_transfer_job_failed(
                    node_db,
                    &job.id,
                    &format!("job entry path is not UTF-8: {error}"),
                    "invalid transfer path",
                );
                return TransferJobOutcome::Failed;
            }
        };
        let namespace = match job.namespace_id.parse::<iroh_docs::NamespaceId>() {
            Ok(namespace) => namespace,
            Err(error) => {
                mark_transfer_job_failed(
                    node_db,
                    &job.id,
                    &format!("job has an invalid namespace: {error}"),
                    "invalid transfer namespace",
                );
                return TransferJobOutcome::Failed;
            }
        };
        if let Err(error) = fetch_transfer_blob(node, namespace, &path).await {
            mark_transfer_job_failed(node_db, &job.id, &error, "transfer fetch error");
            return TransferJobOutcome::Failed;
        }
    }

    let materialization_claimed =
        match node_db.transition_transfer_job_state(&job.id, "fetching", "materializing", None) {
            Ok(claimed) => claimed,
            Err(error) => {
                tracing::error!(%error, job_id = %job.id, "failed to mark transfer job materializing");
                return TransferJobOutcome::Failed;
            }
        };
    if !materialization_claimed {
        return TransferJobOutcome::Skipped;
    }

    match materialize_transfer(node, destination, hash).await {
        Ok(size) => complete_transfer_job(node_db, &job, size),
        Err(error) => {
            mark_transfer_job_failed(node_db, &job.id, &error, "transfer materialization error");
            TransferJobOutcome::Failed
        }
    }
}

/// Fetch a single blob identified by a folder path via the sync engine.
async fn fetch_transfer_blob(node: &IrohNode, namespace_id: iroh_docs::NamespaceId, path: &str) -> Result<()> {
    let sync = SyncEngine::new(
        FolderManager::new(node),
        node.blob_store().clone(),
        node.docs_engine().clone(),
        Some(node.topic_tracker().clone()),
    );
    let filter = FetchFilter::new().with_paths(vec![PathBuf::from(path)]);
    let mut intent = sync.fetch(namespace_id, FetchStrategy::filter(filter)).await?;
    let body = async {
        while let Some(event) = intent.next().await {
            match event {
                SyncEvent::Failed(message) => {
                    return Err(SyncwebError::operation("transfer fetch failed", message));
                }
                SyncEvent::Finished => return Ok(()),
                SyncEvent::Stats(_)
                | SyncEvent::Started
                | SyncEvent::Progress { .. }
                | SyncEvent::Paused
                | SyncEvent::Resumed
                | SyncEvent::Cancelled => {}
            }
        }
        Ok(())
    };
    match tokio::time::timeout(TRANSFER_TIMEOUT, body).await {
        Ok(result) => result,
        Err(_elapsed) => {
            let _ = intent.cancel();
            Ok(())
        }
    }
}

/// Validate that a queued job still points at its allocated root and path.
fn validate_transfer_job(
    node_db: &NodeDatabase,
    job: &TransferJobRecord,
    destination: &Path,
    hash: iroh_blobs::Hash,
) -> Result<()> {
    let root_id = job
        .root_id
        .as_deref()
        .ok_or_else(|| SyncwebError::InvalidConfig("job has no storage root".to_owned()))?;
    let root = node_db
        .list_storage_roots()?
        .into_iter()
        .find(|root| root.id == root_id)
        .ok_or_else(|| SyncwebError::InvalidConfig(format!("storage root not found: {root_id}")))?;
    let job_namespace = job
        .namespace_id
        .parse::<iroh_docs::NamespaceId>()
        .map_err(|error| SyncwebError::operation("job has invalid namespace", error))?;
    let path = String::from_utf8(job.entry_key.clone())
        .map_err(|error| SyncwebError::operation("job entry path is not UTF-8", error))?;
    let root_path = root.path.clone();
    let candidate = AllocationCandidate::new(
        job_namespace,
        PathBuf::from(path),
        hash,
        job.size,
        usize::try_from(job.peer_count).unwrap_or(usize::MAX),
        false,
    );
    let expected = materialization_path(
        &StorageRoot::new(root.id, &root_path, root.min_free).with_enabled(root.enabled),
        &candidate,
    )?;
    if expected != destination {
        return Err(SyncwebError::InvalidConfig(format!(
            "job destination does not match its allocated root: {}",
            destination.display()
        )));
    }
    reject_symlink_components(
        expected.strip_prefix(&root_path).unwrap_or_else(|_| Path::new("")),
        &root_path,
    )?;
    Ok(())
}

async fn materialize_transfer(node: &IrohNode, destination: &Path, hash: iroh_blobs::Hash) -> Result<u64> {
    if let Ok(metadata) = std::fs::symlink_metadata(destination) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(SyncwebError::InvalidConfig(format!(
                "materialization destination is not a regular file: {}",
                destination.display()
            )));
        }
        let existing = std::fs::read(destination)?;
        if blake3::hash(&existing).as_bytes() != hash.as_bytes() {
            return Err(SyncwebError::InvalidConfig(format!(
                "materialization destination has a different blob: {}",
                destination.display()
            )));
        }
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    node.blob_store().export_to_path(hash, destination).await?;
    let bytes = std::fs::read(destination)?;
    if blake3::hash(&bytes).as_bytes() != hash.as_bytes() {
        return Err(SyncwebError::operation(
            "materialized transfer hash does not match",
            destination.display(),
        ));
    }
    u64::try_from(bytes.len()).map_err(|error| SyncwebError::operation("materialized transfer size is invalid", error))
}

fn complete_transfer_job(node_db: &NodeDatabase, job: &TransferJobRecord, size: u64) -> TransferJobOutcome {
    if let Err(error) = node_db.update_transfer_job_progress(&job.id, size, job.peer_count, None, job.retries) {
        tracing::error!(%error, job_id = %job.id, "failed to record transfer completion progress");
        return TransferJobOutcome::Failed;
    }
    match node_db.transition_transfer_job_state(&job.id, "materializing", "completed", None) {
        Ok(true) => TransferJobOutcome::Completed,
        Ok(false) => TransferJobOutcome::Skipped,
        Err(error) => {
            tracing::error!(%error, job_id = %job.id, "failed to record transfer completion");
            TransferJobOutcome::Failed
        }
    }
}

fn mark_transfer_job_failed(node_db: &NodeDatabase, job_id: &str, message: &dyn std::fmt::Display, context: &str) {
    let rendered_message = message.to_string();
    if let Err(error) = node_db.update_transfer_job_state(job_id, "failed", Some(&rendered_message)) {
        tracing::error!(%error, %job_id, context, "failed to record transfer job failure");
    }
}

fn reject_symlink_components(relative: &Path, root: &Path) -> Result<()> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let metadata = match std::fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() {
            return Err(SyncwebError::InvalidConfig(format!(
                "materialization path contains a symlink: {}",
                current.display()
            )));
        }
    }
    Ok(())
}
