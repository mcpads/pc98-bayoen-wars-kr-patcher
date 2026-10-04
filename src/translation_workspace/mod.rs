mod model;
mod output;
mod population;
mod segments;

use std::path::Path;

use anyhow::Result;

pub use model::TranslationWorkspaceReport;

use crate::SourceStructureReport;
use crate::audit_directory::publish_new_directory;

pub(super) fn write_translation_workspace(
    source: &SourceStructureReport,
    output_directory: &Path,
) -> Result<TranslationWorkspaceReport> {
    let report = publish_new_directory(output_directory, |staged| {
        let segments = segments::write_source_segments(source, staged)?;
        let coverage =
            population::catalog_target_file_coverage(&source.localization_assets, &segments)?;
        let entry_count = segments.iter().map(|segment| segment.entry_count).sum();
        let pending_source_file_count = coverage
            .iter()
            .filter(|file| file.disposition.needs_segmentation())
            .count();
        let index = model::ProtectedWorkspaceIndex {
            supported_source_sha256: source.source_sha256.clone(),
            target_population_complete: source.localization_assets.unresolved == 0,
            translation_segmentation_complete: pending_source_file_count == 0,
            target_files: coverage,
            segments,
        };
        output::write_index(staged, &index)?;

        Ok(TranslationWorkspaceReport {
            output_directory: output_directory.to_path_buf(),
            supported_source_sha256: source.source_sha256.clone(),
            segment_count: index.segments.len(),
            entry_count,
            target_file_count: index.target_files.len(),
            pending_source_file_count,
        })
    })?;
    Ok(report)
}
