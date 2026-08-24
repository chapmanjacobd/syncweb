//! Shared folder-sharing helper.
//!
//! The CLI embedded path, the daemon IPC share handler, and `create` all build
//! a folder ticket, optionally pin the folder's blobs, optionally persist the
//! share record, and emit a `syncweb://folder/...` URL. This helper unifies that
//! flow so the call sites share one implementation.

use iroh_docs::{DocTicket, NamespaceId};

use crate::error::Result;

use super::SyncwebFolder;

/// Options controlling how a folder is shared.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct ShareOptions {
    /// Grant write access on the ticket (default: read-only).
    pub writable: bool,
    /// Pin the folder's blobs so shared content is retained.
    pub pin: bool,
    /// Persist the share record so it can be listed and unshared later.
    pub persist: bool,
}

impl ShareOptions {
    #[must_use]
    pub const fn new(writable: bool) -> Self {
        Self {
            writable,
            pin: false,
            persist: false,
        }
    }

    #[must_use]
    pub const fn with_pin(mut self, pin: bool) -> Self {
        self.pin = pin;
        self
    }

    #[must_use]
    pub const fn with_persist(mut self, persist: bool) -> Self {
        self.persist = persist;
        self
    }
}

/// The outcome of sharing a folder.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ShareResult {
    pub namespace: NamespaceId,
    pub writable: bool,
    pub ticket: DocTicket,
    pub url: String,
    pub pinned: usize,
}

impl ShareResult {
    #[must_use]
    pub const fn access(&self) -> &'static str {
        if self.writable { "write" } else { "read" }
    }
}

/// Build a folder share ticket, optionally pinning and persisting it.
///
/// `persist` is invoked with the namespace, access label, and ticket when
/// `options.persist` is set, letting the caller record the share in its own
/// database without coupling this helper to a specific store.
///
/// # Errors
///
/// Returns an error if pinning or ticket creation fails, or the persist
/// callback fails.
pub async fn share_folder<F>(
    folder: &SyncwebFolder,
    options: ShareOptions,
    persist: F,
) -> Result<ShareResult>
where
    F: FnOnce(NamespaceId, &str, &DocTicket) -> Result<()>,
{
    let pinned = if options.pin {
        folder.pin_all_content().await?
    } else {
        0
    };
    let ticket = folder.ticket(options.writable).await?;
    let result = ShareResult {
        namespace: folder.namespace_id(),
        writable: options.writable,
        ticket,
        url: String::new(),
        pinned,
    };
    if options.persist {
        persist(result.namespace, result.access(), &result.ticket)?;
    }
    let url = crate::uri::folder_url(result.namespace, &result.ticket);
    Ok(ShareResult { url, ..result })
}
