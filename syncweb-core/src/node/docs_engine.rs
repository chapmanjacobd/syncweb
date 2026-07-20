use anyhow::Result;
use bytes::Bytes;
use iroh_blobs::Hash;
use iroh_docs::{AuthorId, Entry, NamespaceId, api::Doc, engine::LiveEvent, protocol::Docs};

pub struct DocsEngine {
    docs: Docs,
}

impl DocsEngine {
    pub fn new(docs: &Docs) -> Self {
        Self { docs: docs.clone() }
    }

    pub fn inner(&self) -> &Docs {
        &self.docs
    }

    pub async fn create_namespace(&self) -> Result<Doc> {
        self.docs.create().await
    }

    pub async fn author(&self) -> Result<AuthorId> {
        self.docs.author_default().await
    }

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

    pub async fn get(
        &self,
        doc: &Doc,
        author: AuthorId,
        key: impl AsRef<[u8]>,
    ) -> Result<Option<Entry>> {
        doc.get_exact(author, key, false).await
    }

    pub async fn watch(
        &self,
        doc: &Doc,
    ) -> Result<impl n0_future::Stream<Item = Result<LiveEvent>> + Send + Unpin + 'static> {
        doc.subscribe().await
    }

    pub fn namespace_id(&self, doc: &Doc) -> NamespaceId {
        doc.id()
    }
}
