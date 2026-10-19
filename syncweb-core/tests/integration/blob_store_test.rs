use syncweb_core::node::identity::IdentityManager;
use syncweb_core::node::iroh_node::{DiscoveryConfig, IrohNode, RelayMode};
use syncweb_core::sync::{TransferRequest, fetch_blob_requests};
use tokio_util::sync::CancellationToken;

use crate::test_utils::TestDirectory;

async fn test_node(
    directory: &TestDirectory,
    name: &str,
    relay_map: Option<iroh::RelayMap>,
) -> anyhow::Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    let relay_mode = relay_map.map_or(RelayMode::Default, |map| RelayMode::Custom { map, insecure: true });
    Ok(IrohNode::new(
        identity,
        root.join("data"),
        relay_mode,
        crate::test_utils::empty_member_keys(),
        DiscoveryConfig::disabled(),
    )
    .await?)
}

#[tokio::test]
async fn test_add_bytes() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let hash = node.blob_store().add_bytes(b"phase one blob").await?;

    anyhow::ensure!(node.blob_store().has(hash).await?);

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_has_blob() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let hash = node.blob_store().add_bytes(b"blob data").await?;
    anyhow::ensure!(node.blob_store().has(hash).await?);

    let fake_hash = iroh_blobs::Hash::from_bytes([0_u8; 32]);
    anyhow::ensure!(!node.blob_store().has(fake_hash).await?);

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_get_blob() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let hash = node.blob_store().add_bytes(b"blob data").await?;

    let bytes = node.blob_store().get(hash).await?;
    anyhow::ensure!(bytes == b"blob data".as_slice());

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_blob_ticket() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let hash = node.blob_store().add_bytes(b"blob data").await?;

    let ticket = node.blob_store().ticket(node.endpoint(), hash);
    anyhow::ensure!(ticket.hash() == hash);
    anyhow::ensure!(ticket.to_string().starts_with("blob"));

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_content_pinning() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let hash = node.blob_store().add_bytes(b"pinned blob").await?;

    node.blob_store().pin("syncweb/test-pinned", hash).await?;
    anyhow::ensure!(node.blob_store().is_pinned("syncweb/test-pinned", hash).await?);
    node.blob_store().unpin("syncweb/test-pinned").await?;
    anyhow::ensure!(!node.blob_store().is_pinned("syncweb/test-pinned", hash).await?);

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_classified_pin_gc_preserves_shared_content() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let temporary_hash = node.blob_store().add_bytes(b"temporary").await?;
    let shared_hash = node.blob_store().add_bytes(b"shared").await?;

    let temporary_name = node
        .blob_store()
        .pin_class(syncweb_core::PinClass::Temporary, "download", temporary_hash, Some(10))
        .await?;
    let shared_name = node
        .blob_store()
        .pin_class(syncweb_core::PinClass::Shared, "folder", shared_hash, None)
        .await?;

    let removed = node
        .blob_store()
        .gc_expired_pins(syncweb_core::PinPolicy::default(), 10)
        .await?;
    anyhow::ensure!(removed.iter().any(|pin| pin.name == temporary_name));
    anyhow::ensure!(!node.blob_store().is_pinned(&temporary_name, temporary_hash).await?);
    anyhow::ensure!(node.blob_store().is_pinned(&shared_name, shared_hash).await?);

    let classified = node.blob_store().list_classified_pins().await?;
    anyhow::ensure!(
        classified
            .iter()
            .any(|pin| pin.hash == shared_hash && pin.class == syncweb_core::PinClass::Shared)
    );

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_add_file() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let path = directory.path().join("input.txt");
    std::fs::write(&path, b"file blob")?;

    let hash = node.blob_store().add_file(&path).await?;
    anyhow::ensure!(node.blob_store().get(hash).await? == b"file blob".as_slice());

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_add_file_ref() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let path = directory.path().join("input.txt");
    std::fs::write(&path, b"referenced file")?;

    let referenced_hash = node.blob_store().add_file_ref(&path).await?;
    let copied_hash = node.blob_store().add_bytes(b"referenced file").await?;
    anyhow::ensure!(referenced_hash == copied_hash);
    anyhow::ensure!(node.blob_store().get(referenced_hash).await? == b"referenced file".as_slice());

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_two_nodes_sync_blob() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let (relay_map, _relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let first = test_node(&directory, "first", Some(relay_map.clone())).await?;
    let second = test_node(&directory, "second", Some(relay_map)).await?;
    let hash = first.blob_store().add_bytes(b"shared blob").await?;
    let ticket = first.blob_store().ticket(first.endpoint(), hash);

    second.blob_store().fetch(second.endpoint(), &ticket).await?;
    anyhow::ensure!(second.blob_store().get(hash).await? == b"shared blob".as_slice());

    first.stop().await?;
    second.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_segment_request_falls_back_to_whole_blob_peer() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-whole-blob-compat-test")?;
    let (relay_map, _relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let first = test_node(&directory, "first", Some(relay_map.clone())).await?;
    let second = test_node(&directory, "second", Some(relay_map)).await?;
    let payload = b"whole blob compatibility";
    let hash = first.blob_store().add_bytes(payload).await?;

    let request = TransferRequest::for_segment(hash, payload.len() as u64, 6, 4, vec![first.endpoint().addr()])?;
    let report = fetch_blob_requests(
        second.blob_store(),
        second.endpoint(),
        vec![request],
        CancellationToken::new(),
    )
    .await?;

    anyhow::ensure!(
        report.completed == 1,
        "whole-blob peer should satisfy the segment-shaped request"
    );
    anyhow::ensure!(
        report.bytes == 4,
        "scheduler report should retain the requested segment size"
    );
    anyhow::ensure!(
        second.blob_store().get(hash).await? == payload.as_slice(),
        "compatibility fallback must verify and store the complete blob"
    );

    first.stop().await?;
    second.stop().await?;
    Ok(())
}
