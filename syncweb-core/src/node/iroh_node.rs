use iroh::address_lookup::memory::MemoryLookup;
use iroh::protocol::Router;
use iroh_blobs::BlobsProtocol;
use iroh_blobs::{Hash, api::blobs::BlobStatus, ticket::BlobTicket};
use iroh_docs::protocol::Docs;
use iroh_gossip::net::Gossip;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::error::{Result, SyncwebError};
use crate::net::RelayManager;
use crate::provider_lease::ProviderLease;
use crate::storage::node_db::NodeDatabase;
use crate::sync::{
    EvictionStrategy, ObservationSource, PeerTracker, SegmentChunk, TransferRequest, TransferScheduler,
    TransferSchedulerConfig, TransferSegment,
};

use super::beacon_lookup::{BeaconAddressLookup, DEFAULT_BEACON_PORT};
use super::blob_store::BlobStore;
use super::discovery::{DhtDiscoveryConfig, TopicTracker};
use super::docs_engine::DocsEngine;
use super::identity::IdentityManager;
use super::membership_hook::MembershipHook;

#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum RelayMode {
    Default,
    Custom { map: iroh::RelayMap, insecure: bool },
    None,
}

/// Runtime configuration for the local discovery mechanisms attached to a node.
///
/// `scope` is a 16-byte token derived from the daemon's network name. When
/// present it scopes both mDNS and the UDP beacon so that unrelated networks
/// do not discover each other; `None` keeps the default iroh mDNS service name
/// and a scope-less beacon (used by the network-agnostic default daemon).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DiscoveryConfig {
    /// Whether the mDNS address lookup is registered.
    pub mdns: bool,
    /// Whether the UDP beacon address lookup is registered.
    pub beacon: bool,
    /// Base UDP port the beacon spreads scopes over.
    pub beacon_base_port: u16,
    /// How often the beacon re-broadcasts its endpoint data.
    pub beacon_interval: Duration,
    /// Restrict the beacon to a single network interface by name, if any.
    pub interface: Option<String>,
    /// 16-byte discovery scope derived from the network name, if any.
    pub scope: Option<[u8; 16]>,
    /// Maximum number of DHT bootstrap records published per topic and minute.
    pub dht_max_records_per_minute: usize,
    /// Minimum interval between DHT discovery polls.
    pub dht_read_interval: Duration,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            mdns: true,
            beacon: true,
            beacon_base_port: DEFAULT_BEACON_PORT,
            beacon_interval: Duration::from_secs(1),
            interface: None,
            scope: None,
            dht_max_records_per_minute: crate::node::discovery::DEFAULT_DHT_MAX_RECORDS_PER_MINUTE,
            dht_read_interval: crate::node::discovery::DEFAULT_DHT_READ_INTERVAL,
        }
    }
}

impl DiscoveryConfig {
    /// A discovery configuration with every mechanism disabled.
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            mdns: false,
            beacon: false,
            beacon_base_port: DEFAULT_BEACON_PORT,
            beacon_interval: Duration::from_secs(1),
            interface: None,
            scope: None,
            dht_max_records_per_minute: crate::node::discovery::DEFAULT_DHT_MAX_RECORDS_PER_MINUTE,
            dht_read_interval: crate::node::discovery::DEFAULT_DHT_READ_INTERVAL,
        }
    }
}

/// mDNS service name for a scope, or the default iroh name when unscoped.
fn mdns_service_name(scope: Option<&[u8; 16]>) -> String {
    scope.map_or_else(
        || "irohv1".to_owned(),
        |bytes| format!("syncweb-{}", hex::encode(&bytes[..8])),
    )
}

/// Derive a discovery scope (first 16 bytes of the network id) from a network
/// name. `None` for networks without a name, keeping the default daemon
/// compatible with plain iroh peers on the LAN.
#[must_use]
pub fn discovery_scope(network: Option<&str>) -> Option<[u8; 16]> {
    let name = network?;
    let id = crate::net::NetworkId::from_name(name);
    let bytes = id.as_bytes();
    let mut scope = [0_u8; 16];
    scope.copy_from_slice(&bytes[..16]);
    Some(scope)
}

/// Registers the scoped mDNS and UDP beacon address lookups on an endpoint.
fn register_discovery(endpoint: &iroh::Endpoint, discovery: &DiscoveryConfig) {
    if discovery.mdns {
        register_mdns(endpoint, discovery);
    }
    if discovery.beacon {
        register_beacon(endpoint, discovery);
    }
}

