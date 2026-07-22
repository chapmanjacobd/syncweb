use std::time::Duration;

use anyhow::{Result, ensure};
use ed25519_dalek::SigningKey;
use iroh::{EndpointAddr, PublicKey, SecretKey};
use iroh_blobs::{BlobFormat, Hash, ticket::BlobTicket};
use syncweb_core::indexing::{
    BanSource, FetchFailure, FetchFailureKind, ProviderLease, ProviderLeaseTracker, ProviderReputationStore,
    ProviderTrustAction, ProviderTrustDecision, ProviderTrustRecord, ProviderTrustSignal, ReputationConfig,
    ResilienceConfig, ResilienceService, TrustPolicy, TrustSignalKind, WotService,
};

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn provider(seed: u8) -> PublicKey {
    SecretKey::from_bytes(&[seed; 32]).public()
}

const fn hash(seed: u8) -> Hash {
    Hash::from_bytes([seed; 32])
}

fn lease(seed: u8, content: Hash) -> Result<ProviderLease> {
    let secret = SecretKey::from_bytes(&[seed; 32]);
    let ticket = BlobTicket::new(EndpointAddr::new(secret.public()), content, BlobFormat::Raw);
    Ok(ProviderLease::signed(
        content,
        ticket.to_string(),
        1,
        u64::MAX,
        &secret,
    )?)
}

#[test]
fn test_full_smart_ban_workflow() -> Result<()> {
    let content = hash(1);
    let provider_key = provider(2);
    let mut tracker = ProviderLeaseTracker::default();
    tracker.track_at(lease(2, content)?, 10)?;
    anyhow::ensure!(tracker.health_at(&content, 10, Duration::from_mins(1)).verified == 1);

    let failure = FetchFailure::new_at(FetchFailureKind::NotFound, provider_key, content, 11, "missing");
    tracker.record_failure_at(content, provider_key, failure, 11);
    anyhow::ensure!(tracker.invalidate_lease_at(content, provider_key, 11));
    anyhow::ensure!(tracker.health_at(&content, 11, Duration::from_mins(1)).verified == 0);
    anyhow::ensure!(tracker.is_banned(provider_key, &content, 11));
    Ok(())
}

#[test]
fn test_retroactive_invalidation_workflow() -> Result<()> {
    let content = hash(3);
    let failed = provider(4);
    let successful = provider(5);
    let mut tracker = ProviderLeaseTracker::default();
    tracker.track_at(lease(4, content)?, 10)?;
    tracker.track_at(lease(5, content)?, 10)?;
    tracker.record_failure_at(
        content,
        failed,
        FetchFailure::new_at(FetchFailureKind::Corruption, failed, content, 11, "bad bytes"),
        11,
    );

    let invalidated = tracker.retroactive_invalidate(content, successful, 12);
    anyhow::ensure!(invalidated == vec![failed]);
    let remaining = tracker.leases_at(&content, 12);
    anyhow::ensure!(remaining.len() == 1);
    anyhow::ensure!(remaining.first().is_some_and(|lease| lease.provider == successful));
    Ok(())
}

#[test]
fn test_provider_reputation_across_fetches() {
    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    let mut store = ProviderReputationStore::new(config);
    let provider_key = provider(6);
    store.record_success(provider_key, 10);
    store.record_success(provider_key, 11);
    let healthy = store.score(provider_key, 11);
    store.record_failure(provider_key, FetchFailureKind::Timeout, 12);
    assert!(store.score(provider_key, 12) < healthy);
    assert_eq!(store.reputation(provider_key).total_fetches, 3);
}

#[test]
fn test_trust_stream_aggregation() -> Result<()> {
    let reporter = key(7);
    let provider_key = provider(8);
    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    let mut store = ProviderReputationStore::new(config);
    store.trust_reporter(&provider(7))?;
    let signal =
        ProviderTrustSignal::new_with_time(provider_key, TrustSignalKind::ObservedSuccess, None, 1, &reporter)?;
    anyhow::ensure!(store.ingest_trust_signal(signal.clone())?);
    anyhow::ensure!(!store.ingest_trust_signal(signal)?);
    anyhow::ensure!(store.reputation(provider_key).successful_fetches == 1);
    Ok(())
}

