use std::path::PathBuf;

use iroh_docs::{DocTicket, NamespaceId};

use crate::node::identity::IdentityManager;
use crate::node::iroh_node::{IrohNode, RelayMode};

/// Result of initializing a local synchronized folder.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct InitResult {
    pub path: PathBuf,
    pub namespace: NamespaceId,
    pub ticket: DocTicket,
    pub share_url: String,
}

impl InitResult {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, namespace: NamespaceId, ticket: DocTicket) -> Self {
        let share_url = format!("syncweb://{namespace}?ticket={ticket}");
        Self {
            path: path.into(),
            namespace,
            ticket,
            share_url,
        }
    }
}

/// Open a node with default relay mode.
///
/// # Errors
///
/// Returns an error if the identity cannot be loaded or the node cannot be created.
pub async fn open_node(data_dir: &std::path::Path) -> crate::error::Result<IrohNode> {
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    IrohNode::new(identity, data_dir.join("data"), RelayMode::Default).await
}
