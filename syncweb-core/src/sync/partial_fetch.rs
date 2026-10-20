use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    future::Future,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures_util::stream::{self, StreamExt as FuturesStreamExt};
use iroh::{Endpoint, EndpointAddr};
use iroh_blobs::Hash;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::error::Result;
use crate::folder::SyncwebFolder;
use crate::node::blob_store::BlobStore;
use crate::node::iroh_node::IrohNode;
use crate::sync::conflict::materialize_folder_content_for_folder;

const DEFAULT_TRANSFER_CONCURRENCY: usize = 4;
const DEFAULT_PROVIDER_CONCURRENCY: usize = 2;
const DEFAULT_TRANSFER_ATTEMPTS: usize = 3;
const DEFAULT_TRANSFER_BACKOFF: Duration = Duration::from_millis(100);
const DEFAULT_TRANSFER_MAX_BACKOFF: Duration = Duration::from_secs(2);

/// Configuration for bounded content transfer scheduling.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct TransferSchedulerConfig {
    /// Maximum number of blob requests that may be in flight at once.
    pub max_concurrent: usize,
    /// Maximum number of providers raced for one blob request.
    pub max_provider_concurrent: usize,
    /// Number of provider rounds attempted for each request.
    pub max_attempts: usize,
    /// Delay before the first retry round.
    pub backoff_base: Duration,
    /// Maximum delay between retry rounds.
    pub backoff_max: Duration,
}

impl Default for TransferSchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: DEFAULT_TRANSFER_CONCURRENCY,
            max_provider_concurrent: DEFAULT_PROVIDER_CONCURRENCY,
            max_attempts: DEFAULT_TRANSFER_ATTEMPTS,
            backoff_base: DEFAULT_TRANSFER_BACKOFF,
            backoff_max: DEFAULT_TRANSFER_MAX_BACKOFF,
        }
    }
}

impl TransferSchedulerConfig {
    fn validate(&self) -> Result<()> {
        if self.max_concurrent == 0 {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "transfer scheduler concurrency must be non-zero".to_owned(),
            ));
        }
        if self.max_provider_concurrent == 0 {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "transfer scheduler provider concurrency must be non-zero".to_owned(),
            ));
        }
        if self.max_attempts == 0 {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "transfer scheduler attempts must be non-zero".to_owned(),
            ));
        }
        if self.backoff_max < self.backoff_base {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "transfer scheduler maximum backoff must not be shorter than its base".to_owned(),
            ));
        }
        Ok(())
    }

    fn backoff_for_retry(&self, retry: usize) -> Duration {
        let multiplier = u32::try_from(retry.saturating_sub(1)).unwrap_or(u32::MAX);
        self.backoff_base
            .checked_mul(2_u32.saturating_pow(multiplier))
            .unwrap_or(self.backoff_max)
            .min(self.backoff_max)
    }
}

/// One independently scheduled content request.
///
/// The current Iroh compatibility path fetches complete blobs for segment
/// requests. Segment-aware providers can use the same request boundary later
/// without changing the scheduler's concurrency, retry, or cancellation
/// policy.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct TransferRequest {
    pub hash: Hash,
    pub size: u64,
    pub peers: Vec<EndpointAddr>,
    pub segment: Option<TransferSegment>,
}

impl TransferRequest {
    #[must_use]
    pub const fn new(hash: Hash, size: u64, peers: Vec<EndpointAddr>) -> Self {
        Self {
            hash,
            size,
            peers,
            segment: None,
        }
    }

    /// Construct a request for a bounded, independently addressable blob range.
    ///
    /// Segment requests use the whole-blob Iroh path until a peer advertises a
    /// range protocol. The scheduler still tracks and deduplicates the range
    /// independently, so transport support can be added without changing
    /// callers.
    ///
    /// # Errors
    ///
    /// Returns an error when the range is empty, overflows, or exceeds the
    /// known blob size.
    pub fn for_segment(hash: Hash, blob_size: u64, offset: u64, length: u64, peers: Vec<EndpointAddr>) -> Result<Self> {
        let segment = TransferSegment::new(hash, offset, length)?;
        if segment.end() > blob_size {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "transfer segment exceeds the blob size".to_owned(),
            ));
        }
        Ok(Self {
            hash,
            size: length,
            peers,
            segment: Some(segment),
        })
    }
}

/// A bounded range of a content-addressed blob.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct TransferSegment {
    pub hash: Hash,
    pub offset: u64,
    pub length: u64,
}

impl TransferSegment {
    /// Construct a non-empty range that does not overflow `u64`.
    ///
    /// # Errors
    ///
    /// Returns an error when `length` is zero or `offset + length` overflows.
    pub fn new(hash: Hash, offset: u64, length: u64) -> Result<Self> {
        if length == 0 {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "transfer segment length must be non-zero".to_owned(),
            ));
        }
        offset
            .checked_add(length)
            .ok_or_else(|| crate::error::SyncwebError::InvalidConfig("transfer segment overflows u64".to_owned()))?;
        Ok(Self { hash, offset, length })
    }

    #[must_use]
    pub const fn end(self) -> u64 {
        self.offset.saturating_add(self.length)
    }
}

/// Bytes returned for one independently fetched blob range.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SegmentChunk {
    pub segment: TransferSegment,
    pub bytes: Vec<u8>,
}

