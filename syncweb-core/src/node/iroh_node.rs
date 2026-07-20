use anyhow::{Context, Result};
use iroh::address_lookup::memory::MemoryLookup;
use iroh::protocol::Router;
use iroh_blobs::BlobsProtocol;
use iroh_docs::protocol::Docs;
use iroh_gossip::net::Gossip;
use std::path::PathBuf;
use std::sync::Arc;

use super::discovery::TopicTracker;
use super::identity::IdentityManager;
use super::{blob_store::BlobStore, docs_engine::DocsEngine, gossip_service::GossipService};

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
    pub async fn new(identity: IdentityManager, data_dir: PathBuf) -> Result<Self> {
        tokio::fs::create_dir_all(&data_dir).await?;
        let docs_dir = data_dir.join("docs");
        tokio::fs::create_dir_all(&docs_dir).await?;
        let address_lookup = MemoryLookup::new();
        let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
            .address_lookup(address_lookup.clone())
            .secret_key(identity.secret_key().clone())
            .bind()
            .await
            .context("failed to bind Iroh endpoint")?;

        let blob_store = iroh_blobs::store::fs::FsStore::load(data_dir.join("blobs"))
            .await
            .context("failed to open blob store")?;
        let blobs = BlobsProtocol::new(blob_store.as_ref(), None);

        let gossip = Arc::new(Gossip::builder().spawn(endpoint.clone()));

        let docs = Docs::persistent(docs_dir)
            .spawn(
                endpoint.clone(),
                blobs.store().clone(),
                gossip.as_ref().clone(),
            )
            .await
            .context("failed to open docs store")?;

        let router = Router::builder(endpoint.clone())
            .accept(iroh_blobs::protocol::ALPN, blobs.clone())
            .accept(iroh_docs::ALPN, docs.clone())
            .accept(iroh_gossip::ALPN, gossip.clone())
            .spawn();

        let blob_store = BlobStore::new_with_address_lookup(&blobs, address_lookup);
        let docs_engine = DocsEngine::new(&docs);
        let gossip_service = GossipService::new(&gossip);
        let topic_tracker = TopicTracker::new(&gossip, &endpoint).await?;

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

    pub fn endpoint(&self) -> &iroh::Endpoint {
        &self.endpoint
    }

    pub fn blobs(&self) -> &BlobsProtocol {
        &self.blobs
    }

    pub fn docs(&self) -> &Docs {
        &self.docs
    }

    pub fn gossip(&self) -> &Gossip {
        &self.gossip
    }

    pub fn blob_store(&self) -> &BlobStore {
        &self.blob_store
    }

    pub fn docs_engine(&self) -> &DocsEngine {
        &self.docs_engine
    }

    pub fn gossip_service(&self) -> &GossipService {
        &self.gossip_service
    }

    pub fn topic_tracker(&self) -> &TopicTracker {
        &self.topic_tracker
    }

    pub fn is_running(&self) -> bool {
        !self.router.is_shutdown()
    }

    pub async fn stop(&self) -> Result<()> {
        self.router.shutdown().await?;
        Ok(())
    }

    pub async fn shutdown(self) -> Result<()> {
        self.stop().await
    }
}
