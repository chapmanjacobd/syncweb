use anyhow::{Context, ensure};
use std::time::{Duration, SystemTime};

use iroh::SecretKey;
use iroh_blobs::Hash;
use n0_future::StreamExt;
use syncweb_core::{
    filter::FilterEntry,
    folder::{FolderManager, SyncMode},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
    sync::{
        AreaFilter, AreaOfInterest, DeletedTracker, EfficientPeerCache, EvictionStrategy, PeerTracker, SubscribeParams,
        SyncEngine, SyncEvent, TransferStats,
    },
};

const fn blob(byte: u8) -> Hash {
    Hash::from_bytes([byte; 32])
}

#[test]
fn peer_tracker_enforces_lru_and_fifo_limits() -> anyhow::Result<()> {
    let first_peer = SecretKey::generate().public();
    let second_peer = SecretKey::generate().public();
    let mut lru = PeerTracker::new(2, EvictionStrategy::Lru);
    lru.record_peer(blob(1), first_peer);
    lru.record_peer(blob(2), second_peer);
    anyhow::ensure!(lru.get_peers(&blob(1)).len() == 1);
    lru.record_peer(blob(3), first_peer);
    ensure!(lru.contains(&blob(1), &first_peer));
    ensure!(!lru.contains(&blob(2), &second_peer));

    let mut fifo = PeerTracker::new(2, EvictionStrategy::Fifo);
    fifo.record_peer(blob(1), first_peer);
    fifo.record_peer(blob(2), second_peer);
    anyhow::ensure!(fifo.get_peers(&blob(1)).len() == 1);
    fifo.record_peer(blob(3), first_peer);
    ensure!(!fifo.contains(&blob(1), &first_peer));
    anyhow::ensure!(fifo.len() == 2);
    Ok(())
}

#[test]
fn efficient_peer_cache_tracks_compact_presence() -> anyhow::Result<()> {
    let first = SecretKey::generate().public();
    let second = SecretKey::generate().public();
    let mut cache = EfficientPeerCache::new();
    cache.add_presence(blob(1), first)?;
    cache.add_presence(blob(1), second)?;
    cache.add_presence(blob(1), first)?;
    anyhow::ensure!(cache.has_peer(&blob(1), &first));
    anyhow::ensure!(cache.peers(&blob(1)).len() == 2);
    anyhow::ensure!(cache.remove_presence(&blob(1), &first));
    anyhow::ensure!(!cache.has_peer(&blob(1), &first));
    Ok(())
}

#[test]
fn subscription_limits_and_deleted_entries_are_enforced() -> anyhow::Result<()> {
    let area = AreaFilter::Prefix("docs".into());
    let params = SubscribeParams::ingest_only().with_area(area.clone());
    ensure!(params.ingest_only);
    ensure!(
        params
            .area_filter
            .as_ref()
            .is_some_and(|filter| filter.matches_path(std::path::Path::new("docs/a.txt")))
    );

    let limits = AreaOfInterest::with_limits(area, 100, 2);
    ensure!(limits.permits(1, 50, 50));
    ensure!(!limits.permits(2, 50, 1));
    ensure!(!limits.permits(1, 75, 50));

    let hash = blake3::hash(b"deleted");
    let session = uuid::Uuid::new_v4();
    let mut deleted = DeletedTracker::new();
    deleted.record_deletion(hash, "old.txt", 7, session);
    ensure!(deleted.is_deleted(&hash));
    anyhow::ensure!(deleted.deletion_info(&hash).map(|info| info.size) == Some(7));
    ensure!(deleted.restore(&hash).is_some());
    ensure!(deleted.is_empty());
    Ok(())
}

#[test]
fn transfer_stats_report_rate_and_eta() -> anyhow::Result<()> {
    let stats = TransferStats::from_progress(500, Some(1_000), Duration::from_secs(2), 3);
    anyhow::ensure!(stats.bytes_per_second == 250);
    anyhow::ensure!(stats.eta == Some(Duration::from_secs(2)));
    anyhow::ensure!(stats.peer_count == 3);

    let recent = FilterEntry::new("recent", 0).with_modified(SystemTime::now());
    anyhow::ensure!(recent.size == 0);
    Ok(())
}

