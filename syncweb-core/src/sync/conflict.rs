//! Conflict detection and materialization for divergent folder entries.
//!
//! iroh-docs keeps every variant of a key (one record per author) in its
//! store; the collapsed `single_latest_per_key` views used for normal reads
//! hide all but the winner. This module exposes every variant (so losers stay
//! recoverable) and materializes winner + losers to a mount root:
//!
//! - The winner (greatest `timestamp()`, ties broken deterministically by
//!   author), or a clean three-way merge recorded as a new local entry, keeps
//!   the original path — matching the `single_latest_per_key` collapsed view
//!   used everywhere else.
//! - A single text loser gets a compact `<path>.diff` when that diff is smaller
//!   than the winner; otherwise each loser is written as
//!   `<path>.conflict.<short-hash>`, so no version is ever lost on disk.
//!
//! `receiveonly` folders materialize only the winner; a read-only replica must
//! never resolve a conflict by writing locally.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
    str,
};

use iroh_blobs::Hash;
use iroh_docs::Entry;
use serde::{Deserialize, Serialize};

use crate::{
    error::{Result, SyncwebError},
    folder::{FolderManager, SyncwebFolder},
    node::iroh_node::IrohNode,
};

const MAX_TEXT_DIFF_BYTES: usize = 1_048_576;
const MAX_TEXT_DIFF_LINES: usize = 2_048;
const DIFF_CONTEXT_LINES: usize = 3;

/// Result of a bounded three-way text merge.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ThreeWayMerge {
    /// Both edits can be represented without overlapping changes.
    Clean(Vec<u8>),
    /// The edits overlap and the caller should retain both source variants.
    Conflict,
}

/// Outcome of one materialization pass over a folder.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct MaterializeSummary {
    /// Distinct keys whose canonical content was written to the mount root.
    pub materialized: usize,
    /// Loser variants written as a diff or `<path>.conflict.<short-hash>`.
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
    /// Absolute paths that were written (both winners and conflict artifacts).
    pub written: Vec<PathBuf>,
}

/// Remove acknowledged conflict artifacts from a materialized folder.
///
/// This only deletes loser artifacts (`.diff` and `.conflict.<hash>` files);
/// document entries and the canonical winner are left untouched.
///
/// # Errors
///
/// Returns an error if the folder entries cannot be read, a logical path is
/// unsafe to resolve below `mount_root`, or an artifact cannot be removed.
pub async fn resolve_conflict_artifacts(
    node: &IrohNode,
    folder: &SyncwebFolder,
    mount_root: &Path,
    selected_path: Option<&Path>,
) -> Result<usize> {
    let entries = folder.docs_engine().list_all_entries(folder.doc()).await?;
    let reports = list_conflicts(node, entries).await?;
    let mut removed = 0_usize;
    for report in reports {
        let path = Path::new(&report.path);
        if selected_path.is_some_and(|selected| selected != path) {
            continue;
        }
        let relative = safe_relative_path(path)?;
        let diff = mount_root.join(diff_path(relative));
        if remove_if_exists(&diff).await? {
            removed = removed.saturating_add(1);
        }
        for variant in report.variants.into_iter().filter(|variant| !variant.winner) {
            let artifact = mount_root.join(conflict_path(relative, variant.hash));
            if remove_if_exists(&artifact).await? {
                removed = removed.saturating_add(1);
            }
        }
    }
    Ok(removed)
}

fn safe_relative_path(path: &Path) -> Result<&Path> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(SyncwebError::operation(
            "cannot resolve conflict artifacts",
            format!("unsafe logical path '{}'", path.display()),
        ));
    }
    Ok(path)
}

async fn remove_if_exists(path: &Path) -> Result<bool> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(SyncwebError::operation(
            "failed to remove conflict artifact",
            format!("{}: {error}", path.display()),
        )),
    }
}

/// One author-specific variant of a conflicting document entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ConflictVariant {
    pub hash: Hash,
    pub size: u64,
    pub timestamp: u64,
    pub author: String,
    pub winner: bool,
    pub local: bool,
}

/// All divergent variants for one logical folder path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ConflictReport {
    pub path: String,
    pub variants: Vec<ConflictVariant>,
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

