use anyhow::{Result, ensure};
use ed25519_dalek::SigningKey;

use iroh_blobs::Hash;
use syncweb_core::indexing::{TrustDecision, TrustDelegation, TrustPolicy};

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn author(sk: &SigningKey) -> String {
    hex::encode(sk.verifying_key().to_bytes())
}

#[test]
fn test_delegate_creates_trust_chain() -> Result<()> {
    let from_key = key(1);
    let to_key = key(2);
    let hash = Hash::from_bytes([4_u8; 32]);
    let mut policy = TrustPolicy::with_root(&from_key);

    let delegation = TrustDelegation::new_with_time(&to_key.verifying_key(), None, 1, 0, u64::MAX, &from_key)?;
    policy.add_delegation_at(delegation, 1000)?;

    let to_id = author(&to_key);
    ensure!(
        policy.evaluate_for_at(&to_id, Some(&hash), 2000) == TrustDecision::TrustedDelegation,
        "expected TrustedDelegation"
    );
    ensure!(policy.is_trusted_for_at(&to_id, Some(&hash), 2000), "expected trusted");
    Ok(())
}

#[test]
fn test_delegate_respects_max_depth() -> Result<()> {
    let from_key = key(3);
    let to_key = key(4);
    let third_key = key(5);
    let hash = Hash::from_bytes([4_u8; 32]);
    let mut policy = TrustPolicy::with_root(&from_key);

    let from_to = TrustDelegation::new_with_time(&to_key.verifying_key(), None, 1, 0, u64::MAX, &from_key)?
        .with_max_depth(1, &from_key)?;
    policy.add_delegation_at(from_to, 1000)?;

    let to_third = TrustDelegation::new_with_time(&third_key.verifying_key(), None, 1, 0, u64::MAX, &to_key)?
        .with_max_depth(1, &to_key)?;
    policy.add_delegation_at(to_third, 1000)?;

    let third_id = author(&third_key);
    ensure!(
        !policy.is_trusted_for_at(&third_id, Some(&hash), 2000),
        "third should not be trusted when root->to max_depth=1"
    );
    Ok(())
}

#[test]
fn test_delegate_scoped_to_hash() -> Result<()> {
    let from_key = key(6);
    let to_key = key(7);
    let hash_a = Hash::from_bytes([1_u8; 32]);
    let hash_b = Hash::from_bytes([2_u8; 32]);
    let mut policy = TrustPolicy::with_root(&from_key);

    let delegation = TrustDelegation::new_with_time(&to_key.verifying_key(), Some(hash_a), 1, 0, u64::MAX, &from_key)?;
    policy.add_delegation_at(delegation, 1000)?;

    let to_id = author(&to_key);
    ensure!(
        policy.is_trusted_for_at(&to_id, Some(&hash_a), 2000),
        "hash_a should be trusted"
    );
    ensure!(
        !policy.is_trusted_for_at(&to_id, Some(&hash_b), 2000),
        "hash_b should not be trusted"
    );
    Ok(())
}

#[test]
fn test_delegate_max_depth_chain_allows_one_hop() -> Result<()> {
    let root = key(8);
    let a = key(9);
    let b = key(10);
    let hash = Hash::from_bytes([4_u8; 32]);
    let mut policy = TrustPolicy::with_root(&root);

    let root_to_a =
        TrustDelegation::new_with_time(&a.verifying_key(), None, 1, 0, u64::MAX, &root)?.with_max_depth(2, &root)?;
    policy.add_delegation_at(root_to_a, 1000)?;

    let a_to_b = TrustDelegation::new_with_time(&b.verifying_key(), None, 1, 0, u64::MAX, &a)?;
    policy.add_delegation_at(a_to_b, 1000)?;

    let b_id = author(&b);
    ensure!(
        policy.is_trusted_for_at(&b_id, Some(&hash), 2000),
        "b should be trusted when root->A max_depth=2"
    );
    Ok(())
}

#[test]
fn test_delegate_max_depth_chain_blocks_deep_hops() -> Result<()> {
    let root = key(11);
    let a = key(12);
    let b = key(13);
    let c = key(14);
    let hash = Hash::from_bytes([4_u8; 32]);
    let mut policy = TrustPolicy::with_root(&root);

    let root_to_a =
        TrustDelegation::new_with_time(&a.verifying_key(), None, 1, 0, u64::MAX, &root)?.with_max_depth(2, &root)?;
    policy.add_delegation_at(root_to_a, 1000)?;

    let a_to_b = TrustDelegation::new_with_time(&b.verifying_key(), None, 1, 0, u64::MAX, &a)?;
    policy.add_delegation_at(a_to_b, 1000)?;

    let b_to_c = TrustDelegation::new_with_time(&c.verifying_key(), None, 1, 0, u64::MAX, &b)?;
    policy.add_delegation_at(b_to_c, 1000)?;

    let c_id = author(&c);
    ensure!(
        !policy.is_trusted_for_at(&c_id, Some(&hash), 2000),
        "c should not be trusted when root->A max_depth=2 (root->A->B max_depth satisfied, C is too deep)"
    );
    Ok(())
}

