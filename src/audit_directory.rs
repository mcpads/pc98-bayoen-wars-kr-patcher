use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

pub(crate) fn publish_new_directory<T>(
    output_directory: &Path,
    write_staged: impl FnOnce(&Path) -> Result<T>,
) -> Result<T> {
    if output_directory.exists() {
        bail!(
            "refusing to overwrite existing audit directory: {}",
            output_directory.display()
        );
    }
    let parent = output_directory
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create audit parent {}", parent.display()))?;
    let temporary = tempfile::Builder::new()
        .prefix(".audit-")
        .tempdir_in(parent)
        .with_context(|| format!("failed to stage audit in {}", parent.display()))?;

    let result = write_staged(temporary.path())?;
    let staged_directory = temporary.keep();
    if let Err(error) = fs::rename(&staged_directory, output_directory) {
        let cleanup_result = fs::remove_dir_all(&staged_directory);
        let cleanup_note = cleanup_result
            .err()
            .map(|cleanup_error| format!("; staging cleanup also failed: {cleanup_error}"))
            .unwrap_or_default();
        bail!(
            "failed to publish audit at {}: {}{}",
            output_directory.display(),
            error,
            cleanup_note
        );
    }
    Ok(result)
}

#[cfg(test)]
#[path = "audit_directory_tests.rs"]
mod audit_directory_tests;
