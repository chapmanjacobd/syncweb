use anyhow::Result;
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
};

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
        self.docs.create().await
    }

    /// # Errors
    ///
    /// Returns an error if the ticket cannot be imported.
    pub async fn import_ticket(&self, ticket: DocTicket) -> Result<Doc> {
        self.docs.import(ticket).await
    }

    /// # Errors
    ///
    /// Returns an error if the document cannot be opened.
    pub async fn open(&self, namespace_id: NamespaceId) -> Result<Option<Doc>> {
        self.docs.open(namespace_id).await
    }

    /// # Errors
    ///
    /// Returns an error if the namespace cannot be dropped.
    pub async fn drop_namespace(&self, namespace_id: NamespaceId) -> Result<()> {
        self.docs.drop_doc(namespace_id).await?;
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if the document cannot be shared.
    pub async fn share(&self, doc: &Doc, mode: ShareMode, _endpoint: iroh::EndpointAddr) -> Result<DocTicket> {
        doc.share(mode, AddrInfoOptions::RelayAndAddresses).await
    }

    /// # Errors
    ///
    /// Returns an error if the default author cannot be retrieved.
    pub async fn author(&self) -> Result<AuthorId> {
        self.docs.author_default().await
    }

    /// # Errors
    ///
    /// Returns an error if the author cannot be exported.
    pub async fn export_author(&self, author_id: AuthorId) -> Result<Option<iroh_docs::Author>> {
        self.docs.author_export(author_id).await
    }

    /// # Errors
    ///
    /// Returns an error if the author cannot be imported.
    pub async fn import_author(&self, author: iroh_docs::Author) -> Result<AuthorId> {
        self.docs.author_import(author.clone()).await?;
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
            .await?;
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if getting the document entry fails.
    pub async fn get(&self, doc: &Doc, author: AuthorId, key: impl AsRef<[u8]>) -> Result<Option<Entry>> {
        doc.get_exact(author, key, false).await
    }

    /// # Errors
    ///
    /// Returns an error if watching the document fails.
    pub async fn watch(
        &self,
        doc: &Doc,
    ) -> Result<impl n0_future::Stream<Item = Result<LiveEvent>> + Send + Unpin + 'static> {
        doc.subscribe().await
    }

    #[must_use]
    pub fn namespace_id(&self, doc: &Doc) -> NamespaceId {
        doc.id()
    }
}
