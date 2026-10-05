use std::{error::Error as StdError, fmt::Display, path::PathBuf};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, SyncwebError>;
pub type BoxError = Box<dyn StdError + Send + Sync + 'static>;

/// Flatten an error and its `source()` chain into one line.
///
/// [`SyncwebError::operation`] keeps a `Display` rendering rather than the
/// source, so errors whose `Display` is empty (for example
/// `iroh_blobs::api::Error::Io`, whose message lives in the source `io::Error`)
/// would otherwise surface as a bare variant name.
#[must_use]
pub fn error_chain(error: &dyn StdError) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(current) = cause {
        let message = current.to_string();
        if !message.is_empty() {
            text.push_str(": ");
            text.push_str(&message);
        }
        cause = current.source();
    }
    text
}

/// Top-level error type for syncweb-core library operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SyncwebError {
    #[error("folder not found: {0}")]
    FolderNotFound(String),

    #[error("no synchronized folders are available")]
    NoFolders,

    #[error("folder selector {0:?} is not a namespace ID and more than one synchronized folder is available")]
    AmbiguousFolderSelector(String),

    #[error("folder already managed")]
    FolderAlreadyManaged,

    #[error("folder mode {mode} does not permit local writes")]
    WriteDenied { mode: String },

    #[error("invalid doc ticket: {0}")]
    InvalidTicket(String),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("invalid identity: {0}")]
    InvalidIdentity(String),

    #[error("invalid device ID: {0}")]
    InvalidDeviceId(String),

    #[error("invalid sync mode: {0}")]
    InvalidSyncMode(String),

    #[error("syncthing relay fallback is disabled")]
    RelayDisabled,

    #[error("no syncthing relay is reachable: {reasons}")]
    RelayUnreachable { reasons: String },

    #[error("relay frame exceeds {max} byte limit")]
    RelayFrameTooLarge { max: usize },

    #[error("relay message decode error: {0}")]
    RelayDecode(String),

    #[error("relay URL must use tcp:// scheme")]
    RelayBadScheme,

    #[error("relay URL must contain a host and port: {0}")]
    RelayBadAddress(String),

    #[error("HKDF key derivation failed: {0}")]
    KeyDerivation(String),

    #[error("identity file error at {path}: {source}")]
    Identity {
        path: PathBuf,
        #[source]
        source: BoxError,
    },

    #[error("{context}: {detail}")]
    Operation { context: String, detail: String },

    #[error("blob size exceeds u64::MAX")]
    BlobTooLarge,

    #[error("namespace could not be opened")]
    NamespaceNotAvailable,

    #[error("missing cryptographic signature: {0}")]
    MissingSignature(String),

    #[error("invalid cryptographic signature: {0}")]
    InvalidSignature(String),

    #[error("access denied: {0}")]
    AccessDenied(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl SyncwebError {
    pub fn identity(path: impl Into<PathBuf>, source: impl StdError + Send + Sync + 'static) -> Self {
        Self::Identity {
            path: path.into(),
            source: Box::new(source),
        }
    }

    pub fn operation(context: impl Into<String>, source: impl Display) -> Self {
        Self::Operation {
            context: context.into(),
            detail: source.to_string(),
        }
    }

    pub fn access_denied(reason: impl Into<String>) -> Self {
        Self::AccessDenied(reason.into())
    }
}

#[cfg(test)]
mod error_chain_tests {
    use super::*;

    #[derive(Debug, Error)]
    #[error("outer failure")]
    struct Outer(#[source] Inner);

    #[derive(Debug, Error)]
    #[error("inner failure")]
    struct Inner(#[source] std::io::Error);

    /// The case that motivated the helper: a variant whose `Display` carries no
    /// detail while the useful text lives on the source.
    #[derive(Debug, Error)]
    #[error("Error::Io")]
    struct IoVariant(#[source] std::io::Error);

    #[test]
    fn the_whole_chain_is_flattened_in_order() {
        let error = Outer(Inner(std::io::Error::other("disk went away")));
        assert_eq!(error_chain(&error), "outer failure: inner failure: disk went away");
    }

    #[test]
    fn a_source_only_message_is_included() {
        let error = IoVariant(std::io::Error::other("path is not absolute"));
        let text = error_chain(&error);
        assert!(text.starts_with("Error::Io"), "{text}");
        assert!(text.contains("path is not absolute"), "{text}");
    }

    #[test]
    fn a_source_without_a_message_does_not_add_a_separator() {
        #[derive(Debug, Error)]
        #[error("top")]
        struct EmptySource(#[source] Empty);

        #[derive(Debug, Error)]
        #[error("")]
        struct Empty;

        assert_eq!(error_chain(&EmptySource(Empty)), "top");
    }
}
