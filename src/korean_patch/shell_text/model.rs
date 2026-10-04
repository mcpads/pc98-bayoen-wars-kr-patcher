use std::collections::BTreeMap;

use serde::Serialize;
use v30::AssembledProgram;

use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::PayloadWriteReport;
use super::super::shared_text::{CompiledGaijiBank, SharedGaijiGlyph};

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ShellTextEntryPatchReport {
    pub id: String,
    pub original_file_offset: usize,
    pub file_offset: usize,
    pub com_address: u16,
    pub byte_size: usize,
    pub line_count: usize,
    pub reference_count: usize,
    pub content_sha256: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ShellTextPatchReport {
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
    pub packed_text_bytes: usize,
    pub glyphs: Vec<SharedGaijiGlyph>,
    pub entries: Vec<ShellTextEntryPatchReport>,
    pub writes: Vec<PayloadWriteReport>,
}

pub(crate) struct PatchedShellTextFiles {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: ShellTextPatchReport,
}

pub(super) struct CompiledShellText {
    pub bank: CompiledGaijiBank,
    pub source_file_size: usize,
    pub output_file_size: usize,
    pub installer_offset: usize,
    pub installer: AssembledProgram,
    pub glyph_records_offset: usize,
    pub glyph_records: Vec<u8>,
    pub text_offset: usize,
    pub packed_text: Vec<u8>,
    pub entries: Vec<CompiledShellTextEntry>,
    pub typed_sources: BTreeMap<String, AssembledProgram>,
}

#[derive(Debug)]
pub(super) struct CompiledShellTextEntry {
    pub id: String,
    pub original_file_offset: usize,
    pub file_offset: usize,
    pub com_address: u16,
    pub lines: Vec<String>,
    pub bytes: Vec<u8>,
    pub consumer_offsets: Vec<usize>,
}