/// List divergent content variants, including whether each blob is local.
///
/// Entries with multiple authors but identical content are not conflicts: the
/// materialized bytes are already identical and no user-visible resolution is
/// needed.
///
/// # Errors
///
/// Returns an error if local blob availability cannot be queried.
pub async fn list_conflicts(node: &IrohNode, entries: Vec<Entry>) -> Result<Vec<ConflictReport>> {
    let mut reports = Vec::new();
    for (key, variants) in group_by_key(entries) {
        let Some(winner) = select_winner(&variants) else {
            continue;
        };
        let hashes: BTreeSet<_> = variants.iter().map(Entry::content_hash).collect();
        if hashes.len() < 2 {
            continue;
        }
        let winner_hash = winner.content_hash();
        let winner_timestamp = winner.timestamp();
        let winner_author = winner.author();
        let mut report_variants = Vec::with_capacity(variants.len());
        for variant in variants {
            let hash = variant.content_hash();
            let local = hash != Hash::EMPTY && node.blob_store().has(hash).await?;
            report_variants.push(ConflictVariant {
                hash,
                size: variant.content_len(),
                timestamp: variant.timestamp(),
                author: variant.author().to_string(),
                winner: hash == winner_hash
                    && variant.timestamp() == winner_timestamp
                    && variant.author() == winner_author,
                local,
            });
        }
        report_variants.sort_by(|left, right| {
            right
                .winner
                .cmp(&left.winner)
                .then_with(|| right.timestamp.cmp(&left.timestamp))
                .then_with(|| right.author.cmp(&left.author))
                .then_with(|| left.hash.to_string().cmp(&right.hash.to_string()))
        });
        reports.push(ConflictReport {
            path: String::from_utf8_lossy(&key).into_owned(),
            variants: report_variants,
        });
    }
    reports.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(reports)
}

/// Path for a losing variant: `<path>.conflict.<short-hash>`.
#[must_use]
pub fn conflict_path(path: &Path, hash: Hash) -> PathBuf {
    let short: String = hash.to_hex().chars().take(8).collect();
    PathBuf::from(format!("{}.conflict.{short}", path.to_string_lossy()))
}

/// Path for a compact text diff: `<path>.diff`.
#[must_use]
pub fn diff_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.diff", path.to_string_lossy()))
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
        let existing = tokio::fs::read(destination)
            .await
            .map_err(|error| SyncwebError::operation("failed to read existing folder materialization", error))?;
        let existing_hash = Hash::from_bytes(*blake3::hash(&existing).as_bytes());
        if existing_hash == hash {
            return Ok(false);
        }
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

async fn write_if_different(contents: &[u8], destination: &Path) -> Result<bool> {
    match tokio::fs::read(destination).await {
        Ok(existing) if existing == contents => return Ok(false),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(SyncwebError::operation("failed to read existing conflict diff", error));
        }
    }
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| SyncwebError::operation("failed to create conflict diff directory", error))?;
    }
    tokio::fs::write(destination, contents)
        .await
        .map_err(|error| SyncwebError::operation("failed to materialize conflict diff", error))?;
    Ok(true)
}

