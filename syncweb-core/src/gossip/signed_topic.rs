use serde::{Deserialize, Serialize};

use crate::error::{Result, SyncwebError};

/// Anything that can be signed, broadcast, and verified over gossip.
pub trait SignedGossipMessage: Serialize + for<'de> Deserialize<'de> {
    /// Verify this message's signature. Returns `Ok(())` if valid.
    ///
    /// Types that use bearer-capability auth (e.g. `PrivateLink`) may return
    /// `Ok(())` unconditionally and enforce auth at the application layer.
    ///
    /// # Errors
    ///
    /// Returns an error if the signature is missing, malformed, or invalid.
    fn verify_signature(&self) -> Result<()>;

    /// Serialize for wire transport.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    fn to_wire_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|error| SyncwebError::operation("serialize gossip message", error))
    }

    /// Deserialize from wire transport.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    fn from_wire_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|error| SyncwebError::operation("deserialize gossip message", error))
    }
}
