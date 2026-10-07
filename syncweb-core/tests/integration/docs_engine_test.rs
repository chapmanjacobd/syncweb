use anyhow::Context;
use n0_future::StreamExt;
use syncweb_core::node::identity::IdentityManager;
use syncweb_core::node::iroh_node::{DiscoveryConfig, IrohNode, RelayMode};

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
async fn test_create_namespace() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let doc = node.docs_engine().create_namespace().await?;
    anyhow::ensure!(!doc.id().to_string().is_empty());
    node.stop().await?;
    Ok(())
}

/// Regression: a node-id-only ticket must carry no addresses, forcing a peer
/// to resolve the node over the DHT topic tracker instead of the ticket.
#[tokio::test]
async fn test_share_node_id_only_ticket_has_no_addresses() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let doc = node.docs_engine().create_namespace().await?;

    let id_only = node
        .docs_engine()
        .share_ticket_with_options(&doc, true, iroh_docs::api::protocol::AddrInfoOptions::Id)
        .await?;
    anyhow::ensure!(
        id_only.nodes.iter().all(|n| n.addrs.is_empty()),
        "node-id-only ticket should carry no addresses: {id_only:?}"
    );

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_set_get_entry() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let doc = node.docs_engine().create_namespace().await?;
    let author = node.docs_engine().author().await?;

    node.docs_engine().set(&doc, author, b"key", b"value").await?;
    let entry = node
        .docs_engine()
        .get(&doc, author, b"key")
        .await?
        .context("entry exists")?;
    anyhow::ensure!(entry.key() == b"key".as_slice());

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_watch_entries() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;
    let doc = node.docs_engine().create_namespace().await?;
    let author = node.docs_engine().author().await?;

    let mut events = node.docs_engine().watch(&doc).await?;

    node.docs_engine().set(&doc, author, b"key2", b"value2").await?;

    let event = tokio::time::timeout(std::time::Duration::from_secs(5), events.next())
        .await
        .context("watch event timed out")?
        .context("watch stream closed")?;

    match event {
        Ok(iroh_docs::engine::LiveEvent::InsertLocal { entry }) => {
            anyhow::ensure!(entry.key() == b"key2".as_slice());
        }
        other => anyhow::bail!("unexpected event: {other:?}"),
    }

    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_author_from_secret() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-services-test")?;
    let node = test_node(&directory, "node", None).await?;

    let original_author_id = node.docs_engine().author().await?;
    let author_secret = node
        .docs_engine()
        .export_author(original_author_id)
        .await?
        .context("author exists")?;

    let secret_str = author_secret.to_string();

    let parsed_author = std::str::FromStr::from_str(&secret_str)?;
    let imported_author_id = node.docs_engine().import_author(parsed_author).await?;

    anyhow::ensure!(original_author_id == imported_author_id);
    node.stop().await?;
    Ok(())
}

/// Regression: live document synchronization must propagate entries in *both*
/// directions once both sides are syncing, including the reverse (creator to
/// joiner) direction. The old daemon tore its live intent down on any failed
/// sync attempt and never handed topic-tracker peers to `start_sync`, with the
/// result that a synchronizing peer never received a second insert (the
/// "reverse push" failure seen across two machines).
#[tokio::test]
async fn test_two_node_live_push_propagates_in_both_directions() -> anyhow::Result<()> {
    use std::time::Duration;

    let directory = TestDirectory::new("syncweb-live-push")?;
    let node_a = test_node(&directory, "a", None).await?;
    let node_b = test_node(&directory, "b", None).await?;

    // A creates the namespace; each side learns the other's address from the
    // other's write ticket (the same exchange a real join performs).
    let doc_a = node_a.docs_engine().create_namespace().await?;
    let ticket_a = node_a.docs_engine().share_ticket(&doc_a, true).await?;
    let doc_b = node_b.docs_engine().import_ticket(ticket_a).await?;
    let ticket_b = node_b.docs_engine().share_ticket(&doc_b, true).await?;
    let _doc_a_again = node_a.docs_engine().import_ticket(ticket_b).await?;

    let id_a = node_a.endpoint().id();
    let id_b = node_b.endpoint().id();
    node_a
        .docs_engine()
        .start_sync(&doc_a, vec![iroh::EndpointAddr::new(id_b)])
        .await?;
    node_b
        .docs_engine()
        .start_sync(&doc_b, vec![iroh::EndpointAddr::new(id_a)])
        .await?;

    // Reverse push: A inserts, B must observe the remote insert.
    let mut events_b = node_b.docs_engine().watch(&doc_b).await?;
    let author_a = node_a.docs_engine().author().await?;
    node_a.docs_engine().set(&doc_a, author_a, b"from-a", b"one").await?;
    wait_for_remote(&mut events_b, b"from-a", Duration::from_secs(20)).await?;

    // Forward push: B inserts, A must observe it.
    let mut events_a = node_a.docs_engine().watch(&doc_a).await?;
    let author_b = node_b.docs_engine().author().await?;
    node_b.docs_engine().set(&doc_b, author_b, b"from-b", b"two").await?;
    wait_for_remote(&mut events_a, b"from-b", Duration::from_secs(20)).await?;

    node_a.stop().await?;
    node_b.stop().await?;
    Ok(())
}

async fn wait_for_remote(
    events: &mut (impl n0_future::Stream<Item = syncweb_core::error::Result<iroh_docs::engine::LiveEvent>> + Unpin),
    key: &[u8],
    timeout_duration: std::time::Duration,
) -> anyhow::Result<()> {
    use n0_future::StreamExt;
    tokio::time::timeout(timeout_duration, async {
        while let Some(result) = events.next().await {
            let event = result?;
            if let iroh_docs::engine::LiveEvent::InsertRemote { entry, .. } = event
                && entry.key() == key
            {
                return anyhow::Ok(());
            }
        }
        anyhow::bail!("live event stream closed before receiving {key:?}")
    })
    .await
    .map_err(|elapsed| anyhow::anyhow!("timed out waiting for remote insert {key:?}: {elapsed}"))?
}