#[tokio::test]
async fn sync_engine_emits_lifecycle_and_stats() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("syncweb-engine-{}", uuid::Uuid::new_v4()));
    let identity = IdentityManager::new(root.join("identity.key"))?;
    let node = IrohNode::new(identity, root.join("data"), RelayMode::Default).await?;
    let folders = FolderManager::new(&node);
    let folder = folders.create(SyncMode::SendReceive).await?;
    let engine = SyncEngine::new(
        folders,
        node.blob_store().clone(),
        node.docs_engine().clone(),
        node.gossip_service().clone(),
    );
    let mut intent = engine
        .sync(folder.namespace_id(), syncweb_core::sync::SessionMode::ReconcileOnce)
        .await?;
    anyhow::ensure!(intent.next().await == Some(SyncEvent::Started));
    anyhow::ensure!(matches!(intent.next().await, Some(SyncEvent::Progress { .. })));
    anyhow::ensure!(matches!(intent.next().await, Some(SyncEvent::Stats(_))));
    anyhow::ensure!(intent.next().await == Some(SyncEvent::Finished));
    node.stop().await?;
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn test_progress_tracking_emits_completed_and_total() -> anyhow::Result<()> {
    let stats = TransferStats::from_progress(0, Some(0), Duration::from_millis(100), 0);
    anyhow::ensure!(stats.bytes_transferred == 0);
    anyhow::ensure!(stats.bytes_total == Some(0));
    anyhow::ensure!(stats.peer_count == 0);

    let stats2 = TransferStats::from_progress(1024, Some(4096), Duration::from_secs(1), 2);
    anyhow::ensure!(stats2.bytes_per_second == 1024);
    anyhow::ensure!(stats2.peer_count == 2);
    ensure!(stats2.eta.is_some());
    Ok(())
}

#[test]
fn test_transfer_stats_zero_elapsed_avoids_panic() -> anyhow::Result<()> {
    let stats = TransferStats::from_progress(100, Some(200), Duration::ZERO, 1);
    anyhow::ensure!(stats.bytes_per_second == 0);
    anyhow::ensure!(stats.eta == None);
    Ok(())
}

#[test]
fn test_transfer_stats_unknown_total() -> anyhow::Result<()> {
    let stats = TransferStats::from_progress(500, None, Duration::from_secs(5), 3);
    anyhow::ensure!(stats.bytes_per_second == 100);
    anyhow::ensure!(stats.eta == None);
    anyhow::ensure!(stats.peer_count == 3);
    Ok(())
}

#[test]
fn test_track_peer_availability() -> anyhow::Result<()> {
    let peer = SecretKey::generate().public();
    let mut tracker = PeerTracker::new(10, EvictionStrategy::Lru);

    ensure!(!tracker.contains(&blob(1), &peer));
    anyhow::ensure!(tracker.peer_count(&blob(1)) == 0);

    tracker.record_peer(blob(1), peer);
    ensure!(tracker.contains(&blob(1), &peer));
    anyhow::ensure!(tracker.peer_count(&blob(1)) == 1);

    let peers = tracker.get_peers(&blob(1));
    anyhow::ensure!(peers.len() == 1);
    ensure!(peers.contains(&peer));
    Ok(())
}

#[test]
fn test_peer_tracker_multiple_peers_per_blob() -> anyhow::Result<()> {
    let a = SecretKey::generate().public();
    let b = SecretKey::generate().public();
    let c = SecretKey::generate().public();
    let mut tracker = PeerTracker::new(10, EvictionStrategy::Lru);

    tracker.record_peer(blob(1), a);
    tracker.record_peer(blob(1), b);
    tracker.record_peer(blob(1), c);

    anyhow::ensure!(tracker.peer_count(&blob(1)) == 3);
    ensure!(tracker.contains(&blob(1), &a));
    ensure!(tracker.contains(&blob(1), &b));
    ensure!(tracker.contains(&blob(1), &c));
    Ok(())
}

