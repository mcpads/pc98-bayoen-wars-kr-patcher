use std::collections::BTreeMap;

use serde::Serialize;
use v30::AssembledProgram;

use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::PayloadWriteReport;
use super::super::shared_text::{CompiledGaijiBank, SharedGaijiGlyph};

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MenuTextEntryPatchReport {
    pub id: String,
    pub original_file_offset: usize,
    pub file_offset: usize,
    pub com_address: u16,
    pub byte_size: usize,
    pub line_count: usize,
    pub semantic_reference_count: usize,
    pub runtime_field_count: usize,
    pub content_sha256: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MenuTextPatchReport {
    pub supported_source_sha256: String,
    pub translation_status: String,
    pub build_status: DevelopmentBuildStatus,
    pub font_profile: String,
    pub font_sha256: String,
    pub source_file_size: usize,
    pub output_file_size: usize,
    pub available_gaiji_slots: usize,
    pub used_gaiji_slots: usize,
    pub installer_code_bytes: usize,
    pub glyph_record_bytes: usize,
    pub trampoline_code_bytes: usize,
    pub packed_text_bytes: usize,
    pub semantic_reference_count: usize,
    pub storage_reference_count: usize,
    pub machine_code_reference_count: usize,
    pub metadata_reference_count: usize,
    pub runtime_insert_reference_count: usize,
    pub glyphs: Vec<SharedGaijiGlyph>,
    pub entries: Vec<MenuTextEntryPatchReport>,
    pub writes: Vec<PayloadWriteReport>,
}

pub(crate) struct PatchedMenuTextFiles {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: MenuTextPatchReport,
}

pub(super) struct CompiledMenuText {
    pub bank: CompiledGaijiBank,
    pub source_file_size: usize,
    pub output_file_size: usize,
    pub installer_offset: usize,
    pub installer: AssembledProgram,
    pub glyph_records_offset: usize,
    pub glyph_records: Vec<u8>,
    pub trampoline_offset: usize,
    pub trampoline: AssembledProgram,
    pub text_offset: usize,
    pub packed_text: Vec<u8>,
    pub entries: Vec<CompiledMenuTextEntry>,
    pub typed_sources: BTreeMap<String, AssembledProgram>,
}

#[derive(Debug)]
pub(super) struct CompiledMenuTextEntry {
    pub id: String,
    pub original_file_offset: usize,
    pub file_offset: usize,
    pub com_address: u16,
    pub lines: Vec<String>,
    pub bytes: Vec<u8>,
    pub runtime_field_offsets: BTreeMap<String, usize>,
}
