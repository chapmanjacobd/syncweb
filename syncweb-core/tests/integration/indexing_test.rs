use std::{sync::Arc, time::Duration};

use anyhow::Context;
use iroh_blobs::Hash;
use syncweb_core::{
    folder::{FolderManager, SyncMode},
    indexing::{IndexingDatabase, IndexingEvent, IndexingService, ProviderLease, ReplicationBudget, ResilienceConfig},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
};

use crate::test_utils::TestDirectory;

#[test]
fn indexing_database_initializes_fts_schema() -> anyhow::Result<()> {
    let database = IndexingDatabase::in_memory()?;

    anyhow::ensure!(database.schema_version()? == "1");
    anyhow::ensure!(database.has_fts5()?);
    anyhow::ensure!(database.has_table("indexed_folders")?);
    anyhow::ensure!(database.has_table("indexed_entries")?);
    anyhow::ensure!(database.has_table("index_metadata")?);
    Ok(())
}

#[test]
fn indexing_database_searches_and_updates_entries() -> anyhow::Result<()> {
    let database = IndexingDatabase::in_memory()?;
    let namespace = iroh_docs::NamespaceId::from([1_u8; 32]);
    let hash = Hash::from_bytes([2_u8; 32]);
    database.enable_folder(namespace, "documents")?;
    database.upsert_entry(namespace, b"notes/readme.txt", hash, 12)?;

    let results = database.search("readme", 10)?;
    anyhow::ensure!(results.len() == 1);
    let result = results.first().context("search result should exist")?;
    anyhow::ensure!(result.namespace_id == namespace);
    anyhow::ensure!(result.hash == hash);
    anyhow::ensure!(result.size == 12);

    database.upsert_entry(namespace, b"notes/readme.txt", hash, 24)?;
    anyhow::ensure!(database.entry_count()? == 1);
    let updated_results = database.search("readme", 10)?;
    anyhow::ensure!(
        updated_results
            .first()
            .context("updated search result should exist")?
            .size
            == 24
    );
    Ok(())
}

#[test]
fn indexing_database_serializes_concurrent_access() -> anyhow::Result<()> {
    let database = Arc::new(IndexingDatabase::in_memory()?);
    let namespace = iroh_docs::NamespaceId::from([3_u8; 32]);
    database.enable_folder(namespace, "concurrent")?;

    std::thread::scope(|scope| -> anyhow::Result<()> {
        let mut handles = Vec::new();
        for index in 0_u8..8 {
            let database_handle = Arc::clone(&database);
            handles.push(scope.spawn(move || {
                let key = format!("file-{index}.txt");
                database_handle.upsert_entry(namespace, key.as_bytes(), Hash::from_bytes([index; 32]), 1)
            }));
        }
        for handle in handles {
            match handle.join() {
                Ok(result) => {
                    result?;
                }
                Err(_) => anyhow::bail!("concurrent indexing thread panicked"),
            }
        }
        Ok(())
    })?;

    anyhow::ensure!(database.entry_count()? == 8);
    Ok(())
}

#[tokio::test]
async fn indexing_service_consumes_folder_events() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-indexing-test")?;
    let root = directory.path().join("node");
    let identity = IdentityManager::new(root.join("identity.key"))?;
    let node = IrohNode::new(identity, root.join("data"), RelayMode::Default).await?;
    let folder = FolderManager::new(&node).create(SyncMode::SendReceive).await?;
    let service = IndexingService::in_memory()?;
    let mut events = service.subscribe();

    service.enable_folder(&folder).await?;
    folder.set_blob(b"notes/readme.txt", b"indexed content").await?;

    let indexed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let event = events.recv().await?;
            if let IndexingEvent::EntryIndexed(entry) = event
                && entry.key == b"notes/readme.txt"
            {
                return Ok::<_, anyhow::Error>(entry);
            }
        }
    })
    .await
    .context("index event timed out")??;

    anyhow::ensure!(indexed.namespace_id == folder.namespace_id());
    anyhow::ensure!(service.database().search("readme", 10)?.len() == 1);
    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn catalog_publish_and_search_uses_global_fts() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-indexing-test")?;
    let root = directory.path().join("node");
    let identity = IdentityManager::new(root.join("identity.key"))?;
    let node = IrohNode::new(identity, root.join("data"), RelayMode::Default).await?;
    let folder = FolderManager::new(&node).create(SyncMode::SendReceive).await?;
    folder.set_blob(b"notes/readme.txt", b"catalog content").await?;

    let indexing = IndexingService::in_memory()?;
    let catalog = indexing.catalog_service(
        node.docs_engine(),
        node.blob_store(),
        node.docs_engine().author().await?,
    );
    let namespace = catalog.create_catalog("public files").await?;
    anyhow::ensure!(catalog.publish_folder(&namespace, &folder).await? == 1);

    anyhow::ensure!(indexing.search_local("readme", 10)?.is_empty());
    let results = catalog.search("readme", 10)?;
    anyhow::ensure!(results.len() == 1);
    let record = results.first().context("catalog result should exist")?;
    anyhow::ensure!(record.title == "readme.txt");
    anyhow::ensure!(record.folder_namespace_id == folder.namespace_id());
    anyhow::ensure!(record.catalog_namespace_id == namespace.namespace_id());
    anyhow::ensure!(record.key == b"notes/readme.txt");
    anyhow::ensure!(record.size == 15);
    node.stop().await?;
    Ok(())
}