fn register_mdns(endpoint: &iroh::Endpoint, discovery: &DiscoveryConfig) {
    let service_name = mdns_service_name(discovery.scope.as_ref());
    match iroh_mdns_address_lookup::MdnsAddressLookup::builder()
        .service_name(service_name)
        .build(endpoint.id())
        .map_err(|error| SyncwebError::operation("failed to build mDNS address lookup", error))
        .and_then(|mdns| {
            endpoint
                .address_lookup()
                .map_err(|error| SyncwebError::operation("no address lookup service available", error))
                .map(|lookup| lookup.add(mdns))
        }) {
        Ok(()) => tracing::debug!("mDNS address lookup registered"),
        Err(error) => tracing::warn!(
            %error,
            "mDNS address lookup registration failed — local peer discovery unavailable"
        ),
    }
}

fn register_beacon(endpoint: &iroh::Endpoint, discovery: &DiscoveryConfig) {
    let Ok(beacon) = BeaconAddressLookup::new(
        endpoint.id(),
        discovery.scope,
        discovery.beacon_base_port,
        discovery.beacon_interval,
        discovery.interface.as_deref(),
    ) else {
        return;
    };
    match endpoint.address_lookup() {
        Ok(lookup) => {
            lookup.add(beacon);
            tracing::debug!("beacon address lookup registered");
        }
        Err(error) => tracing::warn!(
            %error,
            "beacon address lookup registration failed — no address lookup service available"
        ),
    }
}

pub struct IrohNode {
    endpoint: iroh::Endpoint,
    router: Arc<Router>,
    blobs: iroh_blobs::BlobsProtocol,
    docs: Docs,
    gossip: Arc<Gossip>,
    blob_store: BlobStore,
    docs_engine: DocsEngine,
    topic_tracker: TopicTracker,
    /// Bounded, expiring observations of which peers successfully served
    /// individual blobs.
    provider_registry: Mutex<PeerTracker>,
    /// The Syncthing (BEP) relay manager, when `bep.enabled` is set. Keeps the
    /// registration loops and the custom transport's backend alive.
    relay: Option<RelayManager>,
    /// Persistent local state used by materialization, including conflict
    /// ancestors that are not part of the replicated document.
    node_db: NodeDatabase,
}

impl IrohNode {
    /// Creates a node and starts accepting the blobs, docs, and gossip protocols.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory structure cannot be created, if binding the endpoint fails, or if starting the background services fails.
    pub async fn new(
        identity: IdentityManager,
        data_dir: PathBuf,
        relay_mode: RelayMode,
        member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
        discovery: DiscoveryConfig,
    ) -> Result<Self> {
        Self::new_with_address_lookup(
            identity,
            data_dir,
            relay_mode,
            MemoryLookup::new(),
            discovery,
            member_keys,
        )
        .await
    }

    /// Creates a node with the Syncthing (BEP) relay transport installed.
    ///
    /// When `relay` is `Some`, the manager's custom transport is registered on
    /// the endpoint and the manager is kept alive for the node's lifetime; the
    /// node's share tickets then carry the node's relay address so peers can
    /// reach it through a Syncthing relay when direct paths fail.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory structure cannot be created, if binding the endpoint fails, or if starting the background services fails.
    pub async fn new_with_relay(
        identity: IdentityManager,
        data_dir: PathBuf,
        relay_mode: RelayMode,
        member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
        discovery: DiscoveryConfig,
        relay: Option<RelayManager>,
    ) -> Result<Self> {
        Self::new_with_address_lookup_and_relay(
            identity,
            data_dir,
            relay_mode,
            MemoryLookup::new(),
            discovery,
            member_keys,
            relay,
        )
        .await
    }

    /// # Errors
    ///
    /// Returns an error if the directory structure cannot be created, if binding the endpoint fails, or if starting the background services fails.
    pub async fn new_with_address_lookup(
        identity: IdentityManager,
        data_dir: PathBuf,
        relay_mode: RelayMode,
        address_lookup: MemoryLookup,
        discovery: DiscoveryConfig,
        member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
    ) -> Result<Self> {
        Self::new_with_address_lookup_and_relay(
            identity,
            data_dir,
            relay_mode,
            address_lookup,
            discovery,
            member_keys,
            None,
        )
        .await
    }

