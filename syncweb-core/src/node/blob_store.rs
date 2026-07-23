use bytes::Bytes;
use iroh::{EndpointAddr, address_lookup::memory::MemoryLookup, endpoint::Endpoint};
use iroh_blobs::{
    BlobFormat, BlobsProtocol, Hash,
    api::{
        Store as BlobApi,
        blobs::{AddPathOptions, BlobReader, ExportMode, ExportOptions, ImportMode},
    },
    protocol::GetRequest,
    ticket::BlobTicket,
};
use n0_future::StreamExt;
use std::path::Path;

use crate::error::{Result, SyncwebError};

#[derive(Clone)]
pub struct BlobStore {
    store: BlobApi,
    address_lookup: MemoryLookup,
}

impl BlobStore {
    #[must_use]
    pub fn new(protocol: &BlobsProtocol) -> Self {
        Self::new_with_address_lookup(protocol, MemoryLookup::new())
    }

    #[must_use]
    pub fn new_with_address_lookup(protocol: &BlobsProtocol, address_lookup: MemoryLookup) -> Self {
        Self {
            store: protocol.store().clone(),
            address_lookup,
        }
    }

    #[must_use]
    pub const fn inner(&self) -> &BlobApi {
        &self.store
    }

    /// Create a reader for the given hash.
    ///
    /// The reader implements [`tokio::io::AsyncRead`] and [`tokio::io::AsyncSeek`],
    /// allowing O(1)-memory streaming to any sink.
    #[must_use]
    pub fn reader(&self, hash: impl Into<Hash>) -> BlobReader {
        self.store.reader(hash)
    }

    /// # Errors
    ///
    /// Returns an error if the blob fails to be added to the store.
    pub async fn add_bytes(&self, data: impl AsRef<[u8]>) -> Result<Hash> {
        Ok(self
            .store
            .add_bytes(Bytes::copy_from_slice(data.as_ref()))
            .await
            .map_err(|error| SyncwebError::operation("failed to add blob bytes", error))?
            .hash)
    }

    /// # Errors
    ///
    /// Returns an error if the file fails to be read or added to the store.
    pub async fn add_file(&self, path: impl AsRef<Path>) -> Result<Hash> {
        Ok(self
            .store
            .add_path(path)
            .await
            .map_err(|error| SyncwebError::operation("failed to add blob file", error))?
            .hash)
    }

    /// Add a file using a reference to its original path instead of copying
    /// its contents into the blob store.
    ///
    /// The source file must remain available and unchanged for as long as the
    /// blob store may need to read this blob.
    ///
    /// # Errors
    ///
    /// Returns an error if the file fails to be read or added to the store.
    pub async fn add_file_ref(&self, path: impl AsRef<Path>) -> Result<Hash> {
        Ok(self
            .store
            .add_path_with_opts(AddPathOptions {
                path: path.as_ref().to_owned(),
                mode: ImportMode::TryReference,
                format: BlobFormat::Raw,
            })
            .await
            .map_err(|error| SyncwebError::operation("failed to add blob file (reference)", error))?
            .hash)
    }

    /// Export a complete blob to a temporary or caller-managed path.
    ///
    /// Uses [`ExportMode::Copy`] — the blob stays in the store and a copy
    /// (or `CoW` reflink) is written to `destination`.
    ///
    /// # Errors
    ///
    /// Returns an error if the blob cannot be read or written.
    pub async fn export_to_path(&self, hash: Hash, destination: impl AsRef<Path>) -> Result<u64> {
        self.export_to_path_with_mode(hash, destination, ExportMode::Copy).await
    }

    /// Export a complete blob with a specific [`ExportMode`].
    ///
    /// `ExportMode::TryReference` moves the blob's internal file to
    /// `destination` and updates the store to track it externally,
    /// avoiding a copy. `ExportMode::Copy` leaves the blob in the store
    /// and writes a copy (or `CoW` reflink) to `destination`.
    ///
    /// # Errors
    ///
    /// Returns an error if the blob cannot be read or written.
    pub async fn export_to_path_with_mode(
        &self,
        hash: Hash,
        destination: impl AsRef<Path>,
        mode: ExportMode,
    ) -> Result<u64> {
        self.store
            .export_with_opts(ExportOptions {
                hash,
                mode,
                target: destination.as_ref().to_owned(),
            })
            .await
            .map_err(|error| SyncwebError::operation("failed to export blob", error))
    }

    /// # Errors
    ///
    /// Returns an error if the store cannot be queried.
    pub async fn has(&self, hash: Hash) -> Result<bool> {
        self.store
            .has(hash)
            .await
            .map_err(|error| SyncwebError::operation("failed to query blob store", error))
    }

    /// # Errors
    ///
    /// Returns an error if the blob cannot be found or read.
    pub async fn get(&self, hash: Hash) -> Result<Bytes> {
        self.store
            .get_bytes(hash)
            .await
            .map_err(|error| SyncwebError::operation("failed to read blob", error))
    }

