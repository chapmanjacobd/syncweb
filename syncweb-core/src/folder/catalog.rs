use std::{str::FromStr, time::Duration};

use ed25519_dalek::SigningKey;
use iroh::{Endpoint, PublicKey, address_lookup::memory::MemoryLookup};
use iroh_blobs::{Hash, ticket::BlobTicket};
use iroh_gossip::{
    TopicId,
    api::{Event, GossipSender, GossipTopic},
};
use n0_future::StreamExt;
use semver::Version;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    editorial::Channel,
    error::{Result, SyncwebError},
    node::gossip_service::GossipService,
    node::signed_message::SignedMessage,
};

const CATALOG_TOPIC_SEED: &[u8] = b"syncweb/public-package-catalog/v1";

/// The public gossip topic used for package announcements.
#[must_use]
pub fn catalog_topic() -> TopicId {
    TopicId::from_bytes(*blake3::hash(CATALOG_TOPIC_SEED).as_bytes())
}

/// Derive a gossip topic ID from an editorial channel name.
///
/// Each channel forms a fully independent gossip swarm so that consumers
/// can subscribe only to the curated views they trust.
#[must_use]
pub fn channel_topic_id(name: impl AsRef<str>) -> TopicId {
    let seed = format!("syncweb/editorial/{}/v1", name.as_ref());
    TopicId::from_bytes(*blake3::hash(seed.as_bytes()).as_bytes())
}

/// Shorthand for [`channel_topic_id`] that accepts a [`Channel`].
#[must_use]
pub fn channel_topic(channel: &Channel) -> TopicId {
    channel_topic_id(&channel.name)
}

/// A gossip announcement for a publicly fetchable collection manifest.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PackageAnnouncement {
    pub collection_id: Uuid,
    pub name: String,
    pub version: String,
    pub sequence: u64,
    pub manifest: Hash,
    pub manifest_ticket: String,
    pub publisher: String,
}

impl PackageAnnouncement {
    /// # Errors
    ///
    /// Returns an error if the ticket is invalid or does not identify the
    /// announced manifest.
    pub fn new(
        collection_id: Uuid,
        name: impl Into<String>,
        version: impl Into<String>,
        sequence: u64,
        manifest: Hash,
        manifest_ticket: impl Into<String>,
        publisher: PublicKey,
    ) -> Result<Self> {
        let announcement = Self {
            collection_id,
            name: name.into(),
            version: version.into(),
            sequence,
            manifest,
            manifest_ticket: manifest_ticket.into(),
            publisher: publisher.to_string(),
        };
        announcement.validate()?;
        Ok(announcement)
    }

    /// # Errors
    ///
    /// Returns an error if the announcement has invalid metadata or a ticket
    /// that does not match the manifest hash.
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(SyncwebError::InvalidConfig(
                "package announcement name cannot be empty".to_owned(),
            ));
        }
        if self.sequence == 0 {
            return Err(SyncwebError::InvalidConfig(
                "package announcement sequence must be greater than zero".to_owned(),
            ));
        }
        Version::parse(&self.version)
            .map_err(|error| SyncwebError::InvalidConfig(format!("invalid package version: {error}")))?;
        let ticket = BlobTicket::from_str(&self.manifest_ticket)
            .map_err(|error| SyncwebError::InvalidTicket(error.to_string()))?;
        if ticket.hash() != self.manifest {
            return Err(SyncwebError::InvalidTicket(
                "package manifest ticket does not match its manifest hash".to_owned(),
            ));
        }
        self.publisher
            .parse::<PublicKey>()
            .map_err(|error| SyncwebError::InvalidIdentity(error.to_string()))?;
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if the announcement is invalid or cannot be encoded.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self)
            .map_err(|error| SyncwebError::operation("failed to serialize package announcement", error))
    }

    /// # Errors
    ///
    /// Returns an error if the announcement cannot be decoded or validated.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let announcement: Self = serde_json::from_slice(bytes.as_ref())
            .map_err(|error| SyncwebError::operation("failed to deserialize package announcement", error))?;
        announcement.validate()?;
        Ok(announcement)
    }

    /// Parse the manifest blob ticket carried by this announcement.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket is invalid.
    pub fn ticket(&self) -> Result<BlobTicket> {
        BlobTicket::from_str(&self.manifest_ticket).map_err(|error| SyncwebError::InvalidTicket(error.to_string()))
    }

    fn matches(&self, query: Option<&str>) -> bool {
        query.is_none_or(|needle| {
            self.name.contains(needle)
                || self.collection_id.to_string().contains(needle)
                || self.version.contains(needle)
        })
    }
}

/// Gossip-backed discovery for publicly published package manifests.
#[derive(Clone)]
pub struct PackageCatalog {
    gossip: GossipService,
    signing_key: SigningKey,
}

impl PackageCatalog {
    #[must_use]
    pub fn new(gossip: &GossipService, endpoint: &Endpoint) -> Self {
        Self {
            gossip: gossip.clone(),
            signing_key: SigningKey::from_bytes(&endpoint.secret_key().to_bytes()),
        }
    }