impl SegmentChunk {
    /// Construct a chunk after checking that its payload matches the range.
    ///
    /// # Errors
    ///
    /// Returns an error when the payload length does not equal the segment
    /// length.
    pub fn new(segment: TransferSegment, bytes: Vec<u8>) -> Result<Self> {
        let length = u64::try_from(bytes.len())
            .map_err(|error| crate::error::SyncwebError::operation("segment payload length is invalid", error))?;
        if length != segment.length {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "segment payload length does not match the requested range".to_owned(),
            ));
        }
        Ok(Self { segment, bytes })
    }
}

/// Bounded assembler for independently fetched content ranges.
///
/// Chunks may arrive in any order and can be supplied by different providers.
/// Overlapping chunks are rejected so a failed or corrupt provider cannot
/// silently overwrite bytes from another provider. [`finish`](Self::finish)
/// requires complete coverage and verifies the final content-addressed hash.
#[derive(Clone, Debug)]
pub struct SegmentAssembler {
    hash: Hash,
    size: u64,
    bytes: Vec<u8>,
    ranges: BTreeMap<u64, u64>,
    covered: u64,
}

impl SegmentAssembler {
    /// Create an assembler with an explicit maximum in-memory size.
    ///
    /// # Errors
    ///
    /// Returns an error when the blob size cannot fit in memory or exceeds
    /// `max_size`.
    pub fn new(hash: Hash, size: u64, max_size: u64) -> Result<Self> {
        if size > max_size {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "segment assembly exceeds the configured size limit".to_owned(),
            ));
        }
        let capacity = usize::try_from(size)
            .map_err(|error| crate::error::SyncwebError::operation("segment assembly size is invalid", error))?;
        Ok(Self {
            hash,
            size,
            bytes: vec![0; capacity],
            ranges: BTreeMap::new(),
            covered: 0,
        })
    }

    /// Add one chunk, preserving range ownership and rejecting overlap.
    ///
    /// # Errors
    ///
    /// Returns an error when the chunk has the wrong hash, is outside the
    /// assembled blob, overlaps an earlier chunk, or cannot be copied safely.
    pub fn add(&mut self, chunk: &SegmentChunk) -> Result<()> {
        if chunk.segment.hash != self.hash {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "segment belongs to a different blob".to_owned(),
            ));
        }
        let start = chunk.segment.offset;
        let end = chunk.segment.end();
        if end > self.size {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "segment exceeds the assembled blob size".to_owned(),
            ));
        }
        if self
            .ranges
            .range(..=start)
            .next_back()
            .is_some_and(|(_, previous_end)| *previous_end > start)
            || self
                .ranges
                .range(start..)
                .next()
                .is_some_and(|(next_start, _)| *next_start < end)
        {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "segment overlaps an earlier chunk".to_owned(),
            ));
        }
        let start_index = usize::try_from(start)
            .map_err(|error| crate::error::SyncwebError::operation("segment offset is invalid", error))?;
        let end_index = usize::try_from(end)
            .map_err(|error| crate::error::SyncwebError::operation("segment end is invalid", error))?;
        self.bytes
            .get_mut(start_index..end_index)
            .ok_or_else(|| crate::error::SyncwebError::InvalidConfig("segment range is not addressable".to_owned()))?
            .copy_from_slice(&chunk.bytes);
        self.ranges.insert(start, end);
        self.covered = self.covered.saturating_add(chunk.segment.length);
        Ok(())
    }

    /// Verify complete coverage and the final content-addressed hash.
    ///
    /// # Errors
    ///
    /// Returns an error when a range is missing, the assembled bytes do not
    /// hash to the requested blob, or the assembler was given inconsistent
    /// ranges.
    pub fn finish(self) -> Result<Vec<u8>> {
        if self.covered != self.size {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "segment assembly is incomplete".to_owned(),
            ));
        }
        let mut cursor = 0_u64;
        for (start, end) in self.ranges {
            if start != cursor {
                return Err(crate::error::SyncwebError::InvalidConfig(
                    "segment assembly contains a gap".to_owned(),
                ));
            }
            cursor = end;
        }
        if cursor != self.size || blake3::hash(&self.bytes).as_bytes() != self.hash.as_bytes() {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "assembled segments failed content hash verification".to_owned(),
            ));
        }
        Ok(self.bytes)
    }
}

/// Aggregate result from a bounded transfer run.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct TransferReport {
    pub requested: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped_duplicates: usize,
    pub cancelled: usize,
    pub bytes: u64,
}

/// Bounded, retrying scheduler for content requests.
#[derive(Clone, Debug)]
pub struct TransferScheduler {
    config: TransferSchedulerConfig,
}