    /// # Errors
    ///
    /// Returns an error if the directory structure cannot be created, if binding the endpoint fails, or if starting the background services fails.
    pub async fn new_with_address_lookup_and_relay(
        identity: IdentityManager,
        data_dir: PathBuf,
        relay_mode: RelayMode,
        address_lookup: MemoryLookup,
        discovery: DiscoveryConfig,
        member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
        relay: Option<RelayManager>,
    ) -> Result<Self> {
        tokio::fs::create_dir_all(&data_dir)
            .await
            .map_err(|error| SyncwebError::operation("failed to create node data directory", error))?;
        let docs_dir = data_dir.join("docs");
        tokio::fs::create_dir_all(&docs_dir)
            .await
            .map_err(|error| SyncwebError::operation("failed to create docs directory", error))?;
        let node_db = NodeDatabase::open(data_dir.join("node.db"))?;

        let base = match relay_mode {
            RelayMode::Default => iroh::Endpoint::builder(iroh::endpoint::presets::N0),
            RelayMode::Custom { map, insecure } => {
                let mut b = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                    .relay_mode(iroh::endpoint::RelayMode::Custom(map));
                if insecure {
                    b = b.ca_tls_config(iroh::tls::CaTlsConfig::insecure_skip_verify());
                }
                b
            }
            RelayMode::None => {
                iroh::Endpoint::builder(iroh::endpoint::presets::N0).relay_mode(iroh::endpoint::RelayMode::Disabled)
            }
        };

        let hook = MembershipHook {
            member_keys: member_keys.clone(),
        };
        let mut builder = base
            .address_lookup(address_lookup.clone())
            .secret_key(identity.secret_key().clone())
            .hooks(hook);
        if let Some(manager) = &relay {
            builder = builder.add_custom_transport(Arc::new(manager.transport()));
        }
        let endpoint = builder
            .bind()
            .await
            .map_err(|error| SyncwebError::operation("failed to bind Iroh endpoint", error))?;

        // Register mDNS and UDP beacon address lookups for local peer discovery.
        // Both are scoped by `discovery.scope` when the daemon belongs to a
        // network; failures are logged and leave the other discovery mechanism
        // (or remote relay/DHT discovery) in charge.
        register_discovery(&endpoint, &discovery);

        let fs_blob_store = iroh_blobs::store::fs::FsStore::load(data_dir.join("blobs"))
            .await
            .map_err(|error| SyncwebError::operation("failed to open blob store", error))?;
        let blobs = BlobsProtocol::new(fs_blob_store.as_ref(), None);

        let gossip = Arc::new(Gossip::builder().spawn(endpoint.clone()));

        let docs = Docs::persistent(docs_dir)
            .spawn(endpoint.clone(), blobs.store().clone(), gossip.as_ref().clone())
            .await
            .map_err(|error| SyncwebError::operation("failed to open docs store", error))?;

        let router = Router::builder(endpoint.clone())
            .accept(iroh_blobs::protocol::ALPN, blobs.clone())
            .accept(iroh_docs::ALPN, docs.clone())
            .accept(iroh_gossip::ALPN, gossip.clone())
            .spawn();

        let blob_store = BlobStore::new_with_address_lookup(&blobs, address_lookup);
        let relay_addr = relay.as_ref().map(RelayManager::local_addr);
        let docs_engine = DocsEngine::new(&docs, blobs.store(), endpoint.id(), relay_addr.clone());
        let topic_tracker = TopicTracker::new(
            &gossip,
            &endpoint,
            relay_addr,
            DhtDiscoveryConfig::new(discovery.dht_max_records_per_minute, discovery.dht_read_interval),
        );

        Ok(Self {
            endpoint,
            router: Arc::new(router),
            blobs,
            docs,
            gossip,
            blob_store,
            docs_engine,
            topic_tracker,
            provider_registry: Mutex::new(PeerTracker::new(16_384, EvictionStrategy::Lru)),
            relay,
            node_db,
        })
    }

    /// The relay custom address this node advertises, when Syncthing (BEP)
    /// relaying is enabled.
    #[must_use]
    pub fn relay_addr(&self) -> Option<iroh_base::CustomAddr> {
        self.relay.as_ref().map(RelayManager::local_addr)
    }

    #[must_use]
    pub const fn endpoint(&self) -> &iroh::Endpoint {
        &self.endpoint
    }

    #[must_use]
    pub const fn blobs(&self) -> &BlobsProtocol {
        &self.blobs
    }

    #[must_use]
    pub const fn docs(&self) -> &Docs {
        &self.docs
    }

    #[must_use]
    pub fn gossip(&self) -> &Gossip {
        &self.gossip
    }

