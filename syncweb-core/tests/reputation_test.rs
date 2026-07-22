use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Result, ensure};
use ed25519_dalek::SigningKey;
use iroh::SecretKey;
use iroh_blobs::Hash;
use syncweb_core::indexing::{
    FetchFailureKind, ProviderReputation, ProviderReputationStore, ProviderTrustAction, ProviderTrustDecision,
    ProviderTrustRecord, ProviderTrustSignal, ReputationConfig, TrustPolicy, TrustSignalKind, WotService,
};

fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn provider(seed: u8) -> iroh::PublicKey {
    SecretKey::from_bytes(&[seed; 32]).public()
}

const fn hash(seed: u8) -> Hash {
    Hash::from_bytes([seed; 32])
}

#[test]
fn reputation_scores_history_and_resets_failure_streak() {
    let mut reputation = ProviderReputation::new(provider(1));
    assert!((reputation.reliability_score(10) - 0.5).abs() < f64::EPSILON);
    for _ in 0..5 {
        reputation.record_success(10);
    }
    assert!(reputation.reliability_score(10) > 0.99);
    reputation.record_failure(FetchFailureKind::Timeout, 11);
    assert_eq!(reputation.consecutive_failures, 1);
    reputation.record_success(12);
    assert_eq!(reputation.consecutive_failures, 0);
    assert!(reputation.reliability_score(12) > 0.5);
    assert!(reputation.reliability_score(12 + 24 * 60 * 60) < 1.0);
}

#[test]
fn reputation_store_applies_min_samples_and_backoff() {
    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    config.temporary_ban_duration = Duration::from_secs(10);
    config.auto_ban_backoff_factor = 2.0;
    config.max_auto_ban_duration = Duration::from_secs(100);
    let mut store = ProviderReputationStore::new(config);
    let key = provider(2);
    store.set_reporter(provider(3));
    for timestamp in 1..=3 {
        store.record_failure(key, FetchFailureKind::NotFound, timestamp);
    }
    assert!(store.is_banned(key, 3));
    assert_eq!(store.auto_ban_until(key), Some(13));
    assert_eq!(store.auto_ban_count(key), 1);
    assert_eq!(store.pending_trust_signals().len(), 1);
    store.record_success(key, 14);
    for timestamp in 15..=17 {
        store.record_failure(key, FetchFailureKind::NotFound, timestamp);
    }
    assert_eq!(store.auto_ban_count(key), 2);
    assert_eq!(store.auto_ban_until(key), Some(37));
}

#[test]
fn trust_signal_round_trips_and_ingests_only_from_trusted_reporters() -> Result<()> {
    let reporter = signing_key(3);
    let provider_key = provider(4);
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let mut signal = ProviderTrustSignal::new(
        provider_key,
        TrustSignalKind::ObservedSuccess,
        Some(hash(5)),
        provider(3),
        now,
        1,
    )?;
    signal.sign(&reporter)?;
    signal.verify_at(now)?;
    let decoded = ProviderTrustSignal::from_bytes(signal.to_bytes()?)?;
    ensure!(decoded == signal);

    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    let mut store = ProviderReputationStore::new(config);
    ensure!(!store.ingest_trust_signal(signal.clone())?);
    store.trust_reporter(&provider(3))?;
    ensure!(store.ingest_trust_signal(signal.clone())?);
    ensure!(!store.ingest_trust_signal(signal)?);
    ensure!(store.reputation(provider_key).successful_fetches == 1);
    Ok(())
}

#[test]
fn manual_provider_trust_respects_scope_sequence_and_self_revocation() -> Result<()> {
    let root = signing_key(6);
    let provider = provider(7);
    let scoped_hash = hash(8);
    let service = WotService::in_memory(TrustPolicy::with_root(&root))?;
    let trusted = ProviderTrustRecord::new_with_time(
        provider,
        ProviderTrustAction::Trust,
        Some(scoped_hash),
        1,
        10,
        "reliable",
        &root,
    )?;
    ensure!(service.apply_provider_trust(trusted)?);
    ensure!(
        service.evaluate_provider_trust(provider, Some(&scoped_hash), 10)? == ProviderTrustDecision::Trusted,
        "scoped trust should apply"
    );
    ensure!(
        service.evaluate_provider_trust(provider, Some(&hash(9)), 10)? == ProviderTrustDecision::Unknown,
        "scoped trust should not apply to another hash"
    );

    let self_key = signing_key(7);
    let distrust = ProviderTrustRecord::new_with_time(
        provider,
        ProviderTrustAction::Distrust,
        None,
        2,
        11,
        "retiring",
        &self_key,
    )?;
    ensure!(service.apply_provider_trust(distrust)?);
    ensure!(
        service.evaluate_provider_trust(provider, Some(&scoped_hash), 11)? == ProviderTrustDecision::Distrusted,
        "self-distrust should supersede older trust"
    );
    Ok(())
}

#[test]
fn reputation_score_zero_all_failures() {
    let mut reputation = ProviderReputation::new(provider(10));
    for _ in 0..10 {
        reputation.record_failure(FetchFailureKind::NotFound, 10);
    }
    let score = reputation.reliability_score(10);
    assert!(score < 0.1, "all failures should produce a low score, got {score}");
}

#[test]
fn reputation_score_mixed_proportional() {
    let mut reputation = ProviderReputation::new(provider(11));
    for _ in 0..7 {
        reputation.record_success(10);
    }
    for _ in 0..3 {
        reputation.record_failure(FetchFailureKind::Timeout, 10);
    }
    let score = reputation.reliability_score(10);
    assert!(
        score > 0.3 && score < 0.9,
        "mixed results should produce a mid-range score, got {score}"
    );
}

