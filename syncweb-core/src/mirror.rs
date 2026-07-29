use iroh::Endpoint;
use iroh::PublicKey;
use iroh_blobs::Hash;
use iroh_docs::NamespaceId;

use crate::error::Result;
use crate::indexing::resilience::ResilienceService;
use crate::node::blob_store::BlobStore;
use crate::node::docs_engine::DocsEngine;

/// Progress event emitted during a mirror operation.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum MirrorEvent {
    /// All blobs to mirror have been discovered.
    Discovered { total: usize },
    /// Starting to fetch a specific blob.
    Fetching { hash: Hash, index: usize, total: usize },
    /// A blob was successfully fetched and pinned.
    Pinned { hash: Hash },
    /// A blob was skipped (already pinned locally).
    Skipped { hash: Hash },
    /// A blob failed to fetch from all providers.
    Failed { hash: Hash, error: String },
}

/// Result of a mirror operation.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct MirrorResult {
    pub total_blobs: usize,
    pub pinned: usize,
    pub skipped: usize,
    pub failed: usize,
    pub dry_run: bool,
}

/// Options for a mirror operation.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct MirrorOptions {
    pub dry_run: bool,
    pub no_sharing: bool,
    pub min_providers: usize,
}

impl MirrorOptions {
    #[must_use]
    pub const fn new(min_providers: usize) -> Self {
        Self {
            dry_run: false,
            no_sharing: false,
            min_providers,
        }
    }

    /// Enable dry-run mode (report without fetching).
    #[must_use]
    pub const fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Enable no-sharing mode (skip lease announcements).
    #[must_use]
    pub const fn with_no_sharing(mut self, no_sharing: bool) -> Self {
        self.no_sharing = no_sharing;
        self
    }
}

impl Default for MirrorOptions {
    fn default() -> Self {
        Self::new(3)
    }
}

/// Context for a mirror operation, grouping arguments to stay under the arg limit.
#[non_exhaustive]
pub struct MirrorContext<'a> {
    pub endpoint: &'a Endpoint,
    pub blobs: &'a BlobStore,
    pub service: &'a ResilienceService,
    pub docs_engine: Option<&'a DocsEngine>,
    pub namespace_ids: Option<&'a [NamespaceId]>,
    pub provider: Option<PublicKey>,
    pub options: &'a MirrorOptions,
    pub progress: Option<tokio::sync::mpsc::UnboundedSender<MirrorEvent>>,
}

impl<'a> MirrorContext<'a> {
    #[must_use]
    pub const fn new(
        endpoint: &'a Endpoint,
        blobs: &'a BlobStore,
        service: &'a ResilienceService,
        docs: Option<(&'a DocsEngine, &'a [NamespaceId])>,
        provider: Option<PublicKey>,
        options: &'a MirrorOptions,
        progress: Option<tokio::sync::mpsc::UnboundedSender<MirrorEvent>>,
    ) -> Self {
        let (docs_engine, namespace_ids) = match docs {
            Some((engine, ids)) => (Some(engine), Some(ids)),
            None => (None, None),
        };
        Self {
            endpoint,
            blobs,
            service,
            docs_engine,
            namespace_ids,
            provider,
            options,
            progress,
        }
    }
}
/// Discover all blob hashes advertised by a given provider.
///
/// # Errors
///
/// Returns an error if the service state lock is poisoned.
pub fn discover_provider_blobs(service: &ResilienceService, provider: &PublicKey) -> Result<Vec<Hash>> {
    service.blobs_for_provider(provider)
}

/// Mirror all blobs from a single provider.
///
/// # Errors
///
/// Returns an error if the node cannot be opened or blob fetching fails.
pub async fn mirror_provider(
    endpoint: &Endpoint,
    blobs: &BlobStore,
    service: &ResilienceService,
    provider: &PublicKey,
    options: &MirrorOptions,
    progress: Option<tokio::sync::mpsc::UnboundedSender<MirrorEvent>>,
) -> Result<MirrorResult> {
    let hashes = discover_provider_blobs(service, provider)?;
    mirror_hashes(endpoint, blobs, service, &hashes, options, progress).await
}