    #[must_use]
    pub const fn blob_store(&self) -> &BlobStore {
        &self.blob_store
    }

    #[must_use]
    pub const fn docs_engine(&self) -> &DocsEngine {
        &self.docs_engine
    }

    /// Return the node-local database used for materialization state.
    #[must_use]
    pub const fn node_db(&self) -> &NodeDatabase {
        &self.node_db
    }

    #[must_use]
    pub const fn topic_tracker(&self) -> &TopicTracker {
        &self.topic_tracker
    }

    /// Return the local provider observation registry.
    #[must_use]
    pub const fn provider_registry(&self) -> &Mutex<PeerTracker> {
        &self.provider_registry
    }

    /// Return fresh provider observations for a blob.
    ///
    /// # Errors
    ///
    /// Returns an error when the provider registry mutex is poisoned.
    pub fn provider_peer_snapshot(&self, hash: &Hash) -> Result<Option<Vec<iroh::PublicKey>>> {
        Ok(self.provider_registry_guard()?.peer_snapshot(hash))
    }

    /// Schedule discovered provider addresses using fresh registry observations
    /// and bounded transfer history.
    ///
    /// Unknown peers remain eligible as a compatibility path for whole-blob
    /// providers that have not advertised a lease or served this node before.
    ///
    /// # Errors
    ///
    /// Returns an error when the provider registry mutex is poisoned.
    pub fn schedule_provider_peers(
        &self,
        hash: &Hash,
        peers: &[iroh::EndpointAddr],
    ) -> Result<Vec<iroh::EndpointAddr>> {
        let mut addresses = HashMap::with_capacity(peers.len());
        let ids = peers
            .iter()
            .map(|peer| {
                addresses.insert(peer.id, peer.clone());
                peer.id
            })
            .collect::<Vec<_>>();
        let ranked = self.provider_registry_guard()?.rank_peers(hash, &ids);
        Ok(ranked.into_iter().filter_map(|peer| addresses.remove(&peer)).collect())
    }

    /// Validate and register a signed provider lease for the local scope.
    ///
    /// The lease only creates advisory provider observations. Content remains
    /// unavailable until a later fetch verifies the blob bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the lease signature, scope, expiry, or content
    /// identifiers are invalid, or when the registry mutex is poisoned.
    pub fn accept_provider_lease(
        &self,
        lease: &ProviderLease,
        now: u64,
        community: Option<&str>,
        folder: &str,
    ) -> Result<usize> {
        self.provider_registry_guard()?
            .record_lease(lease, now, community, folder)
    }

    /// Refresh verified provider leases from the namespace discovery topic.
    ///
    /// Leases are advisory and remote input, so malformed or out-of-scope
    /// leases are ignored while discovery failures remain visible to callers.
    /// Accepted observations are then available to partial-fetch ranking.
    ///
    /// # Errors
    ///
    /// Returns an error when discovery fails or the provider registry mutex is
    /// poisoned by a non-policy failure.
    pub async fn refresh_provider_leases(
        &self,
        namespace_id: iroh_docs::NamespaceId,
        now: u64,
        community: Option<&str>,
    ) -> Result<usize> {
        let leases = self.topic_tracker.find_provider_leases(namespace_id).await?;
        let folder = namespace_id.to_string();
        let mut accepted = 0_usize;
        for lease in leases {
            match self.accept_provider_lease(&lease, now, community, &folder) {
                Ok(count) => accepted = accepted.saturating_add(count),
                Err(
                    SyncwebError::AccessDenied(_)
                    | SyncwebError::InvalidConfig(_)
                    | SyncwebError::InvalidIdentity(_)
                    | SyncwebError::InvalidSignature(_)
                    | SyncwebError::MissingSignature(_)
                    | SyncwebError::InvalidTicket(_),
                ) => {
                    tracing::debug!(%namespace_id, provider = %lease.provider, "ignored out-of-scope provider lease");
                }
                Err(error) => return Err(error),
            }
        }
        Ok(accepted)
    }

    /// Record content imported from an offline archive as locally available.
    ///
    /// The observation is advisory and expires like other provider
    /// observations. Callers should invoke this only after their import and
    /// publication policy has completed successfully.
    ///
    /// # Errors
    ///
    /// Returns an error when the provider registry mutex is poisoned.
    pub fn record_offline_import<I>(&self, hashes: I) -> Result<usize>
    where
        I: IntoIterator<Item = Hash>,
    {
        let provider = self.endpoint.id();
        let mut unique = HashSet::new();
        {
            let mut registry = self.provider_registry_guard()?;
            for hash in hashes {
                if unique.insert(hash) {
                    registry.record_observation(hash, provider, ObservationSource::OfflineImport, 100);
                }
            }
        }
        Ok(unique.len())
    }

