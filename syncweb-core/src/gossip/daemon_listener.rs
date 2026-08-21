use iroh_gossip::TopicId;
use n0_future::StreamExt;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::error::Result;
use crate::gossip::signed_topic::SignedGossipMessage;
use crate::gossip::topic_channel::TopicChannel;
use crate::node::gossip_service::GossipService;

/// Spawn a background listener that subscribes to `topic_id`, verifies each
/// `T` message, and passes it to `handler` for persistence.
///
/// The returned `JoinHandle` runs until shutdown, the topic stream closes, or
/// the handler returns `false`. Errors are logged and end the listener. The
/// handler should be tolerant of individual bad messages (log-and-continue)
/// unless a persistent failure should stop the listener.
pub fn spawn_topic_listener<T, H>(
    gossip_service: GossipService,
    topic_id: TopicId,
    shutdown: broadcast::Receiver<()>,
    name: &'static str,
    handler: H,
) -> JoinHandle<()>
where
    T: SignedGossipMessage + Send + Sync + 'static,
    H: FnMut(T) -> Result<bool> + Send + 'static,
{
    tokio::spawn(async move {
        if let Err(error) = listen_on_topic(gossip_service, topic_id, shutdown, handler).await {
            tracing::error!(%error, topic = name, "gossip listener failed");
        }
    })
}

async fn listen_on_topic<T, H>(
    gossip_service: GossipService,
    topic_id: TopicId,
    mut shutdown: broadcast::Receiver<()>,
    mut handler: H,
) -> Result<()>
where
    T: SignedGossipMessage + Send + Sync + 'static,
    H: FnMut(T) -> Result<bool> + Send,
{
    let (channel, receiver) = TopicChannel::<T>::open_and_join(&gossip_service, topic_id, Vec::new()).await?;
    let mut stream = channel.receive_from(receiver);
    loop {
        tokio::select! {
            _ = shutdown.recv() => {
                tracing::debug!(topic = %topic_id, "gossip listener shutting down");
                break Ok(());
            }
            msg = stream.next() => {
                let Some(message) = msg else {
                    break Ok(());
                };
                match handler(message) {
                    Ok(true) => {}
                    Ok(false) => break Ok(()),
                    Err(error) => break Err(error),
                }
            }
        }
    }
}
