use std::io::Write;
use std::path::Path;

use crate::error::Result;

pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| crate::error::SyncwebError::operation("failed to create directory", e))?;
    }

    let temporary_path = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let result = (|| -> Result<()> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary_path)
            .map_err(|e| crate::error::SyncwebError::operation("failed to create temporary file", e))?;
        file.write_all(data)?;
        file.sync_all()?;
        std::fs::rename(&temporary_path, path)
            .map_err(|e| crate::error::SyncwebError::operation("failed to persist file", e))
    })();

    if result.is_err()
        && let Err(error) = std::fs::remove_file(&temporary_path)
    {
        tracing::warn!(
            path = %temporary_path.display(),
            ?error,
            "failed to clean up temporary file"
        );
    }
    result
}
