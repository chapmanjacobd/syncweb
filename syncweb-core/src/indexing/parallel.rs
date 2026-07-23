use iroh::{PublicKey, endpoint::Endpoint};
use iroh_blobs::{
    Hash,
    get::fsm::{self, ConnectedNext, EndBlobNext, RequestCounters},
    protocol::{ChunkRanges, ChunkRangesExt, ChunkRangesSeq, GetRequest},
    ticket::BlobTicket,
    ALPN,
};
use tokio::task::JoinSet;
use tracing::{debug, info, warn};

use crate::error::{Result, SyncwebError};
use crate::node::blob_store::BlobStore;

const BLAKE3_CHUNK_SIZE: u64 = 1024;

/// Configuration for parallel multi-provider blob download.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParallelDownloadConfig {
    /// Blobs below this size are downloaded sequentially.
    pub min_blob_size: u64,
    /// Maximum parallel connections downloading a single blob.
    pub max_concurrent_connections: usize,
    /// Minimum chunks per range to avoid tiny protocol requests.
    pub min_chunks_per_range: u64,
}

impl Default for ParallelDownloadConfig {
    fn default() -> Self {
        Self {
            min_blob_size: 16 * 1024 * 1024,
            max_concurrent_connections: 5,
            min_chunks_per_range: 64,
        }
    }
}

/// A single chunk-range assignment for one provider.
#[derive(Clone, Debug)]
struct Assignment {
    #[allow(dead_code)]
    provider: PublicKey,
    ticket: BlobTicket,
    range: ChunkRanges,
}

/// Result of a parallel download attempt.
#[derive(Debug)]
#[non_exhaustive]
pub enum TryParallelResult {
    /// Blob is too small or has too few providers — caller should fall back.
    Inapplicable,
    /// Successfully downloaded and imported into the blob store.
    Downloaded(Vec<PublicKey>),
    /// All providers failed — caller may proceed to sequential fallback.
    AllFailed,
}

/// Probe the full blob size from a single provider using the last-chunk trick.
async fn probe_size(
    endpoint: &Endpoint,
    hash: Hash,
    ticket: &BlobTicket,
) -> std::result::Result<u64, SyncwebError> {
    let connection = endpoint
        .connect(ticket.addr().clone(), ALPN)
        .await
        .map_err(|e| SyncwebError::operation("parallel: size probe connect failed", e))?;

    let request = GetRequest::new(
        hash,
        ChunkRangesSeq::from_ranges([ChunkRanges::last_chunk()]),
    );
    let start = fsm::start(connection, request, RequestCounters::default());
    let connected = start
        .next()
        .await
        .map_err(|e| SyncwebError::operation("parallel: size probe negotiation failed", e))?;
    let ConnectedNext::StartRoot(root) = connected
        .next()
        .await
        .map_err(|e| SyncwebError::operation("parallel: size probe request failed", e))?
    else {
        return Err(SyncwebError::operation(
            "parallel: size probe unexpected response",
            "provider did not return a blob",
        ));
    };
    let header = root.next();
    let (_content, size) = header
        .next()
        .await
        .map_err(|e| SyncwebError::operation("parallel: size probe read failed", e))?;
    Ok(size)
}

/// Split a blob into chunk ranges across providers.
///
/// Each range is a non-overlapping interval of chunk indices.  The first
/// `remainder` providers get one extra chunk so the split is as even as
/// possible without leaving gaps.
#[allow(
    clippy::integer_division,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::bool_to_int_with_if
)]
fn assign_chunks(
    providers: &[(PublicKey, BlobTicket)],
    total_chunks: u64,
    max_ranges: usize,
    min_chunks: u64,
) -> Vec<Assignment> {
    if total_chunks == 0 || max_ranges == 0 {
        return Vec::new();
    }
    let actual = max_ranges.min(providers.len());
    if actual == 0 {
        return Vec::new();
    }
    let actual_u64 = u64::try_from(actual).unwrap_or(u64::MAX);
    let base = total_chunks / actual_u64;
    if base == 0 {
        return Vec::new();
    }
    let remainder = total_chunks % actual_u64;
    let mut out: Vec<Assignment> = Vec::with_capacity(actual);
    let mut cursor: u64 = 0;

    for (i, &(provider, ref ticket)) in providers.iter().take(actual).enumerate() {
        let extra: u64 = if u64::try_from(i).unwrap_or(u64::MAX) < remainder { 1 } else { 0 };
        let size = base + extra;
        let start = cursor;
        let end = cursor + size;
        cursor = end;

        if size < min_chunks && !out.is_empty() {
            if let Some(last) = out.last_mut() {
                last.range = last.range.clone() | ChunkRanges::chunks(start..end);
            }
            continue;
        }

        out.push(Assignment {
            provider,
            ticket: ticket.clone(),
            range: ChunkRanges::chunks(start..end),
        });
    }

    if cursor < total_chunks && let Some(last) = out.last_mut() {
        last.range = last.range.clone() | ChunkRanges::chunks(cursor..total_chunks);
    }

    out
}