    /// Subscribe to the public package catalog.
    ///
    /// The caller must retain the returned topic while receiving
    /// announcements.
    ///
    /// # Errors
    ///
    /// Returns an error if the gossip subscription cannot be created.
    pub async fn subscribe(&self, bootstrap: Vec<PublicKey>) -> Result<GossipTopic> {
        self.gossip.subscribe(catalog_topic(), bootstrap).await
    }

    /// Subscribe to the public package catalog and wait for a bootstrap
    /// connection.
    ///
    /// # Errors
    ///
    /// Returns an error if the gossip subscription cannot join its bootstrap
    /// peers.
    pub async fn subscribe_and_join(&self, bootstrap: Vec<PublicKey>) -> Result<GossipTopic> {
        self.gossip.subscribe_and_join(catalog_topic(), bootstrap).await
    }

    /// Broadcast an announcement to the public catalog.
    ///
    /// The announcement is Ed25519-signed so receivers can cryptographically
    /// verify its origin.
    ///
    /// # Errors
    ///
    /// Returns an error if the announcement is invalid or cannot be sent.
    pub async fn announce(&self, sender: &GossipSender, announcement: &PackageAnnouncement) -> Result<()> {
        let content = announcement.to_bytes()?;
        let wire = SignedMessage::sign(&self.signing_key, &content);
        self.gossip.publish(sender, wire).await
    }

    /// Subscribe to a specific editorial channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the gossip subscription cannot be created.
    pub async fn subscribe_channel(&self, channel: &Channel, bootstrap: Vec<PublicKey>) -> Result<GossipTopic> {
        self.gossip.subscribe(channel_topic(channel), bootstrap).await
    }

    /// Subscribe to a channel and block until at least one bootstrap peer is
    /// connected.
    ///
    /// # Errors
    ///
    /// Returns an error if the gossip subscription cannot join its bootstrap
    /// peers.
    pub async fn subscribe_channel_and_join(
        &self,
        channel: &Channel,
        bootstrap: Vec<PublicKey>,
    ) -> Result<GossipTopic> {
        self.gossip.subscribe_and_join(channel_topic(channel), bootstrap).await
    }

    /// Collect matching announcements until the timeout expires.
    ///
    /// Each announcement is verified: the Ed25519 signature is checked and the
    /// claimed `publisher` field must match the signing key.
    /// A timeout is a normal end condition because gossip has no finite
    /// response boundary.
    ///
    /// # Errors
    ///
    /// Returns an error if a received announcement fails signature verification.
    pub async fn search(
        &self,
        topic: &mut GossipTopic,
        query: Option<&str>,
        timeout: Duration,
    ) -> Result<Vec<PackageAnnouncement>> {
        let mut announcements = Vec::new();
        let receive = async {
            while let Some(event) = topic.next().await {
                if let Event::Received(message) =
                    event.map_err(|error| SyncwebError::operation("package catalog event failed", error))?
                {
                    // Verify the Ed25519 signature on the gossip message
                    let (verifying_key, content) = SignedMessage::verify(&message.content).map_err(|error| {
                        SyncwebError::operation("package catalog signature verification failed", error)
                    })?;

                    let announcement = PackageAnnouncement::from_bytes(content)?;

                    // Verify that the claimed publisher matches the signing key
                    let claimed_publisher: PublicKey = announcement
                        .publisher
                        .parse()
                        .map_err(|error| SyncwebError::InvalidIdentity(format!("invalid publisher key: {error}")))?;
                    let actual_publisher = PublicKey::from_bytes(&verifying_key.to_bytes()).map_err(|error| {
                        SyncwebError::InvalidIdentity(format!("derived NodeId from verifying key is invalid: {error}"))
                    })?;
                    if claimed_publisher != actual_publisher {
                        return Err(SyncwebError::operation(
                            "package catalog publisher mismatch",
                            format!("claimed {claimed_publisher}, signed by {actual_publisher}"),
                        ));
                    }

                    if announcement.matches(query)
                        && !announcements.iter().any(|item: &PackageAnnouncement| {
                            item.collection_id == announcement.collection_id
                                && item.version == announcement.version
                                && item.manifest == announcement.manifest
                        })
                    {
                        announcements.push(announcement);
                    }
                }
            }
            Ok::<(), SyncwebError>(())
        };
        if let Ok(result) = tokio::time::timeout(timeout, receive).await {
            result?;
        }
        Ok(announcements)
    }

    /// Add the endpoint address for a discovered ticket to a memory lookup.
    ///
    /// This helper is useful to callers that construct a node with a shared
    /// [`MemoryLookup`] and want to fetch an announcement immediately.
    ///
    /// # Errors
    ///
    /// Returns an error if the announcement's manifest ticket is invalid.
    pub fn register_ticket_endpoint(lookup: &MemoryLookup, announcement: &PackageAnnouncement) -> Result<()> {
        let ticket = announcement.ticket()?;
        lookup.add_endpoint_info(ticket.addr().clone());
        Ok(())
    }
}
