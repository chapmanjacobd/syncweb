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
//! - `net`: Network management and routing configurations.
//! - `node`: Iroh node integration and identity management.
//! - `search`: Find engine for querying synchronized assets.
//! - `sync`: The core synchronization engine and session management.
//!
pub mod error;
pub mod filter;
pub mod folder;
pub mod fs;
pub mod init;
pub mod net;
pub mod node;
pub mod search;
pub mod snapshot;
pub mod sort;
pub mod stat;
pub mod storage;
pub mod sync;

pub use error::{Error, Result, SyncwebError};
