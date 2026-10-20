//! Centralized blob pin-name formatting.
//!
//! Every retention pin is a named tag in the blob store. Names are built from
//! the shared prefixes in [`crate::constants`] plus the entity's identifiers so
//! garbage collection can tell public shares, collections, snapshots, and
//! one-off downloads apart. Formatting lives here so the wire-level naming
//! convention is defined in one place.

use iroh_blobs::Hash;
use iroh_docs::NamespaceId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{
    COLLECTION_MANIFEST_PIN_PREFIX, COLLECTION_PIN_PREFIX, DOWNLOAD_PIN_PREFIX, PUBLIC_PIN_PREFIX, SNAPSHOT_PIN_PREFIX,
};

/// Retention policy for a local blob pin.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum PinClass {
    /// A download that may be removed after its explicit expiry.
    Temporary,
    /// Content intentionally served to other peers.
    Shared,
    /// Content that must survive ordinary cache cleanup.
    Protected,
    /// Repair or opportunistic content that may be reclaimed after expiry.
    Cache,
}

impl PinClass {
    /// Return the stable tag prefix used for this class.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Temporary => DOWNLOAD_PIN_PREFIX,
            Self::Shared => PUBLIC_PIN_PREFIX,
            Self::Protected => SNAPSHOT_PIN_PREFIX,
            Self::Cache => crate::constants::REPLICATION_PIN_PREFIX,
        }
    }

    /// Return whether ordinary garbage collection may remove this class.
    #[must_use]
    pub const fn is_gc_eligible(self) -> bool {
        matches!(self, Self::Temporary | Self::Cache)
    }

    /// Infer a class from an existing pin name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        if name.starts_with(DOWNLOAD_PIN_PREFIX) {
            Some(Self::Temporary)
        } else if name.starts_with(PUBLIC_PIN_PREFIX)
            || name.starts_with(COLLECTION_MANIFEST_PIN_PREFIX)
            || name.starts_with(COLLECTION_PIN_PREFIX)
        {
            Some(Self::Shared)
        } else if name.starts_with(SNAPSHOT_PIN_PREFIX) {
            Some(Self::Protected)
        } else if name.starts_with(crate::constants::REPLICATION_PIN_PREFIX) {
            Some(Self::Cache)
        } else {
            None
        }
    }
}

/// A durable pin with optional expiry metadata encoded in its tag name.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PinRecord {
    pub name: String,
    pub hash: Hash,
    pub class: PinClass,
    pub expires_at: Option<u64>,
}

impl PinRecord {
    /// Decode a pin returned by the blob store.
    #[must_use]
    pub fn from_tag(name: String, hash: Hash) -> Option<Self> {
        let class = PinClass::from_name(&name)?;
        let expires_at = name
            .strip_prefix(class.prefix())
            .and_then(|rest| rest.strip_prefix("ttl/"))
            .and_then(|rest| rest.split('/').next())
            .and_then(|value| value.parse().ok());
        Some(Self {
            name,
            hash,
            class,
            expires_at,
        })
    }

    /// Return whether this pin has passed its explicit expiry.
    #[must_use]
    pub fn is_expired(&self, now: u64) -> bool {
        self.expires_at.is_some_and(|expires_at| expires_at <= now)
    }
}

/// Expiry rules for temporary and cache pins.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PinPolicy {
    pub temporary_ttl: u64,
    pub cache_ttl: u64,
}

impl Default for PinPolicy {
    fn default() -> Self {
        Self {
            temporary_ttl: 24 * 60 * 60,
            cache_ttl: 7 * 24 * 60 * 60,
        }
    }
}

impl PinPolicy {
    /// Return an expiry timestamp for a newly created pin class.
    #[must_use]
    pub const fn expiry_at(self, class: PinClass, now: u64) -> Option<u64> {
        let ttl = match class {
            PinClass::Temporary => self.temporary_ttl,
            PinClass::Cache => self.cache_ttl,
            PinClass::Shared | PinClass::Protected => return None,
        };
        now.checked_add(ttl)
    }