#[test]
fn test_delegate_revoke_removes_delegation() -> Result<()> {
    let from_key = key(15);
    let to_key = key(16);
    let hash = Hash::from_bytes([4_u8; 32]);
    let mut policy = TrustPolicy::with_root(&from_key);

    let delegation = TrustDelegation::new_with_time(&to_key.verifying_key(), None, 1, 0, u64::MAX, &from_key)?;
    policy.add_delegation_at(delegation, 1000)?;

    let to_id = author(&to_key);
    ensure!(
        policy.is_trusted_for_at(&to_id, Some(&hash), 1200),
        "should be trusted before revoke"
    );

    policy.revoke_delegation_at(&author(&from_key), &to_id, None, 1500)?;

    ensure!(
        !policy.is_trusted_for_at(&to_id, Some(&hash), 2000),
        "should not be trusted after revoke"
    );
    Ok(())
}

#[test]
fn test_delegate_revoke_only_affects_target() -> Result<()> {
    let from_key = key(17);
    let to_key = key(18);
    let other_key = key(19);
    let hash = Hash::from_bytes([4_u8; 32]);
    let mut policy = TrustPolicy::with_root(&from_key);

    let to_delegation = TrustDelegation::new_with_time(&to_key.verifying_key(), None, 1, 0, u64::MAX, &from_key)?;
    policy.add_delegation_at(to_delegation, 1000)?;

    let other_delegation = TrustDelegation::new_with_time(&other_key.verifying_key(), None, 1, 0, u64::MAX, &from_key)?;
    policy.add_delegation_at(other_delegation, 1000)?;

    policy.revoke_delegation_at(&author(&from_key), &author(&to_key), None, 1500)?;

    let to_id = author(&to_key);
    let other_id = author(&other_key);
    ensure!(
        !policy.is_trusted_for_at(&to_id, Some(&hash), 2000),
        "to_id should not be trusted"
    );
    ensure!(
        policy.is_trusted_for_at(&other_id, Some(&hash), 2000),
        "other_id should still be trusted"
    );
    Ok(())
}

#[test]
fn test_delegate_revoke_scoped_only_affects_scope() -> Result<()> {
    let from_key = key(20);
    let to_key = key(21);
    let hash_a = Hash::from_bytes([10_u8; 32]);
    let hash_b = Hash::from_bytes([11_u8; 32]);
    let mut policy = TrustPolicy::with_root(&from_key);

    let scoped_a = TrustDelegation::new_with_time(&to_key.verifying_key(), Some(hash_a), 1, 0, u64::MAX, &from_key)?;
    policy.add_delegation_at(scoped_a, 1000)?;

    let scoped_b = TrustDelegation::new_with_time(&to_key.verifying_key(), Some(hash_b), 1, 0, u64::MAX, &from_key)?;
    policy.add_delegation_at(scoped_b, 1000)?;

    policy.revoke_delegation_at(&author(&from_key), &author(&to_key), Some(&hash_a), 1500)?;

    let to_id = author(&to_key);
    ensure!(
        !policy.is_trusted_for_at(&to_id, Some(&hash_a), 2000),
        "scope hash_a should be revoked"
    );
    ensure!(
        policy.is_trusted_for_at(&to_id, Some(&hash_b), 2000),
        "scope hash_b should still be trusted"
    );
    Ok(())
}

#[test]
fn test_delegate_revoke_chain_broken_by_intermediate_revocation() -> Result<()> {
    let root = key(22);
    let a = key(23);
    let b = key(24);
    let hash = Hash::from_bytes([4_u8; 32]);
    let mut policy = TrustPolicy::with_root(&root);

    let root_to_a =
        TrustDelegation::new_with_time(&a.verifying_key(), None, 1, 0, u64::MAX, &root)?.with_max_depth(3, &root)?;
    policy.add_delegation_at(root_to_a, 1000)?;

    let a_to_b = TrustDelegation::new_with_time(&b.verifying_key(), None, 1, 0, u64::MAX, &a)?;
    policy.add_delegation_at(a_to_b, 1000)?;

    let b_id = author(&b);
    ensure!(
        policy.is_trusted_for_at(&b_id, Some(&hash), 1500),
        "b should be trusted before revoke"
    );

    policy.revoke_delegation_at(&author(&a), &b_id, None, 1600)?;

    ensure!(
        !policy.is_trusted_for_at(&b_id, Some(&hash), 2000),
        "b should not be trusted after revoke"
    );
    Ok(())
}
