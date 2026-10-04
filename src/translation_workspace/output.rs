use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::model::ProtectedWorkspaceIndex;

pub(super) fn write_index(staged: &Path, index: &ProtectedWorkspaceIndex) -> Result<()> {
    let path = staged.join("index.json");
    let mut bytes = serde_json::to_vec_pretty(index)?;
    bytes.push(b'\n');
    fs::write(&path, bytes).with_context(|| {
        format!(
            "failed to write translation workspace index {}",
            path.display()
        )
    })
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod output_tests;