impl TransferScheduler {
    /// Construct a scheduler with validated limits.
    ///
    /// # Errors
    ///
    /// Returns an error when a concurrency, retry, or backoff limit is invalid.
    pub fn new(config: TransferSchedulerConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Fetch requests using fresh provider ordering and bounded provider racing.
    ///
    /// Requests for the same hash are deduplicated before work starts. A
    /// cancelled run stops launching new work and cancels in-flight retries.
    ///
    /// # Errors
    ///
    /// Returns an error when scheduler configuration is invalid.
    pub async fn fetch(
        &self,
        node: &IrohNode,
        input_requests: Vec<TransferRequest>,
        cancellation: CancellationToken,
    ) -> Result<TransferReport> {
        let config = self.config.clone();
        self.fetch_with(input_requests, cancellation, move |request, request_cancellation| {
            let request_config = config.clone();
            async move { fetch_request(node, request, request_cancellation, &request_config).await }
        })
        .await
    }

    /// Fetch requests through a caller-provided transport while preserving the
    /// scheduler's deduplication, concurrency, retry, and cancellation rules.
    ///
    /// This is used by ticket-based package and snapshot transfers that do not
    /// own an [`IrohNode`] but still need the same bounded policy.
    ///
    /// # Errors
    ///
    /// Returns an error when scheduler configuration is invalid.
    pub async fn fetch_with<F, Fut>(
        &self,
        input_requests: Vec<TransferRequest>,
        cancellation: CancellationToken,
        fetch: F,
    ) -> Result<TransferReport>
    where
        F: Fn(TransferRequest, CancellationToken) -> Fut + Clone + Send + Sync,
        Fut: Future<Output = Result<u64>> + Send,
    {
        let (unique_requests, skipped_duplicates) = deduplicate_requests(input_requests);
        let report = self.run(unique_requests, cancellation, fetch).await?;
        Ok(TransferReport {
            skipped_duplicates,
            ..report
        })
    }

    async fn run<F, Fut>(
        &self,
        requests: Vec<TransferRequest>,
        cancellation: CancellationToken,
        fetch: F,
    ) -> Result<TransferReport>
    where
        F: Fn(TransferRequest, CancellationToken) -> Fut + Clone + Send + Sync,
        Fut: Future<Output = Result<u64>> + Send,
    {
        let requested = requests.len();
        let mut report = TransferReport {
            requested,
            ..TransferReport::default()
        };
        let mut results = stream::iter(requests.into_iter().map(|request| {
            let request_fetch = fetch.clone();
            let request_cancellation = cancellation.clone();
            async move { request_fetch(request, request_cancellation).await }
        }))
        .buffer_unordered(self.config.max_concurrent);

        loop {
            tokio::select! {
                () = cancellation.cancelled() => {
                    report.cancelled = requested
                        .saturating_sub(report.completed)
                        .saturating_sub(report.failed);
                    return Ok(report);
                }
                result = results.next() => {
                    let Some(next_result) = result else {
                        return Ok(report);
                    };
                    match next_result {
                        Ok(bytes) => {
                            report.completed = report.completed.saturating_add(1);
                            report.bytes = report.bytes.saturating_add(bytes);
                        }
                        Err(_) if cancellation.is_cancelled() => {
                            report.cancelled = requested
                                .saturating_sub(report.completed)
                                .saturating_sub(report.failed);
                            return Ok(report);
                        }
                        Err(_) => {
                            report.failed = report.failed.saturating_add(1);
                        }
                    }
                }
            }
        }
    }
}

fn deduplicate_requests(requests: Vec<TransferRequest>) -> (Vec<TransferRequest>, usize) {
    let mut seen = HashSet::with_capacity(requests.len());
    let mut unique = Vec::with_capacity(requests.len());
    let mut skipped_duplicates = 0_usize;
    for request in requests {
        let key = (
            request.hash,
            request.segment.map(|segment| (segment.offset, segment.length)),
        );
        if seen.insert(key) {
            unique.push(request);
        } else {
            skipped_duplicates = skipped_duplicates.saturating_add(1);
        }
    }
    (unique, skipped_duplicates)
}

async fn fetch_request(
    node: &IrohNode,
    request: TransferRequest,
    cancellation: CancellationToken,
    config: &TransferSchedulerConfig,
) -> Result<u64> {
    let mut last_error = None;
    for attempt in 0..config.max_attempts {
        if cancellation.is_cancelled() {
            return Err(crate::error::SyncwebError::operation(
                "content transfer cancelled",
                "scheduler cancellation requested",
            ));
        }
        match fetch_from_peers(
            &request.peers,
            &cancellation,
            config.max_provider_concurrent,
            |peer, _| async move {
                node.force_fetch_from_peer(&peer, request.hash)
                    .await
                    .map(|()| request.size)
            },
        )
        .await
        {
            Ok(bytes) => return Ok(bytes),
            Err(error) => last_error = Some(error),
        }
        let retry = attempt.saturating_add(1);
        if retry < config.max_attempts {
            tokio::select! {
                () = cancellation.cancelled() => {
                    return Err(crate::error::SyncwebError::operation(
                        "content transfer cancelled",
                        "scheduler cancellation requested",
                    ));
                }
                () = tokio::time::sleep(config.backoff_for_retry(retry)) => {}
            }
        }
    }
    Err(last_error.unwrap_or_else(|| {
        crate::error::SyncwebError::operation("content transfer failed", "no provider was available")
    }))
}

async fn fetch_from_peers<F, Fut>(
    peers: &[EndpointAddr],
    cancellation: &CancellationToken,
    max_concurrent: usize,
    fetch: F,
) -> Result<u64>
where
    F: Fn(EndpointAddr, CancellationToken) -> Fut + Clone + Send + Sync,
    Fut: Future<Output = Result<u64>> + Send,
{
    if peers.is_empty() {
        return Err(crate::error::SyncwebError::operation(
            "content transfer failed",
            "no provider was available",
        ));
    }
    let attempt_cancellation = cancellation.child_token();
    let mut results = stream::iter(peers.iter().cloned().map(|peer| {
        let fetcher = fetch.clone();
        let request_cancellation = attempt_cancellation.clone();
        async move {
            tokio::select! {
                () = request_cancellation.cancelled() => Err(crate::error::SyncwebError::operation(
                    "content transfer cancelled",
                    "provider race cancelled",
                )),
                result = fetcher(peer, request_cancellation.clone()) => result,
            }
        }
    }))
    .buffer_unordered(max_concurrent);
    let mut last_error = None;
    while let Some(result) = results.next().await {
        match result {
            Ok(bytes) => {
                attempt_cancellation.cancel();
                return Ok(bytes);
            }
            Err(error) => last_error = Some(error),
        }
    }
    if cancellation.is_cancelled() {
        return Err(crate::error::SyncwebError::operation(
            "content transfer cancelled",
            "scheduler cancellation requested",
        ));
    }
    Err(last_error.unwrap_or_else(|| {
        crate::error::SyncwebError::operation("content transfer failed", "no provider was available")
    }))
}

/// Fetch content requests through blob tickets using the common scheduler.
///
/// Each request may contain multiple provider addresses. Eligible providers
/// are raced with bounded concurrency for each retry round, and the Iroh blob
/// path verifies the complete content-addressed blob before the request is
/// reported as successful.
///
/// # Errors
///
/// Returns an error when scheduler configuration is invalid.
pub async fn fetch_blob_requests(
    blobs: &BlobStore,
    endpoint: &Endpoint,
    requests: Vec<TransferRequest>,
    cancellation: CancellationToken,
) -> Result<TransferReport> {
    let scheduler = TransferScheduler::new(TransferSchedulerConfig::default())?;
    let shared_blobs = blobs.clone();
    let shared_endpoint = endpoint.clone();
    scheduler
        .fetch_with(requests, cancellation, move |request, request_cancellation| {
            let request_blobs = shared_blobs.clone();
            let request_endpoint = shared_endpoint.clone();
            async move {
                let mut last_error = None;
                for attempt in 0..DEFAULT_TRANSFER_ATTEMPTS {
                    let attempt_result = fetch_from_peers(
                        &request.peers,
                        &request_cancellation,
                        DEFAULT_PROVIDER_CONCURRENCY,
                        |peer, _| {
                            let peer_blobs = request_blobs.clone();
                            let peer_endpoint = request_endpoint.clone();
                            async move {
                                let ticket = peer_blobs.ticket_for_addr(peer, request.hash);
                                peer_blobs.fetch(&peer_endpoint, &ticket).await.map(|()| request.size)
                            }
                        },
                    )
                    .await;
                    match attempt_result {
                        Ok(bytes) => return Ok(bytes),
                        Err(error) => {
                            if request_cancellation.is_cancelled() {
                                return Err(error);
                            }
                            last_error = Some(error);
                        }
                    }
                    let retry = attempt.saturating_add(1);
                    if retry < DEFAULT_TRANSFER_ATTEMPTS {
                        tokio::select! {
                            () = request_cancellation.cancelled() => {
                                return Err(crate::error::SyncwebError::operation(
                                    "content transfer cancelled",
                                    "scheduler cancellation requested",
                                ));
                            }
                            () = tokio::time::sleep(TransferSchedulerConfig::default().backoff_for_retry(retry)) => {}
                        }
                    }
                }
                Err(last_error.unwrap_or_else(|| {
                    crate::error::SyncwebError::operation("content transfer failed", "all providers failed")
                }))
            }
        })
        .await
}

/// Fetch the blobs selected by `strategy` directly from the folder's namespace
/// peers.
///
/// A metadata-only folder syncs entries while deliberately leaving blobs out of
/// the local store; a later doc re-sync does not re-download that content, so
/// `download` on such a folder must fetch each selected blob from its peers
/// directly. Returns the number of bytes fetched.
///
/// # Errors
///
/// Returns an error if the folder's entries cannot be listed.
pub async fn fetch_selected_content(node: &IrohNode, folder: &SyncwebFolder, strategy: &FetchStrategy) -> Result<u64> {
    fetch_selected_content_with_cancel(node, folder, strategy, CancellationToken::new()).await
}

/// Fetch selected folder content with caller-controlled cancellation.
///
/// # Errors
///
/// Returns an error if the folder's entries cannot be listed or the scheduler
/// cannot be configured.
pub async fn fetch_selected_content_with_cancel(
    node: &IrohNode,
    folder: &SyncwebFolder,
    strategy: &FetchStrategy,
    cancellation: CancellationToken,
) -> Result<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    if let Err(error) = node.refresh_provider_leases(folder.namespace_id(), now, None).await {
        tracing::debug!(
            namespace = %folder.namespace_id(),
            %error,
            "provider lease refresh unavailable; continuing with discovered peers"
        );
    }
    let mut candidates = Vec::new();
    for entry in folder.content_entries().await? {
        let hash = entry.content_hash();
        let local = folder.has_local(hash).await?;
        let path = PathBuf::from(String::from_utf8_lossy(entry.key()).into_owned());
        let known_peers = node.provider_peer_snapshot(&hash)?;
        candidates.push(FetchCandidate::with_peer_state(
            path,
            hash,
            entry.content_len(),
            known_peers.map(|peers| peers.len()),
            local,
        ));
    }
    let mut selected = strategy.select(&candidates);
    // Complete short blobs first so a large transfer does not delay the first
    // useful results of a mixed-size download.
    selected.sort_by(|left, right| {
        left.size
            .cmp(&right.size)
            .then(left.path.cmp(&right.path))
            .then(left.hash.cmp(&right.hash))
    });
    let discovered_peers = node
        .topic_tracker()
        .find_peers(folder.namespace_id())
        .await
        .unwrap_or_default();
    let mut requests = Vec::new();
    for candidate in selected {
        if candidate.local {
            continue;
        }
        let scheduled = node.schedule_provider_peers(&candidate.hash, &discovered_peers)?;
        requests.push(TransferRequest::new(candidate.hash, candidate.size, scheduled));
    }
    let scheduler = TransferScheduler::new(TransferSchedulerConfig::default())?;
    Ok(scheduler.fetch(node, requests, cancellation).await?.bytes)
}

