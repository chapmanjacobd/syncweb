use anyhow::Result;
use iroh::protocol::Router;
use iroh_blobs::BlobsProtocol;
use iroh_docs::engine::Engine;
use iroh_gossip::net::Gossip;
use std::path::PathBuf;
use std::sync::Arc;

use super::identity::IdentityManager;

pub struct IrohNode {
    endpoint: iroh::Endpoint,
    router: Arc<Router>,
    blobs: iroh_blobs::BlobsProtocol,
    docs: Arc<Engine>,
    gossip: Arc<Gossip>,
}

impl IrohNode {
    /// Create and start a new IrohNode.
    pub async fn new(identity: IdentityManager, data_dir: PathBuf) -> Result<Self> {
        let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
            .secret_key(identity.secret_key().clone())
            .bind()
            .await?;

        let blob_store = iroh_blobs::store::fs::FsStore::load(data_dir.join("blobs")).await?;
        let blobs = BlobsProtocol::new(blob_store.as_ref(), None);

        let gossip = Arc::new(Gossip::builder().spawn(endpoint.clone()));

        let docs_store = iroh_docs::store::Store::persistent(data_dir.join("docs"))?;
        let downloader = blobs.downloader(&endpoint);
        let author_storage = iroh_docs::engine::DefaultAuthorStorage::Persistent(data_dir.join("default_author"));
        let docs = Arc::new(
            Engine::spawn(
                endpoint.clone(),
                (*gossip).clone(),
                docs_store,
                blobs.store().clone(),
                downloader,
                author_storage,
                None,
            )
            .await?,
        );

        let router = Router::builder(endpoint.clone())
            .accept(iroh_blobs::protocol::ALPN, blobs.clone())
            .accept(iroh_gossip::ALPN, gossip.clone()) // Note: this is not the right ALPN for gossip
            .spawn();

        Ok(Self {
            endpoint,
            router: Arc::new(router),
            blobs,
            docs,
            gossip,
        })
    }

    pub fn endpoint(&self) -> &iroh::Endpoint {
        &self.endpoint
    }

    pub fn blobs(&self) -> &BlobsProtocol {
        &self.blobs
    }

    pub fn docs(&self) -> &Engine {
        &self.docs
    }

    pub fn gossip(&self) -> &Gossip {
        &self.gossip
    }

    pub async fn shutdown(self) -> Result<()> {
        self.router.shutdown().await?;
        self.endpoint.close().await;
        Ok(())
    }
}
