use iroh_blobs::Hash;
use iroh_docs::NamespaceId;

use crate::{Result, net::NetworkManager, storage::node_db::NodeDatabase};

/// Access control context for network-scoped blob and folder operations.
///
/// Wraps a `NetworkManager` and provides methods to verify that the local
/// node can access specific blobs or folders through its network memberships.
#[derive(Clone, Debug)]
pub struct NetworkContext {
    database: NodeDatabase,
    local_node: String,
}

impl NetworkContext {
    /// Create a new network context from a `NetworkManager`.
    #[must_use]
    pub fn from_manager(manager: &NetworkManager) -> Self {
        let local_node = manager.local_node().to_string();
        Self {
            database: manager.database().clone(),
            local_node,
        }
    }

    /// Create a new network context directly from a database and local node ID.
    #[must_use]
    pub fn new(database: NodeDatabase, local_node: &str) -> Self {
        Self {
            database,
            local_node: local_node.to_owned(),
        }
    }

    /// Returns true if the local node can access the given folder namespace
    /// through at least one network membership.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn can_access_folder(&self, namespace_id: &NamespaceId) -> Result<bool> {
        self.database
            .can_access_folder(&namespace_id.to_string(), &self.local_node)
    }

    /// Returns true if the local node can access the given blob through at
    /// least one folder in a network the node belongs to.
    ///
    /// Uses the `blob_folders` reverse index for O(1) lookup.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn can_access_blob(&self, hash: &Hash) -> Result<bool> {
        self.database.can_access_blob(hash, &self.local_node)
    }

    /// List all network IDs (as strings) that contain the given folder.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn networks_for_folder(&self, namespace_id: &NamespaceId) -> Result<Vec<String>> {
        self.database.networks_for_folder(&namespace_id.to_string())
    }
}
