use std::path::PathBuf;

use iroh_docs::{DocTicket, NamespaceId};

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
