use bytes::Bytes;
use iroh_blobs::Hash;
use iroh_docs::{
    AuthorId, DocTicket, Entry, NamespaceId,
    api::{
        Doc,
        protocol::{AddrInfoOptions, ShareMode},
    },
    engine::LiveEvent,
    protocol::Docs,
    store::Query,
};
use n0_future::StreamExt;

use crate::{
    constants::{CATALOG_METADATA_KEY, MODE_KEY, NETWORK_MEMBERS_KEY},
    error::{Result, SyncwebError},
};

/// The kind of namespace being provisioned. Picks the `sys/` metadata key
/// stamped on a freshly created namespace so folder, catalog, and membership
/// provisioning share one code path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProvisionKind {
    Folder,
    Catalog,
    NetworkMembership,
}

impl ProvisionKind {
    const fn metadata_key(self) -> &'static [u8] {
        match self {
            Self::Folder => MODE_KEY,
            Self::Catalog => CATALOG_METADATA_KEY,
            Self::NetworkMembership => NETWORK_MEMBERS_KEY,
        }
    }
}

/// A provisioned namespace and its shareable write ticket.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ProvisionedDoc {
    pub doc: Doc,
    pub ticket: DocTicket,
    pub namespace_id: NamespaceId,
}

#[derive(Clone)]
pub struct DocsEngine {
    docs: Docs,
}

impl DocsEngine {
    #[must_use]
    pub fn new(docs: &Docs) -> Self {
        Self { docs: docs.clone() }
    }

    #[must_use]
    pub const fn inner(&self) -> &Docs {
        &self.docs
    }

    /// # Errors
    ///
    /// Returns an error if the namespace cannot be created.
    pub async fn create_namespace(&self) -> Result<Doc> {
        self.docs
            .create()
            .await
            .map_err(|error| SyncwebError::operation("failed to create namespace", error))
    }

    /// Provision a namespace and a shareable ticket for it.
    ///
    /// With `existing == None` a fresh namespace is created; with
    /// `existing == Some(id)` an already-local namespace is opened instead.
    /// In both cases a write ticket is returned so the caller can hand the
    /// namespace to a peer. Folder, catalog, and membership-doc provisioning
    /// all funnel through this single helper.
    ///
    /// # Errors
    ///
    /// Returns an error if the namespace cannot be created, opened, or shared.
    pub async fn create_or_open_namespace(&self, existing: Option<NamespaceId>) -> Result<(Doc, DocTicket)> {
        let doc = match existing {
            Some(namespace_id) => self
                .open(namespace_id)
                .await?
                .ok_or(SyncwebError::NamespaceNotAvailable)?,
            None => self.create_namespace().await?,
        };
        let ticket = self.share_ticket(&doc, true).await?;
        Ok((doc, ticket))
    }

    /// Provision a namespace of the given kind and stamp its `sys/` metadata.
    ///
    /// Unifies folder, catalog, and network-membership namespace creation:
    /// every kind funnels through [`Self::create_or_open_namespace`] and writes
    /// its kind-specific metadata key under `sys/`.
    ///
    /// # Errors
    ///
    /// Returns an error if the namespace cannot be created, opened, shared, or
    /// stamped.
    pub async fn provision(
        &self,
        kind: ProvisionKind,
        existing: Option<NamespaceId>,
        metadata: impl AsRef<[u8]>,
    ) -> Result<ProvisionedDoc> {
        let (doc, ticket) = self.create_or_open_namespace(existing).await?;
        let author = self.author().await?;
        self.set(&doc, author, kind.metadata_key(), metadata).await?;
        let namespace_id = self.namespace_id(&doc);
        Ok(ProvisionedDoc {
            doc,
            ticket,
            namespace_id,
        })
    }

    /// # Errors
    ///
    /// Returns an error if the ticket cannot be imported.
    pub async fn import_ticket(&self, ticket: DocTicket) -> Result<Doc> {
        self.docs
            .import(ticket)
            .await
            .map_err(|error| SyncwebError::operation("failed to import document ticket", error))
    }

    /// # Errors
    ///
    /// Returns an error if the document cannot be opened.
    pub async fn open(&self, namespace_id: NamespaceId) -> Result<Option<Doc>> {
        self.docs
            .open(namespace_id)
            .await
            .map_err(|error| SyncwebError::operation("failed to open namespace", error))
    }

    /// # Errors
    ///
    /// Returns an error if the namespace cannot be dropped.
    pub async fn drop_namespace(&self, namespace_id: NamespaceId) -> Result<()> {
        self.docs
            .drop_doc(namespace_id)
            .await
            .map_err(|error| SyncwebError::operation("failed to drop namespace", error))?;
        Ok(())
    }

    /// Start live synchronization for a document.
    ///
    /// An empty peer list uses peers already learned by iroh-docs for the
    /// namespace.
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be opened for synchronization.
    pub async fn start_sync(&self, doc: &Doc, peers: Vec<iroh::EndpointAddr>) -> Result<()> {
        doc.start_sync(peers)
            .await
            .map_err(|error| SyncwebError::operation("failed to start document synchronization", error))
    }