/// Write the blobs selected by `strategy` to files under `mount_root`, using
/// the folder-relative entry paths. Returns the number of files written.
///
/// This turns a `download` from a store-only fetch into files on disk.
///
/// # Errors
///
/// Returns an error if `mount_root` cannot be resolved against the current
/// directory, an entry directory cannot be created, or a blob cannot be
/// exported to its destination.
pub async fn materialize_selected_content(
    node: &IrohNode,
    folder: &SyncwebFolder,
    strategy: &FetchStrategy,
    mount_root: &Path,
) -> Result<usize> {
    let mut candidates = Vec::new();
    for entry in folder.content_entries().await? {
        let hash = entry.content_hash();
        let local = folder.has_local(hash).await?;
        let path = PathBuf::from(String::from_utf8_lossy(entry.key()).into_owned());
        let known_peers = node.provider_peer_snapshot(&hash)?;
        candidates.push(FetchCandidate::with_peer_state(
            path,
            hash,
            entry.content_len(),
            known_peers.map(|peers| peers.len()),
            local,
        ));
    }
    let selected = strategy.select(&candidates);
    let selected_paths: BTreeSet<_> = selected.into_iter().map(|candidate| candidate.path).collect();
    let root = absolute_base(mount_root)?;
    let outcome = materialize_folder_content_for_folder(node, folder, &root, Some(&selected_paths)).await?;
    Ok(outcome.summary.materialized.saturating_add(outcome.summary.conflicts))
}

