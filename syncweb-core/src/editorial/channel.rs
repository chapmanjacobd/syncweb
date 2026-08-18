use serde::{Deserialize, Serialize};

/// A named content stream for editorial distribution.
///
/// Channels provide **authoritative views** on top of the default,
/// unfiltered gossip catalog.  Publishers announce content into channels
/// once it reaches a given editorial state (e.g. "curated"), and consumers
/// subscribe only to the channels they trust.
///
/// Under the hood a channel is a separate gossip topic derived from its
/// name, so channel membership is inherently opt-in and decentralised.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Channel {
    /// Human-readable channel name (e.g. `"curated"`, `"latest"`,
    /// `"publisher-acme"`).
    pub name: String,
    /// Optional human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// If non-empty, only announcements from these publishers are accepted.
    ///
    /// Each entry is a hex-encoded Ed25519 public key (with or without the
    /// `ed25519:` prefix).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_publishers: Vec<String>,
}

impl Channel {
    /// Create a new channel with an optional description.
    #[must_use]
    pub fn new(name: impl Into<String>, description: Option<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            description: description.map(Into::into),
            allowed_publishers: Vec::new(),
        }
    }

    /// Create a channel with an explicit publisher allowlist.
    #[must_use]
    pub fn with_publishers(
        name: impl Into<String>,
        description: Option<impl Into<String>>,
        allowed_publishers: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.map(Into::into),
            allowed_publishers,
        }
    }

    /// The gossip topic seed for this channel.
    ///
    /// Topic IDs are derived from a BLAKE3 hash of this seed, so every
    /// channel is a fully independent gossip swarm.
    #[must_use]
    pub fn topic_seed(&self) -> String {
        format!("syncweb/editorial/{}/v1", self.name)
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
