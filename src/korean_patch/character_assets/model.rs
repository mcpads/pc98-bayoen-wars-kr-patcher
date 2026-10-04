use std::collections::BTreeMap;

use serde::Serialize;

use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::PayloadWriteReport;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ArleCharacterFrameReport {
    pub target_asset: String,
    pub target_source_start: usize,
    pub target_width: usize,
    pub target_height: usize,
    pub authoring_frame_index: usize,
    pub role: String,
    pub placement: String,
    pub source_coordinate_reference: Option<String>,
    pub subject_source_height: Option<usize>,
    pub subject_target_height: Option<usize>,
    pub subject_target_baseline_y: Option<usize>,
    pub source_visible_x: usize,
    pub source_visible_y: usize,
    pub source_visible_width: usize,
    pub source_visible_height: usize,
    pub replacement_visible_x: usize,
    pub replacement_visible_y: usize,
    pub replacement_visible_width: usize,
    pub replacement_visible_height: usize,
    pub used_palette_color_count: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ArleCharacterFileReport {
    pub file_name: String,
    pub decoded_size: usize,
    pub decoded_sha256: String,
    pub original_packed_size: usize,
    pub replacement_packed_size: usize,
    pub replacement_packed_sha256: String,
    pub replacement_decode_command_count: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ArleCharacterAssetPatchReport {
    pub supported_source_sha256: String,
    pub build_status: DevelopmentBuildStatus,
    pub approval_status: String,
    pub manifest_id: String,
    pub manifest_sha256: String,
    pub origin_repository: String,
    pub origin_commit: String,
    pub origin_manifest_sha256: String,
    pub artwork_sha256: String,
    pub authoring_sha256: String,
    pub source_palette: String,
    pub source_actor_entry_id: String,
    pub character_slot_index: usize,
    pub palette: String,
    pub palette_table_file_offset: usize,
    pub runtime_palette_rgb4: [[u8; 3]; 16],
    pub runtime_palette_rgb: [[u8; 3]; 16],
    pub quantization: String,
    pub placement: String,
    pub frame_count: usize,
    pub frames: Vec<ArleCharacterFrameReport>,
    pub files: Vec<ArleCharacterFileReport>,
    pub writes: Vec<PayloadWriteReport>,
}

pub(crate) struct PatchedArleCharacterAssetPayload {
    pub(crate) files: BTreeMap<String, Vec<u8>>,
    pub(crate) report: ArleCharacterAssetPatchReport,
}

pub(super) struct PreparedArleAssetFile {
    pub(super) file_name: &'static str,
    pub(super) decoded: Vec<u8>,
    pub(super) packed: Vec<u8>,
}
