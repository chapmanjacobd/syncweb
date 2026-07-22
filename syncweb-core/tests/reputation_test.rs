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