/// Mirror all blobs referenced within a set of namespaces.
///
/// If `provider` is `Some`, only blobs that the given provider has an active
/// lease for are mirrored (provider-scoped within the network).
///
/// # Errors
///
/// Returns an error if the service state lock is poisoned, namespaces cannot
/// be opened, or blob fetching fails.
pub async fn mirror_network(ctx: MirrorContext<'_>) -> Result<MirrorResult> {
    let provider_filter = ctx.provider.as_ref().map(|prov| {
        ctx.service
            .blobs_for_provider(prov)
            .unwrap_or_default()
            .into_iter()
            .collect::<std::collections::HashSet<Hash>>()
    });

    let mut all_hashes: Vec<Hash> = Vec::new();
    let docs_engine = ctx.docs_engine.ok_or_else(|| {
        crate::error::SyncwebError::InvalidConfig("docs_engine is required for network mirror".to_owned())
    })?;
    let ns_ids = ctx.namespace_ids.unwrap_or_default();
    for ns in ns_ids {
        if let Some(doc) = docs_engine.open(*ns).await? {
            let entries = docs_engine.list_latest(&doc).await?;
            for entry in entries {
                let hash = entry.content_hash();
                if provider_filter.as_ref().is_none_or(|filter| filter.contains(&hash)) {
                    all_hashes.push(hash);
                }
            }
        }
    }
    all_hashes.sort_by_key(std::string::ToString::to_string);
    all_hashes.dedup();
    mirror_hashes(
        ctx.endpoint,
        ctx.blobs,
        ctx.service,
        &all_hashes,
        ctx.options,
        ctx.progress,
    )
    .await
}

