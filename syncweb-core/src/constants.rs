//! Centralized protocol-level string constants for the syncweb wire format.
//!
//! All `syncweb/` prefixed topic seeds, signature contexts, blob pin prefixes,
//! document keys, and runtime naming conventions live here so that protocol
//! changes are visible in a single location.

// ---------------------------------------------------------------------------
// Gossip topic seeds (hashed with BLAKE3 to produce TopicId)
// ---------------------------------------------------------------------------

// (No global gossip topics remain after the package-catalog/provider-lease
// gossip channels were removed. All topic derivation is local-only.)

// ---------------------------------------------------------------------------
// Cryptographic signature contexts (Ed25519 domain separators)
// ---------------------------------------------------------------------------

/// Domain separator for signed content links (name pointers).
pub const LINK_SIGNATURE_CONTEXT: &[u8] = b"syncweb/name-pointer/v1\0";

/// Domain separator for signed filter list entries.
pub const FILTER_LIST_CONTEXT: &[u8] = b"syncweb/filter-list/v1\0";

/// Domain separator for signed network membership lists.
pub const MEMBER_LIST_SIGNATURE_CONTEXT: &[u8] = b"syncweb/network-membership/v1\0";

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

/// Iroh-docs key for the owner-signed network member list.
pub const NETWORK_MEMBERS_KEY: &[u8] = b"sys/network/members";

// ---------------------------------------------------------------------------
// URI scheme
// ---------------------------------------------------------------------------

/// The syncweb URI scheme prefix.
pub const LINK_SCHEME: &str = "syncweb://";

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
