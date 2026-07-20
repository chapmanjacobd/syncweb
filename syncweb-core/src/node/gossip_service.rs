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
    pub fn new(gossip: &Gossip) -> Self {
        Self {
            gossip: gossip.clone(),
        }
    }

    pub fn inner(&self) -> &Gossip {
        &self.gossip
    }

    pub async fn subscribe(
        &self,
        topic: TopicId,
        bootstrap: Vec<PublicKey>,
    ) -> Result<GossipTopic> {
        Ok(self.gossip.subscribe(topic, bootstrap).await?)
    }

    pub async fn subscribe_and_join(
        &self,
        topic: TopicId,
        bootstrap: Vec<PublicKey>,
    ) -> Result<GossipTopic> {
        Ok(self.gossip.subscribe_and_join(topic, bootstrap).await?)
    }

    pub async fn publish(&self, sender: &GossipSender, message: impl AsRef<[u8]>) -> Result<()> {
        Ok(sender
            .broadcast(Bytes::copy_from_slice(message.as_ref()))
            .await?)
    }

    pub fn split(topic: GossipTopic) -> (GossipSender, GossipReceiver) {
        topic.split()
    }

    pub fn event_stream(topic: GossipTopic) -> GossipTopic {
        topic
    }
}
