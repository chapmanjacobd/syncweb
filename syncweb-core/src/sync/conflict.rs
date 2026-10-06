//! Conflict detection and materialization for divergent folder entries.
//!
//! iroh-docs keeps every variant of a key (one record per author) in its
//! store; the collapsed `single_latest_per_key` views used for normal reads
//! hide all but the winner. This module exposes every variant (so losers stay
//! recoverable) and materializes winner + losers to a mount root:
//!
//! - The winner (greatest `timestamp()`, ties broken deterministically by
//!   author) always keeps the original path — matching the `single_latest_per_key`
//!   collapsed view used everywhere else.
//! - Each loser is written as `<path>.conflict.<short-hash>`, so no version is
//!   ever lost on disk.
//!
//! `receiveonly` folders materialize only the winner; a read-only replica must
//! never resolve a conflict by writing locally.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use iroh_blobs::Hash;
use iroh_docs::Entry;

use crate::{
    error::{Result, SyncwebError},
    folder::FolderManager,
    node::iroh_node::IrohNode,
};

/// Outcome of one materialization pass over a folder.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct MaterializeSummary {
    /// Distinct keys whose winner was written to the mount root.
    pub materialized: usize,
    /// Loser variants written as `<path>.conflict.<short-hash>`.
    pub conflicts: usize,
    /// Keys whose winner blob is not local yet (skipped, retried later).
    pub pending: usize,
}

/// Outcome plus the list of absolute paths written during a materialization
/// pass, so callers can skip re-processing those exact files.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct MaterializeOutcome {
    /// Aggregate counters from the pass.
    pub summary: MaterializeSummary,
    /// Absolute paths that were written (both winners and conflict copies).
    pub written: Vec<PathBuf>,
}

/// Select the winner among variants of a single key.
///
/// LWW by record timestamp; ties are broken deterministically by author bytes
/// so every node converges without tie-breaking on arrival order.
#[must_use]
pub fn select_winner(entries: &[Entry]) -> Option<&Entry> {
    entries.iter().max_by(|left, right| {
        left.timestamp()
            .cmp(&right.timestamp())
            .then_with(|| left.author().as_bytes().cmp(right.author().as_bytes()))
    })
}

/// Group every non-`sys/` entry by its key, preserving all author variants.
#[must_use]
pub fn group_by_key(entries: Vec<Entry>) -> BTreeMap<Vec<u8>, Vec<Entry>> {
    let mut grouped: BTreeMap<Vec<u8>, Vec<Entry>> = BTreeMap::new();
    for entry in entries {
        if entry.key().starts_with(b"sys/") {
            continue;
        }
        grouped.entry(entry.key().to_vec()).or_default().push(entry);
    }
    grouped
}

/// Path for a losing variant: `<path>.conflict.<short-hash>`.
#[must_use]
pub fn conflict_path(path: &Path, hash: Hash) -> PathBuf {
    let short: String = hash.to_hex().chars().take(8).collect();
    PathBuf::from(format!("{}.conflict.{short}", path.to_string_lossy()))
}

/// Export one blob to `destination` when the blob is local and the file is not
/// already up to date. Returns whether a file was written.
async fn export_if_different(node: &IrohNode, hash: Hash, expected_size: u64, destination: &Path) -> Result<bool> {
    if !node.blob_store().has(hash).await? {
        return Ok(false);
    }
    if let Ok(metadata) = tokio::fs::metadata(destination).await
        && metadata.is_file()
        && metadata.len() == expected_size
    {
        return Ok(false);
    }
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| SyncwebError::operation("failed to create folder materialization directory", error))?;
    }
    node.blob_store()
        .export_to_path(hash, destination)
        .await
        .map_err(|error| SyncwebError::operation("failed to materialize folder entry", error))?;
    Ok(true)
}