#[derive(Clone, Copy)]
enum DiffLine<'a> {
    Equal(&'a str),
    Delete(&'a str),
    Insert(&'a str),
}

fn line_diff<'a>(left: &[&'a str], right: &[&'a str]) -> Option<Vec<DiffLine<'a>>> {
    let width = right.len().checked_add(1)?;
    let height = left.len().checked_add(1)?;
    let cell_count = width.checked_mul(height)?;
    let mut lcs = vec![0_usize; cell_count];
    for left_index in (0..left.len()).rev() {
        for right_index in (0..right.len()).rev() {
            let current = left_index.saturating_mul(width).saturating_add(right_index);
            let next_left = left_index
                .saturating_add(1)
                .saturating_mul(width)
                .saturating_add(right_index);
            let next_right = left_index
                .saturating_mul(width)
                .saturating_add(right_index.saturating_add(1));
            let diagonal = left_index
                .saturating_add(1)
                .saturating_mul(width)
                .saturating_add(right_index.saturating_add(1));
            let left_line = left.get(left_index)?;
            let right_line = right.get(right_index)?;
            let value = if left_line == right_line {
                lcs.get(diagonal)?.saturating_add(1)
            } else {
                (*lcs.get(next_left)?).max(*lcs.get(next_right)?)
            };
            *lcs.get_mut(current)? = value;
        }
    }

    let mut result = Vec::with_capacity(left.len().saturating_add(right.len()));
    let (mut left_index, mut right_index) = (0_usize, 0_usize);
    while left_index < left.len() && right_index < right.len() {
        let left_line = left.get(left_index)?;
        let right_line = right.get(right_index)?;
        if left_line == right_line {
            result.push(DiffLine::Equal(left_line));
            left_index = left_index.saturating_add(1);
            right_index = right_index.saturating_add(1);
        } else {
            let next_left = left_index
                .saturating_add(1)
                .saturating_mul(width)
                .saturating_add(right_index);
            let next_right = left_index
                .saturating_mul(width)
                .saturating_add(right_index.saturating_add(1));
            if lcs.get(next_left)? >= lcs.get(next_right)? {
                result.push(DiffLine::Delete(left_line));
                left_index = left_index.saturating_add(1);
            } else {
                result.push(DiffLine::Insert(right_line));
                right_index = right_index.saturating_add(1);
            }
        }
    }
    while left_index < left.len() {
        result.push(DiffLine::Delete(left.get(left_index)?));
        left_index = left_index.saturating_add(1);
    }
    while right_index < right.len() {
        result.push(DiffLine::Insert(right.get(right_index)?));
        right_index = right_index.saturating_add(1);
    }
    Some(result)
}

struct LineEdit<'a> {
    start: usize,
    end: usize,
    replacement: Vec<&'a str>,
}

fn line_edits<'a>(base: &[&'a str], side: &[&'a str]) -> Option<Vec<LineEdit<'a>>> {
    let operations = line_diff(base, side)?;
    let mut edits = Vec::new();
    let mut base_index = 0_usize;
    let mut current: Option<LineEdit<'a>> = None;
    for operation in operations {
        match operation {
            DiffLine::Equal(_) => {
                if let Some(edit) = current.take() {
                    edits.push(edit);
                }
                base_index = base_index.saturating_add(1);
            }
            DiffLine::Delete(_) => {
                let edit = current.get_or_insert_with(|| LineEdit {
                    start: base_index,
                    end: base_index,
                    replacement: Vec::new(),
                });
                edit.end = edit.end.saturating_add(1);
                base_index = base_index.saturating_add(1);
            }
            DiffLine::Insert(line) => {
                current
                    .get_or_insert_with(|| LineEdit {
                        start: base_index,
                        end: base_index,
                        replacement: Vec::new(),
                    })
                    .replacement
                    .push(line);
            }
        }
    }
    if let Some(edit) = current {
        edits.push(edit);
    }
    Some(edits)
}

fn edits_overlap(left: &LineEdit<'_>, right: &LineEdit<'_>) -> bool {
    if left.start == left.end && right.start == right.end {
        left.start == right.start
    } else if left.start == left.end {
        (right.start..right.end).contains(&left.start)
    } else if right.start == right.end {
        (left.start..left.end).contains(&right.start)
    } else {
        left.start < right.end && right.start < left.end
    }
}

fn apply_edits<'a>(base: &[&'a str], start: usize, end: usize, edits: &[LineEdit<'a>]) -> Option<Vec<&'a str>> {
    let mut output = Vec::new();
    let mut cursor = start;
    for edit in edits {
        if edit.start < cursor || edit.end > end {
            return None;
        }
        output.extend(base.get(cursor..edit.start)?);
        output.extend(edit.replacement.iter().copied());
        cursor = edit.end;
    }
    output.extend(base.get(cursor..end)?);
    Some(output)
}

fn bounded_text_lines(bytes: &[u8]) -> Option<Vec<&str>> {
    if bytes.len() > MAX_TEXT_DIFF_BYTES {
        return None;
    }
    let text = str::from_utf8(bytes).ok()?;
    let lines: Vec<_> = text.split_inclusive('\n').collect();
    (lines.len() <= MAX_TEXT_DIFF_LINES).then_some(lines)
}

/// Merge two bounded UTF-8 line edits against a common ancestor.
///
/// A clean result is returned when edits touch disjoint ranges, or when both
/// sides made the same change. Overlapping edits return [`ThreeWayMerge::Conflict`]
/// so the caller can preserve both variants instead of silently choosing one.
#[must_use]
pub fn merge_text_three_way(base: &[u8], left: &[u8], right: &[u8]) -> Option<ThreeWayMerge> {
    let base_lines = bounded_text_lines(base)?;
    let left_lines = bounded_text_lines(left)?;
    let right_lines = bounded_text_lines(right)?;
    if left == right {
        return Some(ThreeWayMerge::Clean(left.to_vec()));
    }
    if left == base {
        return Some(ThreeWayMerge::Clean(right.to_vec()));
    }
    if right == base {
        return Some(ThreeWayMerge::Clean(left.to_vec()));
    }

    let left_edits = line_edits(&base_lines, &left_lines)?;
    let right_edits = line_edits(&base_lines, &right_lines)?;
    let mut output = Vec::new();
    let (mut left_index, mut right_index, mut cursor) = (0_usize, 0_usize, 0_usize);

    while left_index < left_edits.len() || right_index < right_edits.len() {
        let left_edit = left_edits.get(left_index);
        let right_edit = right_edits.get(right_index);
        let next_start = match (left_edit, right_edit) {
            (Some(left_candidate), Some(right_candidate)) => left_candidate.start.min(right_candidate.start),
            (Some(left_candidate), None) => left_candidate.start,
            (None, Some(right_candidate)) => right_candidate.start,
            (None, None) => break,
        };
        output.extend(base_lines.get(cursor..next_start)?);

        match (left_edit, right_edit) {
            (Some(left_candidate), Some(right_candidate)) if edits_overlap(left_candidate, right_candidate) => {
                let start = left_candidate.start.min(right_candidate.start);
                let mut end = left_candidate.end.max(right_candidate.end);
                let mut left_end = left_index;
                let mut right_end = right_index;
                loop {
                    let left_expanded = if let Some(edit) = left_edits.get(left_end)
                        && (edit.start < end || (edit.start == start && end == start))
                    {
                        end = end.max(edit.end);
                        left_end = left_end.saturating_add(1);
                        true
                    } else {
                        false
                    };
                    let right_expanded = if let Some(edit) = right_edits.get(right_end)
                        && (edit.start < end || (edit.start == start && end == start))
                    {
                        end = end.max(edit.end);
                        right_end = right_end.saturating_add(1);
                        true
                    } else {
                        false
                    };
                    if !left_expanded && !right_expanded {
                        break;
                    }
                }
                let left_result = apply_edits(&base_lines, start, end, left_edits.get(left_index..left_end)?)?;
                let right_result = apply_edits(&base_lines, start, end, right_edits.get(right_index..right_end)?)?;
                if left_result != right_result {
                    return Some(ThreeWayMerge::Conflict);
                }
                output.extend(left_result);
                left_index = left_end;
                right_index = right_end;
                cursor = end;
            }
            (Some(left_candidate), Some(right_candidate)) if left_candidate.end <= right_candidate.start => {
                output.extend(left_candidate.replacement.iter().copied());
                cursor = left_candidate.end;
                left_index = left_index.saturating_add(1);
            }
            (Some(left_candidate), None) => {
                output.extend(left_candidate.replacement.iter().copied());
                cursor = left_candidate.end;
                left_index = left_index.saturating_add(1);
            }
            (_, Some(right_candidate)) => {
                output.extend(right_candidate.replacement.iter().copied());
                cursor = right_candidate.end;
                right_index = right_index.saturating_add(1);
            }
            (None, None) => break,
        }
    }
    output.extend(base_lines.get(cursor..)?);
    let mut merged = String::new();
    for line in output {
        merged.push_str(line);
    }
    Some(ThreeWayMerge::Clean(merged.into_bytes()))
}

fn append_diff_line(output: &mut String, prefix: char, line: &str) {
    output.push(prefix);
    output.push_str(line);
    if !line.ends_with('\n') {
        output.push('\n');
    }
}

fn build_text_diff(winner_bytes: &[u8], loser_bytes: &[u8]) -> Option<Vec<u8>> {
    if winner_bytes.len() > MAX_TEXT_DIFF_BYTES || loser_bytes.len() > MAX_TEXT_DIFF_BYTES {
        return None;
    }
    let winner_text = str::from_utf8(winner_bytes).ok()?;
    let loser_text = str::from_utf8(loser_bytes).ok()?;
    let winner_lines: Vec<_> = winner_text.split_inclusive('\n').collect();
    let loser_lines: Vec<_> = loser_text.split_inclusive('\n').collect();
    if winner_lines.len() > MAX_TEXT_DIFF_LINES || loser_lines.len() > MAX_TEXT_DIFF_LINES {
        return None;
    }
    let operations = line_diff(&winner_lines, &loser_lines)?;
    let first_change = operations.iter().position(|line| !matches!(line, DiffLine::Equal(_)))?;
    let last_change = operations
        .iter()
        .rposition(|line| !matches!(line, DiffLine::Equal(_)))?;
    let start = first_change.saturating_sub(DIFF_CONTEXT_LINES);
    let end = last_change
        .saturating_add(DIFF_CONTEXT_LINES.saturating_add(1))
        .min(operations.len());
    let before = operations.get(..start)?;
    let hunk = operations.get(start..end)?;

    let mut old_before = 0_usize;
    let mut new_before = 0_usize;
    for operation in before {
        match operation {
            DiffLine::Equal(_) => {
                old_before = old_before.saturating_add(1);
                new_before = new_before.saturating_add(1);
            }
            DiffLine::Delete(_) => old_before = old_before.saturating_add(1),
            DiffLine::Insert(_) => new_before = new_before.saturating_add(1),
        }
    }
    let (mut old_count, mut new_count) = (0_usize, 0_usize);
    for operation in hunk {
        match operation {
            DiffLine::Equal(_) => {
                old_count = old_count.saturating_add(1);
                new_count = new_count.saturating_add(1);
            }
            DiffLine::Delete(_) => old_count = old_count.saturating_add(1),
            DiffLine::Insert(_) => new_count = new_count.saturating_add(1),
        }
    }

    let mut output = format!(
        "--- winner\n+++ loser\n@@ -{},{} +{},{} @@\n",
        old_before.saturating_add(1),
        old_count,
        new_before.saturating_add(1),
        new_count
    );
    for operation in hunk {
        match operation {
            DiffLine::Equal(line) => append_diff_line(&mut output, ' ', line),
            DiffLine::Delete(line) => append_diff_line(&mut output, '-', line),
            DiffLine::Insert(line) => append_diff_line(&mut output, '+', line),
        }
    }
    let output_bytes = output.into_bytes();
    (output_bytes.len() < winner_bytes.len()).then_some(output_bytes)
}

async fn try_materialize_text_diff(
    node: &IrohNode,
    winner_hash: Hash,
    loser_hash: Hash,
    path: &Path,
) -> Result<Option<bool>> {
    if !node.blob_store().has(winner_hash).await? || !node.blob_store().has(loser_hash).await? {
        return Ok(None);
    }
    let winner = node.blob_store().get(winner_hash).await?;
    let loser = node.blob_store().get(loser_hash).await?;
    let Some(diff) = build_text_diff(&winner, &loser) else {
        return Ok(None);
    };
    Ok(Some(write_if_different(&diff, &diff_path(path)).await?))
}

async fn try_merge_text_conflict(
    node: &IrohNode,
    ancestor_hash: Hash,
    winner_hash: Hash,
    variants: &[Entry],
) -> Result<Option<Hash>> {
    let side_hashes: BTreeSet<_> = variants
        .iter()
        .map(Entry::content_hash)
        .filter(|hash| *hash != Hash::EMPTY && *hash != ancestor_hash)
        .collect();
    if side_hashes.len() != 2 || !side_hashes.contains(&winner_hash) {
        return Ok(None);
    }
    let Some(loser_hash) = side_hashes.iter().copied().find(|hash| *hash != winner_hash) else {
        return Ok(None);
    };
    if ancestor_hash == Hash::EMPTY
        || !node.blob_store().has(ancestor_hash).await?
        || !node.blob_store().has(winner_hash).await?
        || !node.blob_store().has(loser_hash).await?
    {
        return Ok(None);
    }
    let base = node.blob_store().get(ancestor_hash).await?;
    let winner = node.blob_store().get(winner_hash).await?;
    let loser = node.blob_store().get(loser_hash).await?;
    let Some(ThreeWayMerge::Clean(merged)) = merge_text_three_way(&base, &winner, &loser) else {
        return Ok(None);
    };
    if merged == winner {
        return Ok(None);
    }
    Ok(Some(node.blob_store().add_bytes(&merged).await?))
}

async fn materialize_conflicts(
    node: &IrohNode,
    namespace: iroh_docs::NamespaceId,
    winner_hash: Hash,
    variants: &[Entry],
    path: &Path,
    mount_root: &Path,
) -> Vec<PathBuf> {
    let mut written = Vec::new();
    let losers: Vec<_> = variants
        .iter()
        .filter(|variant| variant.content_hash() != winner_hash && variant.content_hash() != Hash::EMPTY)
        .collect();
    if let [loser] = losers.as_slice() {
        let diff_destination = mount_root.join(diff_path(path));
        let winner_destination = mount_root.join(path);
        match try_materialize_text_diff(node, winner_hash, loser.content_hash(), &winner_destination).await {
            Ok(Some(true)) => {
                written.push(diff_destination);
                return written;
            }
            Ok(Some(false)) => return written,
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(
                    %namespace,
                    path = %diff_destination.display(),
                    %error,
                    "failed to materialize conflict diff; falling back to full copy"
                );
            }
        }
    }
    for loser in losers {
        let loser_path = mount_root.join(conflict_path(path, loser.content_hash()));
        match export_if_different(node, loser.content_hash(), loser.content_len(), &loser_path).await {
            Ok(true) => {
                written.push(loser_path);
            }
            Ok(false) => {}
            Err(error) => {
                tracing::warn!(%namespace, path = %loser_path.display(), %error, "failed to materialize conflict copy");
            }
        }
    }
    written
}

