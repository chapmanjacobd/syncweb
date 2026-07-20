use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use iroh::PublicKey;
use iroh_docs::NamespaceId;
use iroh_gossip::{TopicId, api::GossipTopic};
use serde::{Deserialize, Serialize};

use crate::node::gossip_service::GossipService;
use crate::{Result, SyncwebError};

use super::network::{Network, NetworkId, NetworkOptions, NetworkTicket, network_topic, parse_public_key};

/// Persistent manager for network membership and folder associations.
#[derive(Clone, Debug)]
pub struct NetworkManager {
    path: PathBuf,
    local_node: PublicKey,
    networks: HashMap<NetworkId, Network>,
}

impl NetworkManager {
    /// Open the network database at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if existing state cannot be read or parsed.
    pub fn new(path_arg: impl Into<PathBuf>, local_node: PublicKey) -> Result<Self> {
        let path = path_arg.into();
        let networks = load_networks(&path)?
            .into_iter()
            .map(|network| (network.id, network))
            .collect();
        Ok(Self {
            path,
            local_node,
            networks,
        })
    }

    /// Create and persist a network owned by the local node.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty/duplicate name or failed persistence.
    pub fn create(&mut self, name: &str, options: NetworkOptions) -> Result<NetworkId> {
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
        self.networks
            .insert(id, Network::new(normalized, self.local_node, options));
        self.save()?;
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
        };
        let id = network.id;
        self.networks.insert(id, network);
        self.save()?;
        Ok(id)
    }

    /// Leave a network and remove its local state.
    ///
    /// # Errors
    ///
    /// Returns an error if the network does not exist or persistence fails.
    pub fn leave(&mut self, id: NetworkId) -> Result<()> {
        self.networks
            .remove(&id)
            .ok_or_else(|| SyncwebError::FolderNotFound(format!("network {id}")))?;
        self.save()
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
        };
        self.save()?;
        Ok(ticket)
    }

    /// Generate an invitation usable by any device.
    ///
    /// # Errors
    ///
    /// Returns an error if the network does not exist or the local node is not its owner.
    pub fn invite_any(&self, id: NetworkId) -> Result<NetworkTicket> {
        let network = self.network_as_owner(id)?;
        Ok(NetworkTicket {
            network_id: network.id,
            name: network.name.clone(),
            label: network.label.clone(),
            owner: network.owner,
            invited_node: None,
            members: network.members.clone(),
            folders: network.folders.clone(),
            shared_secret: network.shared_secret,
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
        self.save()
    }

    /// Associate a folder with a network.
    ///
    /// # Errors
    ///
    /// Returns an error if the network does not exist or persistence fails.
    pub fn add_folder(&mut self, id: NetworkId, folder: NamespaceId) -> Result<()> {
        self.network_mut(id)?.folders.insert(folder);
        self.save()
    }

    /// Remove a folder association without changing the folder itself.
    ///
    /// # Errors
    ///
    /// Returns an error if the network does not exist or persistence fails.
    pub fn remove_folder(&mut self, id: NetworkId, folder: NamespaceId) -> Result<()> {
        self.network_mut(id)?.folders.remove(&folder);
        self.save()
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

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let records = self.networks.values().map(NetworkRecord::from).collect::<Vec<_>>();
        let contents = serde_json::to_vec_pretty(&records)
            .map_err(|error| SyncwebError::operation("failed to serialize networks", error))?;
        let temporary = self.path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
        std::fs::write(&temporary, contents)
            .map_err(|error| SyncwebError::operation("failed to write network state", error))?;
        std::fs::rename(&temporary, &self.path)
            .map_err(|error| SyncwebError::operation("failed to persist network state", error))
    }
}

#[derive(Deserialize, Serialize)]
struct NetworkRecord {
    id: String,
    name: String,
    label: String,
    owner: String,
    members: Vec<String>,
    folders: Vec<String>,
    shared_secret: Option<String>,
}

impl From<&Network> for NetworkRecord {
    fn from(network: &Network) -> Self {
        Self {
            id: network.id.to_string(),
            name: network.name.clone(),
            label: network.label.clone(),
            owner: network.owner.to_string(),
            members: network.members.iter().map(ToString::to_string).collect(),
            folders: network.folders.iter().map(ToString::to_string).collect(),
            shared_secret: network.shared_secret.map(hex::encode),
        }
    }
}

impl TryFrom<NetworkRecord> for Network {
    type Error = SyncwebError;

    fn try_from(record: NetworkRecord) -> Result<Self> {
        if record.name.trim().is_empty() {
            return Err(SyncwebError::InvalidConfig("network name cannot be empty".to_owned()));
        }
        let id: NetworkId = record.id.parse()?;
        if id != NetworkId::from_name(record.name.trim()) {
            return Err(SyncwebError::InvalidConfig(
                "network ID does not match its name".to_owned(),
            ));
        }
        let members = record
            .members
            .into_iter()
            .map(|member| parse_public_key(&member))
            .collect::<Result<HashSet<_>>>()?;
        let folders = record
            .folders
            .into_iter()
            .map(|namespace| {
                namespace
                    .parse()
                    .map_err(|error| SyncwebError::InvalidConfig(format!("invalid folder namespace: {error}")))
            })
            .collect::<Result<HashSet<_>>>()?;
        let shared_secret = record
            .shared_secret
            .map(|secret| {
                let bytes = hex::decode(secret)
                    .map_err(|error| SyncwebError::InvalidConfig(format!("invalid network secret: {error}")))?;
                bytes.try_into().map_err(|_length_error| {
                    SyncwebError::InvalidConfig("network secret must contain 32 bytes".to_owned())
                })
            })
            .transpose()?;
        Ok(Self {
            id,
            name: record.name,
            label: record.label,
            topic: network_topic(id),
            owner: parse_public_key(&record.owner)?,
            members,
            folders,
            shared_secret,
        })
    }
}

fn load_networks(path: &Path) -> Result<Vec<Network>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path).map_err(|error| SyncwebError::operation("failed to read network state", error))?;
    let records: Vec<NetworkRecord> = serde_json::from_slice(&bytes)
        .map_err(|error| SyncwebError::operation("failed to parse network state", error))?;
    records.into_iter().map(Network::try_from).collect()
}
