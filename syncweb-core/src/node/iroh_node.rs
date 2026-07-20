use iroh::address_lookup::memory::MemoryLookup;
use iroh::protocol::Router;
use iroh_blobs::BlobsProtocol;
use iroh_docs::protocol::Docs;
use iroh_gossip::net::Gossip;
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::{Result, SyncwebError};

use super::discovery::TopicTracker;
use super::identity::IdentityManager;
use super::{blob_store::BlobStore, docs_engine::DocsEngine, gossip_service::GossipService};

#[non_exhaustive]
pub enum RelayMode {
    Default,
    Custom { map: iroh::RelayMap, insecure: bool },
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
    pub async fn new(identity: IdentityManager, data_dir: PathBuf, relay_mode: RelayMode) -> Result<Self> {
        Self::new_with_address_lookup(identity, data_dir, relay_mode, MemoryLookup::new()).await
    }

    /// # Errors
    ///
    /// Returns an error if the directory structure cannot be created, if binding the endpoint fails, or if starting the background services fails.
    pub async fn new_with_address_lookup(
        identity: IdentityManager,
        data_dir: PathBuf,
        relay_mode: RelayMode,
        address_lookup: MemoryLookup,
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
        };

        let endpoint = builder
            .address_lookup(address_lookup.clone())
            .secret_key(identity.secret_key().clone())
            .bind()
            .await
            .map_err(|error| SyncwebError::operation("failed to bind Iroh endpoint", error))?;

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