    fn provider_registry_guard(&self) -> Result<std::sync::MutexGuard<'_, PeerTracker>> {
        self.provider_registry
            .lock()
            .map_err(|error| SyncwebError::operation("provider registry mutex is poisoned", error))
    }

    async fn record_fetch_observation(
        &self,
        hash: Hash,
        peer: iroh::PublicKey,
        success: bool,
        started: Instant,
    ) -> Result<()> {
        let bytes = if success {
            match self.blob_store.stat(hash).await? {
                BlobStatus::Complete { size } => size,
                BlobStatus::Partial { .. } => {
                    return Err(SyncwebError::InvalidConfig(
                        "blob fetch succeeded but the blob is incomplete".to_owned(),
                    ));
                }
                BlobStatus::NotFound => {
                    return Err(SyncwebError::InvalidConfig(
                        "blob fetch succeeded but the blob is not present locally".to_owned(),
                    ));
                }
            }
        } else {
            0
        };
        let mut registry = self.provider_registry_guard()?;
        registry.record_transfer(peer, success, bytes, started.elapsed());
        if success {
            registry.record_peer(hash, peer);
        } else {
            registry.record_failure(hash, peer);
        }
        drop(registry);
        Ok(())
    }

    /// Fetch a blob from a discovered peer and update provider observations.
    ///
    /// Failed requests enter a short negative-cache cooldown so stale
    /// providers do not cause repeated connection storms.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is in its retry cooldown, the
    /// provider registry is poisoned, or the blob fetch fails.
    pub async fn force_fetch_from_peer(&self, peer: &iroh::EndpointAddr, hash: Hash) -> Result<()> {
        if !self.provider_registry_guard()?.should_try_peer(&hash, &peer.id) {
            return Err(SyncwebError::operation(
                "provider temporarily unavailable",
                "recent request failure is in the retry cooldown",
            ));
        }
        let started = Instant::now();
        let result = self.blob_store.force_fetch_from_peer(&self.endpoint, peer, hash).await;
        let observation_result = self
            .record_fetch_observation(hash, peer.id, result.is_ok(), started)
            .await;
        match (result, observation_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(fetch), Ok(())) => Err(fetch),
            (Ok(()), Err(observation_error)) => Err(observation_error),
            (Err(fetch), Err(observation_error)) => Err(SyncwebError::operation(
                "blob fetch and provider observation failed",
                format!("{fetch}; {observation_error}"),
            )),
        }
    }

    /// Return whether a fresh signed provider observation advertises range
    /// transfers for this blob and peer.
    ///
    /// Unknown providers deliberately return `false`, preserving the
    /// whole-blob compatibility path.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider registry mutex is poisoned.
    pub fn provider_supports_segments(&self, hash: &Hash, peer: &iroh::PublicKey) -> Result<bool> {
        Ok(self
            .provider_registry_guard()?
            .provider_capabilities(hash, peer)
            .is_some_and(|capabilities| capabilities.supports_segments))
    }

    /// Fetch one verified byte range from a provider that advertised range
    /// support.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is unavailable, the range cannot be
    /// verified, or provider accounting cannot be updated.
    pub async fn fetch_segment_from_peer(
        &self,
        peer: &iroh::EndpointAddr,
        segment: TransferSegment,
    ) -> Result<SegmentChunk> {
        if !self.provider_registry_guard()?.should_try_peer(&segment.hash, &peer.id) {
            return Err(SyncwebError::operation(
                "provider temporarily unavailable",
                "recent request failure is in the retry cooldown",
            ));
        }
        let started = Instant::now();
        let result = self
            .blob_store
            .fetch_segment_from_peer(&self.endpoint, peer, segment.hash, segment.offset, segment.length)
            .await;
        match result {
            Ok(bytes) => {
                let chunk = SegmentChunk::new(segment, bytes)?;
                let mut registry = self.provider_registry_guard()?;
                registry.record_transfer(peer.id, true, segment.length, started.elapsed());
                registry.record_peer(segment.hash, peer.id);
                drop(registry);
                Ok(chunk)
            }
            Err(error) => {
                let mut registry = self.provider_registry_guard()?;
                registry.record_transfer(peer.id, false, 0, started.elapsed());
                registry.record_failure(segment.hash, peer.id);
                drop(registry);
                Err(error)
            }
        }
    }

    /// Fetch a blob through a provider ticket and update provider observations.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is in its retry cooldown, the
    /// provider registry is poisoned, or the blob fetch fails.
    pub async fn force_fetch(&self, ticket: &BlobTicket) -> Result<()> {
        let hash = ticket.hash();
        let peer = ticket.addr().id;
        if !self.provider_registry_guard()?.should_try_peer(&hash, &peer) {
            return Err(SyncwebError::operation(
                "provider temporarily unavailable",
                "recent request failure is in the retry cooldown",
            ));
        }
        let started = Instant::now();
        let result = self.blob_store.force_fetch(&self.endpoint, ticket).await;
        let observation_result = self.record_fetch_observation(hash, peer, result.is_ok(), started).await;
        match (result, observation_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(fetch), Ok(())) => Err(fetch),
            (Ok(()), Err(observation_error)) => Err(observation_error),
            (Err(fetch), Err(observation_error)) => Err(SyncwebError::operation(
                "blob fetch and provider observation failed",
                format!("{fetch}; {observation_error}"),
            )),
        }
    }

    /// Fetch a blob through the first working provider ticket and pin it.
    ///
    /// # Errors
    ///
    /// Returns an error if no ticket can fetch the blob, provider observation
    /// recording fails, or pinning the fetched blob fails.
    pub async fn fetch_blob_and_pin(&self, content_hash: Hash, tickets: &[BlobTicket]) -> Result<()> {
        if !self.blob_store.has(content_hash).await? {
            let candidates: Vec<BlobTicket> = tickets
                .iter()
                .filter(|ticket| ticket.hash() == content_hash)
                .cloned()
                .collect();
            let peers = candidates.iter().map(|ticket| ticket.addr().clone()).collect();
            let scheduler = TransferScheduler::new(TransferSchedulerConfig::default())?;
            let report = scheduler
                .fetch(
                    self,
                    vec![TransferRequest::new(content_hash, 0, peers)],
                    CancellationToken::new(),
                )
                .await?;
            if report.completed != 1 {
                return Err(SyncwebError::operation(
                    "failed to fetch blob",
                    "all provider tickets failed",
                ));
            }
        }
        let pin_name = crate::pins::download_pin(content_hash);
        self.blob_store.pin(&pin_name, content_hash).await?;
        Ok(())
    }

    /// Fetch a blob through a provider ticket and update provider observations.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is in its retry cooldown, the
    /// provider registry is poisoned, or the blob fetch fails.
    pub async fn fetch_blob(&self, ticket: &BlobTicket) -> Result<()> {
        let hash = ticket.hash();
        let peer = ticket.addr().id;
        if !self.provider_registry_guard()?.should_try_peer(&hash, &peer) {
            return Err(SyncwebError::operation(
                "provider temporarily unavailable",
                "recent request failure is in the retry cooldown",
            ));
        }
        let started = Instant::now();
        let result = self.blob_store.fetch(&self.endpoint, ticket).await;
        let observation_result = self.record_fetch_observation(hash, peer, result.is_ok(), started).await;
        match (result, observation_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(fetch), Ok(())) => Err(fetch),
            (Ok(()), Err(observation_error)) => Err(observation_error),
            (Err(fetch), Err(observation_error)) => Err(SyncwebError::operation(
                "blob fetch and provider observation failed",
                format!("{fetch}; {observation_error}"),
            )),
        }
    }

    #[must_use]
    pub fn is_running(&self) -> bool {
        !self.router.is_shutdown()
    }

    /// # Errors
    ///
    /// Returns an error if the router fails to shutdown properly.
    pub async fn stop(&self) -> Result<()> {
        self.blob_store
            .inner()
            .sync_db()
            .await
            .map_err(|error| SyncwebError::operation("failed to flush blob store", error))?;
        self.router
            .shutdown()
            .await
            .map_err(|error| SyncwebError::operation("failed to stop node router", error))?;
        if let Some(relay) = &self.relay {
            relay.shutdown().await;
        }
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if stopping the node fails.
    pub async fn shutdown(self) -> Result<()> {
        self.stop().await
    }
}

impl Drop for IrohNode {
    fn drop(&mut self) {
        if !self.router.is_shutdown() {
            tracing::warn!("IrohNode dropped without calling stop() — router shutdown may be incomplete");
        }
    }
}
