use anyhow::Result;
use distributed_topic_tracker::{
    AutoDiscoveryGossip, Config, DefaultSecretRotation, RecordPublisher, RotationHandle, Topic, TopicId,
};
use ed25519_dalek::SigningKey;
use iroh::{Endpoint, PublicKey};
use iroh_docs::NamespaceId;
use iroh_gossip::net::Gossip;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct TopicTracker {
    gossip: Gossip,
    signing_key: SigningKey,
    topics: Arc<Mutex<HashMap<[u8; 32], Topic>>>,
}

impl TopicTracker {
    #[must_use]
    pub fn new(gossip: &Gossip, endpoint: &Endpoint) -> Self {
        Self {
            gossip: gossip.clone(),
            signing_key: SigningKey::from_bytes(&endpoint.secret_key().to_bytes()),
            topics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// # Errors
    ///
    /// Returns an error if joining the topic fails.
    pub async fn announce(&self, namespace_id: NamespaceId) -> Result<()> {
        let _ = self.topic(namespace_id).await?;
        Ok(())
    }

    /// # Errors
    ///
    /// Returns an error if the topic cannot be joined or gossip neighbors cannot be retrieved.
    pub async fn find_peers(&self, namespace_id: NamespaceId) -> Result<Vec<PublicKey>> {
        let topic = self.topic(namespace_id).await?;
        let receiver = topic.gossip_receiver().await?;
        Ok(receiver.neighbors().await?.into_iter().collect())
    }

    async fn topic(&self, namespace_id: NamespaceId) -> Result<Topic> {
        let key = *namespace_id.as_bytes();
        let existing = self.topics.lock().await.get(&key).cloned();
        if let Some(topic) = existing {
            return Ok(topic);
        }

        let topic_id = TopicId::from_hash(&key);
        let publisher = RecordPublisher::builder(topic_id, self.signing_key.clone(), namespace_id.as_bytes().to_vec())
            .config(Config::default())
            .secret_rotation(RotationHandle::new(DefaultSecretRotation))
            .build();
        let topic = self
            .gossip
            .subscribe_and_join_with_auto_discovery_no_wait(publisher)
            .await?;
        self.topics.lock().await.insert(key, topic.clone());
        Ok(topic)
    }
}
