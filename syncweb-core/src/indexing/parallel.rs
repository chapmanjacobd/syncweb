use std::time::{Duration, Instant};

use iroh::{PublicKey, endpoint::Endpoint};
use iroh_blobs::{
    ALPN, Hash,
    get::fsm::{self, ConnectedNext, EndBlobNext, RequestCounters},
    protocol::{ChunkRanges, ChunkRangesExt, ChunkRangesSeq, GetRequest},
    ticket::BlobTicket,
};
use tokio::task::JoinSet;
use tracing::{debug, info, warn};

use crate::error::{Result, SyncwebError};
use crate::node::blob_store::BlobStore;

/// Default estimated rate: 125 bytes/ms ≈ 1 Mbps.
const DEFAULT_RATE_BYTES_PER_MS: u64 = 125;
/// Timeout multiplier for phase 1 (8× estimated completion).
const TIMEOUT_FACTOR: f64 = 8.0;
/// Tighter multiplier for retry phase.
const RETRY_TIMEOUT_FACTOR: f64 = 4.0;

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

/// A provider assigned to download one or more stripe ranges.
#[derive(Clone, Debug)]
struct Assignment {
    provider: PublicKey,
    ticket: BlobTicket,
    ranges: ChunkRanges,
    estimated_bytes: u64,
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

/// Outcome of a single download task.
struct TaskOutcome {
    provider: PublicKey,
    bytes_written: u64,
    elapsed: Duration,
}

/// Probe the full blob size from a single provider using the last-chunk trick.
async fn probe_size(endpoint: &Endpoint, hash: Hash, ticket: &BlobTicket) -> std::result::Result<u64, SyncwebError> {
    let connection = endpoint
        .connect(ticket.addr().clone(), ALPN)
        .await
        .map_err(|e| SyncwebError::operation("parallel: size probe connect failed", e))?;

    let request = GetRequest::new(hash, ChunkRangesSeq::from_ranges([ChunkRanges::last_chunk()]));
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

fn stripe_score(provider: &PublicKey, stripe_index: u64) -> u64 {
    let mut data = [0_u8; 40];
    data[..32].copy_from_slice(provider.as_bytes());
    data[32..40].copy_from_slice(&stripe_index.to_be_bytes());
    let hash = blake3::hash(&data);
    let first_8: [u8; 8] = hash.as_bytes()[..8].try_into().unwrap_or([0_u8; 8]);
    u64::from_be_bytes(first_8)
}

/// Assign stripes to providers using deterministic scoring.
///
/// Each stripe is 1024 chunks (1 MiB).  The provider with the highest
/// `hash(provider, stripe_index)` is chosen for each stripe.
fn assign_chunks_striped(
    providers: &[(PublicKey, BlobTicket)],
    total_chunks: u64,
    n_providers: usize,
    total_size: u64,
) -> Vec<Assignment> {
    let n = n_providers.min(providers.len());
    if total_chunks == 0 || n == 0 {
        return Vec::new();
    }

    let n_u64 = u64::try_from(n).unwrap_or(u64::MAX);

    let mut provider_data: Vec<(PublicKey, BlobTicket, ChunkRanges)> = providers
        .iter()
        .take(n)
        .map(|(pk, ticket)| (*pk, ticket.clone(), ChunkRanges::empty()))
        .collect();

    let n_stripes = total_chunks.div_ceil(1024);
    for s in 0..n_stripes {
        let start = s.saturating_mul(1024);
        if start >= total_chunks {
            break;
        }
        let end = start.saturating_add(1024).min(total_chunks);
        let stripe_range = ChunkRanges::chunks(start..end);

        let best_idx = provider_data
            .iter()
            .enumerate()
            .max_by_key(|(_, (pk, _, _))| stripe_score(pk, s))
            .map_or(0, |(i, _)| i);
        if let Some((_, _, target)) = provider_data.get_mut(best_idx) {
            *target = target.clone() | stripe_range;
        }
    }

    let bytes_per_provider = total_size.checked_div(n_u64).unwrap_or(0);
    let remainder = total_size.checked_rem(n_u64).unwrap_or(0);

    provider_data
        .into_iter()
        .filter(|(_, _, ranges)| !ranges.is_empty())
        .enumerate()
        .map(|(i, (pk, ticket, ranges))| {
            let i_u64 = u64::try_from(i).unwrap_or(u64::MAX);
            let extra = u64::from(u8::from(i_u64 < remainder));
            Assignment {
                provider: pk,
                ticket,
                ranges,
                estimated_bytes: bytes_per_provider.saturating_add(extra),
            }
        })
        .collect()
}

/// Download all stripe ranges for one provider and write to a file.
async fn download_assignment(
    endpoint: &Endpoint,
    hash: Hash,
    ticket: &BlobTicket,
    ranges: ChunkRanges,
    writer: iroh_io::File,
    deadline: Duration,
) -> std::result::Result<TaskOutcome, (PublicKey, SyncwebError)> {
    let provider = ticket.addr().id;
    let t0 = Instant::now();

    let connection = endpoint
        .connect(ticket.addr().clone(), ALPN)
        .await
        .map_err(|e| (provider, SyncwebError::operation("parallel: connect failed", e)))?;

    let request = GetRequest::blob_ranges(hash, ranges);
    let start = fsm::start(connection, request, RequestCounters::default());

    let result = tokio::time::timeout(deadline, async {
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
                SyncwebError::operation("parallel: unexpected response", "provider did not return a blob"),
            ));
        };
        let header = root.next();
        let (content, _size) = header
            .next()
            .await
            .map_err(|e| (provider, SyncwebError::operation("parallel: header read failed", e)))?;
        let end = content
            .write_all(writer)
            .await
            .map_err(|e| (provider, SyncwebError::operation("parallel: write failed", e)))?;
        let EndBlobNext::Closing(closing) = end.next() else {
            return Err((
                provider,
                SyncwebError::operation("parallel: unexpected end of stream", "expected closing"),
            ));
        };
        let stats = closing
            .next()
            .await
            .map_err(|e| (provider, SyncwebError::operation("parallel: close failed", e)))?;
        let elapsed = t0.elapsed();
        Ok(TaskOutcome {
            provider,
            bytes_written: stats.payload_bytes_read,
            elapsed,
        })
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_elapsed) => Err((
            provider,
            SyncwebError::operation("parallel: timeout", "download exceeded deadline"),
        )),
    }
}

