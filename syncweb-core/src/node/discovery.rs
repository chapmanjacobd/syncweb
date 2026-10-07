use distributed_topic_tracker::{
    AutoDiscoveryGossip, Config, DefaultSecretRotation, RecordPublisher, RotationHandle, Topic, TopicId,
};
use ed25519_dalek::SigningKey;
use iroh::{Endpoint, EndpointAddr, PublicKey, TransportAddr};
use iroh_base::CustomAddr;
use iroh_docs::NamespaceId;
use iroh_gossip::net::Gossip;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;

use crate::error::{Result, SyncwebError};

/// How often a node republishes its address presence record and refreshes its
/// view of peers for the DHT topic.
///
/// The tracker's bootstrap records only carry node ids, so a peer that learned
/// another node out of band (a node-id-only ticket, the DHT itself, or a prior
/// connection) has no address to dial. Each node therefore also publishes a
/// presence record carrying its full [`EndpointAddr`] (direct addresses, iroh
/// relay, and the Syncthing relay custom address) and, in the same background
/// loop, reads other nodes' presence records so `find_peers` can return
/// resolved addresses instead of bare node ids.
const PRESENCE_INTERVAL: Duration = Duration::from_secs(10);

/// Marker distinguishing a presence record from the tracker's own bootstrap
/// records, which carry a fixed `GossipRecordContent` layout. A peer ignores
/// any record whose tag does not match.
const PRESENCE_TAG: [u8; 8] = *b"syncweb!";

/// A peer address announcement stored in the namespace's DHT topic.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PresenceRecord {
    tag: [u8; 8],
    node_id: [u8; 32],
    addrs: Vec<TransportAddr>,
}

/// A joined topic and the record publishers used to read/write the DHT.
#[derive(Clone)]
struct TopicEntry {
    topic: Topic,
    bootstrap: RecordPublisher,
    presence: RecordPublisher,
}

/// Peer addresses discovered per topic, keyed by topic hash.
type PeerCache = Arc<Mutex<HashMap<[u8; 32], HashMap<PublicKey, EndpointAddr>>>>;

#[derive(Clone)]
pub struct TopicTracker {
    gossip: Gossip,
    topics: Arc<Mutex<HashMap<[u8; 32], TopicEntry>>>,
    peers: PeerCache,
    endpoint: Endpoint,
    relay_addr: Option<CustomAddr>,
}

impl TopicTracker {
    #[must_use]
    pub fn new(gossip: &Gossip, endpoint: &Endpoint, relay_addr: Option<CustomAddr>) -> Self {
        Self {
            gossip: gossip.clone(),
            topics: Arc::new(Mutex::new(HashMap::new())),
            peers: Arc::new(Mutex::new(HashMap::new())),
            endpoint: endpoint.clone(),
            relay_addr,
        }
    }

    /// # Errors
    ///
    /// Returns an error if joining the topic fails.
    pub async fn announce(&self, namespace_id: NamespaceId) -> Result<()> {
        self.topic(namespace_id).await?;
        Ok(())
    }

    /// Return the peers discovered for a namespace so far, with any addresses
    /// that have been resolved via the DHT presence records. This is
    /// deliberately non-blocking: the DHT reads happen in the background loop,
    /// so this call never stalls a daemon task on network latency.
    ///
    /// # Errors
    ///
    /// Returns an error if the topic cannot be joined or gossip neighbors cannot be retrieved.
    pub async fn find_peers(&self, namespace_id: NamespaceId) -> Result<Vec<EndpointAddr>> {
        let entry = self.topic(namespace_id).await?;
        let key = *namespace_id.as_bytes();
        let own_id = self.endpoint.id();
        let mut peers: HashMap<PublicKey, EndpointAddr> =
            self.peers.lock().await.get(&key).cloned().unwrap_or_default();

        // Include currently-connected gossip neighbors (as bare node ids) so a
        // blob fetch or sync that runs before the background collector's first
        // pass can still reach already-connected peers.
        let receiver = entry
            .topic
            .gossip_receiver()
            .await
            .map_err(|error| SyncwebError::operation("failed to get discovery receiver", error))?;
        let neighbors = receiver
            .neighbors()
            .await
            .map_err(|error| SyncwebError::operation("failed to find discovery peers", error))?;
        for neighbor in neighbors {
            if neighbor != own_id {
                peers.entry(neighbor).or_insert_with(|| EndpointAddr::new(neighbor));
            }
        }

        Ok(peers.into_values().collect())
    }

