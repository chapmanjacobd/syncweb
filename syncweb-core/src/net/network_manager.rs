use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use iroh::PublicKey;
use iroh_docs::NamespaceId;
use iroh_gossip::{TopicId, api::GossipTopic};

use crate::node::gossip_service::GossipService;
use crate::storage::node_db::NodeDatabase;
use crate::{Result, SyncwebError};

use super::network::{Network, NetworkId, NetworkOptions, NetworkTicket, network_topic};
use super::network_log::{NetworkEventType, NetworkLogger};

/// Persistent manager for network membership and folder associations.
#[derive(Clone, Debug)]
pub struct NetworkManager {
    db: NodeDatabase,
    local_node: PublicKey,
    networks: HashMap<NetworkId, Network>,
    logger: Option<NetworkLogger>,
    member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
    has_public_network: Arc<AtomicBool>,
}

impl NetworkManager {
    /// Open the network database.
    ///
    /// # Errors
    ///
    /// Returns an error if existing state cannot be read.
    pub fn new(
        db: NodeDatabase,
        local_node: PublicKey,
        member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
        has_public_network: Arc<AtomicBool>,
    ) -> Result<Self> {
        let networks = db
            .list_networks()?
            .into_iter()
            .map(|network| (network.id, network))
            .collect();
        let mgr = Self {
            db,
            local_node,
            networks,
            logger: None,
            member_keys,
            has_public_network,
        };
        mgr.sync_member_keys();
        Ok(mgr)
    }

    /// Open the network database with a network logger.
    ///
    /// # Errors
    ///
    /// Returns an error if existing state cannot be read.
    pub fn with_logger(
        db: NodeDatabase,
        local_node: PublicKey,
        logger: NetworkLogger,
        member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
        has_public_network: Arc<AtomicBool>,
    ) -> Result<Self> {
        let mut mgr = Self::new(db, local_node, member_keys, has_public_network)?;
        mgr.logger = Some(logger);
        Ok(mgr)
    }

    /// Return a reference to the network logger, if set.
    #[must_use]
    pub const fn logger(&self) -> &Option<NetworkLogger> {
        &self.logger
    }

    /// Set the network logger after construction.
    pub fn set_logger(&mut self, logger: NetworkLogger) {
        self.logger = Some(logger);
    }

    /// Return a reference to the underlying database.
    #[must_use]
    pub const fn database(&self) -> &NodeDatabase {
        &self.db
    }

    /// Return the local node's public key.
    #[must_use]
    pub const fn local_node(&self) -> &iroh::PublicKey {
        &self.local_node
    }

    /// Create and persist a network owned by the local node.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty/duplicate name or failed persistence.
    pub fn create(&mut self, name: &str, options: NetworkOptions) -> Result<NetworkId> {
        self.create_with_doc_ticket(name, options, None)
    }

    /// Create a network with an optional membership doc ticket.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty/duplicate name or failed persistence.
    pub fn create_with_doc_ticket(
        &mut self,
        name: &str,
        options: NetworkOptions,
        doc_ticket: Option<String>,
    ) -> Result<NetworkId> {
        let normalized = name.trim();
        if normalized.is_empty() {
            return Err(SyncwebError::InvalidConfig("network name cannot be empty".to_owned()));
        }
        let id = NetworkId::from_name(normalized);
        if self.networks.contains_key(&id) {
            return Err(SyncwebError::InvalidConfig(format!(
                "network {normalized:?} already exists"
            )));
        }
        let mut network = Network::new(normalized, self.local_node, options);
        network.doc_ticket = doc_ticket;
        self.db.create_network(&network)?;
        self.networks.insert(id, network.clone());
        self.log_event(
            &id,
            NetworkEventType::MemberAdded,
            Some(&self.local_node),
            Some("created"),
        );
        if network.is_invite_only() {
            self.log_event(
                &id,
                NetworkEventType::TicketCreated,
                None,
                Some("invite-only network created"),
            );
        }
        Ok(id)
    }

    /// Join and persist the network represented by a ticket.
    ///
    /// # Errors
    ///
    /// Returns an error if the ticket is for another device or persistence fails.
    pub fn join(&mut self, ticket: NetworkTicket) -> Result<NetworkId> {
        if ticket.name.trim().is_empty() || NetworkId::from_name(ticket.name.trim()) != ticket.network_id {
            return Err(SyncwebError::InvalidTicket(
                "network ticket ID does not match its network name".to_owned(),
            ));
        }
        if ticket.invited_node.is_some_and(|invited| invited != self.local_node) {
            return Err(SyncwebError::InvalidTicket(
                "network ticket was issued for another device".to_owned(),
            ));
        }
        let mut members = ticket.members;
        members.insert(self.local_node);
        let network = Network {
            id: ticket.network_id,
            name: ticket.name,
            label: ticket.label,
            topic: network_topic(ticket.network_id),
            owner: ticket.owner,
            members,
            folders: ticket.folders,
            shared_secret: ticket.shared_secret,
            doc_ticket: ticket.doc_ticket,
            is_public: ticket.is_public,
        };
        let id = network.id;
        self.db.create_network(&network)?;
        self.networks.insert(id, network);
        self.log_event(
            &id,
            NetworkEventType::MemberAdded,
            Some(&self.local_node),
            Some("joined via ticket"),
        );
        self.sync_member_keys();
        self.log_event(&id, NetworkEventType::TicketAccepted, None, Some("network joined"));
        Ok(id)
    }

