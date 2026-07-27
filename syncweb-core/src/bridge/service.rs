use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use iroh_docs::NamespaceId;
use iroh_gossip::api::GossipSender;
use tokio::sync::{RwLock, broadcast};

use crate::{
    error::{Result, SyncwebError},
    node::{gossip_service::GossipService, iroh_node::IrohNode},
};

const DOC_MAP_FILENAME: &str = "doc_map.json";

#[derive(Clone, Debug)]
pub struct ConnectedPeer {
    pub node_id: String,
    pub first_seen_secs: u64,
    pub last_seen_secs: u64,
}

#[derive(Clone)]
pub struct BridgeService {
    node: Arc<IrohNode>,
    doc_map: Arc<RwLock<HashMap<String, NamespaceId>>>,
    gossip_topics: Arc<RwLock<HashMap<String, GossipSender>>>,
    pub connected_peers: Arc<RwLock<HashMap<String, ConnectedPeer>>>,
    blocked_peers: Arc<RwLock<HashSet<String>>>,
    shutdown: broadcast::Sender<()>,
    data_dir: PathBuf,
}

impl BridgeService {
    /// Create a new bridge service.
    ///
    /// # Errors
    ///
    /// Returns an error if the blocklist or doc map cannot be loaded.
    pub fn new(node: Arc<IrohNode>, data_dir: PathBuf, shutdown: broadcast::Sender<()>) -> Result<Self> {
        let blocklist_path = data_dir.join("blocklist.txt");
        let blocked_peers = load_blocklist(&blocklist_path).unwrap_or_default();
        let doc_map = load_doc_map(&data_dir.join(DOC_MAP_FILENAME)).unwrap_or_default();

        Ok(Self {
            node,
            doc_map: Arc::new(RwLock::new(doc_map)),
            gossip_topics: Arc::new(RwLock::new(HashMap::new())),
            connected_peers: Arc::new(RwLock::new(HashMap::new())),
            blocked_peers: Arc::new(RwLock::new(blocked_peers)),
            shutdown,
            data_dir,
        })
    }

    #[must_use]
    pub const fn node(&self) -> &Arc<IrohNode> {
        &self.node
    }

    #[must_use]
    pub fn shutdown_receiver(&self) -> broadcast::Receiver<()> {
        self.shutdown.subscribe()
    }

    #[must_use]
    pub fn is_blocked(&self, node_id: &str) -> bool {
        self.blocked_peers.blocking_read().contains(node_id)
    }

    /// Get or create a doc namespace for a `collection_id`.
    ///
    /// # Errors
    ///
    /// Returns an error if the namespace cannot be created or opened.
    pub async fn get_or_create_collection(&self, collection_id: &str) -> Result<NamespaceId> {
        {
            let map = self.doc_map.read().await;
            if let Some(ns) = map.get(collection_id).copied() {
                return Ok(ns);
            }
        }

        let mut map = self.doc_map.write().await;
        if let Some(ns) = map.get(collection_id).copied() {
            return Ok(ns);
        }

        let doc = self.node.docs_engine().create_namespace().await?;
        let namespace = self.node.docs_engine().namespace_id(&doc);
        self.node.docs_engine().start_sync(&doc, Vec::new()).await?;
        map.insert(collection_id.to_owned(), namespace);
        drop(map);
        self.persist_doc_map().await;
        Ok(namespace)
    }

    /// Get an existing collection namespace.
    ///
    /// # Errors
    ///
    /// Returns an error if the namespace cannot be opened.
    pub async fn get_collection(&self, collection_id: &str) -> Result<NamespaceId> {
        let map = self.doc_map.read().await;
        map.get(collection_id).copied().ok_or_else(|| {
            SyncwebError::operation(
                "collection not found",
                format!("no namespace for collection_id: {collection_id}"),
            )
        })
    }

    /// Insert a doc namespace for a `collection_id` (used by `import_collection`).
    pub async fn insert_collection(&self, collection_id: &str, namespace: NamespaceId) {
        {
            let mut map = self.doc_map.write().await;
            map.insert(collection_id.to_owned(), namespace);
        }
        self.persist_doc_map().await;
    }

    async fn persist_doc_map(&self) {
        let path = self.data_dir.join(DOC_MAP_FILENAME);
        let content = self.serialize_doc_map().await;
        if let Some(data) = content {
            let _ = tokio::fs::write(&path, data).await;
        }
    }