/// Run a phase of parallel downloads.
async fn run_phase(
    assignments: &[Assignment],
    endpoint: &Endpoint,
    hash: Hash,
    file_handle: &std::fs::File,
    rate_bytes_per_ms: u64,
    timeout_factor: f64,
) -> (Vec<TaskOutcome>, Vec<(PublicKey, SyncwebError)>) {
    let mut join_set: JoinSet<std::result::Result<TaskOutcome, (PublicKey, SyncwebError)>> = JoinSet::new();

    for asgn in assignments {
        let f_clone = match file_handle.try_clone() {
            Ok(f) => f,
            Err(e) => {
                warn!(%hash, error = %e, "parallel: failed to clone file handle");
                continue;
            }
        };
        let writer = iroh_io::File::from_std(f_clone);
        let ep = endpoint.clone();
        let ticket = asgn.ticket.clone();
        let ranges = asgn.ranges.clone();

        let rate = rate_bytes_per_ms.max(1);
        let estimated_ms = asgn.estimated_bytes.checked_div(rate).unwrap_or(u64::MAX);
        let estimated = Duration::from_millis(estimated_ms);
        let deadline = estimated.mul_f64(timeout_factor);

        join_set.spawn(async move { download_assignment(&ep, hash, &ticket, ranges, writer, deadline).await });
    }

    let mut successes = Vec::new();
    let mut failures = Vec::new();

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(outcome)) => {
                successes.push(outcome);
            }
            Ok(Err((provider, error))) => {
                warn!(%provider, %hash, error = %error, "parallel: download failed");
                failures.push((provider, error));
            }
            Err(e) => {
                warn!(%hash, error = %e, "parallel: task panicked");
            }
        }
    }

    (successes, failures)
}

/// Compute observed rate in bytes/ms from successful outcomes.
fn compute_rate_bytes_per_ms(successes: &[TaskOutcome]) -> u64 {
    let total_ms: u128 = successes.iter().map(|o| o.elapsed.as_millis()).sum();
    let total_bytes: u64 = successes.iter().map(|o| o.bytes_written).sum();
    if total_ms == 0 {
        return DEFAULT_RATE_BYTES_PER_MS;
    }
    let ms = u64::try_from(total_ms.min(u128::from(u64::MAX))).unwrap_or(u64::MAX);
    total_bytes.checked_div(ms).unwrap_or(1).max(1)
}

/// Collect stripe ranges that need retry from failed assignments.
fn collect_unclaimed(assignments: &[Assignment], successes: &[TaskOutcome]) -> ChunkRanges {
    let successful_providers: Vec<PublicKey> = successes.iter().map(|o| o.provider).collect();
    let mut unclaimed = ChunkRanges::empty();
    for asgn in assignments {
        if !successful_providers.contains(&asgn.provider) {
            unclaimed |= asgn.ranges.clone();
        }
    }
    unclaimed
}

/// Run phase 2 retry for unclaimed stripe ranges.
async fn run_retry_phase(
    assignments: &[Assignment],
    phase1_ok: &[TaskOutcome],
    providers: &[(PublicKey, BlobTicket)],
    temp_path: &std::path::Path,
    download_size: u64,
    hash: Hash,
    endpoint: &Endpoint,
) -> Result<()> {
    if phase1_ok.is_empty() {
        return Ok(());
    }
    let unclaimed = collect_unclaimed(assignments, phase1_ok);
    if unclaimed.is_empty() {
        return Ok(());
    }
    let survivors: Vec<(PublicKey, BlobTicket)> = providers
        .iter()
        .filter(|(p, _)| phase1_ok.iter().any(|o| o.provider == *p))
        .cloned()
        .collect();

    if survivors.len() < 2 {
        return Ok(());
    }

    info!(
        %hash,
        n_survivors = survivors.len(),
        "parallel: retrying unclaimed stripes"
    );

    let retry_file = std::fs::File::open(temp_path)
        .map_err(|e| SyncwebError::operation("parallel: failed to reopen temp file", e))?;

    let unclaimed_chunks = estimate_chunks_in_ranges(&unclaimed, download_size.div_ceil(1024));
    let retry_assignments = assign_chunks_striped(&survivors, unclaimed_chunks, survivors.len(), download_size);

    let rate = compute_rate_bytes_per_ms(phase1_ok);
    let (_retry_ok, _) = run_phase(
        &retry_assignments,
        endpoint,
        hash,
        &retry_file,
        rate,
        RETRY_TIMEOUT_FACTOR,
    )
    .await;

    drop(retry_file);
    Ok(())
}