    /// Leave a network and remove its local state.
    ///
    /// # Errors
    ///
    /// Returns an error if the network does not exist or persistence fails.
    pub fn leave(&mut self, id: NetworkId) -> Result<()> {
        self.log_event(
            &id,
            NetworkEventType::MemberRemoved,
            Some(&self.local_node),
            Some("left network"),
        );
        self.networks
            .remove(&id)
            .ok_or_else(|| SyncwebError::FolderNotFound(format!("network {id}")))?;
        self.sync_member_keys();
        self.db.delete_network(id)
    }

    /// Generate a device-bound invitation and add the device as a member.
    ///
    /// # Errors
    ///
    /// Returns an error if the network does not exist, the local node is not
    /// its owner, or persistence fails.
    pub fn invite(&mut self, id: NetworkId, device: PublicKey) -> Result<NetworkTicket> {
        let network = self.network_mut_as_owner(id)?;
        network.members.insert(device);
        let ticket = NetworkTicket {
            network_id: network.id,
            name: network.name.clone(),
            label: network.label.clone(),
            owner: network.owner,
            invited_node: Some(device),
            members: network.members.clone(),
            folders: network.folders.clone(),
            shared_secret: network.shared_secret,
            doc_ticket: network.doc_ticket.clone(),
            is_public: network.is_public,
        };
        self.db.add_member(id, device)?;
        self.sync_member_keys();
        self.log_event(&id, NetworkEventType::MemberAdded, Some(&device), Some("invited"));
        self.log_event(
            &id,
            NetworkEventType::TicketCreated,
            None,
            Some("device-bound ticket generated"),
        );
        Ok(ticket)
    }

    /// Generate an invitation usable by any device.
    ///
    /// # Errors
    ///
    /// Returns an error if the network does not exist or the local node is not its owner.
    pub fn invite_any(&self, id: NetworkId) -> Result<NetworkTicket> {
        let network = self.network_as_owner(id)?;
        self.log_event(
            &id,
            NetworkEventType::TicketCreated,
            None,
            Some("open ticket generated"),
        );
        Ok(NetworkTicket {
            network_id: network.id,
            name: network.name.clone(),
            label: network.label.clone(),
            owner: network.owner,
            invited_node: None,
            members: network.members.clone(),
            folders: network.folders.clone(),
            shared_secret: network.shared_secret,
            doc_ticket: network.doc_ticket.clone(),
            is_public: network.is_public,
        })
    }

    /// Remove a member from a locally owned network.
    ///
    /// # Errors
    ///
    /// Returns an error if the network does not exist, authorization fails,
    /// the owner is targeted, or persistence fails.
    pub fn kick(&mut self, id: NetworkId, device: &PublicKey) -> Result<()> {
        let network = self.network_mut_as_owner(id)?;
        if network.owner == *device {
            return Err(SyncwebError::InvalidConfig(
                "the network owner cannot be kicked".to_owned(),
            ));
        }
        if !network.members.remove(device) {
            return Err(SyncwebError::InvalidConfig("device is not a network member".to_owned()));
        }
        self.db.remove_member(id, device)?;
        self.sync_member_keys();
        self.log_event(&id, NetworkEventType::MemberRemoved, Some(device), Some("kicked"));
        Ok(())
    }

    /// Associate a folder with a network.
    ///
    /// # Errors
    ///
    /// Returns an error if the network does not exist or persistence fails.
    pub fn add_folder(&mut self, id: NetworkId, folder: NamespaceId) -> Result<()> {
        self.network_mut(id)?.folders.insert(folder);
        self.log_event(
            &id,
            NetworkEventType::FolderAdded,
            None,
            Some(&format!("folder {folder}")),
        );
        self.db.add_folder_to_network(id, folder)
    }

    /// Remove a folder association without changing the folder itself.
    ///
    /// # Errors
    ///
    /// Returns an error if the network does not exist or persistence fails.
    pub fn remove_folder(&mut self, id: NetworkId, folder: NamespaceId) -> Result<()> {
        self.network_mut(id)?.folders.remove(&folder);
        self.log_event(
            &id,
            NetworkEventType::FolderRemoved,
            None,
            Some(&format!("folder {folder}")),
        );
        self.db.remove_folder_from_network(id, folder)
    }

