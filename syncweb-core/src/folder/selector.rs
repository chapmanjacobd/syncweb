//! Shared folder-selector resolution.
//!
//! A "selector" is whatever the user typed to name a folder: a namespace ID, a
//! filesystem path, a unique namespace prefix, or `.`. The same rules apply
//! whether the folder list comes from a running daemon (`IpcClient`) or from an
//! embedded node, so the algorithm lives here and both sides feed it their own
//! `(namespace, mount path)` pairs.

use std::path::{Path, PathBuf};

use crate::error::{Result, SyncwebError};

/// One candidate folder: its namespace ID and its mount path, if any.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FolderMount {
    pub namespace: String,
    pub mount: Option<PathBuf>,
}

impl FolderMount {
    #[must_use]
    pub fn new(namespace: impl Into<String>, mount: Option<PathBuf>) -> Self {
        Self {
            namespace: namespace.into(),
            mount,
        }
    }
}

/// A selector resolved against a set of folders.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ResolvedSelector {
    pub namespace: String,
    /// The mount root the selector landed in, when it matched a path.
    pub mount_root: Option<PathBuf>,
    /// The sub-path inside the mount, for path selectors.
    pub remainder: PathBuf,
}

/// Canonicalize `path`, falling back to an absolute join with the current
/// directory when the path does not exist yet.
#[must_use]
pub fn canonicalize(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        }
    })
}

/// Resolve a selector to a namespace, using only namespace-prefix/single-folder
/// matching (no path lookup).
///
/// # Errors
///
/// Returns [`SyncwebError::NoFolders`] when there are no folders, or
/// [`SyncwebError::AmbiguousFolderSelector`] when the selector is not a
/// namespace ID and several folders exist.
pub fn resolve_namespace(selector: &str, folders: &[String]) -> Result<String> {
    if let Ok(namespace) = selector.parse::<iroh_docs::NamespaceId>() {
        return Ok(namespace.to_string());
    }
    if let Some(found) = folders.iter().find(|namespace| namespace.starts_with(selector)) {
        return Ok(found.clone());
    }
    match folders {
        [only] => Ok(only.clone()),
        [] => Err(SyncwebError::NoFolders),
        _ => Err(SyncwebError::AmbiguousFolderSelector(selector.to_owned())),
    }
}

/// Resolve a selector against `folders`, matching paths by longest common
/// prefix and producing the mount root plus the sub-path remainder.
///
/// Namespace IDs resolve directly. Path selectors pick the mount whose
/// canonical root is the longest prefix of the canonical selector. As a
/// fallback, a selector that does not exist on disk may match a namespace
/// prefix or the sole folder.
///
/// # Errors
///
/// Returns [`SyncwebError::NoFolders`] or [`SyncwebError::AmbiguousFolderSelector`]
/// for the fallback cases, and a descriptive error when the selector is not
/// inside any managed folder.
pub fn resolve_selector(selector: &Path, folders: &[FolderMount]) -> Result<ResolvedSelector> {
    let selector_str = selector.to_string_lossy();
    if let Ok(namespace) = selector_str.parse::<iroh_docs::NamespaceId>() {
        let mount_root = folders
            .iter()
            .find(|folder| folder.namespace == selector_str)
            .and_then(|folder| folder.mount.clone());
        return Ok(ResolvedSelector {
            namespace: namespace.to_string(),
            mount_root,
            remainder: PathBuf::new(),
        });
    }

    let canonical = canonicalize(selector);
    let mut best: Option<(String, PathBuf, PathBuf)> = None;
    for folder in folders {
        let Some(mount) = folder.mount.as_ref().filter(|mount| !mount.as_os_str().is_empty()) else {
            continue;
        };
        let root = canonicalize(mount);
        if let Ok(remainder) = canonical.strip_prefix(&root) {
            let better = best
                .as_ref()
                .is_none_or(|(_, best_root, _)| root.as_os_str().len() > best_root.as_os_str().len());
            if better {
                best = Some((folder.namespace.clone(), root, remainder.to_path_buf()));
            }
        }
    }
    if let Some((namespace, root, remainder)) = best {
        return Ok(ResolvedSelector {
            namespace,
            mount_root: Some(root),
            remainder,
        });
    }

    if !selector.exists() {
        let namespaces: Vec<String> = folders.iter().map(|folder| folder.namespace.clone()).collect();
        if let Ok(namespace) = resolve_namespace(&selector_str, &namespaces) {
            let mount_root = folders
                .iter()
                .find(|folder| folder.namespace == namespace)
                .and_then(|folder| folder.mount.clone())
                .filter(|mount| !mount.as_os_str().is_empty());
            return Ok(ResolvedSelector {
                namespace,
                mount_root,
                remainder: PathBuf::new(),
            });
        }
    }

    Err(SyncwebError::operation(
        format!(
            "{} is not inside of a Syncweb folder — run `syncweb ls --local-only {}` to list the disk directly",
            selector.display(),
            selector.display()
        ),
        "no matching mount",
    ))
}
