use std::collections::BTreeMap;

use serde::Serialize;

use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::PayloadWriteReport;
use super::super::shared_text::{CompiledGaijiBank, SharedGaijiGlyph};

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadSystemTextEntryPatchReport {
    pub id: String,
    pub original_file_offset: usize,
    pub file_offset: usize,
    pub com_address: u16,
    pub byte_size: usize,
    pub reference_count: usize,
    pub machine_code_reference_count: usize,
    pub metadata_reference_count: usize,
    pub runtime_insert_byte_offset: Option<usize>,
    pub runtime_insert_byte_capacity: Option<usize>,
    pub content_sha256: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadSystemTextPatchReport {
    pub supported_source_sha256: String,
    pub translation_status: String,
    pub build_status: DevelopmentBuildStatus,
    pub font_profile: String,
    pub font_sha256: String,
    pub available_gaiji_slots: usize,
    pub used_gaiji_slots: usize,
    pub storage_capacity: usize,
    pub packed_storage_bytes: usize,
    pub storage_headroom: usize,
    pub semantic_reference_count: usize,
    pub storage_reference_count: usize,
    pub machine_code_reference_count: usize,
    pub metadata_reference_count: usize,
    pub runtime_insert_reference_count: usize,
    pub glyphs: Vec<SharedGaijiGlyph>,
    pub entries: Vec<MadSystemTextEntryPatchReport>,
    pub writes: Vec<PayloadWriteReport>,
}

pub(crate) struct PatchedMadSystemTextPayload {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: MadSystemTextPatchReport,
}

pub(in crate::korean_patch) struct CompiledMadSystemText {
    pub bank: CompiledGaijiBank,
    pub text_region_start: usize,
    pub text_region_end: usize,
    pub packed_storage_bytes: usize,
    pub populated_storage_bytes: usize,
    pub text_region_replacement: Vec<u8>,
    pub entries: Vec<CompiledMadSystemTextEntry>,
}

#[derive(Debug)]
pub(in crate::korean_patch) struct CompiledMadSystemTextEntry {
    pub id: String,
    pub original_file_offset: usize,
    pub file_offset: usize,
    pub com_address: u16,
    pub lines: Vec<String>,
    pub runtime_insert_byte_offset: Option<usize>,
    pub runtime_insert_byte_capacity: Option<usize>,
    pub bytes: Vec<u8>,
}
