use std::collections::BTreeMap;

use serde::Serialize;

use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::PayloadWriteReport;
use super::super::shared_text::{CompiledGaijiBank, SharedGaijiGlyph};

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct DialogueTextEntryPatchReport {
    pub id: String,
    pub group_id: String,
    pub presentation_variant: u16,
    pub text_pointer_offset: usize,
    pub original_file_offset: usize,
    pub file_offset: usize,
    pub com_address: u16,
    pub byte_size: usize,
    pub line_count: usize,
    pub content_sha256: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct DialogueTextPatchReport {
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
    pub group_count: usize,
    pub entry_count: usize,
    pub reference_count: usize,
    pub machine_code_reference_count: usize,
    pub group_pointer_reference_count: usize,
    pub text_pointer_reference_count: usize,
    pub glyphs: Vec<SharedGaijiGlyph>,
    pub entries: Vec<DialogueTextEntryPatchReport>,
    pub writes: Vec<PayloadWriteReport>,
}

pub(crate) struct PatchedDialogueTextPayload {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: DialogueTextPatchReport,
}

pub(in crate::korean_patch) struct CompiledDialogueText {
    pub bank: CompiledGaijiBank,
    pub record_table_start: usize,
    pub record_table_end: usize,
    pub record_table_replacement: Vec<u8>,
    pub text_region_start: usize,
    pub text_region_end: usize,
    pub packed_storage_bytes: usize,
    pub text_region_replacement: Vec<u8>,
    pub entries: Vec<CompiledDialogueTextEntry>,
}

#[derive(Debug)]
pub(in crate::korean_patch) struct CompiledDialogueTextEntry {
    pub id: String,
    pub group_id: String,
    pub presentation_variant: u16,
    pub text_pointer_offset: usize,
    pub original_file_offset: usize,
    pub file_offset: usize,
    pub com_address: u16,
    pub lines: Vec<String>,
    pub bytes: Vec<u8>,
}
