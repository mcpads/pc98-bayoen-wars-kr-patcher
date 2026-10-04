use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::audit_directory::publish_new_directory;
use crate::{AudioAuditEntry, AudioAuditReport};

pub(super) struct RenderedSample {
    pub entry: AudioAuditEntry,
    pub output_file: String,
    pub wav: Vec<u8>,
}

pub(super) fn publish_samples(
    output_directory: &Path,
    samples: Vec<RenderedSample>,
) -> Result<AudioAuditReport> {
    let entries = publish_new_directory(output_directory, |staged| {
        let mut entries = Vec::with_capacity(samples.len());
        for sample in samples {
            let path = staged.join(&sample.output_file);
            fs::write(&path, &sample.wav)
                .with_context(|| format!("failed to write audio preview {}", path.display()))?;
            entries.push(sample.entry);
        }
        Ok(entries)
    })?;

    Ok(AudioAuditReport {
        output_directory: output_directory.to_path_buf(),
        source_asset: "SAMPA".to_owned(),
        sample_count: entries.len(),
        entries,
    })
}