/// Resolve `base` to an absolute path without touching the filesystem.
///
/// Blob export rejects relative destinations, so a folder registered with a
/// relative mount path (the CLI default is a relative `--data-dir`) still needs
/// an absolute root to materialize into.
fn absolute_base(base: &Path) -> Result<PathBuf> {
    if base.is_absolute() {
        return Ok(base.to_path_buf());
    }
    let cwd = std::env::current_dir()
        .map_err(|error| crate::error::SyncwebError::operation("failed to resolve current directory", error))?;
    Ok(cwd.join(base))
}

/// Whether everything selected by `strategy` is already present locally.
///
/// A download of already-local content does not need a doc re-sync: fetching
/// blobs the local node already holds can fail on the publishing node (there is
/// no remote peer to fetch from), so the caller can skip straight to
/// materialization. Returns `false` when the strategy selects nothing or when
/// any selected blob is not yet local.
///
/// # Errors
///
/// Returns an error if the folder's entries cannot be listed.
pub async fn strategy_selects_only_local(folder: &SyncwebFolder, strategy: &FetchStrategy) -> Result<bool> {
    let mut candidates = Vec::new();
    for entry in folder.content_entries().await? {
        let hash = entry.content_hash();
        let local = folder.has_local(hash).await?;
        let path = PathBuf::from(String::from_utf8_lossy(entry.key()).into_owned());
        candidates.push(FetchCandidate::new(path, hash, entry.content_len(), 0, local));
    }
    let selected = strategy.select(&candidates);
    Ok(!selected.is_empty() && selected.iter().all(|candidate| candidate.local))
}

/// A document blob considered for a partial fetch.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct FetchCandidate {
    pub path: PathBuf,
    pub hash: Hash,
    pub size: u64,
    pub peer_count: usize,
    /// Whether `peer_count` comes from a fresh provider observation.
    pub peer_count_known: bool,
    pub local: bool,
}

impl FetchCandidate {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, hash: Hash, size: u64, peer_count: usize, local: bool) -> Self {
        Self::with_peer_state(path, hash, size, Some(peer_count), local)
    }

    #[must_use]
    pub fn with_peer_state(
        path: impl Into<PathBuf>,
        hash: Hash,
        size: u64,
        peer_count: Option<usize>,
        local: bool,
    ) -> Self {
        Self {
            path: path.into(),
            hash,
            size,
            peer_count: peer_count.unwrap_or(0),
            peer_count_known: peer_count.is_some(),
            local,
        }
    }
}

/// Constraints for selecting a subset of folder content.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FetchFilter {
    pub paths: Option<Vec<PathBuf>>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub min_peers: Option<usize>,
    pub max_peers: Option<usize>,
    pub min_count: Option<usize>,
    pub max_count: Option<usize>,
}

