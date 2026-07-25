use std::path::Path;

use async_trait::async_trait;
use iroh_blobs::Hash;

use crate::error::Result;

/// Lightweight metadata for a folder or subscription entry.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EntryLike {
    pub path: String,
    pub hash: Hash,
    pub size: u64,
}

/// Common trait for folders and subscriptions that commands can operate on.
#[async_trait]
pub trait FolderLike: Send + Sync {
    fn namespace_id(&self) -> String;
    fn label(&self) -> String;
    fn kind(&self) -> &'static str;
    fn path(&self) -> Option<&Path>;
    async fn list_entries(&self) -> Result<Vec<EntryLike>>;
}

/// A public blob subscription. Stores enough to identify the blob and
/// reconstruct the ticket (hash + provider). Size is cached since it never
/// changes for content-addressed blobs.
#[derive(Clone, Debug)]
pub struct PublicSubscription {
    hash: Hash,
    provider: Option<iroh::EndpointAddr>,
    size: u64,
}

impl PublicSubscription {
    #[must_use]
    pub const fn new(hash: Hash, provider: Option<iroh::EndpointAddr>, size: u64) -> Self {
        Self { hash, provider, size }
    }

    #[must_use]
    pub const fn hash(&self) -> Hash {
        self.hash
    }

    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    #[must_use]
    pub const fn provider(&self) -> Option<&iroh::EndpointAddr> {
        self.provider.as_ref()
    }

    /// Reconstruct a blob ticket from the stored hash and provider.
    /// Returns `None` if no provider was recorded with this subscription.
    #[must_use]
    pub fn ticket(&self) -> Option<iroh_blobs::ticket::BlobTicket> {
        self.provider
            .as_ref()
            .map(|addr| iroh_blobs::ticket::BlobTicket::new(addr.clone(), self.hash, iroh_blobs::BlobFormat::Raw))
    }

    #[must_use]
    pub fn label(&self) -> String {
        let hex = self.hash.to_string();
        format!("{}..", &hex[..hex.len().min(12)])
    }
}

#[async_trait]
impl FolderLike for PublicSubscription {
    fn namespace_id(&self) -> String {
        format!("blob:{}", self.hash())
    }

    fn label(&self) -> String {
        self.label()
    }

    fn kind(&self) -> &'static str {
        "subscription"
    }

    fn path(&self) -> Option<&Path> {
        None
    }

    async fn list_entries(&self) -> Result<Vec<EntryLike>> {
        Ok(vec![EntryLike {
            path: self.label(),
            hash: self.hash(),
            size: self.size(),
        }])
    }
}
