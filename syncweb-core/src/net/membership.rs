//! Network membership-doc provisioning and refresh.
//!
//! Each network owns an iroh-docs namespace that carries its signed member
//! list. This service creates that namespace (provisioned like folders and
//! catalogs via [`DocsEngine::provision`]) and re-signs the member list after
//! membership changes.

use std::path::Path;

use ed25519_dalek::SigningKey;

use crate::{
    error::Result,
    init::open_node,
    net::{Network, NetworkManager, membership_doc::write_member_list},
};

/// Provision the owner-signed membership doc for a network and persist its
/// read-only ticket on the network so members can import it for removal
/// detection.
///
/// # Errors
///
/// Returns an error if the namespace cannot be created, the member list cannot
/// be signed or written, or the network manager cannot persist the doc ticket.
pub async fn provision(data_dir: &Path, network: &Network, manager: &mut NetworkManager) -> Result<()> {
    let node = open_node(data_dir).await?;
    let docs = node.docs_engine();
    let bytes = signed_member_list_bytes(&node, network)?;
    let provisioned = docs
        .provision(crate::node::docs_engine::ProvisionKind::NetworkMembership, None, bytes)
        .await?;
    let read_ticket = docs.share_ticket(&provisioned.doc, false).await?;
    node.stop().await?;
    manager.set_doc_ticket(network.id, &Some(read_ticket.to_string()))?;
    Ok(())
}

/// Re-sign and re-publish the member list after a membership change
/// (invite/kick) by the owner.
///
/// # Errors
///
/// Returns an error if the doc ticket is missing or cannot be imported, or the
/// member list cannot be signed or written.
pub async fn refresh(data_dir: &Path, network: &Network) -> Result<()> {
    let Some(ref doc_ticket) = network.doc_ticket else {
        return Ok(());
    };
    let node = open_node(data_dir).await?;
    let docs = node.docs_engine();
    let doc = docs
        .import_ticket(
            doc_ticket.parse().map_err(|error| {
                crate::SyncwebError::InvalidTicket(format!("invalid membership doc ticket: {error}"))
            })?,
        )
        .await?;
    let author = docs.author().await?;
    let signing = SigningKey::from_bytes(&node.endpoint().secret_key().to_bytes());
    write_member_list(docs, node.blob_store(), &doc, author, &signing, network).await?;
    node.stop().await?;
    Ok(())
}

fn signed_member_list_bytes(node: &crate::node::iroh_node::IrohNode, network: &Network) -> Result<Vec<u8>> {
    let signing = SigningKey::from_bytes(&node.endpoint().secret_key().to_bytes());
    let mut list = crate::net::membership_doc::SignedMemberList::from_network(network);
    list.sequence = 1;
    list.sign(&signing)?;
    serde_json::to_vec(&list).map_err(|error| crate::SyncwebError::operation("failed to serialize member list", error))
}
