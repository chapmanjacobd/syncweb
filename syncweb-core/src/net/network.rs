use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
    str::FromStr,
};

use iroh::PublicKey;
use iroh_docs::NamespaceId;
use iroh_gossip::TopicId;
use serde::{Deserialize, Serialize};

use crate::{Result, SyncwebError};

/// Stable identifier derived from a network name.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NetworkId([u8; 32]);

impl NetworkId {
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"network:");
        hasher.update(name.as_bytes());
        Self(*hasher.finalize().as_bytes())
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Display for NetworkId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&hex::encode(self.0))
    }
}

impl FromStr for NetworkId {
    type Err = SyncwebError;

    fn from_str(value: &str) -> Result<Self> {
        let bytes =
            hex::decode(value).map_err(|error| SyncwebError::InvalidConfig(format!("invalid network ID: {error}")))?;
        let array: [u8; 32] = bytes
            .try_into()
            .map_err(|_length_error| SyncwebError::InvalidConfig("network ID must contain 32 bytes".to_owned()))?;
        Ok(Self(array))
    }
}

/// Options used when creating a network.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct NetworkOptions {
    pub label: String,
    pub invite_only: bool,
}

impl NetworkOptions {
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    #[must_use]
    pub const fn invite_only(mut self, invite_only: bool) -> Self {
        self.invite_only = invite_only;
        self
    }
}

/// A named group of devices and synchronized folders.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Network {
    pub id: NetworkId,
    pub name: String,
    pub label: String,
    pub topic: TopicId,
    pub owner: PublicKey,
    pub members: HashSet<PublicKey>,
    pub folders: HashSet<NamespaceId>,
    pub(crate) shared_secret: Option<[u8; 32]>,
}

impl Network {
    #[must_use]
    pub fn new(name_arg: impl Into<String>, owner: PublicKey, options: NetworkOptions) -> Self {
        let name = name_arg.into().trim().to_owned();
        let id = NetworkId::from_name(&name);
        let topic = network_topic(id);
        let mut members = HashSet::new();
        members.insert(owner);
        Self {
            id,
            name,
            label: options.label,
            topic,
            owner,
            members,
            folders: HashSet::new(),
            shared_secret: options.invite_only.then(rand::random),
        }
    }

    #[must_use]
    pub fn is_member(&self, node_id: &PublicKey) -> bool {
        self.members.contains(node_id)
    }

    #[must_use]
    pub const fn topic(&self) -> TopicId {
        self.topic
    }

    #[must_use]
    pub const fn is_invite_only(&self) -> bool {
        self.shared_secret.is_some()
    }
}

/// Shareable capability for joining a network.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkTicket {
    pub network_id: NetworkId,
    pub name: String,
    pub label: String,
    pub owner: PublicKey,
    pub invited_node: Option<PublicKey>,
    pub members: HashSet<PublicKey>,
    pub folders: HashSet<NamespaceId>,
    pub(crate) shared_secret: Option<[u8; 32]>,
}

impl NetworkTicket {
    #[must_use]
    pub const fn is_invite_only(&self) -> bool {
        self.shared_secret.is_some()
    }
}

impl Display for NetworkTicket {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let wire = TicketWire::from(self);
        let bytes = serde_json::to_vec(&wire).map_err(|_serialization_error| std::fmt::Error)?;
        write!(formatter, "syncweb://network/{}", hex::encode(bytes))
    }
}

impl FromStr for NetworkTicket {
    type Err = SyncwebError;

    fn from_str(value: &str) -> Result<Self> {
        let encoded = value.strip_prefix("syncweb://network/").ok_or_else(|| {
            SyncwebError::InvalidTicket("network ticket must start with syncweb://network/".to_owned())
        })?;
        let bytes = hex::decode(encoded)
            .map_err(|error| SyncwebError::InvalidTicket(format!("invalid network ticket encoding: {error}")))?;
        let wire: TicketWire = serde_json::from_slice(&bytes)
            .map_err(|error| SyncwebError::InvalidTicket(format!("invalid network ticket: {error}")))?;
        wire.try_into()
    }
}

#[derive(Deserialize, Serialize)]
struct TicketWire {
    network_id: String,
    name: String,
    label: String,
    owner: String,
    invited_node: Option<String>,
    members: Vec<String>,
    folders: Vec<String>,
    shared_secret: Option<String>,
}

impl From<&NetworkTicket> for TicketWire {
    fn from(ticket: &NetworkTicket) -> Self {
        let mut members = ticket.members.iter().map(ToString::to_string).collect::<Vec<_>>();
        members.sort_unstable();
        let mut folders = ticket.folders.iter().map(ToString::to_string).collect::<Vec<_>>();
        folders.sort_unstable();
        Self {
            network_id: ticket.network_id.to_string(),
            name: ticket.name.clone(),
            label: ticket.label.clone(),
            owner: ticket.owner.to_string(),
            invited_node: ticket.invited_node.map(|node| node.to_string()),
            members,
            folders,
            shared_secret: ticket.shared_secret.map(hex::encode),
        }
    }
}

impl TryFrom<TicketWire> for NetworkTicket {
    type Error = SyncwebError;

    fn try_from(wire: TicketWire) -> Result<Self> {
        if wire.name.trim().is_empty() {
            return Err(SyncwebError::InvalidTicket("network name cannot be empty".to_owned()));
        }
        let network_id: NetworkId = wire.network_id.parse()?;
        if network_id != NetworkId::from_name(wire.name.trim()) {
            return Err(SyncwebError::InvalidTicket(
                "network ticket ID does not match its network name".to_owned(),
            ));
        }
        let members = wire
            .members
            .into_iter()
            .map(|member| parse_public_key(&member))
            .collect::<Result<HashSet<_>>>()?;
        let folders = wire
            .folders
            .into_iter()
            .map(|folder| {
                folder
                    .parse()
                    .map_err(|error| SyncwebError::InvalidTicket(format!("invalid folder namespace: {error}")))
            })
            .collect::<Result<HashSet<_>>>()?;
        let shared_secret = wire
            .shared_secret
            .map(|secret| {
                let bytes = hex::decode(secret)
                    .map_err(|error| SyncwebError::InvalidTicket(format!("invalid network secret: {error}")))?;
                bytes.try_into().map_err(|_length_error| {
                    SyncwebError::InvalidTicket("network secret must contain 32 bytes".to_owned())
                })
            })
            .transpose()?;
        Ok(Self {
            network_id,
            name: wire.name,
            label: wire.label,
            owner: parse_public_key(&wire.owner)?,
            invited_node: wire.invited_node.map(|node| parse_public_key(&node)).transpose()?,
            members,
            folders,
            shared_secret,
        })
    }
}

pub(crate) fn network_topic(id: NetworkId) -> TopicId {
    let digest = blake3::hash(format!("syncweb/net/{id}").as_bytes());
    TopicId::from_bytes(*digest.as_bytes())
}

pub(crate) fn parse_public_key(value: &str) -> Result<PublicKey> {
    value
        .parse()
        .map_err(|error| SyncwebError::InvalidTicket(format!("invalid node ID: {error}")))
}
