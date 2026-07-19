use iroh_blobs::api::Store as BlobApi;
use iroh_blobs::BlobsProtocol;

pub struct BlobStore {
    store: BlobApi,
}

impl BlobStore {
    pub fn new(protocol: &BlobsProtocol) -> Self {
        Self {
            store: protocol.store().clone(),
        }
    }

    pub fn inner(&self) -> &BlobApi {
        &self.store
    }
}