    async fn serialize_doc_map(&self) -> Option<String> {
        let serializable: HashMap<String, NamespaceId> = {
            let guard = self.doc_map.read().await;
            guard.iter().map(|(k, v)| (k.clone(), *v)).collect()
        };
        serde_json::to_string_pretty(&serializable).ok()
    }

    /// Subscribe to a gossip topic.
    ///
    /// # Errors
    ///
    /// Returns an error if subscription fails.
    pub async fn subscribe_gossip(&self, topic_str: &str, with_discovery: bool) -> Result<GossipSender> {
        if let Some(sender) = self.gossip_topics.read().await.get(topic_str) {
            return Ok(sender.clone());
        }

        let topic_id = topic_id_from_str(topic_str);
        let topic = if with_discovery {
            self.node
                .gossip_service()
                .subscribe_and_join(topic_id, Vec::new())
                .await?
        } else {
            self.node.gossip_service().subscribe(topic_id, Vec::new()).await?
        };
        let (sender, _receiver) = GossipService::split(topic);

        self.gossip_topics
            .write()
            .await
            .insert(topic_str.to_owned(), sender.clone());
        Ok(sender)
    }

    /// Leave a gossip topic.
    pub async fn leave_gossip(&self, topic_str: &str) {
        self.gossip_topics.write().await.remove(topic_str);
    }

    /// Send a gossip message to a topic.
    ///
    /// # Errors
    ///
    /// Returns an error if the topic is not subscribed or publishing fails.
    pub async fn send_gossip(&self, topic_str: &str, message: &[u8]) -> Result<()> {
        let sender = self
            .gossip_topics
            .read()
            .await
            .get(topic_str)
            .ok_or_else(|| {
                SyncwebError::operation("gossip topic not subscribed", format!("not subscribed to: {topic_str}"))
            })?
            .clone();
        self.node.gossip_service().publish(&sender, message).await
    }

    /// Add a connected peer.
    pub fn add_peer(&self, node_id: String, first_seen: u64, last_seen: u64) {
        let peer = ConnectedPeer {
            node_id: node_id.clone(),
            first_seen_secs: first_seen,
            last_seen_secs: last_seen,
        };
        self.connected_peers.blocking_write().insert(node_id, peer);
    }

    /// Block a peer.
    ///
    /// # Errors
    ///
    /// Returns an error if the blocklist file cannot be written.
    pub fn block_peer(&self, node_id: &str) -> Result<()> {
        self.blocked_peers.blocking_write().insert(node_id.to_owned());
        self.save_blocklist()
    }

    /// Unblock a peer.
    ///
    /// # Errors
    ///
    /// Returns an error if the blocklist file cannot be written.
    pub fn unblock_peer(&self, node_id: &str) -> Result<()> {
        self.blocked_peers.blocking_write().remove(node_id);
        self.save_blocklist()
    }

    /// Get blocked peers.
    #[must_use]
    pub fn get_blocked_peers(&self) -> Vec<String> {
        let peers = self.blocked_peers.blocking_read();
        peers.iter().cloned().collect()
    }

    fn save_blocklist(&self) -> Result<()> {
        let path = self.data_dir.join("blocklist.txt");
        let peers = self.get_blocked_peers();
        let content = peers.join("\n");
        std::fs::write(&path, content).map_err(|error| SyncwebError::operation("failed to write blocklist", error))?;
        Ok(())
    }
}

fn topic_id_from_str(topic_str: &str) -> iroh_gossip::TopicId {
    let hash = *blake3::hash(topic_str.as_bytes()).as_bytes();
    let mut arr = [0_u8; 32];
    arr.copy_from_slice(&hash[..32]);
    iroh_gossip::TopicId::from_bytes(arr)
}

fn load_blocklist(path: &Path) -> Result<HashSet<String>> {
    let content =
        std::fs::read_to_string(path).map_err(|error| SyncwebError::operation("failed to read blocklist", error))?;
    Ok(content.lines().map(String::from).collect())
}

fn load_doc_map(path: &Path) -> Result<HashMap<String, NamespaceId>> {
    let content =
        std::fs::read_to_string(path).map_err(|error| SyncwebError::operation("failed to read doc map", error))?;
    serde_json::from_str(&content).map_err(|error| SyncwebError::operation("failed to parse doc map", error))
}
