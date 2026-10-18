//! Local filesystem operations used by syncweb.

use std::path::Path;

pub(crate) mod atomic;
pub mod exporter;
pub mod importer;
pub mod scanner;
pub mod watcher;

pub use exporter::{ExportEntry, Exporter, ParallelExporter};
pub use importer::{ImportEntry, Importer, ParallelImporter};
pub use scanner::{CONFLICT_COPY_MARKER, FileEntry, FileType, IgnoreFilter, ParallelScanner, Scanner, ThreadCount};
pub use watcher::{FsEvent, FsWatcher};

/// Render a filesystem-relative path in the portable form used by sync keys.
#[must_use]
pub fn logical_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