#[test]
fn test_manual_provider_trust_workflow() -> Result<()> {
    let root = key(9);
    let provider_key = provider(10);
    let service = WotService::in_memory(TrustPolicy::with_root(&root))?;
    let vouch =
        ProviderTrustRecord::new_with_time(provider_key, ProviderTrustAction::Vouch, None, 1, 10, "reliable", &root)?;
    anyhow::ensure!(service.apply_provider_trust(vouch)?);
    anyhow::ensure!(service.evaluate_provider_trust(provider_key, None, 10)? == ProviderTrustDecision::Trusted);
    let distrust = ProviderTrustRecord::new_with_time(
        provider_key,
        ProviderTrustAction::Distrust,
        None,
        2,
        11,
        "bad fetches",
        &root,
    )?;
    anyhow::ensure!(service.apply_provider_trust(distrust)?);
    anyhow::ensure!(service.evaluate_provider_trust(provider_key, None, 11)? == ProviderTrustDecision::Distrusted);
    Ok(())
}

#[test]
fn test_ban_and_trust_interplay() -> Result<()> {
    let content = hash(11);
    let provider_key = provider(12);
    let resilience = ResilienceService::new(ResilienceConfig::default());
    resilience.record_lease(lease(12, content)?)?;
    let wot = WotService::in_memory(TrustPolicy::with_root(&key(13)))?;
    let trusted = ProviderTrustRecord::new_with_time(
        provider_key,
        ProviderTrustAction::Trust,
        None,
        1,
        10,
        "trusted",
        &key(13),
    )?;
    wot.apply_provider_trust(trusted)?;
    anyhow::ensure!(wot.evaluate_provider_trust(provider_key, None, 10)? == ProviderTrustDecision::Trusted);
    resilience.ban_provider(provider_key, "manual", None, None)?;
    anyhow::ensure!(resilience.health(&content)?.verified == 0);
    resilience.unban_provider(provider_key, None)?;
    anyhow::ensure!(resilience.health(&content)?.verified == 1);
    Ok(())
}

#[test]
fn test_consecutive_failure_auto_ban() {
    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    config.temporary_ban_duration = Duration::from_secs(10);
    let mut store = ProviderReputationStore::new(config);
    let provider_key = provider(14);
    for timestamp in 1..=3 {
        store.record_failure(provider_key, FetchFailureKind::NotFound, timestamp);
    }
    assert!(store.is_banned(provider_key, 3));
    assert!(!store.is_banned(provider_key, 14));
}

#[test]
fn test_wot_delegation_provider_trust() -> Result<()> {
    let root = key(15);
    let delegate = key(16);
    let provider_key = provider(17);
    let service = WotService::in_memory(TrustPolicy::with_root(&root))?;
    let delegation = syncweb_core::indexing::TrustDelegation::new_with_time(
        &delegate.verifying_key(),
        None,
        1,
        10,
        u64::MAX,
        &root,
    )?;
    anyhow::ensure!(service.add_delegation_at(delegation, 10)?);
    let record = ProviderTrustRecord::new_with_time(
        provider_key,
        ProviderTrustAction::Trust,
        None,
        1,
        11,
        "delegated trust",
        &delegate,
    )?;
    anyhow::ensure!(service.apply_provider_trust(record)?);
    anyhow::ensure!(service.evaluate_provider_trust(provider_key, None, 11)? == ProviderTrustDecision::Trusted);
    Ok(())
}

#[test]
fn test_full_replication_with_smart_ban() -> Result<()> {
    let content = hash(18);
    let failed = provider(19);
    let successful = provider(20);
    let resilience = ResilienceService::new(ResilienceConfig::default());
    resilience.record_lease(lease(19, content)?)?;
    resilience.record_lease(lease(20, content)?)?;
    resilience.record_failure(
        content,
        failed,
        FetchFailure::new(FetchFailureKind::NotFound, failed, content, "missing"),
    )?;
    anyhow::ensure!(resilience.invalidate_provider(content, failed)?);
    anyhow::ensure!(resilience.health(&content)?.verified == 1);
    anyhow::ensure!(resilience.health(&content)?.verified_providers == vec![successful]);
    Ok(())
}

#[test]
fn test_manual_ban_records_source() {
    let content = hash(21);
    let provider_key = provider(22);
    let mut tracker = ProviderLeaseTracker::default();
    let record = tracker.ban_provider(
        provider_key,
        Some(content),
        "manual",
        BanSource::Manual,
        Some(Duration::from_secs(10)),
        100,
    );
    assert_eq!(record.source, BanSource::Manual);
    assert!(tracker.is_banned(provider_key, &content, 109));
    assert!(!tracker.is_banned(provider_key, &content, 110));
}