    /// # Errors
    ///
    /// Returns an error if the document cannot be shared.
    pub async fn share(&self, doc: &Doc, mode: ShareMode) -> Result<DocTicket> {
        doc.share(mode, AddrInfoOptions::RelayAndAddresses)
            .await
            .map_err(|error| SyncwebError::operation("failed to share document", error))
    }

    /// Share a read or write ticket for a document.
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be shared.
    pub async fn share_ticket(&self, doc: &Doc, writable: bool) -> Result<DocTicket> {
        let mode = if writable { ShareMode::Write } else { ShareMode::Read };
        self.share(doc, mode).await
    }

    /// # Errors
    ///
    /// Returns an error if the default author cannot be retrieved.
    pub async fn author(&self) -> Result<AuthorId> {
        self.docs
            .author_default()
            .await
            .map_err(|error| SyncwebError::operation("failed to retrieve document author", error))
    }

    /// # Errors
    ///
    /// Returns an error if the author cannot be exported.
    pub async fn export_author(&self, author_id: AuthorId) -> Result<Option<iroh_docs::Author>> {
        self.docs
            .author_export(author_id)
            .await
            .map_err(|error| SyncwebError::operation("failed to export document author", error))
    }

    /// # Errors
    ///
    /// Returns an error if the author cannot be imported.
    pub async fn import_author(&self, author: iroh_docs::Author) -> Result<AuthorId> {
        self.docs
            .author_import(author.clone())
            .await
            .map_err(|error| SyncwebError::operation("failed to import document author", error))?;
        Ok(author.id())
    }

    /// # Errors
    ///
    /// Returns an error if setting the document entry fails.
    pub async fn set(
        &self,
        doc: &Doc,
        author: AuthorId,
        key: impl AsRef<[u8]>,
        value: impl AsRef<[u8]>,
    ) -> Result<Hash> {
        doc.set_bytes(
            author,
            Bytes::copy_from_slice(key.as_ref()),
            Bytes::copy_from_slice(value.as_ref()),
        )
        .await
        .map_err(|error| SyncwebError::operation("failed to set document entry", error))
    }

    /// # Errors
    ///
    /// Returns an error if setting the document blob fails.
    pub async fn set_blob(
        &self,
        doc: &Doc,
        author: AuthorId,
        key: impl AsRef<[u8]>,
        hash: Hash,
        size: u64,
    ) -> Result<()> {
        doc.set_hash(author, Bytes::copy_from_slice(key.as_ref()), hash, size)
            .await
            .map_err(|error| SyncwebError::operation("failed to set document blob", error))?;
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if getting the document entry fails.
    pub async fn get(&self, doc: &Doc, author: AuthorId, key: impl AsRef<[u8]>) -> Result<Option<Entry>> {
        doc.get_exact(author, key, false)
            .await
            .map_err(|error| SyncwebError::operation("failed to get document entry", error))
    }

    /// Read the latest entry for a key regardless of which author wrote it.
    ///
    /// # Errors
    ///
    /// Returns an error if the document query fails.
    pub async fn get_any(&self, doc: &Doc, key: impl AsRef<[u8]>) -> Result<Option<Entry>> {
        let entries = doc
            .get_many(Query::single_latest_per_key().key_exact(key))
            .await
            .map_err(|error| SyncwebError::operation("failed to query document entries", error))?;
        tokio::pin!(entries);
        n0_future::StreamExt::next(&mut entries)
            .await
            .transpose()
            .map_err(|error| SyncwebError::operation("failed to read document query result", error))
    }

    /// Read the latest non-empty entry for every document key.
    ///
    /// # Errors
    ///
    /// Returns an error if the document query fails.
    pub async fn list_latest(&self, doc: &Doc) -> Result<Vec<Entry>> {
        let entries = doc
            .get_many(Query::single_latest_per_key().build())
            .await
            .map_err(|error| SyncwebError::operation("failed to query document entries", error))?;
        tokio::pin!(entries);
        let mut output = Vec::new();
        while let Some(entry_result) = n0_future::StreamExt::next(&mut entries).await {
            let entry =
                entry_result.map_err(|error| SyncwebError::operation("failed to read document entry", error))?;
            output.push(entry);
        }
        Ok(output)
    }

    /// Delete the current value for a document key.
    ///
    /// # Errors
    ///
    /// Returns an error if the document entry cannot be deleted.
    pub async fn delete(&self, doc: &Doc, author: AuthorId, key: impl AsRef<[u8]>) -> Result<()> {
        doc.del(author, key.as_ref().to_vec())
            .await
            .map_err(|error| SyncwebError::operation("failed to delete document entry", error))?;
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if watching the document fails.
    pub async fn watch(
        &self,
        doc: &Doc,
    ) -> Result<impl n0_future::Stream<Item = Result<LiveEvent>> + Send + Unpin + 'static> {
        let stream = doc
            .subscribe()
            .await
            .map_err(|error| SyncwebError::operation("failed to subscribe to document", error))?;
        Ok(stream.map(|event| event.map_err(|error| SyncwebError::operation("document event failed", error))))
    }

    #[must_use]
    pub fn namespace_id(&self, doc: &Doc) -> NamespaceId {
        doc.id()
    }
}
