mod test_utils;

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use iroh::address_lookup::memory::MemoryLookup;
use iroh_blobs::Hash;
use n0_future::StreamExt;
use syncweb_core::{
    folder::{Capability, FolderManager, SyncMode},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
    sync::{FetchCandidate, FetchFilter, FetchStrategy, HealthReport, SyncEngine, SyncEvent},
};

use crate::test_utils::TestDirectory;

fn candidate(path: &str, peers: usize, size: u64) -> FetchCandidate {
    FetchCandidate::new(path, Hash::new(path.as_bytes()), size, peers, false)
}

#[test]
fn fetch_filter_selects_least_seeded_blobs_and_honors_limits() {
    let candidates = vec![
        candidate("well", 5, 10),
        candidate("rare", 0, 20),
        candidate("under", 1, 30),
    ];
    let filter = FetchFilter::new().with_max_peers(1).with_max_count(2);
    let selected = filter.select(&candidates);

    assert_eq!(
        selected.iter().map(|item| item.path.clone()).collect::<Vec<_>>(),
        vec![PathBuf::from("rare"), PathBuf::from("under")]
    );
    assert_eq!(FetchStrategy::filter(filter).select(&candidates).len(), 2);

    let min_peers = FetchFilter::new().with_min_peers(1);
    let selected_min_peers = min_peers.select(&candidates);
    assert_eq!(
        selected_min_peers
            .iter()
            .map(|item| item.path.clone())
            .collect::<Vec<_>>(),
        vec![PathBuf::from("under"), PathBuf::from("well")]
    );
}

#[test]
fn fetch_filter_applies_paths_and_size_ranges() {
    let candidates = vec![candidate("audio/a.flac", 1, 100), candidate("docs/a.txt", 1, 10)];
    let filter = FetchFilter::new()
        .with_paths(vec![PathBuf::from("audio")])
        .with_min_size(50)
        .with_max_size(150);
    let selected = filter.select(&candidates);
    assert_eq!(selected.len(), 1);
    assert!(
        selected
            .first()
            .is_some_and(|item| item.path.as_path() == Path::new("audio/a.flac"))
    );
}

#[test]
fn health_report_groups_seed_counts() {
    let candidates = vec![candidate("a", 0, 1), candidate("b", 2, 2), candidate("c", 4, 3)];
    let report = HealthReport::from_candidates(&candidates, 4);
    assert_eq!(report.total, 3);
    assert_eq!(report.well_seeded, 1);
    assert_eq!(report.under_seeded, 1);
    assert_eq!(report.unseeded, 1);
    assert!(
        report
            .least_seeded
            .first()
            .is_some_and(|item| item.path.as_path() == Path::new("a"))
    );
}

#[tokio::test]
async fn test_download_max_peers() -> anyhow::Result<()> {
    let directory = TestDirectory::new("syncweb-pfetch-test")?;
    let (relay_map, relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let memory_lookup = MemoryLookup::new();

    let root_a = directory.path().join("seeder");
    let identity_a = IdentityManager::new(root_a.join("identity.key"))?;
    let node_a = IrohNode::new_with_address_lookup(
        identity_a,
        root_a.join("data"),
        RelayMode::Custom {
            map: relay_map.clone(),
            insecure: true,
        },
        memory_lookup.clone(),
    )
    .await?;

    let root_b = directory.path().join("downloader");
    let identity_b = IdentityManager::new(root_b.join("identity.key"))?;
    let node_b = IrohNode::new_with_address_lookup(
        identity_b,
        root_b.join("data"),
        RelayMode::Custom {
            map: relay_map,
            insecure: true,
        },
        memory_lookup.clone(),
    )
    .await?;

    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(node_a.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(node_b.endpoint().id()).with_relay_url(relay_url));

    let manager_a = FolderManager::new(&node_a);
    let folder_a = manager_a.create(SyncMode::SendReceive).await?;
    folder_a.grant(node_a.endpoint().id(), Capability::Admin).await;
    folder_a.set_blob("file_a.txt", b"content_a").await?;
    folder_a.set_blob("file_b.txt", b"content_b").await?;

    node_a.topic_tracker().announce(folder_a.namespace_id()).await?;
    let ticket = folder_a.ticket(node_a.endpoint().addr(), true).await?;

    let manager_b = FolderManager::new(&node_b);
    let folder_b = manager_b.join(ticket.to_string(), SyncMode::ReceiveOnly).await?;
    node_b.topic_tracker().announce(folder_b.namespace_id()).await?;

    let entry = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Some(entry) = node_b
                .docs_engine()
                .get(folder_b.doc(), folder_a.author(), "file_a.txt")
                .await?
            {
                return anyhow::Ok(entry);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .context("timed out waiting for initial sync")?
    .context("initial sync entry should exist")?;
    let original_hash = entry.content_hash();

    let sync = SyncEngine::new(
        FolderManager::new(&node_b),
        node_b.blob_store().clone(),
        node_b.docs_engine().clone(),
        node_b.gossip_service().clone(),
    );

    let filter = FetchFilter::new().with_max_peers(1);
    let strategy = FetchStrategy::filter(filter);
    let mut intent = sync.fetch(folder_b.namespace_id(), strategy).await?;

    let mut download_completed = false;
    while let Some(event) = intent.next().await {
        match event {
            SyncEvent::Finished => {
                download_completed = true;
                break;
            }
            SyncEvent::Failed(message) => {
                anyhow::bail!("download failed: {message}");
            }
            SyncEvent::Started
            | SyncEvent::Progress { .. }
            | SyncEvent::Stats(_)
            | SyncEvent::Paused
            | SyncEvent::Resumed
            | SyncEvent::Cancelled
            | _ => {}
        }
    }
    anyhow::ensure!(download_completed, "download should complete");

    anyhow::ensure!(
        node_b.blob_store().get(original_hash).await? == b"content_a".as_slice(),
        "downloaded content should match"
    );

    node_a.stop().await?;
    node_b.stop().await?;
    Ok(())
}
