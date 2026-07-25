use iroh_gossip::{TopicId, api::GossipTopic};

use crate::error::Result;
use crate::node::gossip_service::GossipService;

/// Subscribe to multiple gossip topics and return the subscribed topics.
///
/// The caller can split each `GossipTopic` into sender/receiver pairs and
/// create `TopicChannel` instances for typed publish and receive.
///
/// # Errors
///
/// Returns an error if any subscription fails.
pub async fn subscribe_topics(
    gossip_service: &GossipService,
    topics: &[(TopicId, Vec<iroh::PublicKey>)],
) -> Result<Vec<GossipTopic>> {
    let mut subscribed = Vec::with_capacity(topics.len());
    for (topic_id, bootstrap) in topics {
        let topic = gossip_service.subscribe(*topic_id, bootstrap.clone()).await?;
        subscribed.push(topic);
    }
    Ok(subscribed)
}
