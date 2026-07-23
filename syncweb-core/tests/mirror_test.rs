mod test_utils;

use anyhow::{Result, ensure};
use syncweb_core::indexing::{ProviderLease, ReplicationBudget, ResilienceConfig, ResilienceService};
use syncweb_core::node::identity::IdentityManager;
use syncweb_core::node::iroh_node::{IrohNode, RelayMode};

use crate::test_utils::TestDirectory;

async fn test_node(directory: &TestDirectory, name: &str, relay_map: Option<iroh::RelayMap>) -> Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    let relay_mode = relay_map.map_or(RelayMode::Default, |map| RelayMode::Custom { map, insecure: true });
    Ok(IrohNode::new(identity, root.join("data"), relay_mode).await?)
}

#[tokio::test]
async fn test_mirror_fetches_and_pins_blob() -> Result<()> {
    let directory = TestDirectory::new("syncweb-mirror-test")?;
    let (relay_map, _relay_url, _server) = iroh::test_utils::run_relay_server().await?;

    let provider = test_node(&directory, "provider", Some(relay_map.clone())).await?;
    let consumer = test_node(&directory, "consumer", Some(relay_map)).await?;

    let blob_content = b"mirror me";
    let hash = provider.blob_store().add_bytes(blob_content).await?;
    let ticket = provider.blob_store().ticket(provider.endpoint(), hash);

    // Budget > verified providers forces ensure_replication to fetch
    let resilience = ResilienceService::new(ResilienceConfig::new(ReplicationBudget::new(2)));
    let lease = ProviderLease::signed(hash, ticket.to_string(), 1, u64::MAX, provider.endpoint().secret_key())?;
    resilience.record_lease(lease)?;

    let result = resilience
        .ensure_replication(consumer.endpoint(), consumer.blob_store(), hash)
        .await
        .expect("ensure_replication should succeed");

    ensure!(result.pinned, "blob should be pinned after replication: {result:#?}");
    ensure!(consumer.blob_store().has(hash).await?, "blob should exist on consumer");
    let blob_content_out = consumer.blob_store().get(hash).await?.to_vec();
    ensure!(
        blob_content_out == blob_content,
        "blob content should match: expected {blob_content:?}, got {blob_content_out:?}"
    );

    provider.stop().await?;
    consumer.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_mirror_short_circuits_when_budget_met() -> Result<()> {
    let directory = TestDirectory::new("syncweb-mirror-budget-test")?;
    let (relay_map, _relay_url, _server) = iroh::test_utils::run_relay_server().await?;

    let provider = test_node(&directory, "provider", Some(relay_map.clone())).await?;
    let consumer = test_node(&directory, "consumer", Some(relay_map)).await?;

    let blob_content = b"already satisfied";
    let hash = provider.blob_store().add_bytes(blob_content).await?;
    let ticket = provider.blob_store().ticket(provider.endpoint(), hash);

    // Budget == verified providers → no fetch needed, returns early
    let resilience = ResilienceService::new(ResilienceConfig::new(ReplicationBudget::new(1)));
    let lease = ProviderLease::signed(hash, ticket.to_string(), 1, u64::MAX, provider.endpoint().secret_key())?;
    resilience.record_lease(lease)?;

    let result = resilience
        .ensure_replication(consumer.endpoint(), consumer.blob_store(), hash)
        .await
        .expect("ensure_replication should succeed");

    ensure!(!result.pinned, "should not pin when budget already met");
    ensure!(
        !result.short_circuited,
        "should not short-circuit when budget already met"
    );
    ensure!(
        result.fetched_from.is_empty(),
        "should not fetch when budget already met"
    );

    provider.stop().await?;
    consumer.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_mirror_pins_when_already_local() -> Result<()> {
    let directory = TestDirectory::new("syncweb-mirror-local-test")?;
    let (relay_map, _relay_url, _server) = iroh::test_utils::run_relay_server().await?;

    let provider = test_node(&directory, "provider", Some(relay_map.clone())).await?;
    let consumer = test_node(&directory, "consumer", Some(relay_map)).await?;

    let blob_content = b"already local";
    let hash = provider.blob_store().add_bytes(blob_content).await?;
    let ticket = provider.blob_store().ticket(provider.endpoint(), hash);

    // Pre-fetch so the blob is local
    consumer.blob_store().fetch(consumer.endpoint(), &ticket).await?;
    anyhow::ensure!(consumer.blob_store().has(hash).await?);

    // Budget > verified forces the code past the budget check
    let resilience = ResilienceService::new(ResilienceConfig::new(ReplicationBudget::new(2)));
    let lease = ProviderLease::signed(hash, ticket.to_string(), 1, u64::MAX, provider.endpoint().secret_key())?;
    resilience.record_lease(lease)?;

    let result = resilience
        .ensure_replication(consumer.endpoint(), consumer.blob_store(), hash)
        .await?;

    anyhow::ensure!(result.pinned, "should pin when already local: {result:#?}");

    provider.stop().await?;
    consumer.stop().await?;
    Ok(())
}
