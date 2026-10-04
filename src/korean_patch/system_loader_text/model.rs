use std::collections::BTreeMap;

use serde::Serialize;
use v30::AssembledProgram;

use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::{PayloadWriteReport, UnpackedWriteReport};
use super::super::shared_text::{CompiledGaijiBank, SharedGaijiGlyph};
use crate::external_text::SystemLoaderRuntimeCatalog;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SystemLoaderTextEntryPatchReport {
    pub id: String,
    pub original_file_offset: usize,
    pub resident_offset: u16,
    pub byte_size: usize,
    pub reference_count: usize,
    pub content_sha256: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SystemLoaderTextPatchReport {
    pub supported_source_sha256: String,
    pub translation_status: String,
    pub build_status: DevelopmentBuildStatus,
    pub font_profile: String,
    pub font_sha256: String,
    pub source_file_size: usize,
    pub output_file_size: usize,
    pub source_resident_byte_count: u16,
    pub output_resident_byte_count: u16,
    pub resident_extension_byte_count: usize,
    pub shifted_tail_byte_count: usize,
    pub shifted_tail_sha256: String,
    pub available_gaiji_slots: usize,
    pub used_gaiji_slots: usize,
    pub installer_code_bytes: usize,
    pub glyph_record_bytes: usize,
    pub packed_text_bytes: usize,
    pub trampoline_code_bytes: usize,
    pub reference_count: usize,
    pub glyphs: Vec<SharedGaijiGlyph>,
    pub entries: Vec<SystemLoaderTextEntryPatchReport>,
    pub resident_writes: Vec<UnpackedWriteReport>,
    pub writes: Vec<PayloadWriteReport>,
}

pub(crate) struct PatchedSystemLoaderTextFiles {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: SystemLoaderTextPatchReport,
}

pub(super) struct CompiledSystemLoaderText {
    pub bank: CompiledGaijiBank,
    pub runtime: SystemLoaderRuntimeCatalog,
    pub source_bytes: Vec<u8>,
    pub reconstructed_source: Vec<u8>,
    pub output_bytes: Vec<u8>,
    pub output_resident_byte_count: u16,
    pub resident_extension_byte_count: usize,
    pub shifted_tail_sha256: String,
    pub installer_offset: usize,
    pub installer: AssembledProgram,
    pub glyph_records_offset: usize,
    pub glyph_records: Vec<u8>,
    pub text_offset: usize,
    pub packed_text: Vec<u8>,
    pub entries: Vec<CompiledSystemLoaderTextEntry>,
    pub trampoline_offset: usize,
    pub trampoline: AssembledProgram,
    pub typed_sources: BTreeMap<String, AssembledProgram>,
    pub resident_writes: Vec<UnpackedWriteReport>,
}

#[derive(Debug)]
pub(super) struct CompiledSystemLoaderTextEntry {
    pub id: String,
    pub original_file_offset: usize,
    pub resident_offset: u16,
    pub lines: Vec<String>,
    pub bytes: Vec<u8>,
}
