//! The `syncweb://` URI and ticket codec.
//!
//! Folder shares are encoded as `syncweb://folder/<namespace>?ticket=<ticket>`.
//! This module is the single writer, reader, and validator for that format so
//! share URLs round-trip through the CLI, the daemon, and the core library.

use iroh_docs::{DocTicket, NamespaceId};

use crate::constants::LINK_SCHEME;
use crate::error::{Result, SyncwebError};

/// The `syncweb://folder/<namespace>?ticket=<ticket>` share URL.
#[must_use]
pub fn folder_url(namespace: NamespaceId, ticket: &DocTicket) -> String {
    format!("{LINK_SCHEME}folder/{namespace}?ticket={ticket}")
}

/// Whether a value is a `syncweb://folder/` share URL.
#[must_use]
pub fn is_folder_url(value: &str) -> bool {
    value.starts_with("syncweb://folder/")
}

/// Parse a folder share URL or a bare iroh-docs ticket into a [`DocTicket`].
///
/// Both `syncweb://folder/<namespace>?ticket=<ticket>` and a plain
/// `<ticket>` are accepted.
///
/// # Errors
///
/// Returns an error if neither a query `ticket=` parameter nor a valid ticket
/// is present.
pub fn parse_folder_ticket(value: &str) -> Result<DocTicket> {
    let mut ticket_raw = value;
    if let Some(rest) = value.strip_prefix(LINK_SCHEME)
        && let Some((_, query)) = rest.split_once('?')
    {
        for param in query.split('&') {
            if let Some(ticket) = param.strip_prefix("ticket=") {
                ticket_raw = ticket;
                break;
            }
        }
    }
    ticket_raw
        .parse::<DocTicket>()
        .map_err(|error| SyncwebError::InvalidTicket(error.to_string()))
}