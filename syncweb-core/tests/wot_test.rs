use anyhow::Result;
use ed25519_dalek::SigningKey;
use std::time::{SystemTime, UNIX_EPOCH};

use iroh_blobs::Hash;
use syncweb_core::indexing::{
    Attestation, AttestationKind, MetadataEntry, ModerationAction, ModerationContext, ModerationRecord,
    ModerationScope, ProviderTrustAction, ProviderTrustDecision, ProviderTrustRecord, RevocationRecord, TrustDecision,
    TrustDelegation, TrustPolicy, WotService,
};

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

const fn content() -> Hash {
    Hash::from_bytes([42_u8; 32])
}

#[test]
fn wot_metadata_append_requires_trusted_authors() -> Result<()> {
    let root = key(1);
    let untrusted = key(2);
    let service = WotService::in_memory(TrustPolicy::with_root(&root))?;

    let accepted = MetadataEntry::new_with_time(content(), "title", "trusted", 1, 1, &root)?;
    anyhow::ensure!(
        service.append_metadata(&accepted)?,
        "trusted metadata should be appended"
    );

    let rejected = MetadataEntry::new_with_time(content(), "title", "untrusted", 2, 2, &untrusted)?;
    anyhow::ensure!(
        service.append_metadata(&rejected).is_err(),
        "untrusted metadata should be rejected"
    );
    Ok(())
}

#[test]
fn wot_metadata_is_indexed_and_searchable() -> Result<()> {
    let root = key(3);
    let service = WotService::in_memory(TrustPolicy::with_root(&root))?;
    let entry = MetadataEntry::new_with_time(content(), "transcription", "the searchable phrase", 1, 1, &root)?;
    service.append_metadata(&entry)?;

    let results = service.search("searchable", 10)?;
    anyhow::ensure!(results.len() == 1, "metadata should be found through FTS");
    anyhow::ensure!(
        results
            .first()
            .is_some_and(|result| result.value == "the searchable phrase"),
        "search result should contain the indexed value"
    );
    Ok(())
}

#[test]
fn wot_trust_policy_evaluates_signed_delegations() -> Result<()> {
    let root = key(4);
    let delegate = key(5);
    let other_content = Hash::from_bytes([43_u8; 32]);
    let mut policy = TrustPolicy::with_root(&root);
    let delegation = TrustDelegation::new_with_time(&delegate.verifying_key(), Some(content()), 1, 0, u64::MAX, &root)?;
    anyhow::ensure!(
        policy.add_delegation_at(delegation, 10)?,
        "delegation should be accepted"
    );
    anyhow::ensure!(
        policy.evaluate_for_at(&hex::encode(delegate.verifying_key().to_bytes()), Some(&content()), 10)
            == TrustDecision::TrustedDelegation,
        "delegate should be trusted for its scope"
    );
    anyhow::ensure!(
        !policy.is_trusted_for_at(
            &hex::encode(delegate.verifying_key().to_bytes()),
            Some(&other_content),
            10
        ),
        "delegate should not be trusted outside its scope"
    );
    Ok(())
}

#[test]
fn wot_delegation_signature_rejects_tampering() -> Result<()> {
    let root = key(6);
    let delegate = key(7);
    let mut delegation = TrustDelegation::new_with_time(&delegate.verifying_key(), None, 1, 0, u64::MAX, &root)?;
    delegation.delegate = hex::encode(key(8).verifying_key().to_bytes());
    anyhow::ensure!(
        delegation.verify_signature().is_err(),
        "tampered delegation should fail cryptographic verification"
    );
    Ok(())
}

#[test]
fn wot_self_revocation_hides_and_blocks_new_metadata() -> Result<()> {
    let root = key(9);
    let service = WotService::in_memory(TrustPolicy::with_root(&root))?;
    let entry = MetadataEntry::new_with_time(content(), "title", "before revocation", 1, 1, &root)?;
    service.append_metadata(&entry)?;

    let revocation = RevocationRecord::new(content(), 1, "publisher request", &root)?;
    anyhow::ensure!(service.revoke_self(revocation)?, "revocation should be accepted");
    anyhow::ensure!(service.is_revoked(&content())?, "content should be revoked");
    anyhow::ensure!(
        service.search("before", 10)?.is_empty(),
        "revoked content should not be searchable"
    );
    anyhow::ensure!(
        service
            .append_metadata(&MetadataEntry::new_with_time(content(), "title", "after", 2, 2, &root,)?)
            .is_err(),
        "revoked content should reject new metadata"
    );
    Ok(())
}

