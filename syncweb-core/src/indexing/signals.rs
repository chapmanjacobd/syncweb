//! The unified signed-signal message broadcast on [`crate::constants::SIGNAL_TOPIC`].
//!
//! Attestations, moderation reports, and provider trust signals share the same
//! shape (an Ed25519-signed claim keyed by a subject) and were previously
//! broadcast on three separate gossip topics. They now travel on one topic and
//! are discriminated by [`SignedSignal`].

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::gossip::SignedGossipMessage;
use crate::indexing::ReportRecord;
use crate::indexing::reputation::ProviderTrustSignal;
use crate::indexing::wot::Attestation;

/// A signed signal broadcast on the unified gossip topic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
#[non_exhaustive]
pub enum SignedSignal {
    /// A `WoT` content attestation (license / provenance / derivative claim).
    Attestation(Attestation),
    /// A moderation report about a content hash.
    Report(ReportRecord),
    /// A provider trust observation.
    Trust(ProviderTrustSignal),
}

impl SignedSignal {
    /// Return a stable, human-readable label for this signal kind.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            SignedSignal::Attestation(_) => "attestation",
            SignedSignal::Report(_) => "report",
            SignedSignal::Trust(_) => "trust",
        }
    }
}

impl SignedGossipMessage for SignedSignal {
    fn verify_signature(&self) -> Result<()> {
        match self {
            SignedSignal::Attestation(attestation) => attestation.verify_signature(),
            SignedSignal::Report(report) => report.verify_signature(),
            SignedSignal::Trust(signal) => signal.verify_signature(),
        }
    }
}
