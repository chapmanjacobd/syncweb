use serde::{Deserialize, Serialize};

/// Controls whether a synchronization session exits after one reconciliation
/// or remains subscribed to changes.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Default)]
#[non_exhaustive]
pub enum SessionMode {
    #[default]
    ReconcileOnce,
    Continuous,
}

impl SessionMode {
    #[must_use]
    pub const fn is_continuous(self) -> bool {
        matches!(self, Self::Continuous)
    }
}