impl FetchFilter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            paths: None,
            min_size: None,
            max_size: None,
            min_peers: None,
            max_peers: None,
            min_count: None,
            max_count: None,
        }
    }

    #[must_use]
    pub fn with_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.paths = Some(paths);
        self
    }

    #[must_use]
    pub const fn with_min_size(mut self, size: u64) -> Self {
        self.min_size = Some(size);
        self
    }

    #[must_use]
    pub const fn with_max_size(mut self, size: u64) -> Self {
        self.max_size = Some(size);
        self
    }

    #[must_use]
    pub const fn with_min_peers(mut self, peers: usize) -> Self {
        self.min_peers = Some(peers);
        self
    }

    #[must_use]
    pub const fn with_max_peers(mut self, peers: usize) -> Self {
        self.max_peers = Some(peers);
        self
    }

    #[must_use]
    pub const fn with_min_count(mut self, count: usize) -> Self {
        self.min_count = Some(count);
        self
    }

    #[must_use]
    pub const fn with_max_count(mut self, count: usize) -> Self {
        self.max_count = Some(count);
        self
    }

    /// Return candidates satisfying this filter.
    ///
    /// Candidates with fewer peers are selected first so a capped fetch
    /// improves folder seeding rather than repeatedly selecting common blobs.
    #[must_use]
    pub fn select(&self, candidates: &[FetchCandidate]) -> Vec<FetchCandidate> {
        let mut selected = candidates
            .iter()
            .filter(|candidate| self.matches(candidate))
            .cloned()
            .collect::<Vec<_>>();
        selected.sort_by(|left, right| {
            left.peer_count
                .cmp(&right.peer_count)
                .then(left.path.cmp(&right.path))
                .then(left.hash.cmp(&right.hash))
        });
        if let Some(max_count) = self.max_count {
            selected.truncate(max_count);
        }
        selected
    }

    #[must_use]
    pub fn matches(&self, candidate: &FetchCandidate) -> bool {
        self.paths.as_ref().is_none_or(|paths| {
            paths
                .iter()
                .any(|path| candidate.path == *path || candidate.path.starts_with(path))
        }) && self.min_size.is_none_or(|size| candidate.size >= size)
            && self.max_size.is_none_or(|size| candidate.size <= size)
            && self
                .min_peers
                .is_none_or(|peers| candidate.peer_count_known && candidate.peer_count >= peers)
            && self
                .max_peers
                .is_none_or(|peers| !candidate.peer_count_known || candidate.peer_count <= peers)
    }
}

/// Controls whether a fetch reconciles all content or only matching content.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum FetchStrategy {
    #[default]
    All,
    Filter(FetchFilter),
}

impl FetchStrategy {
    #[must_use]
    pub const fn filter(filter: FetchFilter) -> Self {
        Self::Filter(filter)
    }

    #[must_use]
    pub fn select(&self, candidates: &[FetchCandidate]) -> Vec<FetchCandidate> {
        match self {
            Self::All => candidates.to_vec(),
            Self::Filter(filter) => filter.select(candidates),
        }
    }
}

/// Per-blob availability information used by the health command.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BlobHealth {
    pub path: PathBuf,
    pub hash: Hash,
    pub size: u64,
    pub peer_count: usize,
    pub peer_count_known: bool,
    pub local: bool,
}

/// Aggregate availability counters for a folder.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct HealthReport {
    pub total: usize,
    pub well_seeded: usize,
    pub under_seeded: usize,
    pub unseeded: usize,
    /// Blobs whose fresh provider count is unavailable.
    pub unknown: usize,
    pub least_seeded: Vec<BlobHealth>,
}

impl HealthReport {
    /// Build a report using `well_seeded_threshold` as the minimum healthy peer count.
    ///
    /// Unknown provider counts are tracked separately rather than treated as
    /// unseeded content.
    #[must_use]
    pub fn from_candidates(candidates: &[FetchCandidate], well_seeded_threshold: usize) -> Self {
        let mut least_seeded = candidates
            .iter()
            .map(|candidate| BlobHealth {
                path: candidate.path.clone(),
                hash: candidate.hash,
                size: candidate.size,
                peer_count: candidate.peer_count,
                peer_count_known: candidate.peer_count_known,
                local: candidate.local,
            })
            .collect::<Vec<_>>();
        least_seeded.sort_by(|left, right| left.peer_count.cmp(&right.peer_count).then(left.path.cmp(&right.path)));
        let total = candidates.len();
        let well_seeded = candidates
            .iter()
            .filter(|candidate| candidate.peer_count_known && candidate.peer_count >= well_seeded_threshold)
            .count();
        let unseeded = candidates
            .iter()
            .filter(|candidate| candidate.peer_count_known && candidate.peer_count == 0)
            .count();
        let unknown = candidates
            .iter()
            .filter(|candidate| !candidate.peer_count_known)
            .count();
        let under_seeded = candidates
            .iter()
            .filter(|candidate| {
                candidate.peer_count_known && candidate.peer_count > 0 && candidate.peer_count < well_seeded_threshold
            })
            .count();
        Self {
            total,
            well_seeded,
            under_seeded,
            unseeded,
            unknown,
            least_seeded,
        }
    }