    /// Return the pins that are safe to remove during this GC pass.
    #[must_use]
    pub fn gc_candidates(self, pins: &[PinRecord], now: u64) -> Vec<&PinRecord> {
        pins.iter()
            .filter(|pin| pin.class.is_gc_eligible() && pin.is_expired(now))
            .collect()
    }
}

/// Build a class-aware pin name. Expiry is included only for reclaimable
/// classes, making old names remain readable and conservatively retained.
#[must_use]
pub fn classified_pin_name(class: PinClass, scope: impl AsRef<str>, hash: Hash, expires_at: Option<u64>) -> String {
    let scope = scope.as_ref();
    match expires_at {
        Some(expires_at) => format!("{}ttl/{expires_at}/{scope}/{hash}", class.prefix()),
        None => format!("{}scope/{scope}/{hash}", class.prefix()),
    }
}

/// Pin name for a publicly shared folder blob.
#[must_use]
pub fn public_blob_pin(namespace_id: NamespaceId, hash: Hash) -> String {
    format!("{PUBLIC_PIN_PREFIX}{namespace_id}/{hash}")
}

/// Pin name for a published collection manifest blob.
#[must_use]
pub fn collection_manifest_pin(hash: Hash) -> String {
    format!("{COLLECTION_MANIFEST_PIN_PREFIX}{hash}")
}

/// Pin name for a content blob referenced by a published collection.
#[must_use]
pub fn collection_content_pin(collection_id: Uuid, hash: Hash) -> String {
    format!("{COLLECTION_PIN_PREFIX}{collection_id}/{hash}")
}

/// Pin name for a snapshot manifest blob.
#[must_use]
pub fn snapshot_manifest_pin(id: Hash) -> String {
    format!("{SNAPSHOT_PIN_PREFIX}{id}/manifest")
}

/// Pin name for a content blob referenced by a snapshot.
#[must_use]
pub fn snapshot_content_pin(id: Hash, hash: Hash) -> String {
    format!("{SNAPSHOT_PIN_PREFIX}{id}/blob/{hash}")
}

/// Pin name for a one-off downloaded blob.
#[must_use]
pub fn download_pin(hash: Hash) -> String {
    format!("{DOWNLOAD_PIN_PREFIX}{hash}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(seed: u8) -> Hash {
        Hash::from_bytes([seed; 32])
    }

    #[test]
    fn class_names_round_trip_with_expiry() {
        let name = classified_pin_name(PinClass::Temporary, "download", hash(1), Some(200));
        let record = PinRecord::from_tag(name.clone(), hash(1)).expect("classified pin");
        assert_eq!(record.name, name);
        assert_eq!(record.class, PinClass::Temporary);
        assert_eq!(record.expires_at, Some(200));
        assert!(record.is_expired(200));
        assert!(!record.is_expired(199));
    }

    #[test]
    fn gc_only_selects_expired_reclaimable_classes() {
        let pins = vec![
            PinRecord::from_tag(
                classified_pin_name(PinClass::Temporary, "old", hash(1), Some(10)),
                hash(1),
            )
            .expect("temporary pin"),
            PinRecord::from_tag(
                classified_pin_name(PinClass::Cache, "fresh", hash(2), Some(100)),
                hash(2),
            )
            .expect("cache pin"),
            PinRecord::from_tag(classified_pin_name(PinClass::Shared, "shared", hash(3), None), hash(3))
                .expect("shared pin"),
            PinRecord::from_tag(
                classified_pin_name(PinClass::Protected, "protected", hash(4), Some(1)),
                hash(4),
            )
            .expect("protected pin"),
        ];
        let candidates = PinPolicy::default().gc_candidates(&pins, 50);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].hash, hash(1));
    }

    #[test]
    fn legacy_names_are_classified_but_retained_without_expiry() {
        let record = PinRecord::from_tag(download_pin(hash(5)), hash(5)).expect("legacy download pin");
        assert_eq!(record.class, PinClass::Temporary);
        assert_eq!(record.expires_at, None);
        assert!(
            PinPolicy::default()
                .gc_candidates(std::slice::from_ref(&record), u64::MAX)
                .is_empty()
        );
    }
}
