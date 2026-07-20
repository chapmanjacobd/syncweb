use anyhow::Result;
use bytes::Bytes;
use iroh::{EndpointAddr, address_lookup::memory::MemoryLookup, endpoint::Endpoint};
use iroh_blobs::api::Store as BlobApi;
use iroh_blobs::{BlobsProtocol, Hash, ticket::BlobTicket};
use std::path::Path;

#[derive(Clone)]
pub struct BlobStore {
    store: BlobApi,
    address_lookup: MemoryLookup,
}

impl BlobStore {
    pub fn new(protocol: &BlobsProtocol) -> Self {
        Self::new_with_address_lookup(protocol, MemoryLookup::new())
    }

    pub fn new_with_address_lookup(protocol: &BlobsProtocol, address_lookup: MemoryLookup) -> Self {
        Self {
            store: protocol.store().clone(),
            address_lookup,
        }
    }

    pub fn inner(&self) -> &BlobApi {
        &self.store
    }

    pub async fn add_bytes(&self, data: impl AsRef<[u8]>) -> Result<Hash> {
        Ok(self
            .store
            .add_bytes(Bytes::copy_from_slice(data.as_ref()))
            .await?
            .hash)
    }

    pub async fn add_file(&self, path: impl AsRef<Path>) -> Result<Hash> {
        Ok(self.store.add_path(path).await?.hash)
    }

    pub async fn has(&self, hash: Hash) -> Result<bool> {
        Ok(self.store.has(hash).await?)
    }

    pub async fn get(&self, hash: Hash) -> Result<Bytes> {
        Ok(self.store.get_bytes(hash).await?)
    }

    pub fn ticket(&self, endpoint: &Endpoint, hash: Hash) -> BlobTicket {
        self.ticket_for_addr(endpoint.addr(), hash)
    }

    pub fn ticket_for_addr(&self, addr: EndpointAddr, hash: Hash) -> BlobTicket {
        BlobTicket::new(addr, hash, iroh_blobs::BlobFormat::Raw)
    }

    pub async fn fetch(&self, endpoint: &Endpoint, ticket: &BlobTicket) -> Result<()> {
        self.address_lookup.add_endpoint_info(ticket.addr().clone());
        self.store
            .downloader(endpoint)
            .download(ticket.hash_and_format(), [ticket.addr().id])
            .await?;
        Ok(())
    }
}
