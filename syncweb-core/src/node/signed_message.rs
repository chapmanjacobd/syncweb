use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use iroh::PublicKey;

use crate::error::{Result, SyncwebError};

/// Wire format header size: 32-byte `VerifyingKey` + 64-byte Signature.
const HEADER_LEN: usize = 96;

/// Ed25519-signed gossip envelope that prevents identity spoofing.
///
/// Wire format: `[verifying_key: 32 bytes][signature: 64 bytes][content: N bytes]`
///
/// Since iroh's [`PublicKey`] (`NodeId`) **is** an Ed25519 [`VerifyingKey`], a valid
/// signature cryptographically proves the message originated from the claimed
/// `NodeId`.
#[derive(Copy, Clone)]
#[non_exhaustive]
pub struct SignedMessage;

impl SignedMessage {
    /// Sign `content` with the node's Ed25519 secret key.
    ///
    /// Returns the serialized wire format:
    /// `[verifying_key: 32B][signature: 64B][content]`.
    #[must_use]
    pub fn sign(secret_key: &SigningKey, content: &[u8]) -> Vec<u8> {
        let signature = secret_key.sign(content);
        let mut wire = Vec::with_capacity(HEADER_LEN.saturating_add(content.len()));
        wire.extend_from_slice(&secret_key.verifying_key().to_bytes());
        wire.extend_from_slice(&signature.to_bytes());
        wire.extend_from_slice(content);
        wire
    }

    /// Verify a wire-format signed message and extract the original content.
    ///
    /// Returns `(verifying_key, content_bytes)` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if the message is shorter than 96 bytes, the signature
    /// is invalid, or the key bytes do not represent a valid Ed25519 public key.
    pub fn verify(wire_bytes: &[u8]) -> Result<(VerifyingKey, Vec<u8>)> {
        if wire_bytes.len() < HEADER_LEN {
            return Err(SyncwebError::operation(
                "signed message too short",
                format!("got {} bytes, need at least {}", wire_bytes.len(), HEADER_LEN),
            ));
        }

        let (key_bytes, rest) = wire_bytes.split_at(32);
        let (sig_bytes, content) = rest.split_at(64);

        let verifying_key = VerifyingKey::from_bytes(
            key_bytes
                .try_into()
                .map_err(|error| SyncwebError::operation("invalid verifying key bytes", error))?,
        )
        .map_err(|error| SyncwebError::operation("invalid Ed25519 verifying key", error))?;

        let signature = Signature::from_bytes(
            sig_bytes
                .try_into()
                .map_err(|error| SyncwebError::operation("invalid signature bytes", error))?,
        );

        verifying_key
            .verify(content, &signature)
            .map_err(|error| SyncwebError::operation("gossip message signature verification failed", error))?;

        Ok((verifying_key, content.to_vec()))
    }

    /// Verify a wire-format message and check that the embedded `NodeId` matches
    /// the signing key. This is a convenience wrapper around [`verify`].
    ///
    /// # Errors
    ///
    /// Returns an error if verification fails or the `NodeId` does not match.
    pub fn verify_with_expected(wire_bytes: &[u8], expected_node_id: &PublicKey) -> Result<Vec<u8>> {
        let (verifying_key, content) = Self::verify(wire_bytes)?;
        let actual_node_id = PublicKey::from_bytes(&verifying_key.to_bytes())
            .map_err(|error| SyncwebError::operation("failed to derive NodeId from verifying key", error))?;
        if actual_node_id != *expected_node_id {
            return Err(SyncwebError::operation(
                "gossip message NodeId mismatch",
                format!("expected {expected_node_id}, got {actual_node_id}"),
            ));
        }
        Ok(content)
    }

    /// The length of the signing header in bytes.
    #[must_use]
    pub const fn header_len() -> usize {
        HEADER_LEN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_signing_key() -> SigningKey {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let mut seed = [0_u8; 32];
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        seed[..8].copy_from_slice(&count.to_le_bytes());
        SigningKey::from_bytes(&seed)
    }

    #[test]
    fn test_sign_and_verify_roundtrip() {
        let secret = random_signing_key();
        let content = b"hello gossip";

        let wire = SignedMessage::sign(&secret, content);
        let (key, decoded) = SignedMessage::verify(&wire).unwrap();

        assert_eq!(decoded, content);
        assert_eq!(key, secret.verifying_key());
    }

    #[test]
    fn test_verify_rejects_tampered_content() {
        let secret = random_signing_key();
        let wire = SignedMessage::sign(&secret, b"hello");

        let mut tampered = wire;
        *tampered
            .get_mut(HEADER_LEN)
            .expect("tampered should be long enough to have HEADER_LEN") ^= 0xFF; // flip a bit in the content
        assert!(SignedMessage::verify(&tampered).is_err());
    }

    #[test]
    fn test_verify_rejects_tampered_signature() {
        let secret = random_signing_key();
        let wire = SignedMessage::sign(&secret, b"hello");

        let mut tampered = wire;
        *tampered
            .get_mut(40)
            .expect("tampered should be long enough to have byte at index 40") ^= 0xFF; // flip a bit in the signature
        assert!(SignedMessage::verify(&tampered).is_err());
    }

    #[test]
    fn test_verify_rejects_short_message() {
        assert!(SignedMessage::verify(&[0_u8; 32]).is_err());
        assert!(SignedMessage::verify(&[0_u8; 95]).is_err());
    }

    #[test]
    fn test_verify_rejects_empty_message() {
        assert!(SignedMessage::verify(&[]).is_err());
    }

    #[test]
    fn test_node_id_derived_from_verifying_key() {
        let secret = random_signing_key();
        let wire = SignedMessage::sign(&secret, b"hello");

        let (key, _) = SignedMessage::verify(&wire).unwrap();

        // iroh PublicKey (NodeId) must match the Ed25519 verifying key
        let node_id = PublicKey::from_bytes(&key.to_bytes()).unwrap();
        let expected = PublicKey::from_bytes(&secret.verifying_key().to_bytes()).unwrap();
        assert_eq!(node_id, expected);
    }

    #[test]
    fn test_verify_with_expected_success() {
        let secret = random_signing_key();
        let node_id = PublicKey::from_bytes(&secret.verifying_key().to_bytes()).unwrap();
        let wire = SignedMessage::sign(&secret, b"hello");

        let content = SignedMessage::verify_with_expected(&wire, &node_id).unwrap();
        assert_eq!(content, b"hello");
    }

    #[test]
    fn test_verify_with_expected_rejects_wrong_key() {
        let alice = random_signing_key();
        let bob = random_signing_key();
        let bob_node_id = PublicKey::from_bytes(&bob.verifying_key().to_bytes()).unwrap();

        let wire = SignedMessage::sign(&alice, b"hello");
        assert!(SignedMessage::verify_with_expected(&wire, &bob_node_id).is_err());
    }

    #[test]
    fn test_header_len_constant() {
        assert_eq!(SignedMessage::header_len(), 96);
    }
}