    async fn topic(&self, namespace_id: NamespaceId) -> Result<TopicEntry> {
        let key = *namespace_id.as_bytes();
        let existing = self.topics.lock().await.get(&key).cloned();
        if let Some(entry) = existing {
            return Ok(entry);
        }

        let topic_id = TopicId::from_hash(&key);
        let secret = namespace_id.as_bytes().to_vec();
        let node_secret = self.endpoint.secret_key().to_bytes();
        let signing_key = SigningKey::from_bytes(&node_secret);
        let bootstrap = RecordPublisher::builder(topic_id.clone(), signing_key, secret.clone())
            .config(Config::default())
            .secret_rotation(RotationHandle::new(DefaultSecretRotation))
            .build();
        let topic = self
            .gossip
            .subscribe_and_join_with_auto_discovery_no_wait(bootstrap.clone())
            .await
            .map_err(|error| SyncwebError::operation("failed to join discovery topic", error))?;
        let presence = RecordPublisher::builder(topic_id, presence_signing_key(&node_secret), secret)
            .config(Config::default())
            .build();

        let entry = TopicEntry {
            topic,
            bootstrap,
            presence,
        };
        self.topics.lock().await.insert(key, entry.clone());
        Self::spawn_discovery_loop(
            entry.clone(),
            self.endpoint.clone(),
            self.relay_addr.clone(),
            self.peers.clone(),
            key,
        );
        Ok(entry)
    }

    fn spawn_discovery_loop(
        entry: TopicEntry,
        endpoint: Endpoint,
        relay_addr: Option<CustomAddr>,
        peers: PeerCache,
        topic_key: [u8; 32],
    ) {
        tokio::spawn(async move {
            let cancel = tokio_util::sync::CancellationToken::new();
            let mut ticker = tokio::time::interval(PRESENCE_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let own_id = endpoint.id();
            loop {
                ticker.tick().await;
                publish_presence(&entry, &endpoint, relay_addr.as_ref(), cancel.clone()).await;
                let discovered = collect_peers(&entry, own_id, cancel.clone()).await;
                peers.lock().await.insert(topic_key, discovered);
            }
        });
    }
}

/// Derive a stable signing key for presence records from the node secret so
/// presence records do not collide with the tracker's bootstrap records in the
/// DHT's per-publisher dedup.
fn presence_signing_key(node_secret: &[u8; 32]) -> SigningKey {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(b"syncweb-presence");
    hasher.update(node_secret);
    let seed: [u8; 32] = hasher.finalize().into();
    SigningKey::from_bytes(&seed)
}

/// The addresses to advertise: the endpoint's current direct and relay
/// addresses plus the Syncthing relay custom address when relaying is enabled.
fn presence_addrs(endpoint: &Endpoint, relay_addr: Option<&CustomAddr>) -> Vec<TransportAddr> {
    let mut addrs: Vec<TransportAddr> = endpoint.addr().addrs.into_iter().collect();
    if let Some(relay) = relay_addr {
        addrs.push(TransportAddr::Custom(relay.clone()));
    }
    addrs
}

/// Publish this node's address presence record for the current minute.
async fn publish_presence(
    entry: &TopicEntry,
    endpoint: &Endpoint,
    relay_addr: Option<&CustomAddr>,
    cancel: tokio_util::sync::CancellationToken,
) {
    let content = PresenceRecord {
        tag: PRESENCE_TAG,
        node_id: *endpoint.id().as_bytes(),
        addrs: presence_addrs(endpoint, relay_addr),
    };
    let Ok(record) = entry
        .presence
        .new_record(distributed_topic_tracker::unix_minute(0), content)
    else {
        return;
    };
    if let Err(error) = entry.presence.publish_record(record, cancel).await {
        tracing::debug!(%error, "failed to publish discovery presence record");
    }
}

/// Read peer addresses (presence records and node-id-only bootstrap records)
/// and gossip neighbors for the current and previous minute.
async fn collect_peers(
    entry: &TopicEntry,
    own_id: PublicKey,
    cancel: tokio_util::sync::CancellationToken,
) -> HashMap<PublicKey, EndpointAddr> {
    let mut peers: HashMap<PublicKey, EndpointAddr> = HashMap::new();
    let now = distributed_topic_tracker::unix_minute(0);
    for minute in [now.saturating_sub(1), now] {
        let records = entry
            .bootstrap
            .get_records(minute, cancel.clone())
            .await
            .unwrap_or_default();
        for record in records {
            if let Ok(presence) = record.content::<PresenceRecord>()
                && presence.tag == PRESENCE_TAG
            {
                if let Some(node_id) = PublicKey::from_bytes(&presence.node_id).ok()
                    && node_id != own_id
                {
                    peers
                        .entry(node_id)
                        .or_insert_with(|| EndpointAddr::from_parts(node_id, presence.addrs.clone()));
                }
                continue;
            }
            if let Some(node_id) = PublicKey::from_bytes(&record.pub_key()).ok()
                && node_id != own_id
            {
                peers.entry(node_id).or_insert_with(|| EndpointAddr::new(node_id));
            }
        }
    }

    if let Ok(receiver) = entry.topic.gossip_receiver().await
        && let Ok(neighbors) = receiver
            .neighbors()
            .await
            .map_err(|error| SyncwebError::operation("failed to find discovery peers", error))
    {
        for neighbor in neighbors {
            if neighbor != own_id {
                peers.entry(neighbor).or_insert_with(|| EndpointAddr::new(neighbor));
            }
        }
    }

    peers
}