/// Download one chunk range from a provider and write it into `writer`.
async fn download_range(
    endpoint: &Endpoint,
    hash: Hash,
    ticket: &BlobTicket,
    range: ChunkRanges,
    mut writer: iroh_io::File,
) -> std::result::Result<PublicKey, (PublicKey, SyncwebError)> {
    let provider = ticket.addr().id;
    let connection = endpoint
        .connect(ticket.addr().clone(), ALPN)
        .await
        .map_err(|e| (provider, SyncwebError::operation("parallel: connect failed", e)))?;

    let request = GetRequest::blob_ranges(hash, range);
    let start = fsm::start(connection, request, RequestCounters::default());
    let connected = start
        .next()
        .await
        .map_err(|e| (provider, SyncwebError::operation("parallel: negotiation failed", e)))?;
    let ConnectedNext::StartRoot(root) = connected
        .next()
        .await
        .map_err(|e| (provider, SyncwebError::operation("parallel: request failed", e)))?
    else {
        return Err((
            provider,
            SyncwebError::operation(
                "parallel: unexpected response",
                "provider did not return a blob",
            ),
        ));
    };
    let header = root.next();
    let end = header
        .write_all(&mut writer)
        .await
        .map_err(|e| (provider, SyncwebError::operation("parallel: write failed", e)))?;
    let EndBlobNext::Closing(closing) = end.next() else {
        return Err((
            provider,
            SyncwebError::operation(
                "parallel: unexpected end of stream",
                "expected closing",
            ),
        ));
    };
    closing
        .next()
        .await
        .map_err(|e| (provider, SyncwebError::operation("parallel: close failed", e)))?;

    Ok(provider)
}

/// Try to download a blob in parallel from multiple providers.
///
/// This function:
/// 1. Probes the blob size from the best provider.
/// 2. If the blob is large enough and enough providers exist, splits the
///    blob into chunk ranges and spawns one connection per range.
/// 3. Each connection writes its range to a shared sparse temporary file.
/// 4. On success the file is imported into the blob store and removed.
///
/// # Errors
///
/// Returns an error if the temporary file cannot be created or written, or
/// if the assembled blob does not match the expected hash.
#[allow(clippy::cognitive_complexity)]
pub async fn try_fetch_parallel(
    endpoint: &Endpoint,
    blobs: &BlobStore,
    hash: Hash,
    providers: &[(PublicKey, BlobTicket)],
    config: &ParallelDownloadConfig,
) -> Result<TryParallelResult> {
    if providers.len() < 2 {
        return Ok(TryParallelResult::Inapplicable);
    }

    // Probe size from the best-ranked provider.
    let size_result = match providers.first() {
        Some((_, ticket)) => probe_size(endpoint, hash, ticket).await,
        None => return Ok(TryParallelResult::Inapplicable),
    };
    let total_size = match size_result {
        Ok(s) => s,
        Err(e) => {
            debug!(%hash, error = %e, "parallel: size probe failed, falling back");
            return Ok(TryParallelResult::Inapplicable);
        }
    };
    if total_size < config.min_blob_size {
        return Ok(TryParallelResult::Inapplicable);
    }

    let total_chunks = total_size.div_ceil(BLAKE3_CHUNK_SIZE);
    let n_ranges = providers.len().min(config.max_concurrent_connections);

    let assignments =
        assign_chunks(providers, total_chunks, n_ranges, config.min_chunks_per_range);
    if assignments.is_empty() {
        return Ok(TryParallelResult::Inapplicable);
    }

    // Create a temporary sparse file.
    let temp_dir = std::env::temp_dir().join("syncweb-parallel");
    tokio::fs::create_dir_all(&temp_dir).await?;
    let temp_path = temp_dir.join(format!("{hash}.tmp"));

    let file = std::fs::File::create_new(&temp_path)
        .map_err(|e| SyncwebError::operation("parallel: failed to create temp file", e))?;
    file.set_len(total_size)
        .map_err(|e| SyncwebError::operation("parallel: failed to set temp file length", e))?;

    // Spawn one tokio task per chunk range.
    let mut join_set: JoinSet<std::result::Result<PublicKey, (PublicKey, SyncwebError)>> =
        JoinSet::new();

    for asgn in &assignments {
        let f_clone = file
            .try_clone()
            .map_err(|e| SyncwebError::operation("parallel: failed to clone file handle", e))?;
        let writer = iroh_io::File::from_std(f_clone);
        let ep = endpoint.clone();
        let ticket = asgn.ticket.clone();
        let range = asgn.range.clone();
        join_set.spawn(async move { download_range(&ep, hash, &ticket, range, writer).await });
    }

    // The main handle is no longer needed — the tasks have their own clones.
    drop(file);

    // Collect results.
    let mut successes: Vec<PublicKey> = Vec::new();
    let mut all_failed = true;

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(provider)) => {
                successes.push(provider);
                all_failed = false;
            }
            Ok(Err((provider, error))) => {
                warn!(%provider, %hash, error = %error, "parallel: chunk download failed");
            }
            Err(e) => {
                warn!(%hash, error = %e, "parallel: download task panicked");
            }
        }
    }

    if all_failed {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Ok(TryParallelResult::AllFailed);
    }

    // Sync data to disk.
    {
        let f = std::fs::File::open(&temp_path)
            .map_err(|e| SyncwebError::operation("parallel: failed to open temp file for sync", e))?;
        f.sync_all()
            .map_err(|e| SyncwebError::operation("parallel: failed to sync temp file", e))?;
    }

    // Import into the blob store (copy mode).  The store computes the hash,
    // so we verify the assembly was correct.
    info!(%hash, path = %temp_path.display(), "parallel: importing assembled blob");
    let actual_hash = blobs.add_file(&temp_path).await?;
    if actual_hash != hash {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(SyncwebError::operation(
            "parallel: assembled blob hash mismatch",
            format!("expected {hash}, got {actual_hash}"),
        ));
    }

    // Clean up temp file.
    tokio::fs::remove_file(&temp_path).await?;

    info!(
        %hash,
        successes = successes.len(),
        all_succeeded = successes.len() == assignments.len(),
        "parallel: download complete"
    );
    Ok(TryParallelResult::Downloaded(successes))
}