#[test]
fn test_resilience_respects_provider_trust_distrust() -> Result<()> {
    let content = hash(30);
    let distrusted = provider(31);
    let mut tracker = ProviderLeaseTracker::default();
    tracker.track_at(lease(30, content)?, 10)?;
    tracker.track_at(lease(31, content)?, 10)?;
    let providers = tracker.providers(&content);
    ensure!(providers.contains(&distrusted));

    let wot_key = key(32);
    let wot = WotService::in_memory(TrustPolicy::with_root(&wot_key))?;
    let distrust = ProviderTrustRecord::new_with_time(
        distrusted,
        ProviderTrustAction::Distrust,
        None,
        1,
        10,
        "unreliable",
        &wot_key,
    )?;
    wot.apply_provider_trust(distrust)?;
    ensure!(wot.evaluate_provider_trust(distrusted, None, 10)? == ProviderTrustDecision::Distrusted);

    let resilience = ResilienceService::with_wot(ResilienceConfig::default(), wot);
    resilience.record_lease(lease(30, content)?)?;
    resilience.record_lease(lease(31, content)?)?;
    let ranked = resilience.responsible_providers(&content)?;
    ensure!(!ranked.contains(&distrusted), "distrusted provider should be excluded");
    Ok(())
}

#[test]
fn test_resilience_respects_provider_trust_trust() -> Result<()> {
    let content = hash(40);
    let trusted = provider(41);
    let wot_key = key(43);
    let wot = WotService::in_memory(TrustPolicy::with_root(&wot_key))?;
    let trust =
        ProviderTrustRecord::new_with_time(trusted, ProviderTrustAction::Trust, None, 1, 10, "verified", &wot_key)?;
    wot.apply_provider_trust(trust)?;

    let resilience = ResilienceService::with_wot(ResilienceConfig::default(), wot);
    resilience.record_lease(lease(41, content)?)?;
    resilience.record_lease(lease(42, content)?)?;
    let ranked = resilience.responsible_providers(&content)?;
    ensure!(
        ranked.first() == Some(&trusted),
        "trusted provider should be ranked first, got {ranked:?}"
    );
    Ok(())
}

#[test]
fn test_resilience_reputation_weighted_selection() -> Result<()> {
    let content = hash(50);
    let good = provider(51);
    let bad = provider(52);
    let config = ResilienceConfig::new(syncweb_core::indexing::ReplicationBudget::new(2).with_responsible_peers(2));
    let mut rep_config = ReputationConfig::default();
    rep_config.min_samples = 1;
    let resilience = ResilienceService::with_reputation(config, rep_config, None);
    resilience.record_lease(lease(50, content)?)?;
    resilience.record_lease(lease(51, content)?)?;
    resilience.record_lease(lease(52, content)?)?;
    let rank_store = resilience.reputation_store();
    {
        #[allow(clippy::unwrap_in_result)]
        let mut store = rank_store.lock().unwrap();
        store.record_success(good, 10);
        store.record_success(good, 11);
        store.record_failure(bad, FetchFailureKind::NotFound, 10);
        store.record_failure(bad, FetchFailureKind::NotFound, 11);
    }
    let ranked = resilience.responsible_providers(&content)?;
    ensure!(
        ranked.first() == Some(&good),
        "higher-reputation provider should be ranked first, got {ranked:?}"
    );
    Ok(())
}

#[test]
fn test_resilience_no_wot_service() -> Result<()> {
    let content = hash(60);
    let resilience = ResilienceService::new(ResilienceConfig::default());
    resilience.record_lease(lease(60, content)?)?;
    let ranked = resilience.responsible_providers(&content)?;
    ensure!(ranked.len() == 1, "without WoT, all providers should be returned");
    Ok(())
}

#[test]
fn test_resilience_reputation_on_fetch_success() -> Result<()> {
    let config = ResilienceConfig::default();
    let mut rep_config = ReputationConfig::default();
    rep_config.min_samples = 1;
    let resilience = ResilienceService::with_reputation(config, rep_config, None);
    let key = provider(70);
    let hash_val = hash(71);
    resilience.record_success(&hash_val, &key)?;
    let score = resilience.reputation_score(key)?;
    ensure!(score > 0.5, "success should improve score, got {score}");
    Ok(())
}

