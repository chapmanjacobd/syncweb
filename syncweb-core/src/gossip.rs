pub mod daemon_listener;
pub mod signed_topic;
pub mod topic_channel;

pub use daemon_listener::subscribe_topics;
pub use signed_topic::SignedGossipMessage;
pub use topic_channel::TopicChannel;

/// Derive a deterministic gossip [`TopicId`] from a byte-string seed.
#[must_use]
pub fn gossip_topic_id(seed: &[u8]) -> iroh_gossip::TopicId {
    iroh_gossip::TopicId::from_bytes(*blake3::hash(seed).as_bytes())
}
