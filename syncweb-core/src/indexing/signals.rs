//! The unified signed-signal message broadcast on [`crate::constants::SIGNAL_TOPIC`].
//!
//! Attestations are Ed25519-signed claims keyed by a content hash. Reports and
//! provider trust signals were previously broadcast here too, but were culled:
//! moderation reports had no consumer, and provider trust observations are now
//! local-only (see plan 017). The enum is retained so future signal kinds can
//! share the topic without a schema break.

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::gossip::SignedGossipMessage;
use crate::indexing::wot::Attestation;

/// A signed signal broadcast on the unified gossip topic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
#[non_exhaustive]
pub enum SignedSignal {
    /// A `WoT` content attestation (license / provenance / derivative claim).
    Attestation(Attestation),
}

impl SignedGossipMessage for SignedSignal {
    fn verify_signature(&self) -> Result<()> {
        match self {
            SignedSignal::Attestation(attestation) => attestation.verify_signature(),
        }
    }
}
