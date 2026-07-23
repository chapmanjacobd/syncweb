use std::{error::Error as StdError, fmt::Display, path::PathBuf};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, SyncwebError>;
pub type BoxError = Box<dyn StdError + Send + Sync + 'static>;

/// Top-level error type for syncweb-core library operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SyncwebError {
    #[error("folder not found: {0}")]
    FolderNotFound(String),

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
}

pub type Error = SyncwebError;
