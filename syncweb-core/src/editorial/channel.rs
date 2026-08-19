use serde::{Deserialize, Serialize};

use crate::constants::CHANNEL_TOPIC_PREFIX;

/// The transport backing a channel's content distribution.
///
/// Gossip channels are ephemeral pub/sub — announcements are only
/// visible to online subscribers.  Catalog channels are backed by
/// iroh-docs and provide persistent, durable discovery with full-text
/// search.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum ChannelBackend {
    /// Ephemeral gossip pub/sub (the default).
    #[default]
    Gossip,
    /// Persistent iroh-docs catalog with FTS indexing.
    Catalog,
}

/// A named content stream for editorial distribution.
///
/// Channels provide **authoritative views** on top of the default,
/// unfiltered gossip catalog.  Publishers announce content into channels
/// once it reaches a given editorial state (e.g. "curated"), and consumers
/// subscribe only to the channels they trust.
///
/// Under the hood a channel is either a separate gossip topic derived from
/// its name (ephemeral), or an iroh-docs catalog namespace (persistent),
/// depending on its [`ChannelBackend`].
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Channel {
    /// Human-readable channel name (e.g. `"curated"`, `"latest"`,
    /// `"publisher-acme"`).
    pub name: String,
    /// Optional human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The transport backing this channel.
    #[serde(default)]
    pub backend: ChannelBackend,
}

impl Channel {
    /// Create a new gossip-backed channel with an optional description.
    #[must_use]
    pub fn new(name: impl Into<String>, description: Option<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            description: description.map(Into::into),
            backend: ChannelBackend::Gossip,
        }
    }

    /// Create a channel with an explicit backend.
    #[must_use]
    pub fn with_backend(
        name: impl Into<String>,
        description: Option<impl Into<String>>,
        backend: ChannelBackend,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.map(Into::into),
            backend,
        }
    }

    /// The gossip topic seed for this channel.
    ///
    /// Topic IDs are derived from a BLAKE3 hash of this seed, so every
    /// channel is a fully independent gossip swarm.
    #[must_use]
    pub fn topic_seed(&self) -> String {
        format!("{CHANNEL_TOPIC_PREFIX}/{name}/v1", name = self.name)
    }

    /// The iroh-docs catalog name for a catalog-backed channel.
    ///
    /// Returns `None` if the channel uses the gossip backend.
    #[must_use]
    pub fn catalog_name(&self) -> Option<String> {
        match self.backend {
            ChannelBackend::Gossip => None,
            ChannelBackend::Catalog => Some(format!("syncweb/catalog/{}", self.name)),
        }
    }

    /// Whether this channel uses the catalog (iroh-docs) backend.
    #[must_use]
    pub const fn is_catalog(&self) -> bool {
        matches!(self.backend, ChannelBackend::Catalog)
    }

    /// Whether content should appear on this channel.
    ///
    /// By default every channel accepts all content; publishers apply
    /// editorial policy *before* calling announce.
    #[must_use]
    pub const fn accepts_all(&self) -> bool {
        true
    }
}
