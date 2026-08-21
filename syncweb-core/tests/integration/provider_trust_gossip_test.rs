use anyhow::ensure;
use ed25519_dalek::SigningKey;
use iroh::SecretKey;
use syncweb_core::indexing::{ProviderTrustAction, ProviderTrustRecord, ProviderTrustSignal, TrustSignalKind};

fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn provider(seed: u8) -> iroh::PublicKey {
    SecretKey::from_bytes(&[seed; 32]).public()
}

#[test]
fn test_from_trust_record_vouch_converts_to_signal() -> anyhow::Result<()> {
    let key = signing_key(1);
    let provider_key = provider(2);
    let record = ProviderTrustRecord::new(
        provider_key,
        ProviderTrustAction::Vouch,
        None,
        1,
        None,
        "good provider",
        &key,
    )?;
    let signal = ProviderTrustSignal::from_trust_record(&record, &key)?;
    ensure!(signal.provider == provider_key, "provider mismatch");
    ensure!(
        signal.signal == TrustSignalKind::ObservedSuccess,
        "signal kind mismatch"
    );
    ensure!(signal.sequence == 1, "sequence mismatch");
    ensure!(signal.verify().is_ok(), "signal verification failed");
    Ok(())
}

#[test]
fn test_from_trust_record_distrust_converts_to_signal() -> anyhow::Result<()> {
    let key = signing_key(3);
    let provider_key = provider(4);
    let record = ProviderTrustRecord::new(
        provider_key,
        ProviderTrustAction::Distrust,
        None,
        1,
        None,
        "unreliable",
        &key,
    )?;
    let signal = ProviderTrustSignal::from_trust_record(&record, &key)?;
    ensure!(signal.provider == provider_key, "provider mismatch");
    ensure!(
        signal.signal == TrustSignalKind::ObservedFailure,
        "signal kind mismatch"
    );
    ensure!(signal.verify().is_ok(), "signal verification failed");
    Ok(())
}

#[test]
fn test_from_trust_record_rejects_non_vouch_distrust() -> anyhow::Result<()> {
    let key = signing_key(5);
    let provider_key = provider(6);
    let record = ProviderTrustRecord::new(provider_key, ProviderTrustAction::Trust, None, 1, None, "trusted", &key)?;
    let result = ProviderTrustSignal::from_trust_record(&record, &key);
    ensure!(result.is_err(), "Trust action should not be convertible");
    Ok(())
}

#[test]
fn test_from_trust_record_rejects_warn_action() -> anyhow::Result<()> {
    let key = signing_key(7);
    let provider_key = provider(8);
    let record = ProviderTrustRecord::new(provider_key, ProviderTrustAction::Warn, None, 1, None, "warning", &key)?;
    let result = ProviderTrustSignal::from_trust_record(&record, &key);
    ensure!(result.is_err(), "Warn action should not be convertible");
    Ok(())
}