/// Import the assembled blob and verify its hash.
async fn import_assembled_blob(blobs: &BlobStore, temp_path: &std::path::Path, hash: Hash) -> Result<()> {
    {
        let f = std::fs::File::open(temp_path)
            .map_err(|e| SyncwebError::operation("parallel: failed to open temp file for sync", e))?;
        f.sync_all()
            .map_err(|e| SyncwebError::operation("parallel: failed to sync temp file", e))?;
    }

    info!(%hash, path = %temp_path.display(), "parallel: importing assembled blob");
    let actual_hash = blobs.add_file(temp_path).await?;
    if actual_hash != hash {
        let _ = tokio::fs::remove_file(temp_path).await;
        return Err(SyncwebError::operation(
            "parallel: assembled blob hash mismatch",
            format!("expected {hash}, got {actual_hash}"),
        ));
    }
    tokio::fs::remove_file(temp_path).await?;
    Ok(())
}

/// Try to download a blob in parallel from multiple providers.
///
/// # Phase 1 (stripe interleaving)
/// Chunks are divided into 1 MiB stripes assigned via deterministic scoring.
/// Each provider gets a deadline of 8× estimated completion based on observed
/// download rate.
///
/// # Phase 2 (retry)
/// Stripes that failed in phase 1 are reassigned to surviving providers with
/// a tighter 4× deadline.
///
/// # Errors
///
/// Returns an error if the temporary file cannot be created, or if the
/// assembled blob does not match the expected hash.
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

    let total_size = match providers.first() {
        Some((_, ticket)) => match probe_size(endpoint, hash, ticket).await {
            Ok(s) => s,
            Err(e) => {
                debug!(%hash, error = %e, "parallel: size probe failed, falling back");
                return Ok(TryParallelResult::Inapplicable);
            }
        },
        None => return Ok(TryParallelResult::Inapplicable),
    };
    if total_size < config.min_blob_size {
        return Ok(TryParallelResult::Inapplicable);
    }

    let total_chunks = total_size.div_ceil(1024);
    let n_providers = providers.len().min(config.max_concurrent_connections);

    let assignments = assign_chunks_striped(providers, total_chunks, n_providers, total_size);
    if assignments.is_empty() {
        return Ok(TryParallelResult::Inapplicable);
    }

    let temp_dir = std::env::temp_dir().join("syncweb-parallel");
    tokio::fs::create_dir_all(&temp_dir).await?;
    let temp_path = temp_dir.join(format!("{hash}.tmp"));

    let file = std::fs::File::create_new(&temp_path)
        .map_err(|e| SyncwebError::operation("parallel: failed to create temp file", e))?;
    file.set_len(total_size)
        .map_err(|e| SyncwebError::operation("parallel: failed to set temp file length", e))?;

    // Phase 1: download all stripes
    let (phase1_ok, phase1_err) = run_phase(
        &assignments,
        endpoint,
        hash,
        &file,
        DEFAULT_RATE_BYTES_PER_MS,
        TIMEOUT_FACTOR,
    )
    .await;

    drop(file);

    // Phase 2: retry unclaimed ranges with surviving providers
    run_retry_phase(
        &assignments,
        &phase1_ok,
        providers,
        &temp_path,
        total_size,
        hash,
        endpoint,
    )
    .await?;

    let all_successful: Vec<PublicKey> = phase1_ok.iter().map(|o| o.provider).collect();
    if all_successful.is_empty() {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Ok(TryParallelResult::AllFailed);
    }

    import_assembled_blob(blobs, &temp_path, hash).await?;

    info!(
        %hash,
        successes = all_successful.len(),
        failures = phase1_err.len(),
        "parallel: download complete"
    );
    Ok(TryParallelResult::Downloaded(all_successful))
}

/// Count chunks covered by a finite `ChunkRanges` set.
fn estimate_chunks_in_ranges(ranges: &ChunkRanges, max_chunks: u64) -> u64 {
    let bounds = ranges.boundaries();
    let mut total = 1_u64;
    let mut i = 0_usize;
    while let Some(start) = bounds.get(i) {
        let next = i.saturating_add(1);
        let Some(end_ref) = bounds.get(next) else {
            break;
        };
        let end = end_ref.0.min(max_chunks);
        if let Some(count) = end.checked_sub(start.0) {
            total = total.saturating_add(count);
        }
        i = i.saturating_add(2);
    }
    total
}