#[test]
fn test_resilience_reputation_on_fetch_failure() -> Result<()> {
    let config = ResilienceConfig::default();
    let mut rep_config = ReputationConfig::default();
    rep_config.min_samples = 1;
    let resilience = ResilienceService::with_reputation(config, rep_config, None);
    let key = provider(72);
    let hash_val = hash(73);
    resilience.record_failure(
        hash_val,
        key,
        FetchFailure::new(FetchFailureKind::NotFound, key, hash_val, "missing"),
    )?;
    resilience.record_failure(
        hash_val,
        key,
        FetchFailure::new(FetchFailureKind::NotFound, key, hash_val, "missing"),
    )?;
    let score = resilience.reputation_score(key)?;
    ensure!(score < 0.5, "failure should decrease score, got {score}");
    Ok(())
}

#[test]
fn test_trust_gossip_is_batched() {
    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    let mut store = ProviderReputationStore::new(config);
    store.set_reporter(provider(80));
    let key = provider(81);
    for t in 1..=20 {
        store.record_fetch_result(key, t % 2 == 0, FetchFailureKind::Unknown, t);
    }
    let signals = store.take_pending_trust_signals();
    assert!(
        signals.len() < 20,
        "trust signals should be batched, got {}",
        signals.len()
    );
}

#[test]
fn test_failure_record_retention_under_sustained_failures() {
    let mut tracker = ProviderLeaseTracker::with_max_failures_per_provider(10);
    let content = hash(90);
    let key = provider(91);
    for t in 1..=50 {
        tracker.record_failure_at(
            content,
            key,
            FetchFailure::new_at(FetchFailureKind::NotFound, key, content, t, "missing"),
            t,
        );
    }
    let record = tracker.failure_record(&content, &key).expect("should have record");
    assert!(
        record.failures.len() <= 10,
        "failure details should be capped at 10, got {}",
        record.failures.len()
    );
    assert_eq!(
        tracker.failure_count(&content, &key),
        50,
        "aggregate count should reflect all failures"
    );
}

#[test]
fn test_rejoined_provider_receives_exponential_ban() {
    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    config.temporary_ban_duration = Duration::from_secs(10);
    config.auto_ban_backoff_factor = 2.0;
    config.max_auto_ban_duration = Duration::from_hours(24);
    let mut store = ProviderReputationStore::new(config);
    let key = provider(92);

    for t in 1..=3 {
        store.record_failure(key, FetchFailureKind::NotFound, t);
    }
    let first_until = store.auto_ban_until(key).expect("first ban");
    assert_eq!(first_until, 13, "first ban should last 10s");

    for t in 14..=16 {
        store.record_failure(key, FetchFailureKind::NotFound, t);
    }
    let second_until = store.auto_ban_until(key).expect("second ban");
    assert!(second_until > first_until, "second ban should be longer than first");
}

#[test]
fn test_permanent_ban_persists() {
    let content = hash(93);
    let key = provider(94);
    let mut tracker = ProviderLeaseTracker::default();
    tracker.ban_provider(key, Some(content), "permanent", BanSource::Manual, None, 100);
    assert!(
        tracker.is_banned(key, &content, 1000),
        "permanent ban should still be active"
    );
    assert!(
        tracker.is_banned(key, &content, 999_999),
        "permanent ban should never expire"
    );
}

#[test]
fn test_manual_ban_overrides_automated() {
    let content = hash(95);
    let key = provider(96);
    let mut tracker = ProviderLeaseTracker::default();
    tracker.ban_provider(
        key,
        Some(content),
        "automated",
        BanSource::Automated,
        Some(Duration::from_secs(10)),
        100,
    );
    tracker.ban_provider(
        key,
        Some(content),
        "manual override",
        BanSource::Manual,
        Some(Duration::from_secs(5)),
        200,
    );
    let records = tracker.ban_records(&content, 200);
    assert!(
        records.iter().any(|r| r.source == BanSource::Manual),
        "manual ban should take precedence"
    );
    assert!(!tracker.is_banned(key, &content, 210), "short manual ban should expire");
    assert!(tracker.is_banned(key, &content, 204), "manual ban should be active");
}

#[test]
fn test_global_ban_applies_to_all_hashes() {
    let content_a = hash(97);
    let content_b = hash(98);
    let key = provider(99);
    let mut tracker = ProviderLeaseTracker::default();
    tracker.ban_provider(
        key,
        None,
        "global ban",
        BanSource::Manual,
        Some(Duration::from_mins(1)),
        100,
    );
    assert!(
        tracker.is_banned(key, &content_a, 100),
        "global ban should affect hash A"
    );
    assert!(
        tracker.is_banned(key, &content_b, 100),
        "global ban should affect hash B"
    );
    let all_banned = tracker.banned_providers(&content_a, 100);
    assert!(all_banned.contains(&key));
}
