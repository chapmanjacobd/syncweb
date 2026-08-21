//! Centralized protocol-level string constants for the syncweb wire format.
//!
//! All `syncweb/` prefixed topic seeds, signature contexts, blob pin prefixes,
//! document keys, and runtime naming conventions live here so that protocol
//! changes are visible in a single location.

// ---------------------------------------------------------------------------
// Gossip topic seeds (hashed with BLAKE3 to produce TopicId)
// ---------------------------------------------------------------------------

/// Public package catalog gossip topic seed.
pub const CATALOG_TOPIC: &[u8] = b"syncweb/public-package-catalog/v1";

/// Unified signed-signal gossip topic seed.
///
/// Attestations, moderation reports, and provider trust signals are all
/// broadcast on this single topic, discriminated by [`crate::indexing::SignedSignal`].
pub const SIGNAL_TOPIC: &[u8] = b"syncweb/signed-signals/v1";

/// Provider lease gossip topic seed.
pub const RESILIENCE_TOPIC: &[u8] = b"syncweb/provider-leases/v1";

// ---------------------------------------------------------------------------
// Cryptographic signature contexts (Ed25519 domain separators)
// ---------------------------------------------------------------------------

/// Domain separator for signed content links (name pointers).
pub const LINK_SIGNATURE_CONTEXT: &[u8] = b"syncweb/name-pointer/v1\0";

/// Domain separator for signed filter list entries.
pub const FILTER_LIST_CONTEXT: &[u8] = b"syncweb/filter-list/v1\0";

/// Domain separator for signed provider leases.
pub const PROVIDER_LEASE_SIGNATURE_CONTEXT: &[u8] = b"syncweb/provider-lease/v1\0";

/// Domain separator for signed provider trust signals.
pub const REPUTATION_SIGNAL_CONTEXT: &[u8] = b"syncweb/provider-trust/v1\0";

/// Domain separator for signed network membership lists.
pub const MEMBER_LIST_SIGNATURE_CONTEXT: &[u8] = b"syncweb/network-membership/v1\0";

/// Domain separator for network document namespace derivation.
pub const NETWORK_DOC_NAMESPACE_CONTEXT: &[u8] = b"syncweb/network-doc/v1\0";

/// Domain separator for `WoT` metadata signatures.
pub const METADATA_CONTEXT: &[u8] = b"syncweb/wot/metadata/v1\0";

/// Domain separator for `WoT` delegation signatures.
pub const DELEGATION_CONTEXT: &[u8] = b"syncweb/wot/delegation/v1\0";

/// Domain separator for `WoT` revocation signatures.
pub const REVOCATION_CONTEXT: &[u8] = b"syncweb/wot/revocation/v1\0";

/// Domain separator for `WoT` moderation signatures.
pub const MODERATION_CONTEXT: &[u8] = b"syncweb/wot/moderation/v1\0";

/// Domain separator for `WoT` attestation signatures.
pub const ATTESTATION_CONTEXT: &[u8] = b"syncweb/wot/attestation/v1\0";

/// Domain separator for `WoT` provider trust signatures.
pub const PROVIDER_TRUST_CONTEXT: &[u8] = b"syncweb/wot/provider-trust/v1\0";

// ---------------------------------------------------------------------------
// Blob pin prefixes
// ---------------------------------------------------------------------------

/// Pin prefix for snapshot blobs.
pub const SNAPSHOT_PIN_PREFIX: &str = "syncweb/snapshot/";

/// Pin prefix for replication-rescue blobs.
pub const REPLICATION_PIN_PREFIX: &str = "syncweb/replication/";

/// Pin prefix for publicly shared folder blobs.
pub const PUBLIC_PIN_PREFIX: &str = "syncweb/public/";

/// Pin prefix for one-off download blobs.
pub const DOWNLOAD_PIN_PREFIX: &str = "syncweb/download/";

/// Pin prefix for collection manifest blobs.
pub const COLLECTION_MANIFEST_PIN_PREFIX: &str = "syncweb/collection-manifest/";

/// Pin prefix for collection content blobs.
pub const COLLECTION_PIN_PREFIX: &str = "syncweb/collection/";

// ---------------------------------------------------------------------------
// Document key prefixes (iroh-docs namespaces)
// ---------------------------------------------------------------------------

/// Iroh-docs key for the folder sync mode.
pub const MODE_KEY: &[u8] = b"sys/syncweb/mode";

/// Iroh-docs key for catalog namespace metadata.
pub const CATALOG_METADATA_KEY: &[u8] = b"sys/syncweb/catalog/metadata";

// ---------------------------------------------------------------------------
// URI scheme
// ---------------------------------------------------------------------------

/// The syncweb URI scheme prefix.
pub const LINK_SCHEME: &str = "syncweb://";

// ---------------------------------------------------------------------------
// Dynamic topic seed format strings
// ---------------------------------------------------------------------------

/// Base prefix for per-channel gossip topic seeds.
///
/// Use `format!("{}/{{name}}/v1", CHANNEL_TOPIC_PREFIX)`.
pub const CHANNEL_TOPIC_PREFIX: &str = "syncweb/catalog";

// ---------------------------------------------------------------------------
// Runtime filesystem naming conventions
// ---------------------------------------------------------------------------

/// Prefix for the daemon runtime socket file.
pub const RUNTIME_SOCKET_FILE_PREFIX: &str = "syncweb-";

/// Directory name for installed collection packages.
pub const PACKAGES_DIR_NAME: &str = ".syncweb-packages";

/// Prefix for drop-export staging directories.
pub const DROP_EXPORT_STAGING_PREFIX: &str = ".syncweb-drop-";

/// Prefix for drop-export blob staging files.
pub const DROP_EXPORT_BLOB_STAGING_PREFIX: &str = ".syncweb-drop-blob-";

/// Prefix for drop-import staging directories.
pub const DROP_IMPORT_STAGING_PREFIX: &str = ".syncweb-drop-import-";

/// Prefix for materialization staging directories.
pub const MATERIALIZATION_STAGING_PREFIX: &str = ".syncweb-materialize-";

/// Prefix for CLI drop source staging directories.
pub const DROP_SOURCE_STAGING_PREFIX: &str = ".syncweb-drop-source-";