/// Materialize every entry of `folder` under `mount_root`, resolving conflicts.
///
/// The winner of each key keeps its original path; losing variants are written
/// as `<path>.conflict.<short-hash>`. A `receiveonly` folder materializes only
/// the winner (it must never resolve a conflict by writing locally). Blobs that
/// are not yet local are skipped and reported as `pending`.
///
/// # Errors
///
/// Returns an error if the folder's document cannot be read.
pub async fn materialize_folder_content(
    node: &IrohNode,
    manager: &FolderManager,
    namespace: iroh_docs::NamespaceId,
    mount_root: &Path,
) -> Result<MaterializeOutcome> {
    let folder = manager.get(namespace).await?;
    let entries = folder.docs_engine().list_all_entries(folder.doc()).await?;
    let grouped = group_by_key(entries);
    let mut summary = MaterializeSummary::default();
    let mut written = Vec::new();
    for (key, variants) in grouped {
        let path = PathBuf::from(String::from_utf8_lossy(&key).into_owned());
        let Some(winner) = select_winner(&variants) else {
            continue;
        };
        let winner_hash = winner.content_hash();
        let winner_size = winner.content_len();
        let destination = mount_root.join(&path);
        match export_if_different(node, winner_hash, winner_size, &destination).await {
            Ok(true) => {
                summary.materialized = summary.materialized.saturating_add(1);
                written.push(destination);
            }
            Ok(false) if !node.blob_store().has(winner_hash).await? => {
                summary.pending = summary.pending.saturating_add(1);
            }
            Ok(false) => {}
            Err(error) => {
                tracing::warn!(%namespace, path = %path.display(), %error, "failed to materialize winner");
            }
        }
        if !folder.is_writable() {
            continue;
        }
        for loser in variants
            .iter()
            .filter(|variant| variant.content_hash() != winner_hash && variant.content_hash() != Hash::EMPTY)
        {
            let loser_path = mount_root.join(conflict_path(&path, loser.content_hash()));
            match export_if_different(node, loser.content_hash(), loser.content_len(), &loser_path).await {
                Ok(true) => {
                    summary.conflicts = summary.conflicts.saturating_add(1);
                    written.push(loser_path);
                }
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(%namespace, path = %loser_path.display(), %error, "failed to materialize conflict copy");
                }
            }
        }
    }
    Ok(MaterializeOutcome { summary, written })
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh_docs::{AuthorId, NamespaceId, Record};

    fn entry(timestamp: u64, author: u8, hash_byte: u8, key: &[u8]) -> Entry {
        let mut author_bytes = [0_u8; 32];
        author_bytes[0] = author;
        let author_id = AuthorId::from(author_bytes);
        let mut hash_bytes = [0_u8; 32];
        hash_bytes[0] = hash_byte;
        let id = iroh_docs::RecordIdentifier::new(NamespaceId::from([0; 32]), author_id, key);
        let record = Record::new(Hash::from_bytes(hash_bytes), 16, timestamp);
        Entry::new(id, record)
    }

    fn author_id(byte: u8) -> AuthorId {
        let mut bytes = [0_u8; 32];
        bytes[0] = byte;
        AuthorId::from(bytes)
    }

    #[test]
    fn winner_is_latest_timestamp() {
        let worth = entry(5, 2, 2, b"a.txt");
        let older = entry(3, 1, 1, b"a.txt");
        let variants = [older, worth];
        let winner = select_winner(&variants);
        assert_eq!(winner.expect("winner should be selected").timestamp(), 5);
        assert_eq!(winner.expect("winner should be selected").author(), author_id(2));
    }

    #[test]
    fn winner_tie_breaks_by_author() {
        let left = entry(10, 3, 1, b"a.txt");
        let right = entry(10, 7, 2, b"a.txt");
        let variants = [left, right];
        let winner = select_winner(&variants);
        assert_eq!(winner.expect("winner should be selected").author(), author_id(7));
    }

    #[test]
    fn missing_variants_yield_none() {
        assert!(select_winner(&[]).is_none());
    }

    #[test]
    fn grouping_skips_system_keys() {
        let grouped = group_by_key(vec![
            entry(1, 1, 1, b"a.txt"),
            entry(2, 2, 2, b"a.txt"),
            entry(3, 3, 3, b"sys/syncweb/mode"),
        ]);
        assert!(grouped.contains_key(b"a.txt".as_slice()));
        assert!(!grouped.contains_key(b"sys/syncweb/mode".as_slice()));
        assert_eq!(grouped.get(b"a.txt".as_slice()).map_or(0, Vec::len), 2);
    }

    #[test]
    fn conflict_path_embeds_short_hash() {
        let mut bytes = [0_u8; 32];
        bytes[0] = 0xab;
        bytes[31] = 0xcd;
        let hash = Hash::from_bytes(bytes);
        let path = conflict_path(Path::new("report.md"), hash);
        let text = path.to_string_lossy();
        assert!(text.starts_with("report.md.conflict."));
        assert_eq!(text.len(), "report.md.conflict.".len() + 8);
    }

    #[test]
    fn empty_hash_is_distinct_from_content() {
        let mut bytes = [0_u8; 32];
        bytes[0] = 0xab;
        let content = Hash::from_bytes(bytes);
        assert_ne!(Hash::EMPTY, content);
    }
}
