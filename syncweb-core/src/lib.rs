//! # syncweb-core
//!
//! Core library for `syncweb`, enabling delay-tolerant web surfing and decentralized synchronization.
//!
//! This crate provides the foundational building blocks for the syncweb application, including:
//! - Decentralized folder synchronization and package management.
//! - Network and node management using the Iroh stack.
//! - File system scanning, filtering, and statistical analysis.
//! - Delay-tolerant networking capabilities.
//!
//! ## Modules
//!
//! - `error`: Common error types and `Result` aliases.
//! - `filter`: Tools for filtering files during synchronization and scanning.
//! - `folder`: Management of synchronized folders, collections, and packages.
//! - `fs`: File system utilities, including parallel scanning.
//! - `indexing`: Opt-in SQLite/FTS5 indexing for synchronized folders.
//! - `net`: Network management and routing configurations.
//! - `node`: Iroh node integration and identity management.
//! - `search`: Find engine for querying synchronized assets.
//! - `sync`: The core synchronization engine and session management.
//!
pub mod error;
pub mod filter;
pub mod folder;
pub mod fs;
pub mod indexing;
pub mod init;
pub mod net;
pub mod node;
pub mod schedule;
pub mod search;
pub mod snapshot;
pub mod sort;
pub mod stat;
pub mod stats;
pub mod storage;
pub mod sync;
pub mod verify;

pub use error::{Error, Result, SyncwebError};
pub use folder::drop_export;
pub use folder::drop_import;
pub use folder::drop_import::{DropImportOptions, DropImportResult, DropImporter, import_drop};
pub use folder::drop_verify::{
    DropVerificationResult, DropVerifier, DropVerifyResult, verify_drop, verify_drop_reader,
};