/// Internal: mirror a list of hashes.
async fn mirror_hashes(
    endpoint: &Endpoint,
    blobs: &BlobStore,
    service: &ResilienceService,
    hashes: &[Hash],
    options: &MirrorOptions,
    progress: Option<tokio::sync::mpsc::UnboundedSender<MirrorEvent>>,
) -> Result<MirrorResult> {
    let total = hashes.len();
    if let Some(ref sender) = progress {
        let _ = sender.send(MirrorEvent::Discovered { total });
    }

    if options.dry_run {
        return Ok(MirrorResult {
            total_blobs: total,
            pinned: 0,
            skipped: 0,
            failed: 0,
            dry_run: true,
        });
    }

    let mut pinned = 0_usize;
    let mut skipped = 0_usize;
    let mut failed = 0_usize;

    for (index, hash) in hashes.iter().enumerate() {
        if let Some(ref sender) = progress {
            let _ = sender.send(MirrorEvent::Fetching {
                hash: *hash,
                index,
                total,
            });
        }

        if blobs.has(*hash).await? {
            skipped = skipped.saturating_add(1);
            if let Some(ref sender) = progress {
                let _ = sender.send(MirrorEvent::Skipped { hash: *hash });
            }
            continue;
        }

        match service.ensure_replication(endpoint, blobs, *hash).await {
            Ok(result) if result.pinned => {
                pinned = pinned.saturating_add(1);
                if let Some(ref sender) = progress {
                    let _ = sender.send(MirrorEvent::Pinned { hash: *hash });
                }
            }
            Ok(result) if result.fetched_from.is_empty() && !result.pinned => {
                failed = failed.saturating_add(1);
                if let Some(ref sender) = progress {
                    let _ = sender.send(MirrorEvent::Failed {
                        hash: *hash,
                        error: "no providers available".to_owned(),
                    });
                }
            }
            Ok(_) => {
                failed = failed.saturating_add(1);
                if let Some(ref sender) = progress {
                    let _ = sender.send(MirrorEvent::Failed {
                        hash: *hash,
                        error: "blob was fetched but not pinned".to_owned(),
                    });
                }
            }
            Err(error) => {
                failed = failed.saturating_add(1);
                if let Some(ref sender) = progress {
                    let _ = sender.send(MirrorEvent::Failed {
                        hash: *hash,
                        error: error.to_string(),
                    });
                }
            }
        }
    }

    Ok(MirrorResult {
        total_blobs: total,
        pinned,
        skipped,
        failed,
        dry_run: false,
    })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use iroh::{EndpointAddr, SecretKey};
    use iroh_blobs::{BlobFormat, Hash, ticket::BlobTicket};

    use crate::indexing::resilience::{ProviderLease, ReplicationBudget, ResilienceConfig, ResilienceService};

    fn signed_lease_for_test(seed: u8, hash: Hash, sequence: u64) -> Result<(ProviderLease, SecretKey)> {
        let secret_key = SecretKey::from_bytes(&[seed; 32]);
        let ticket = BlobTicket::new(EndpointAddr::new(secret_key.public()), hash, BlobFormat::Raw).to_string();
        let lease = ProviderLease::signed(hash, ticket, sequence, u64::MAX, &secret_key)?;
        Ok((lease, secret_key))
    }

    fn make_service_with_leases(lease_count: usize) -> Result<(ResilienceService, Vec<iroh::PublicKey>)> {
        let service = ResilienceService::new(ResilienceConfig::new(ReplicationBudget::new(1)));
        let mut providers = Vec::new();
        for i in 0..lease_count {
            let hash = Hash::new(format!("blob-{i}").as_bytes());
            let seed: u8 = i
                .try_into()
                .map_err(|e: std::num::TryFromIntError| anyhow::anyhow!("seed overflow: {e}"))?;
            let (lease, sk) = signed_lease_for_test(seed, hash, 1)?;
            service.record_lease(&lease)?;
            providers.push(sk.public());
        }
        Ok((service, providers))
    }

    #[test]
    fn discover_provider_blobs_returns_all_hashes_for_given_provider() -> Result<()> {
        let (service, providers) = make_service_with_leases(3)?;
        let first = providers.first().ok_or_else(|| anyhow::anyhow!("no providers"))?;
        let hashes = super::discover_provider_blobs(&service, first)?;
        anyhow::ensure!(hashes.len() == 1, "provider 0 should have 1 blob");
        let first_hash = hashes.first().ok_or_else(|| anyhow::anyhow!("no hashes"))?;
        anyhow::ensure!(*first_hash == Hash::new(b"blob-0"));
        Ok(())
    }

    #[test]
    fn discover_provider_blobs_returns_empty_for_unknown_provider() -> Result<()> {
        let (service, _providers) = make_service_with_leases(2)?;
        let unknown = SecretKey::from_bytes(&[99; 32]).public();
        let hashes = super::discover_provider_blobs(&service, &unknown)?;
        anyhow::ensure!(hashes.is_empty(), "unknown provider should have no blobs");
        Ok(())
    }

    #[test]
    fn mirror_options_defaults() {
        let opts = super::MirrorOptions::default();
        assert!(!opts.dry_run);
        assert!(!opts.no_sharing);
        assert_eq!(opts.min_providers, 3);
    }

    #[test]
    fn mirror_result_json_round_trip() -> Result<()> {
        let result = super::MirrorResult {
            total_blobs: 10,
            pinned: 7,
            skipped: 2,
            failed: 1,
            dry_run: false,
        };
        let json = serde_json::to_string(&result)?;
        let deserialized: super::MirrorResult = serde_json::from_str(&json)?;
        anyhow::ensure!(deserialized.total_blobs == 10);
        anyhow::ensure!(deserialized.pinned == 7);
        anyhow::ensure!(deserialized.skipped == 2);
        anyhow::ensure!(deserialized.failed == 1);
        anyhow::ensure!(!deserialized.dry_run);
        Ok(())
    }
}
