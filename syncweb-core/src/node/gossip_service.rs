use anyhow::Result;
use bytes::Bytes;
use iroh::PublicKey;
use iroh_gossip::{
    TopicId,
    api::{GossipReceiver, GossipSender, GossipTopic},
    net::Gossip,
};

pub struct GossipService {
    gossip: Gossip,
}

impl GossipService {
    #[must_use]
    pub fn new(gossip: &Gossip) -> Self {
        Self { gossip: gossip.clone() }
    }

    #[must_use]
    pub const fn inner(&self) -> &Gossip {
        &self.gossip
    }

    /// # Errors
    ///
    /// Returns an error if subscribing to the topic fails.
    pub async fn subscribe(&self, topic: TopicId, bootstrap: Vec<PublicKey>) -> Result<GossipTopic> {
        Ok(self.gossip.subscribe(topic, bootstrap).await?)
    }

    /// # Errors
    ///
    /// Returns an error if subscribing or joining the topic fails.
    pub async fn subscribe_and_join(&self, topic: TopicId, bootstrap: Vec<PublicKey>) -> Result<GossipTopic> {
        Ok(self.gossip.subscribe_and_join(topic, bootstrap).await?)
    }

    /// # Errors
    ///
    /// Returns an error if the message cannot be published to the topic.
    pub async fn publish(&self, sender: &GossipSender, message: impl AsRef<[u8]>) -> Result<()> {
        Ok(sender.broadcast(Bytes::copy_from_slice(message.as_ref())).await?)
    }

    #[must_use]
    pub fn split(topic: GossipTopic) -> (GossipSender, GossipReceiver) {
        topic.split()
    }

    #[must_use]
    pub const fn event_stream(topic: GossipTopic) -> GossipTopic {
        topic
    }
}
