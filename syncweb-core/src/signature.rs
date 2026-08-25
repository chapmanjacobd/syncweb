//! Shared Ed25519 signing helpers used by signed protocol payloads.
//!
//! Collection manifests, mutable name pointers, filter lists, and network
//! membership lists all embed a lowercase-hex Ed25519 signature over their
//! unsigned serialized bytes. These helpers centralize the hex encoding and
//! verification loops so the payload types only implement their own unsigned
//! serialization (which stays byte-exact) and key derivation.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::error::{Result, SyncwebError};

/// Hex-encode the ed25519 public key of a signing key.
#[must_use]
pub(crate) fn public_key_hex(signing_key: &SigningKey) -> String {
    hex::encode(signing_key.verifying_key().to_bytes())
}

/// Sign a message and return the signature as lowercase hex.
#[must_use]
pub(crate) fn sign_hex(signing_key: &SigningKey, message: &[u8]) -> String {
    hex::encode(signing_key.sign(message).to_bytes())
}

/// Verify a hex-encoded ed25519 signature against a hex-encoded public key.
///
/// # Errors
///
/// Returns an error if the public key or signature is malformed or the
/// signature does not verify.
pub(crate) fn verify_hex(public_key_hex: &str, message: &[u8], signature_hex: &str) -> Result<()> {
    decode_verifying_key(public_key_hex)?
        .verify(message, &decode_signature(signature_hex)?)
        .map_err(|error| SyncwebError::InvalidSignature(format!("signature verification failed: {error}")))
}

/// Decode a hex-encoded ed25519 verifying key.
///
/// # Errors
///
/// Returns an error if the hex is malformed or the key is not 32 bytes.
pub(crate) fn decode_verifying_key(encoded: &str) -> Result<VerifyingKey> {
    let bytes = hex::decode(encoded)
        .map_err(|error| SyncwebError::InvalidSignature(format!("invalid public key hex: {error}")))?;
    let key_bytes: [u8; 32] = bytes.try_into().map_err(|decoded: Vec<u8>| {
        SyncwebError::InvalidSignature(format!("public key must be 32 bytes, got {}", decoded.len()))
    })?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|error| SyncwebError::InvalidSignature(format!("invalid public key: {error}")))
}

/// Decode a hex-encoded ed25519 signature.
///
/// # Errors
///
/// Returns an error if the hex is malformed or the signature is not 64 bytes.
pub(crate) fn decode_signature(encoded: &str) -> Result<Signature> {
    let bytes = hex::decode(encoded)
        .map_err(|error| SyncwebError::InvalidSignature(format!("invalid signature hex: {error}")))?;
    Signature::from_slice(&bytes).map_err(|error| SyncwebError::InvalidSignature(format!("invalid signature: {error}")))
}
