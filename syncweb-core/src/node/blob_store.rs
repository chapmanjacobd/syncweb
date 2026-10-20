use bytes::Bytes;
use iroh::{EndpointAddr, address_lookup::memory::MemoryLookup, endpoint::Endpoint};
use iroh_blobs::{
    BlobFormat, BlobsProtocol, Hash,
    api::{
        Store as BlobApi,
        blobs::{AddPathOptions, BlobReader, BlobStatus, ExportMode, ExportOptions, ImportMode},
    },
    protocol::GetRequest,
    ticket::BlobTicket,
};
use n0_future::StreamExt;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

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

    /// Register a peer's known addresses so later connections (such as blob
    /// downloads that only carry the peer's node id) can resolve the peer
    /// without LAN discovery.
    pub fn add_endpoint_addr(&self, addr: EndpointAddr) {
        self.address_lookup.add_endpoint_info(addr);
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
        let absolute_path = absolute_path(path)?;
        Ok(self
            .store
            .add_path(absolute_path)
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
        let absolute_path = absolute_path(path)?;
        Ok(self
            .store
            .add_path_with_opts(AddPathOptions {
                path: absolute_path,
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
            .map_err(|error| SyncwebError::operation("failed to export blob", crate::error::error_chain(&error)))
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

    /// Get metadata (size) for a blob.
    ///
    /// # Errors
    ///
    /// Returns an error if the blob is not found or the store cannot be queried.
    pub async fn stat(&self, hash: Hash) -> Result<BlobStatus> {
        self.store
            .blobs()
            .status(hash)
            .await
            .map_err(|error| SyncwebError::operation("failed to stat blob", error))
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
        raw_blob_ticket(addr, hash)
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

    /// Pin content with an explicit retention class and optional expiry.
    ///
    /// The returned name is stable and can be used to remove the pin later.
    ///
    /// # Errors
    ///
    /// Returns an error if the tag cannot be written.
    pub async fn pin_class(
        &self,
        class: crate::pins::PinClass,
        scope: impl AsRef<str>,
        hash: Hash,
        expires_at: Option<u64>,
    ) -> Result<String> {
        let name = crate::pins::classified_pin_name(class, scope, hash, expires_at);
        self.pin(&name, hash).await?;
        Ok(name)
    }

    /// List pins belonging to one of the local retention classes.
    ///
    /// Unknown legacy tags are ignored so a cleanup pass can never remove an
    /// unrelated application tag.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be queried.
    pub async fn list_classified_pins(&self) -> Result<Vec<crate::pins::PinRecord>> {
        Ok(self
            .list_pins("")
            .await?
            .into_iter()
            .filter_map(|(name, hash)| crate::pins::PinRecord::from_tag(name, hash))
            .collect())
    }

    /// Remove expired temporary/cache pins and leave shared/protected pins
    /// untouched. Unpinned blobs become eligible for the blob store's normal
    /// garbage collector.
    ///
    /// # Errors
    ///
    /// Returns an error if pins cannot be listed or removed.
    pub async fn gc_expired_pins(
        &self,
        policy: crate::pins::PinPolicy,
        now: u64,
    ) -> Result<Vec<crate::pins::PinRecord>> {
        let pins = self.list_classified_pins().await?;
        let expired = policy
            .gc_candidates(&pins, now)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        for pin in &expired {
            self.unpin(&pin.name).await?;
        }
        Ok(expired)
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

    /// Fetch a blob through the first working provider ticket and pin it,
    /// skipping the fetch when the blob is already present locally.
    ///
    /// # Errors
    ///
    /// Returns an error if the blob cannot be fetched through any ticket.
    pub async fn fetch_and_pin(&self, endpoint: &Endpoint, content_hash: Hash, tickets: &[BlobTicket]) -> Result<()> {
        if !self.has(content_hash).await? {
            let candidates: Vec<BlobTicket> = tickets
                .iter()
                .filter(|ticket| ticket.hash() == content_hash)
                .cloned()
                .collect();
            let peers = candidates.iter().map(|ticket| ticket.addr().clone()).collect();
            let report = crate::sync::partial_fetch::fetch_blob_requests(
                self,
                endpoint,
                vec![crate::sync::partial_fetch::TransferRequest::new(content_hash, 0, peers)],
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
        self.pin(&pin_name, content_hash).await?;
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

    /// Force-fetch a blob from a known peer, connecting by its discovered
    /// address (which may include a relay or custom transport address).
    ///
    /// # Errors
    ///
    /// Returns an error if the connection or download fails.
    pub async fn force_fetch_from_peer(
        &self,
        endpoint: &Endpoint,
        peer: &iroh::EndpointAddr,
        hash: Hash,
    ) -> Result<()> {
        self.address_lookup.add_endpoint_info(peer.clone());
        let conn = endpoint
            .connect(peer.id, iroh_blobs::protocol::ALPN)
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

fn absolute_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    std::path::absolute(path.as_ref())
        .map_err(|error| SyncwebError::operation("failed to resolve blob file path", error))
}

/// Build an unauthenticated blob ticket for a content hash hosted by an address.
#[must_use]
pub fn raw_blob_ticket(addr: EndpointAddr, hash: Hash) -> BlobTicket {
    BlobTicket::new(addr, hash, BlobFormat::Raw)
}