#[test]
fn reputation_store_ranking_orders_by_score() -> Result<()> {
    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    let mut store = ProviderReputationStore::new(config);
    let good = provider(12);
    let bad = provider(13);
    let neutral = provider(14);
    store.record_success(good, 10);
    store.record_success(good, 11);
    store.record_failure(bad, FetchFailureKind::NotFound, 10);
    store.record_failure(bad, FetchFailureKind::NotFound, 11);
    let ranked = store.rank_provider_list(11, hash(1), &[good, bad, neutral]);
    ensure!(ranked.first() == Some(&good), "good provider should rank first");
    ensure!(ranked.last() == Some(&bad), "bad provider should rank last");
    Ok(())
}

#[test]
fn reputation_store_skip_unreliable() {
    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    let mut store = ProviderReputationStore::new(config);
    let unreliable = provider(15);
    store.record_failure(unreliable, FetchFailureKind::NotFound, 10);
    store.record_failure(unreliable, FetchFailureKind::NotFound, 11);
    assert!(
        store.should_skip_provider(unreliable, 11, 0.5),
        "provider below threshold should be skipped"
    );
    let reliable = provider(16);
    store.record_success(reliable, 10);
    store.record_success(reliable, 11);
    assert!(
        !store.should_skip_provider(reliable, 11, 0.5),
        "provider above threshold should not be skipped"
    );
}

#[test]
fn reputation_store_purge_removes_stale_entries() {
    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    let mut store = ProviderReputationStore::new(config);
    let key = provider(17);
    store.record_success(key, 10);
    store.record_success(key, 11);
    assert!(store.reputation(key).total_fetches > 0);
    store.purge_stale(100, Duration::from_mins(1));
    let rep = store.reputation(key);
    assert_eq!(rep.total_fetches, 0, "purged reputation should return to default");
}

#[test]
fn trust_signal_verify_rejects_invalid_signature() -> Result<()> {
    let reporter = signing_key(20);
    let provider_key = provider(21);
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let mut signal = ProviderTrustSignal::new(
        provider_key,
        TrustSignalKind::ObservedSuccess,
        None,
        provider(20),
        now,
        1,
    )?;
    signal.sign(&reporter)?;
    let mut tampered = signal.clone();
    tampered.signature = Some("deadbeef".repeat(8));
    ensure!(
        tampered.verify_at(now).is_err(),
        "invalid signature should fail verification"
    );
    Ok(())
}

#[test]
fn trust_signal_verify_rejects_expired() -> Result<()> {
    let reporter = signing_key(22);
    let provider_key = provider(23);
    let mut signal = ProviderTrustSignal::new(
        provider_key,
        TrustSignalKind::ObservedFailure,
        None,
        provider(22),
        100,
        1,
    )?;
    signal.sign(&reporter)?;
    let far_future = 100 + 168 * 3600 + 1;
    ensure!(
        signal.verify_at(far_future).is_err(),
        "expired signal should fail verification"
    );
    Ok(())
}

#[test]
fn trust_signal_not_emitted_for_ordinary_success() {
    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    let mut store = ProviderReputationStore::new(config);
    store.set_reporter(provider(30));
    let key = provider(31);
    for t in 1..=5 {
        store.record_success(key, t);
    }
    assert!(
        store.pending_trust_signals().is_empty(),
        "steady-state successes should not emit a signal"
    );
}

#[test]
fn trust_signals_coalesce_duplicate_observations() -> Result<()> {
    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    let mut store = ProviderReputationStore::new(config);
    store.set_reporter(provider(40));
    let key = provider(41);
    store.record_fetch_result(key, true, FetchFailureKind::Unknown, 10);
    store.record_fetch_result(key, false, FetchFailureKind::NotFound, 11);
    store.record_fetch_result(key, false, FetchFailureKind::NotFound, 12);
    let signals = store.take_pending_trust_signals();
    ensure!(
        signals.len() <= 2,
        "duplicate observations should be coalesced, got {}",
        signals.len()
    );
    Ok(())
}

#[test]
fn trust_signal_emitted_on_reputation_transition() {
    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    let mut store = ProviderReputationStore::new(config);
    store.set_reporter(provider(50));
    let key = provider(51);
    for t in 1..=5 {
        store.record_success(key, t);
    }
    assert!(store.score(key, 5) > 0.5);
    store.record_failure(key, FetchFailureKind::NotFound, 1000);
    store.record_failure(key, FetchFailureKind::NotFound, 1001);
    store.record_failure(key, FetchFailureKind::NotFound, 1002);
    assert!(store.score(key, 1002) < 0.5);
    assert!(
        !store.pending_trust_signals().is_empty(),
        "reliable→unreliable transition should emit a signal"
    );
}

#[test]
fn reputation_auto_ban_backoff_never_exceeds_maximum() {
    let mut config = ReputationConfig::default();
    config.min_samples = 1;
    config.temporary_ban_duration = Duration::from_secs(10);
    config.auto_ban_backoff_factor = 100.0;
    config.max_auto_ban_duration = Duration::from_mins(1);
    let mut store = ProviderReputationStore::new(config);
    let key = provider(60);
    for t in 1..=20 {
        store.record_failure(key, FetchFailureKind::NotFound, t);
    }
    let until = store.auto_ban_until(key).expect("should be banned");
    assert!(
        until <= 80,
        "ban should not exceed max_auto_ban_duration (60s from last failure at t=20), got until={until}"
    );
}

#[test]
fn reputation_new_key_returns_neutral_score() {
    let config = ReputationConfig::default();
    let store = ProviderReputationStore::new(config);
    let key = provider(70);
    let result = store.score(key, 100);
    assert!(
        (result - 0.5).abs() < f64::EPSILON,
        "unknown key should have neutral 0.5 score"
    );
}
