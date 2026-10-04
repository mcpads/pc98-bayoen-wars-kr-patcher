use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TranslationWorkspaceReport {
    pub output_directory: PathBuf,
    pub supported_source_sha256: String,
    pub segment_count: usize,
    pub entry_count: usize,
    pub target_file_count: usize,
    pub pending_source_file_count: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(super) struct ProtectedWorkspaceIndex {
    pub supported_source_sha256: String,
    pub target_population_complete: bool,
    pub translation_segmentation_complete: bool,
    pub target_files: Vec<TargetFileCoverage>,
    pub segments: Vec<SourceSegmentReference>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(super) struct SourceSegmentReference {
    pub id: String,
    pub source_file: String,
    pub surface: String,
    pub path: String,
    pub entry_count: usize,
    pub content_sha256: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(super) struct TargetFileCoverage {
    pub source_file: String,
    pub disposition: TargetFileDisposition,
    pub segment_ids: Vec<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum TargetFileDisposition {
    SourceUnitsExtracted,
    FontProducer,
    NeedsSourceSegmentation,
}

impl TargetFileDisposition {
    pub(super) fn needs_segmentation(self) -> bool {
        self == Self::NeedsSourceSegmentation
    }
}