    /// Build a report with verified peer counts from a `hash -> verified_count` map.
    ///
    /// Each candidate's `peer_count` is overridden with the value from
    /// `peers_per_hash` when present. This lets the health display reflect live
    /// provider-lease data instead of always showing zero.
    #[must_use]
    pub fn from_candidates_with_peers_per_hash(
        candidates: &[FetchCandidate],
        peers_per_hash: &HashMap<Hash, usize>,
        well_seeded_threshold: usize,
    ) -> Self {
        let enriched: Vec<FetchCandidate> = candidates
            .iter()
            .map(|c| {
                let peers = peers_per_hash.get(&c.hash).copied();
                FetchCandidate::with_peer_state(
                    &c.path,
                    c.hash,
                    c.size,
                    peers.or_else(|| c.peer_count_known.then_some(c.peer_count)),
                    c.local,
                )
            })
            .collect();
        Self::from_candidates(&enriched, well_seeded_threshold)
    }
}

#[cfg(test)]
mod scheduler_tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use super::*;

    fn request(seed: u8, size: u64) -> TransferRequest {
        TransferRequest::new(Hash::from_bytes([seed; 32]), size, Vec::new())
    }

    #[test]
    fn segment_requests_validate_bounds_and_deduplicate_by_range() {
        let hash = Hash::from_bytes([1; 32]);
        assert!(TransferRequest::for_segment(hash, 100, 10, 0, Vec::new()).is_err());
        assert!(TransferRequest::for_segment(hash, 100, 90, 11, Vec::new()).is_err());
        assert!(TransferSegment::new(hash, u64::MAX, 2).is_err());

        let first = TransferRequest::for_segment(hash, 100, 0, 10, Vec::new()).expect("valid segment");
        let second = TransferRequest::for_segment(hash, 100, 10, 10, Vec::new()).expect("valid segment");
        let duplicate = TransferRequest::for_segment(hash, 100, 0, 10, Vec::new()).expect("valid segment");
        let (unique, skipped) = deduplicate_requests(vec![first, second, duplicate]);
        assert_eq!(unique.len(), 2);
        assert_eq!(skipped, 1);
    }

    #[test]
    fn segment_assembler_accepts_disjoint_provider_chunks_and_verifies_hash() {
        let payload = b"multi-provider content".to_vec();
        let hash = Hash::from_bytes(*blake3::hash(&payload).as_bytes());
        let payload_size = u64::try_from(payload.len()).expect("payload length fits u64");
        let mut assembler =
            SegmentAssembler::new(hash, payload_size, payload_size).expect("assembler should be created");
        let first = TransferSegment::new(hash, 0, 6).expect("first range");
        let second_length = u64::try_from(payload.len().saturating_sub(6)).expect("payload length fits u64");
        let second = TransferSegment::new(hash, 6, second_length).expect("second range");
        assembler
            .add(
                &SegmentChunk::new(second, payload.get(6..).expect("second range exists").to_vec())
                    .expect("second chunk"),
            )
            .expect("second chunk should be accepted");
        assembler
            .add(
                &SegmentChunk::new(first, payload.get(..6).expect("first range exists").to_vec()).expect("first chunk"),
            )
            .expect("first chunk should be accepted");
        assert_eq!(assembler.finish().expect("assembled payload"), payload);
    }

    #[test]
    fn segment_assembler_rejects_overlap_gaps_and_corrupt_hashes() {
        let payload = b"0123456789".to_vec();
        let hash = Hash::from_bytes(*blake3::hash(&payload).as_bytes());
        let payload_size = u64::try_from(payload.len()).expect("payload length fits u64");
        let mut assembler = SegmentAssembler::new(hash, payload_size, 100).expect("assembler");
        let first = TransferSegment::new(hash, 0, 5).expect("first range");
        let overlap = TransferSegment::new(hash, 4, 6).expect("overlap range");
        assembler
            .add(
                &SegmentChunk::new(first, payload.get(..5).expect("first range exists").to_vec()).expect("first chunk"),
            )
            .expect("first chunk should be accepted");
        assert!(
            assembler
                .add(
                    &SegmentChunk::new(overlap, payload.get(4..).expect("overlap range exists").to_vec())
                        .expect("overlap chunk")
                )
                .is_err()
        );
        assert!(assembler.finish().is_err(), "the missing tail must be rejected");

        let wrong_hash = Hash::from_bytes([9; 32]);
        let mut corrupt = SegmentAssembler::new(wrong_hash, payload_size, 100).expect("assembler");
        let full = TransferSegment::new(wrong_hash, 0, payload_size).expect("full range");
        corrupt
            .add(&SegmentChunk::new(full, payload).expect("full chunk"))
            .expect("full chunk should be accepted");
        assert!(corrupt.finish().is_err(), "content hash mismatch must be rejected");
    }

    #[test]
    fn scheduler_rejects_invalid_limits() {
        assert!(
            TransferScheduler::new(TransferSchedulerConfig {
                max_concurrent: 0,
                ..TransferSchedulerConfig::default()
            })
            .is_err()
        );
        assert!(
            TransferScheduler::new(TransferSchedulerConfig {
                max_provider_concurrent: 0,
                ..TransferSchedulerConfig::default()
            })
            .is_err()
        );
        assert!(
            TransferScheduler::new(TransferSchedulerConfig {
                max_attempts: 0,
                ..TransferSchedulerConfig::default()
            })
            .is_err()
        );
        assert!(
            TransferScheduler::new(TransferSchedulerConfig {
                backoff_base: Duration::from_secs(2),
                backoff_max: Duration::from_secs(1),
                ..TransferSchedulerConfig::default()
            })
            .is_err()
        );
    }

    #[tokio::test]
    async fn scheduler_deduplicates_and_bounds_concurrency() {
        let scheduler = TransferScheduler::new(TransferSchedulerConfig {
            max_concurrent: 2,
            max_attempts: 1,
            ..TransferSchedulerConfig::default()
        })
        .expect("valid scheduler configuration");
        let active_counter = Arc::new(AtomicUsize::new(0));
        let peak_counter = Arc::new(AtomicUsize::new(0));
        let (requests, skipped_duplicates) =
            deduplicate_requests(vec![request(1, 10), request(1, 10), request(2, 20), request(3, 30)]);
        let report = scheduler
            .run(requests, CancellationToken::new(), {
                let active_for_scheduler = Arc::clone(&active_counter);
                let peak_for_scheduler = Arc::clone(&peak_counter);
                move |request, _| {
                    let active_for_request = Arc::clone(&active_for_scheduler);
                    let peak_for_request = Arc::clone(&peak_for_scheduler);
                    async move {
                        let current = active_for_request.fetch_add(1, Ordering::SeqCst).saturating_add(1);
                        peak_for_request.fetch_max(current, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(5)).await;
                        active_for_request.fetch_sub(1, Ordering::SeqCst);
                        Ok(request.size)
                    }
                }
            })
            .await
            .expect("scheduler should finish");
        assert_eq!(report.requested, 3);
        assert_eq!(skipped_duplicates, 1);
        assert_eq!(report.completed, 3);
        assert_eq!(report.bytes, 60);
        assert!(peak_counter.load(Ordering::SeqCst) <= 2);
        assert!(peak_counter.load(Ordering::SeqCst) > 1);
    }

    #[tokio::test]
    async fn scheduler_reports_cancellation() {
        let scheduler = TransferScheduler::new(TransferSchedulerConfig {
            max_concurrent: 1,
            max_attempts: 1,
            ..TransferSchedulerConfig::default()
        })
        .expect("valid scheduler configuration");
        let cancellation = CancellationToken::new();
        let trigger = cancellation.clone();
        let trigger_task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            trigger.cancel();
        });
        let report = scheduler
            .run(
                vec![request(1, 10), request(2, 20)],
                cancellation,
                |_, token| async move {
                    tokio::select! {
                        () = token.cancelled() => Err(crate::error::SyncwebError::operation(
                            "content transfer cancelled",
                            "test cancellation",
                        )),
                        () = tokio::time::sleep(Duration::from_secs(1)) => Ok(1),
                    }
                },
            )
            .await
            .expect("scheduler should report cancellation");
        trigger_task.await.expect("cancellation task should finish");
        assert!(report.cancelled > 0);
        assert_eq!(report.completed, 0);
    }

    #[tokio::test]
    async fn provider_race_is_bounded_and_uses_the_first_success() {
        let peers = vec![
            EndpointAddr::new(iroh::SecretKey::generate().public()),
            EndpointAddr::new(iroh::SecretKey::generate().public()),
            EndpointAddr::new(iroh::SecretKey::generate().public()),
        ];
        let successful_peer = peers.get(1).expect("three peers should be available").id;
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let result = fetch_from_peers(&peers, &CancellationToken::new(), 2, {
            let callback_active = Arc::clone(&active);
            let callback_peak = Arc::clone(&peak);
            move |peer, _| {
                let attempt_active = Arc::clone(&callback_active);
                let attempt_peak = Arc::clone(&callback_peak);
                async move {
                    let current = attempt_active.fetch_add(1, Ordering::SeqCst).saturating_add(1);
                    attempt_peak.fetch_max(current, Ordering::SeqCst);
                    if peer.id == successful_peer {
                        tokio::time::sleep(Duration::from_millis(5)).await;
                        attempt_active.fetch_sub(1, Ordering::SeqCst);
                        Ok(7)
                    } else {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        attempt_active.fetch_sub(1, Ordering::SeqCst);
                        Err(crate::error::SyncwebError::operation("provider failed", "test failure"))
                    }
                }
            }
        })
        .await
        .expect("one provider should succeed");
        assert_eq!(result, 7);
        assert!(peak.load(Ordering::SeqCst) <= 2);
    }
}

#[cfg(test)]
mod absolute_base_tests {
    use super::*;

    #[test]
    fn absolute_paths_are_returned_unchanged() {
        let base = if cfg!(windows) {
            std::env::current_dir().unwrap().join("srv/folder")
        } else {
            PathBuf::from("/srv/folder")
        };
        assert_eq!(absolute_base(&base).unwrap(), base);
    }

    #[test]
    fn relative_paths_are_anchored_to_the_current_directory() {
        let base = Path::new("relative/folder");
        let resolved = absolute_base(base).unwrap();
        assert!(
            resolved.is_absolute(),
            "blob export rejects relative roots: {resolved:?}"
        );
        assert!(resolved.ends_with(base), "{resolved:?} should end with {base:?}");
    }

    #[test]
    fn dot_resolves_to_the_current_directory() {
        let resolved = absolute_base(Path::new(".")).unwrap();
        assert!(resolved.is_absolute());
    }
}