    #[must_use]
    pub fn ticket(&self, endpoint: &Endpoint, hash: Hash) -> BlobTicket {
        self.ticket_for_addr(endpoint.addr(), hash)
    }

    #[must_use]
    pub fn ticket_for_addr(&self, addr: EndpointAddr, hash: Hash) -> BlobTicket {
        BlobTicket::new(addr, hash, iroh_blobs::BlobFormat::Raw)
    }

    /// Pin a blob with a durable named tag so garbage collection cannot remove it.
    ///
    /// # Errors
    ///
    /// Returns an error if the tag cannot be written.
    pub async fn pin(&self, name: impl AsRef<str>, hash: Hash) -> Result<()> {
        self.store
            .tags()
            .set(name.as_ref(), hash)
            .await
            .map_err(|error| SyncwebError::operation("failed to pin blob", error))
    }

    /// Remove a durable pin from a blob.
    ///
    /// # Errors
    ///
    /// Returns an error if the tag cannot be removed.
    pub async fn unpin(&self, name: impl AsRef<str>) -> Result<()> {
        self.store
            .tags()
            .delete(name.as_ref())
            .await
            .map(|_deleted| ())
            .map_err(|error| SyncwebError::operation("failed to unpin blob", error))
    }

    /// # Errors
    ///
    /// Returns an error if the pin cannot be queried.
    pub async fn is_pinned(&self, name: impl AsRef<str>, hash: Hash) -> Result<bool> {
        self.store
            .tags()
            .get(name.as_ref())
            .await
            .map(|tag_info| tag_info.is_some_and(|tag| tag.hash == hash))
            .map_err(|error| SyncwebError::operation("failed to query blob pin", error))
    }

    /// List all complete blobs currently in the store.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be queried.
    pub async fn list_hashes(&self) -> Result<Vec<Hash>> {
        self.store
            .blobs()
            .list()
            .hashes()
            .await
            .map_err(|error| SyncwebError::operation("failed to list blobs", error))
    }

    /// List named pins whose names start with `prefix`.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be queried.
    pub async fn list_pins(&self, prefix: impl AsRef<[u8]>) -> Result<Vec<(String, Hash)>> {
        let mut tags = self
            .store
            .tags()
            .list_prefix(prefix)
            .await
            .map_err(|error| SyncwebError::operation("failed to list blob pins", error))?;
        let mut pins = Vec::new();
        while let Some(tag_result) = tags.next().await {
            let tag = tag_result.map_err(|error| SyncwebError::operation("failed to read blob pin", error))?;
            let name = String::from_utf8(tag.name.as_ref().to_vec())
                .map_err(|error| SyncwebError::operation("blob pin name is not UTF-8", error))?;
            pins.push((name, tag.hash));
        }
        Ok(pins)
    }

    /// # Errors
    ///
    /// Returns an error if the blob cannot be fetched from the remote endpoint.
    pub async fn fetch(&self, endpoint: &Endpoint, ticket: &BlobTicket) -> Result<()> {
        self.address_lookup.add_endpoint_info(ticket.addr().clone());
        self.store
            .downloader(endpoint)
            .download(ticket.hash_and_format(), [ticket.addr().id])
            .await
            .map_err(|error| SyncwebError::operation("failed to fetch blob", error))?;
        Ok(())
    }

    /// Force-fetch a blob from a remote peer, re-downloading even if the blob
    /// is already present locally. Uses `iroh-blobs`' low-level `execute_get`
    /// API which does not check local availability.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection or download fails.
    pub async fn force_fetch(&self, endpoint: &Endpoint, ticket: &BlobTicket) -> Result<()> {
        self.address_lookup.add_endpoint_info(ticket.addr().clone());
        let hash = ticket.hash();
        let conn = endpoint
            .connect(ticket.addr().id, iroh_blobs::protocol::ALPN)
            .await
            .map_err(|error| SyncwebError::operation("failed to connect to provider", error))?;
        self.force_fetch_from_conn(conn, hash).await
    }

    /// Force-fetch a blob from a known peer, connecting by public key alone.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection or download fails.
    pub async fn force_fetch_from_peer(&self, endpoint: &Endpoint, peer: &iroh::PublicKey, hash: Hash) -> Result<()> {
        let conn = endpoint
            .connect(*peer, iroh_blobs::protocol::ALPN)
            .await
            .map_err(|error| SyncwebError::operation("failed to connect to peer", error))?;
        self.force_fetch_from_conn(conn, hash).await
    }

    async fn force_fetch_from_conn(&self, conn: iroh::endpoint::Connection, hash: Hash) -> Result<()> {
        let request = GetRequest::blob(hash);
        self.store
            .remote()
            .execute_get(conn, request)
            .await
            .map_err(|error| SyncwebError::operation("failed to force-fetch blob", error))?;
        Ok(())
    }
}
