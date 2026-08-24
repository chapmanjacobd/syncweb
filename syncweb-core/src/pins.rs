//! Centralized blob pin-name formatting.
//!
//! Every retention pin is a named tag in the blob store. Names are built from
//! the shared prefixes in [`crate::constants`] plus the entity's identifiers so
//! garbage collection can tell public shares, collections, snapshots, and
//! one-off downloads apart. Formatting lives here so the wire-level naming
//! convention is defined in one place.

use iroh_blobs::Hash;
use iroh_docs::NamespaceId;
use uuid::Uuid;

use crate::constants::{
    COLLECTION_MANIFEST_PIN_PREFIX, COLLECTION_PIN_PREFIX, DOWNLOAD_PIN_PREFIX, PUBLIC_PIN_PREFIX,
    SNAPSHOT_PIN_PREFIX,
};

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