/// Materialize every entry of `folder` under `mount_root`, resolving conflicts.
///
/// The winner of each key, or a clean three-way merge, keeps its original path.
/// A single text loser is written as `<path>.diff` when the compact diff is
/// smaller than the winner; otherwise losing variants are written as
/// `<path>.conflict.<short-hash>`.
/// A `receiveonly` folder materializes only the winner (it must never resolve a
/// conflict by writing locally). Blobs that are not yet local are skipped and
/// reported as `pending`.
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
    materialize_folder_content_for_folder(node, &folder, mount_root, None).await
}

/// Materialize selected entries of an already-open folder, resolving conflicts
/// for each selected path.
///
/// `selected_paths` is `None` for the complete folder or contains the
/// folder-relative paths selected by a filtered download.
pub(crate) async fn materialize_folder_content_for_folder(
    node: &IrohNode,
    folder: &SyncwebFolder,
    mount_root: &Path,
    selected_paths: Option<&BTreeSet<PathBuf>>,
) -> Result<MaterializeOutcome> {
    let namespace = folder.namespace_id();
    let namespace_string = namespace.to_string();
    let entries = folder.docs_engine().list_all_entries(folder.doc()).await?;
    let grouped = group_by_key(entries);
    let mut summary = MaterializeSummary::default();
    let mut written = Vec::new();
    for (key, variants) in grouped {
        let path = PathBuf::from(String::from_utf8_lossy(&key).into_owned());
        if selected_paths.is_some_and(|paths| !paths.contains(&path)) {
            continue;
        }
        let safe_path = safe_relative_path(&path)?;
        let Some(winner) = select_winner(&variants) else {
            continue;
        };
        let winner_hash = winner.content_hash();
        let winner_size = winner.content_len();
        let distinct_hashes: BTreeSet<_> = variants.iter().map(Entry::content_hash).collect();
        let has_conflict = distinct_hashes.len() > 1;
        let writable = folder.mode().can_write_locally() && folder.is_writable();
        let ancestor = node.node_db().load_conflict_ancestor(&namespace_string, &key)?;
        let tracked_ancestor_hash = ancestor.map(|(hash, _)| hash);
        let already_resolved = tracked_ancestor_hash.is_some_and(|hash| hash == winner_hash);
        let mut materialization_hash = winner_hash;
        let mut materialization_size = winner_size;
        let clean_merge = if writable
            && has_conflict
            && let Some(ancestor_hash) = tracked_ancestor_hash
            && let Some(merged_hash) = try_merge_text_conflict(node, ancestor_hash, winner_hash, &variants).await?
        {
            let merged_size = u64::try_from(node.blob_store().get(merged_hash).await?.len())
                .map_err(|error| SyncwebError::operation("merged conflict content is too large", error))?;
            folder.set_blob_ref(&key, merged_hash, merged_size).await?;
            materialization_hash = merged_hash;
            materialization_size = merged_size;
            true
        } else {
            false
        };
        let destination = mount_root.join(safe_path);
        match export_if_different(node, materialization_hash, materialization_size, &destination).await {
            Ok(true) => {
                summary.materialized = summary.materialized.saturating_add(1);
                written.push(destination.clone());
            }
            Ok(false) if !node.blob_store().has(materialization_hash).await? => {
                summary.pending = summary.pending.saturating_add(1);
            }
            Ok(false) => {}
            Err(error) => {
                tracing::warn!(%namespace, path = %path.display(), %error, "failed to materialize winner");
            }
        }
        if !node.blob_store().has(materialization_hash).await? {
            continue;
        }
        if clean_merge {
            node.node_db().record_conflict_ancestor(
                &namespace_string,
                &key,
                materialization_hash,
                materialization_size,
            )?;
        }
        if !clean_merge && !has_conflict {
            node.node_db()
                .record_conflict_ancestor(&namespace_string, &key, winner_hash, winner_size)?;
        }
        if has_conflict && writable && !clean_merge && !already_resolved {
            let conflict_paths =
                materialize_conflicts(node, namespace, winner_hash, &variants, safe_path, mount_root).await;
            summary.conflicts = summary.conflicts.saturating_add(conflict_paths.len());
            written.extend(conflict_paths);
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
    fn text_diff_is_used_when_smaller_than_winner() {
        let winner = format!("{}old value\n{}", "unchanged\n".repeat(20), "unchanged\n".repeat(20));
        let loser = format!("{}new value\n{}", "unchanged\n".repeat(20), "unchanged\n".repeat(20));
        let diff = build_text_diff(winner.as_bytes(), loser.as_bytes()).expect("text diff should be selected");
        let text = String::from_utf8(diff).expect("diff should be UTF-8");
        assert!(text.contains("@@"));
        assert!(text.contains("-old value"));
        assert!(text.contains("+new value"));
        assert!(text.len() < winner.len());
    }

    #[test]
    fn binary_and_large_content_fall_back_to_full_copy() {
        assert!(build_text_diff(b"\0binary", b"\0changed").is_none());
        let large = vec![b'a'; MAX_TEXT_DIFF_BYTES.saturating_add(1)];
        assert!(build_text_diff(&large, b"small").is_none());
    }

    #[test]
    fn three_way_merge_combines_disjoint_line_edits() {
        let base = b"one\ntwo\nthree\n";
        let left = b"ONE\ntwo\nthree\n";
        let right = b"one\ntwo\nTHREE\n";
        assert_eq!(
            merge_text_three_way(base, left, right),
            Some(ThreeWayMerge::Clean(b"ONE\ntwo\nTHREE\n".to_vec()))
        );
    }

    #[test]
    fn three_way_merge_accepts_identical_edits() {
        let base = b"one\ntwo\n";
        let edited = b"one\nTWO\n";
        assert_eq!(
            merge_text_three_way(base, edited, edited),
            Some(ThreeWayMerge::Clean(edited.to_vec()))
        );
    }

    #[test]
    fn three_way_merge_reports_overlapping_edits() {
        let base = b"one\ntwo\nthree\n";
        let left = b"one\nLEFT\nthree\n";
        let right = b"one\nRIGHT\nthree\n";
        assert_eq!(merge_text_three_way(base, left, right), Some(ThreeWayMerge::Conflict));
    }

    #[test]
    fn three_way_merge_rejects_binary_and_oversized_content() {
        assert_eq!(merge_text_three_way(&[0xff, 0], &[0xff, 1], &[0xff, 2]), None);
        let large = vec![b'a'; MAX_TEXT_DIFF_BYTES.saturating_add(1)];
        assert_eq!(merge_text_three_way(b"base", &large, b"right"), None);
    }

    #[test]
    fn diff_path_is_suffixed_after_the_original_name() {
        assert_eq!(diff_path(Path::new("report.md")), PathBuf::from("report.md.diff"));
    }

    #[test]
    fn empty_hash_is_distinct_from_content() {
        let mut bytes = [0_u8; 32];
        bytes[0] = 0xab;
        let content = Hash::from_bytes(bytes);
        assert_ne!(Hash::EMPTY, content);
    }
}
