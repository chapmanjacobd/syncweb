use anyhow::Result;

/// Topic-based discovery placeholder.
/// Will use distributed-topic-tracker types in a follow-up.
pub struct TopicTracker;

impl TopicTracker {
    pub async fn new(_gossip: &iroh_gossip::net::Gossip, _endpoint: &iroh::Endpoint) -> Result<Self> {
        Ok(Self)
    }
}
