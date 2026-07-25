use std::marker::PhantomData;
use std::sync::Arc;

use bytes::Bytes;
use iroh_gossip::{
    TopicId,
    api::{ApiError, Event, GossipSender},
    net::Gossip,
};
use n0_future::{Stream, StreamExt};

use crate::error::{Result, SyncwebError};
use crate::gossip::signed_topic::SignedGossipMessage;

/// A typed gossip topic that can publish and subscribe to one message type.
///
/// Uses `Gossip` (syncweb's wrapper around `iroh_gossip::net::Gossip`) for
/// lifecycle management. `TopicChannel` only handles publish and
/// per-subscription stream filtering — the underlying gossip connection,
/// subscription, and peer bootstrap are managed by `GossipService`.
pub struct TopicChannel<T: SignedGossipMessage + Send + Sync> {
    gossip: Arc<Gossip>,
    topic_id: TopicId,
    sender: GossipSender,
    _phantom: PhantomData<T>,
}

impl<T: SignedGossipMessage + Send + Sync> TopicChannel<T> {
    /// Create a deterministic topic ID from a byte-string topic name, and
    /// store a sender for publishing.
    #[must_use]
    pub fn new(gossip: Arc<Gossip>, topic_name: &[u8], sender: GossipSender) -> Self {
        Self {
            gossip,
            topic_id: TopicId::from_bytes(*blake3::hash(topic_name).as_bytes()),
            sender,
            _phantom: PhantomData,
        }
    }

    /// Publish a signed message to the topic. All subscribers receive it.
    ///
    /// # Errors
    ///
    /// Returns an error if signature verification or broadcasting fails.
    pub async fn publish(&self, message: &T) -> Result<()> {
        message.verify_signature()?;
        let wire_bytes = message.to_wire_bytes()?;
        let bytes = Bytes::from(wire_bytes);
        self.sender
            .broadcast(bytes)
            .await
            .map_err(|error| SyncwebError::operation("failed to publish gossip message", error))?;
        Ok(())
    }

    /// Receive verified messages from an already-subscribed gossip event stream.
    ///
    /// The caller must already have subscribed to `self.topic_id` via
    /// `GossipService`. This method wraps the raw event stream with
    /// deserialization and signature verification.
    ///
    /// Returns a stream of verified, deserialized `T` messages. Messages that
    /// fail deserialization or signature verification are silently dropped
    /// (logged at debug level).
    pub fn receive_from(
        &self,
        stream: impl Stream<Item = std::result::Result<Event, ApiError>> + Send + 'static,
    ) -> impl Stream<Item = T> + Send {
        stream.filter_map(|event_result| match event_result {
            Ok(Event::Received(msg)) => {
                let parsed = match T::from_wire_bytes(&msg.content) {
                    Ok(m) => m,
                    Err(error) => {
                        tracing::debug!("failed to deserialize gossip message: {error}");
                        return None;
                    }
                };
                if let Err(error) = parsed.verify_signature() {
                    tracing::debug!("gossip message signature verification failed: {error}");
                    return None;
                }
                Some(parsed)
            }
            Ok(_) => None,
            Err(error) => {
                tracing::debug!("gossip stream error: {error}");
                None
            }
        })
    }

    /// Collect messages matching a filter within a timeout.
    ///
    /// The caller must already be subscribed to the topic via `GossipService`.
    /// `stream` is the raw gossip event stream from the subscription.
    ///
    /// # Errors
    ///
    /// Returns an empty vector on timeout (not an error).
    #[expect(clippy::future_not_send)]
    pub async fn collect_for(
        &self,
        stream: impl Stream<Item = std::result::Result<Event, ApiError>> + Send + 'static,
        filter: impl Fn(&T) -> bool + Send + 'static,
        timeout_duration: std::time::Duration,
    ) -> Result<Vec<T>> {
        let mut filtered = Box::pin(self.receive_from(stream));
        let mut results = Vec::new();
        let _ = tokio::time::timeout(timeout_duration, async {
            while let Some(msg) = filtered.next().await {
                if filter(&msg) {
                    results.push(msg);
                }
            }
        })
        .await;
        Ok(results)
    }

    #[must_use]
    pub const fn topic_id(&self) -> TopicId {
        self.topic_id
    }

    #[must_use]
    pub fn gossip(&self) -> &Gossip {
        &self.gossip
    }
}
