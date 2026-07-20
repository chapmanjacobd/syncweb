pub mod error;
pub mod filter;
pub mod folder;
pub mod fs;
pub mod init;
pub mod net;
pub mod node;
pub mod search;
pub mod sort;
pub mod stat;
pub mod storage;
pub mod sync;

/// Compatibility module for callers that prefer `syncweb_core::find`.
pub mod find {
    pub use crate::search::*;
}

/// Compatibility exports for the original Phase 3 CLI-oriented module layout.
pub mod cli_commands {
    pub mod find {
        pub use crate::search::*;
    }
    pub mod sort {
        pub use crate::sort::*;
    }
    pub mod stat {
        pub use crate::stat::*;
    }
    pub mod init {
        pub use crate::init::*;
    }
}

pub use error::{Error, Result, SyncwebError};