#[test]
fn test_age_based_eviction_lru() -> anyhow::Result<()> {
    let a = SecretKey::generate().public();
    let b = SecretKey::generate().public();
    let c = SecretKey::generate().public();
    let mut tracker = PeerTracker::new(2, EvictionStrategy::Lru);

    tracker.record_peer(blob(1), a);
    tracker.record_peer(blob(2), b);

    // Access blob(1) to make it "recently used" – LRU should evict blob(2) first.
    let _ = tracker.get_peers(&blob(1));

    tracker.record_peer(blob(3), c);

    ensure!(tracker.contains(&blob(1), &a));
    ensure!(!tracker.contains(&blob(2), &b));
    ensure!(tracker.len() <= 2);
    Ok(())
}

#[test]
fn test_age_based_eviction_fifo() -> anyhow::Result<()> {
    let a = SecretKey::generate().public();
    let b = SecretKey::generate().public();
    let c = SecretKey::generate().public();
    let mut tracker = PeerTracker::new(2, EvictionStrategy::Fifo);

    tracker.record_peer(blob(1), a);
    tracker.record_peer(blob(2), b);

    // Even after accessing blob(1), FIFO should evict the oldest entry (blob(1)).
    let _ = tracker.get_peers(&blob(1));

    tracker.record_peer(blob(3), c);

    ensure!(!tracker.contains(&blob(1), &a));
    ensure!(tracker.contains(&blob(2), &b));
    ensure!(tracker.len() <= 2);
    Ok(())
}

#[test]
fn test_max_cache_size_is_respected() -> anyhow::Result<()> {
    let mut tracker = PeerTracker::new(3, EvictionStrategy::Lru);
    for i in 0..10 {
        let peer = SecretKey::generate().public();
        tracker.record_peer(blob(i), peer);
    }
    ensure!(tracker.len() <= 3, "cache should not exceed max size");
    Ok(())
}

#[test]
fn test_peer_tracker_expiry_removes_stale_entries() -> anyhow::Result<()> {
    let peer = SecretKey::generate().public();
    let mut tracker = PeerTracker::with_expiry(10, EvictionStrategy::Lru, Duration::ZERO);

    tracker.record_peer(blob(1), peer);
    ensure!(tracker.contains(&blob(1), &peer));

    // With zero expiry, tick should evict immediately.
    tracker.tick_and_maybe_evict();
    ensure!(!tracker.contains(&blob(1), &peer));
    ensure!(tracker.is_empty());
    Ok(())
}

#[test]
fn test_peer_tracker_on_peer_disconnected() -> anyhow::Result<()> {
    let a = SecretKey::generate().public();
    let b = SecretKey::generate().public();
    let mut tracker = PeerTracker::new(10, EvictionStrategy::Lru);

    tracker.record_peer(blob(1), a);
    tracker.record_peer(blob(1), b);
    tracker.record_peer(blob(2), a);

    tracker.on_peer_disconnected(&a);

    ensure!(!tracker.contains(&blob(1), &a));
    ensure!(!tracker.contains(&blob(2), &a));
    ensure!(tracker.contains(&blob(1), &b));
    Ok(())
}

#[test]
fn test_peer_tracker_on_blob_fetched() -> anyhow::Result<()> {
    let peer = SecretKey::generate().public();
    let mut tracker = PeerTracker::new(10, EvictionStrategy::Lru);

    tracker.on_blob_fetched(blob(1), peer);
    ensure!(tracker.contains(&blob(1), &peer));
    anyhow::ensure!(tracker.peer_count(&blob(1)) == 1);
    Ok(())
}

#[test]
fn test_peer_cache_lookup_perf() -> anyhow::Result<()> {
    let mut tracker = PeerTracker::new(10_000, EvictionStrategy::Lru);
    let peers: Vec<_> = (0..100).map(|_| SecretKey::generate().public()).collect();

    for i in 0..1_000_u16 {
        let idx = usize::from(i % 100);
        let peer = *peers.get(idx).context("idx < 100")?;
        tracker.record_peer(blob(u8::try_from(idx).context("unwrap failed")?), peer);
    }

    let start = std::time::Instant::now();
    for i in 0..1_000_u16 {
        let idx = usize::from(i % 100);
        let _ = tracker.get_peers(&blob(u8::try_from(idx).context("unwrap failed")?));
    }
    let elapsed = start.elapsed();
    // 1000 lookups should complete well under 100ms.
    ensure!(
        elapsed < Duration::from_millis(100),
        "1000 lookups took {elapsed:?}, expected < 100ms"
    );
    Ok(())
}

