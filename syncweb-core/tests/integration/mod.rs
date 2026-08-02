use crate::test_utils::{TestDirectory, empty_member_keys};
use syncweb_core::node::{
    identity::IdentityManager,
    iroh_node::{DiscoveryConfig, IrohNode, RelayMode},
};

/// Create a test Iroh node within the given directory.
///
/// # Errors
///
/// Returns an error if the identity cannot be loaded or the node cannot connect.
pub async fn test_node(directory: &TestDirectory, name: &str) -> anyhow::Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    Ok(IrohNode::new(
        identity,
        root.join("data"),
        RelayMode::Default,
        empty_member_keys(),
        DiscoveryConfig::disabled(),
    )
    .await?)
}

mod actor_test;
mod archive_export_test;
mod archive_import_test;
mod archive_verify_test;
mod blob_store_test;
mod collection_publish_test;
mod collection_test;
mod config_test;
mod daemon_archive_test;
mod daemon_integration_test;
mod docs_engine_test;
mod exporter_test;
mod filter_test;
mod find_test;
mod folder_test;
mod gossip_service_test;
mod identity_test;
mod importer_test;
mod indexing_test;
mod iroh_node_test;
mod lazy_fetch_test;
mod mirror_test;
mod moderation_test;
mod network_test;
mod package_test;
mod partial_fetch_test;
mod provider_trust_gossip_test;
mod relay_test;
mod reputation_test;
mod scanner_test;
mod schedule_stats_test;
mod search_test;
mod session_test;
mod smart_ban_test;
mod snapshot_test;
mod sort_enrich_test;
mod sort_test;
mod stat_test;
mod sync_test;
mod topic_tracker_test;
mod trust_delegate_test;
mod wot_test;