#[tokio::test]
async fn catalog_subscription_syncs_records_over_iroh_docs() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-indexing-test")?;
    let (relay_map, _relay_url, _relay_server) = iroh::test_utils::run_relay_server().await?;
    let publisher_root = directory.path().join("publisher");
    let publisher_identity = IdentityManager::new(publisher_root.join("identity.key"))?;
    let publisher = IrohNode::new(
        publisher_identity,
        publisher_root.join("data"),
        RelayMode::Custom {
            map: relay_map.clone(),
            insecure: true,
        },
    )
    .await?;
    let subscriber_root = directory.path().join("subscriber");
    let subscriber_identity = IdentityManager::new(subscriber_root.join("identity.key"))?;
    let subscriber = IrohNode::new(
        subscriber_identity,
        subscriber_root.join("data"),
        RelayMode::Custom {
            map: relay_map,
            insecure: true,
        },
    )
    .await?;

    let folder = FolderManager::new(&publisher).create(SyncMode::SendReceive).await?;
    folder.set_blob(b"catalog/readme.md", b"remote catalog entry").await?;
    let publisher_indexing = IndexingService::in_memory()?;
    let publisher_catalog = publisher_indexing.catalog_service(
        publisher.docs_engine(),
        publisher.blob_store(),
        publisher.docs_engine().author().await?,
    );
    let catalog = publisher_catalog.create_catalog("shared catalog").await?;
    publisher_catalog.publish_folder(&catalog, &folder).await?;
    let ticket = publisher_catalog
        .ticket(&catalog, publisher.endpoint().addr(), false)
        .await?;

    let subscriber_indexing = IndexingService::in_memory()?;
    let subscriber_catalog = subscriber_indexing.catalog_service(
        subscriber.docs_engine(),
        subscriber.blob_store(),
        subscriber.docs_engine().author().await?,
    );
    subscriber_catalog.subscribe(ticket).await?;

    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if subscriber_catalog.search("readme", 10)?.len() == 1 {
                return Ok::<_, anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .context("catalog sync timed out")??;

    let result = subscriber_catalog
        .search("readme", 10)?
        .first()
        .context("remote catalog result should exist")?
        .clone();
    anyhow::ensure!(result.key == b"catalog/readme.md");
    anyhow::ensure!(result.folder_namespace_id == folder.namespace_id());

    publisher.stop().await?;
    subscriber.stop().await?;
    Ok(())
}

#[tokio::test]
async fn resilience_fetches_and_pins_when_verified_availability_is_low() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-indexing-test")?;
    let (relay_map, _relay_url, _relay_server) = iroh::test_utils::run_relay_server().await?;
    let publisher_root = directory.path().join("resilience-publisher");
    let publisher_identity = IdentityManager::new(publisher_root.join("identity.key"))?;
    let publisher = IrohNode::new(
        publisher_identity,
        publisher_root.join("data"),
        RelayMode::Custom {
            map: relay_map.clone(),
            insecure: true,
        },
    )
    .await?;
    let subscriber_root = directory.path().join("resilience-subscriber");
    let subscriber_identity = IdentityManager::new(subscriber_root.join("identity.key"))?;
    let subscriber = IrohNode::new(
        subscriber_identity,
        subscriber_root.join("data"),
        RelayMode::Custom {
            map: relay_map,
            insecure: true,
        },
    )
    .await?;

    let hash = publisher.blob_store().add_bytes(b"resilient content").await?;
    let ticket = publisher.blob_store().ticket(publisher.endpoint(), hash);
    let mut lease = ProviderLease::new_with_times(hash, ticket.to_string(), 1, 0, u64::MAX)?;
    lease.sign_with_secret_key(publisher.endpoint().secret_key())?;

    let indexing = IndexingService::in_memory()?;
    let resilience = indexing.resilience_service(ResilienceConfig::new(
        ReplicationBudget::new(2).with_max_jitter(Duration::ZERO),
    ));
    resilience.record_lease(lease)?;
    let result = resilience
        .ensure_replication(subscriber.endpoint(), subscriber.blob_store(), hash)
        .await?;

    anyhow::ensure!(result.pinned);
    anyhow::ensure!(result.fetched_from == vec![publisher.endpoint().id()]);
    anyhow::ensure!(subscriber.blob_store().has(hash).await?);
    anyhow::ensure!(
        subscriber
            .blob_store()
            .list_pins(b"syncweb/replication/")
            .await?
            .iter()
            .any(|(_, pinned_hash)| *pinned_hash == hash)
    );

    publisher.stop().await?;
    subscriber.stop().await?;
    Ok(())
}