#[test]
fn wot_trusted_moderation_hides_and_restores_metadata() -> Result<()> {
    let root = key(10);
    let service = WotService::in_memory(TrustPolicy::with_root(&root))?;
    let entry = MetadataEntry::new_with_time(content(), "title", "moderated content", 1, 1, &root)?;
    service.append_metadata(&entry)?;

    let hide = ModerationRecord::new(content(), ModerationAction::Hide, 1, "policy violation", &root)?;
    anyhow::ensure!(service.apply_moderation(hide)?, "hide decision should be accepted");
    anyhow::ensure!(
        service.search("moderated", 10)?.is_empty(),
        "hidden metadata should not be searchable"
    );

    let restore = ModerationRecord::new(content(), ModerationAction::Restore, 2, "reviewed", &root)?;
    anyhow::ensure!(service.apply_moderation(restore)?, "newer restore should be accepted");
    anyhow::ensure!(
        service.search("moderated", 10)?.len() == 1,
        "restored metadata should be searchable"
    );
    Ok(())
}

#[test]
fn wot_moderation_scopes_are_evaluated_locally() -> Result<()> {
    let root = key(12);
    let namespace = iroh_docs::NamespaceId::from([7_u8; 32]);
    let service = WotService::in_memory(TrustPolicy::with_root(&root))?;
    let record = ModerationRecord::new_scoped(
        content(),
        ModerationScope::File {
            namespace_id: namespace,
            key: b"blocked.txt".to_vec(),
        },
        ModerationAction::Hide,
        1,
        "blocked in this file",
        &root,
    )?;
    service.apply_moderation(record)?;

    anyhow::ensure!(
        service.moderation_decision(&ModerationContext::new(content()))? == ModerationAction::Show,
        "file moderation should not affect an unscoped context"
    );
    anyhow::ensure!(
        service.moderation_decision(&ModerationContext::new(content()).for_file(namespace, b"blocked.txt"))?
            == ModerationAction::Hide,
        "file moderation should affect the matching context"
    );
    anyhow::ensure!(service.list_moderation(None)?.len() == 1);
    Ok(())
}

#[test]
fn wot_attestations_verify_and_require_trust() -> Result<()> {
    let root = key(11);
    let service = WotService::in_memory(TrustPolicy::with_root(&root))?;
    let attestation = Attestation::new(content(), AttestationKind::License, "CC-BY-4.0", 1, &root)?;
    service.verify_attestation(&attestation)?;
    anyhow::ensure!(
        service.append_attestation(attestation.clone())?,
        "attestation should be accepted"
    );

    let mut tampered = attestation;
    tampered.value = "proprietary".to_owned();
    anyhow::ensure!(
        service.verify_attestation(&tampered).is_err(),
        "tampered attestation should fail verification"
    );
    Ok(())
}

fn pk(seed: u8) -> iroh::PublicKey {
    iroh::SecretKey::from_bytes(&[seed; 32]).public()
}

#[test]
fn provider_trust_record_expires_and_is_ignored() -> Result<()> {
    let root = key(20);
    let provider = pk(21);
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let service = WotService::in_memory(TrustPolicy::with_root(&root))?;
    let record = ProviderTrustRecord::new_with_time(
        provider,
        ProviderTrustAction::Trust,
        None,
        1,
        now - 100,
        "expires soon",
        &root,
    )?
    .with_expiration(Some(now + 100), &root)?;
    service.apply_provider_trust(record)?;
    anyhow::ensure!(
        service.evaluate_provider_trust(provider, None, now)? == ProviderTrustDecision::Trusted,
        "should be trusted before expiry"
    );
    anyhow::ensure!(
        service.evaluate_provider_trust(provider, None, now + 200)? == ProviderTrustDecision::Unknown,
        "should be unknown after expiry"
    );
    Ok(())
}

#[test]
fn provider_trust_evaluate_conflicting_records() -> Result<()> {
    let root = key(22);
    let other = key(23);
    let provider = pk(24);
    let service = WotService::in_memory(TrustPolicy::with_root(&root))?;
    service.trust_author(&root.verifying_key())?;
    service.trust_author(&other.verifying_key())?;
    let trust = ProviderTrustRecord::new_with_time(provider, ProviderTrustAction::Trust, None, 1, 10, "vouch", &root)?;
    service.apply_provider_trust(trust)?;
    let distrust = ProviderTrustRecord::new_with_time(
        provider,
        ProviderTrustAction::Distrust,
        None,
        1,
        10,
        "bad provider",
        &other,
    )?;
    service.apply_provider_trust(distrust)?;
    anyhow::ensure!(
        service.evaluate_provider_trust(provider, None, 10)? == ProviderTrustDecision::Conflicting,
        "mixed trust/distrust at same sequence should be Conflicting"
    );
    Ok(())
}

#[test]
fn provider_trust_untrusted_issuer_is_rejected() -> Result<()> {
    let root = key(25);
    let untrusted = key(26);
    let provider = pk(27);
    let service = WotService::in_memory(TrustPolicy::with_root(&root))?;
    let record = ProviderTrustRecord::new_with_time(
        provider,
        ProviderTrustAction::Trust,
        None,
        1,
        10,
        "from untrusted",
        &untrusted,
    )?;
    anyhow::ensure!(
        service.apply_provider_trust(record).is_err(),
        "untrusted issuer should be rejected"
    );
    Ok(())
}