#[test]
fn test_bitmask_set_clear_check() -> anyhow::Result<()> {
    let peer = SecretKey::generate().public();
    let mut cache = EfficientPeerCache::new();

    cache.add_presence(blob(1), peer)?;
    anyhow::ensure!(cache.has_peer(&blob(1), &peer));

    anyhow::ensure!(cache.remove_presence(&blob(1), &peer));
    anyhow::ensure!(!cache.has_peer(&blob(1), &peer));

    // Removing again should return false.
    anyhow::ensure!(!cache.remove_presence(&blob(1), &peer));
    Ok(())
}

#[test]
fn test_memory_efficiency_under_1mb() -> anyhow::Result<()> {
    let mut cache = EfficientPeerCache::new();
    let peers: Vec<_> = (0..1000).map(|_| SecretKey::generate().public()).collect();

    for (i, peer) in peers.iter().enumerate() {
        for j in 0..10 {
            let hash = blob(u8::try_from((i * 10 + j) % 255).unwrap_or(0));
            cache.add_presence(hash, *peer)?;
        }
    }

    let usage = cache.memory_usage();
    anyhow::ensure!(
        usage < 1_024 * 1024,
        "cache uses {usage} bytes, expected < 1MB for 1000 peers"
    );
    Ok(())
}

#[test]
fn test_compressed_indices_active_peers_per_blob() -> anyhow::Result<()> {
    let a = SecretKey::generate().public();
    let b = SecretKey::generate().public();
    let c = SecretKey::generate().public();
    let mut cache = EfficientPeerCache::new();

    cache.add_presence(blob(1), a)?;
    cache.add_presence(blob(1), b)?;
    cache.add_presence(blob(2), a)?;
    cache.add_presence(blob(2), c)?;

    anyhow::ensure!(cache.active_peer_count(&blob(1)) == 2);
    anyhow::ensure!(cache.active_peer_count(&blob(2)) == 2);

    let peers1 = cache.peers(&blob(1));
    anyhow::ensure!(peers1.contains(&a));
    anyhow::ensure!(peers1.contains(&b));
    anyhow::ensure!(!peers1.contains(&c));

    Ok(())
}

#[test]
fn test_efficient_cache_remove_peer_cleans_all_blobs() -> anyhow::Result<()> {
    let a = SecretKey::generate().public();
    let b = SecretKey::generate().public();
    let mut cache = EfficientPeerCache::new();

    cache.add_presence(blob(1), a)?;
    cache.add_presence(blob(2), a)?;
    cache.add_presence(blob(1), b)?;

    cache.remove_peer(&a);

    anyhow::ensure!(!cache.has_peer(&blob(1), &a));
    anyhow::ensure!(!cache.has_peer(&blob(2), &a));
    anyhow::ensure!(cache.has_peer(&blob(1), &b));
    Ok(())
}

#[test]
fn test_efficient_cache_is_idempotent() -> anyhow::Result<()> {
    let peer = SecretKey::generate().public();
    let mut cache = EfficientPeerCache::new();

    cache.add_presence(blob(1), peer)?;
    cache.add_presence(blob(1), peer)?;
    cache.add_presence(blob(1), peer)?;

    anyhow::ensure!(cache.active_peer_count(&blob(1)) == 1);
    Ok(())
}

#[test]
fn test_subscribe_params_default_allows_everything() -> anyhow::Result<()> {
    let params = SubscribeParams::default();
    ensure!(!params.ingest_only);
    ensure!(params.ignore_session.is_none());
    ensure!(params.area_filter.is_none());
    ensure!(params.area_of_interest.is_none());
    ensure!(params.accepts(std::path::Path::new("any/file.txt"), &blob(1)));
    Ok(())
}

#[test]
fn test_subscribe_params_ignore_session() -> anyhow::Result<()> {
    let session = uuid::Uuid::new_v4();
    let params = SubscribeParams::ignore_session(session);
    anyhow::ensure!(params.ignore_session == Some(session));
    ensure!(!params.ingest_only);
    Ok(())
}

