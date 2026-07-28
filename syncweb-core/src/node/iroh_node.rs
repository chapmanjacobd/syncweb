use iroh::address_lookup::memory::MemoryLookup;
use iroh::protocol::Router;
use iroh_blobs::BlobsProtocol;
use iroh_docs::protocol::Docs;
use iroh_gossip::net::Gossip;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::error::{Result, SyncwebError};

use super::discovery::TopicTracker;
use super::identity::IdentityManager;
use super::membership_hook::MembershipHook;
use super::{blob_store::BlobStore, docs_engine::DocsEngine, gossip_service::GossipService};

#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum RelayMode {
    Default,
    Custom { map: iroh::RelayMap, insecure: bool },
    None,
}

pub struct IrohNode {
    endpoint: iroh::Endpoint,
    router: Arc<Router>,
    blobs: iroh_blobs::BlobsProtocol,
    docs: Docs,
    gossip: Arc<Gossip>,
    blob_store: BlobStore,
    docs_engine: DocsEngine,
    gossip_service: GossipService,
    topic_tracker: TopicTracker,
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
    ) -> Result<Self> {
        Self::new_with_address_lookup(identity, data_dir, relay_mode, MemoryLookup::new(), member_keys).await
    }

    /// # Errors
    ///
    /// Returns an error if the directory structure cannot be created, if binding the endpoint fails, or if starting the background services fails.
    pub async fn new_with_address_lookup(
        identity: IdentityManager,
        data_dir: PathBuf,
        relay_mode: RelayMode,
        address_lookup: MemoryLookup,
        member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
    ) -> Result<Self> {
        tokio::fs::create_dir_all(&data_dir)
            .await
            .map_err(|error| SyncwebError::operation("failed to create node data directory", error))?;
        let docs_dir = data_dir.join("docs");
        tokio::fs::create_dir_all(&docs_dir)
            .await
            .map_err(|error| SyncwebError::operation("failed to create docs directory", error))?;

        let builder = match relay_mode {
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
        let endpoint = builder
            .address_lookup(address_lookup.clone())
            .secret_key(identity.secret_key().clone())
            .hooks(hook)
            .bind()
            .await
            .map_err(|error| SyncwebError::operation("failed to bind Iroh endpoint", error))?;

        // Register mDNS address lookup for local network peer discovery.
        if let Err(error) = iroh_mdns_address_lookup::MdnsAddressLookup::builder()
            .build(endpoint.id())
            .map_err(|error| SyncwebError::operation("failed to build mDNS address lookup", error))
            .and_then(|mdns| {
                endpoint
                    .address_lookup()
                    .map_err(|error| SyncwebError::operation("no address lookup service available", error))
                    .map(|lookup| lookup.add(mdns))
            })
        {
            tracing::warn!(%error, "mDNS address lookup registration failed — local peer discovery unavailable");
        } else {
            tracing::debug!("mDNS address lookup registered");
        }

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
        let docs_engine = DocsEngine::new(&docs);
        let gossip_service = GossipService::new(&gossip);
        let topic_tracker = TopicTracker::new(&gossip, &endpoint);

        Ok(Self {
            endpoint,
            router: Arc::new(router),
            blobs,
            docs,
            gossip,
            blob_store,
            docs_engine,
            gossip_service,
            topic_tracker,
        })
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

    #[must_use]
    pub const fn gossip_service(&self) -> &GossipService {
        &self.gossip_service
    }

    #[must_use]
    pub const fn topic_tracker(&self) -> &TopicTracker {
        &self.topic_tracker
    }

    #[must_use]
    pub fn is_running(&self) -> bool {
        !self.router.is_shutdown()
    }

    /// # Errors
    ///
    /// Returns an error if the router fails to shutdown properly.
    pub async fn stop(&self) -> Result<()> {
        self.router
            .shutdown()
            .await
            .map_err(|error| SyncwebError::operation("failed to stop node router", error))?;
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