    /// Subscribe to the network's deterministic gossip topic.
    ///
    /// The returned topic must be retained by the caller for membership to
    /// remain active.
    ///
    /// # Errors
    ///
    /// Returns an error if the network does not exist or gossip rejects the
    /// subscription.
    pub async fn subscribe(&self, id: NetworkId, gossip: &GossipService) -> Result<GossipTopic> {
        let network = self
            .networks
            .get(&id)
            .ok_or_else(|| SyncwebError::FolderNotFound(format!("network {id}")))?;
        if !network.is_member(&self.local_node) {
            return Err(SyncwebError::InvalidConfig(
                "local device is not a member of this network".to_owned(),
            ));
        }
        let mut bootstrap = network
            .members
            .iter()
            .copied()
            .filter(|member| *member != self.local_node)
            .collect::<Vec<_>>();
        bootstrap.sort_unstable();
        gossip.subscribe(network.topic, bootstrap).await
    }

    /// Return the gossip topic associated with a network.
    ///
    /// # Errors
    ///
    /// Returns an error if the network does not exist.
    pub fn topic(&self, id: NetworkId) -> Result<TopicId> {
        self.networks
            .get(&id)
            .map(|network| network.topic)
            .ok_or_else(|| SyncwebError::FolderNotFound(format!("network {id}")))
    }

    #[must_use]
    pub fn list(&self) -> Vec<&Network> {
        let mut networks = self.networks.values().collect::<Vec<_>>();
        networks.sort_by(|left, right| left.name.cmp(&right.name));
        networks
    }

    #[must_use]
    pub fn get(&self, id: &NetworkId) -> Option<&Network> {
        self.networks.get(id)
    }

    #[must_use]
    pub fn get_by_name(&self, name: &str) -> Option<&Network> {
        self.networks.values().find(|network| network.name == name)
    }

    /// Check if the local node can access a given folder namespace through any network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn can_access_folder(&self, namespace_id: &NamespaceId) -> Result<bool> {
        self.db
            .can_access_folder(&namespace_id.to_string(), &self.local_node.to_string())
    }

    /// Return the network IDs (as strings) that contain a given folder.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn networks_for_folder(&self, namespace_id: &NamespaceId) -> Result<Vec<String>> {
        self.db.networks_for_folder(&namespace_id.to_string())
    }

    /// Return the first network ID that contains a given folder, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn network_for_folder(&self, namespace_id: &NamespaceId) -> Result<Option<String>> {
        Ok(self
            .db
            .networks_for_folder(&namespace_id.to_string())?
            .into_iter()
            .next())
    }

    /// List all folder namespace IDs associated with a network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn folders_for_network(&self, network_id: &NetworkId) -> Result<Vec<NamespaceId>> {
        let folders = self.db.folders_for_network(&network_id.to_string())?;
        folders
            .into_iter()
            .map(|f| {
                f.parse()
                    .map_err(|error| SyncwebError::operation("invalid namespace in database", error))
            })
            .collect()
    }

    /// List all members of a network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn members_of_network(&self, network_id: &NetworkId) -> Result<Vec<iroh::PublicKey>> {
        let members = self.db.members_of_network(&network_id.to_string())?;
        members
            .into_iter()
            .map(|m| {
                m.parse()
                    .map_err(|error| SyncwebError::operation("invalid member key in database", error))
            })
            .collect()
    }

    /// Sync the member_keys set and has_public_network flag from all networks.
    fn sync_member_keys(&self) {
        let keys: HashSet<iroh::PublicKey> = self
            .networks
            .values()
            .flat_map(|n| n.members.iter().copied())
            .collect();
        *self.member_keys.blocking_write() = keys;
        let public = self.networks.values().any(|n| n.is_public);
        self.has_public_network.store(public, Ordering::Relaxed);
    }

    fn log_event(
        &self,
        network_id: &NetworkId,
        event: NetworkEventType,
        peer: Option<&iroh::PublicKey>,
        details: Option<&str>,
    ) {
        if let Some(ref logger) = self.logger {
            let peer_str = peer.map(std::string::ToString::to_string);
            let _ = logger.record_event(&network_id.to_string(), event, peer_str.as_deref(), details);
        }
    }

    fn network_mut(&mut self, id: NetworkId) -> Result<&mut Network> {
        self.networks
            .get_mut(&id)
            .ok_or_else(|| SyncwebError::FolderNotFound(format!("network {id}")))
    }

    fn network_as_owner(&self, id: NetworkId) -> Result<&Network> {
        let network = self
            .networks
            .get(&id)
            .ok_or_else(|| SyncwebError::FolderNotFound(format!("network {id}")))?;
        if network.owner != self.local_node {
            return Err(SyncwebError::InvalidConfig(
                "only the network owner can manage invitations".to_owned(),
            ));
        }
        Ok(network)
    }

    fn network_mut_as_owner(&mut self, id: NetworkId) -> Result<&mut Network> {
        if self.network_as_owner(id)?.owner != self.local_node {
            return Err(SyncwebError::InvalidConfig(
                "only the network owner can manage members".to_owned(),
            ));
        }
        self.network_mut(id)
    }
}