#[test]
fn test_area_of_interest_unlimited_permits_all() -> anyhow::Result<()> {
    let area = AreaOfInterest::unlimited(AreaFilter::All);
    ensure!(area.permits(0, 0, u64::MAX));
    ensure!(!area.is_limit_reached(0, 0));
    Ok(())
}

#[test]
fn test_area_of_interest_count_limit() -> anyhow::Result<()> {
    let area = AreaOfInterest::with_count_limit(AreaFilter::All, 3);
    ensure!(area.permits(0, 0, 100));
    ensure!(area.permits(1, 0, 100));
    ensure!(area.permits(2, 0, 100));
    ensure!(!area.permits(3, 0, 100));
    ensure!(area.is_limit_reached(3, 0));
    Ok(())
}

#[test]
fn test_area_of_interest_size_limit() -> anyhow::Result<()> {
    let area = AreaOfInterest::with_size_limit(AreaFilter::All, 1000);
    ensure!(area.permits(0, 0, 500));
    ensure!(area.permits(0, 500, 500));
    ensure!(!area.permits(0, 500, 600));
    ensure!(area.is_limit_reached(0, 1000));
    Ok(())
}

#[test]
fn test_deleted_tracker_records_and_restores() -> anyhow::Result<()> {
    let hash = blake3::hash(b"file-content");
    let session = uuid::Uuid::new_v4();
    let mut tracker = DeletedTracker::new();

    tracker.record_deletion(hash, "data.txt", 42, session);
    ensure!(tracker.is_deleted(&hash));
    anyhow::ensure!(tracker.len() == 1);

    let info = tracker.deletion_info(&hash).context("should have info")?;
    anyhow::ensure!(info.path == std::path::PathBuf::from("data.txt"));
    anyhow::ensure!(info.size == 42);
    anyhow::ensure!(info.deleted_by == session);

    let restored = tracker.restore(&hash).context("should restore")?;
    anyhow::ensure!(restored.size == 42);
    ensure!(!tracker.is_deleted(&hash));
    ensure!(tracker.is_empty());
    Ok(())
}

#[test]
fn test_deleted_tracker_undelete_alias() -> anyhow::Result<()> {
    let hash = blake3::hash(b"another");
    let mut tracker = DeletedTracker::new();
    tracker.record_deletion(hash, "old.txt", 7, uuid::Uuid::new_v4());

    let restored = tracker.undelete(&hash).context("undelete should work")?;
    anyhow::ensure!(restored.path == std::path::PathBuf::from("old.txt"));
    ensure!(tracker.is_empty());
    Ok(())
}

#[test]
fn test_deleted_tracker_iter() -> anyhow::Result<()> {
    let h1 = blake3::hash(b"a");
    let h2 = blake3::hash(b"b");
    let mut tracker = DeletedTracker::new();
    tracker.record_deletion(h1, "a.txt", 1, uuid::Uuid::new_v4());
    tracker.record_deletion(h2, "b.txt", 2, uuid::Uuid::new_v4());

    anyhow::ensure!(tracker.iter().count() == 2);
    Ok(())
}

#[test]
fn test_area_filter_prefix_matches() -> anyhow::Result<()> {
    let filter = AreaFilter::Prefix(std::path::PathBuf::from("docs"));
    ensure!(filter.matches_path(std::path::Path::new("docs/readme.md")));
    ensure!(filter.matches_path(std::path::Path::new("docs/sub/file.txt")));
    ensure!(!filter.matches_path(std::path::Path::new("src/main.rs")));
    Ok(())
}

#[test]
fn test_area_filter_hash_range() -> anyhow::Result<()> {
    let start = [0_u8; 32];
    let mut end = [0_u8; 32];
    end[0] = 0x80;
    let filter = AreaFilter::HashRange(start, end);

    let low = Hash::from_bytes([0x40; 32]);
    let high = Hash::from_bytes([0xC0; 32]);

    ensure!(filter.matches_hash(&low));
    ensure!(!filter.matches_hash(&high));
    Ok(())
}
