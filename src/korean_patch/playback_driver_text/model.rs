use std::collections::BTreeMap;

use serde::Serialize;
use v30::AssembledProgram;

use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::{PayloadWriteReport, UnpackedWriteReport};
use super::super::shared_text::{CompiledGaijiBank, SharedGaijiGlyph};
use crate::external_text::PackedSoundDriverRuntimeCatalog;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct PlaybackDriverTextEntryPatchReport {
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
pub struct PlaybackDriverPatchReport {
    pub file_name: String,
    pub source_packed_size: usize,
    pub output_packed_size: usize,
    pub source_unpacked_size: usize,
    pub output_unpacked_size: usize,
    pub resident_end_com_address: u16,
    pub first_nonresident_file_offset: usize,
    pub available_gaiji_slots: usize,
    pub used_gaiji_slots: usize,
    pub installer_code_bytes: usize,
    pub glyph_record_bytes: usize,
    pub text_storage_capacity: usize,
    pub text_storage_used: usize,
    pub reference_count: usize,
    pub machine_reference_count: usize,
    pub metadata_reference_count: usize,
    pub glyphs: Vec<SharedGaijiGlyph>,
    pub entries: Vec<PlaybackDriverTextEntryPatchReport>,
    pub unpacked_writes: Vec<UnpackedWriteReport>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct PlaybackDriverTextPatchReport {
    pub supported_source_sha256: String,
    pub translation_status: String,
    pub build_status: DevelopmentBuildStatus,
    pub font_profile: String,
    pub font_sha256: String,
    pub drivers: Vec<PlaybackDriverPatchReport>,
    pub writes: Vec<PayloadWriteReport>,
}

pub(crate) struct PatchedPlaybackDriverTextFiles {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: PlaybackDriverTextPatchReport,
}

pub(super) struct CompiledPlaybackDriver {
    pub file_name: &'static str,
    pub bank: CompiledGaijiBank,
    pub runtime: PackedSoundDriverRuntimeCatalog,
    pub source_packed_size: usize,
    pub source_unpacked: Vec<u8>,
    pub output_unpacked: Vec<u8>,
    pub output_packed: Vec<u8>,
    pub installer_offset: usize,
    pub installer: AssembledProgram,
    pub glyph_records_offset: usize,
    pub glyph_records: Vec<u8>,
    pub text_storage_writes: Vec<TextStorageWrite>,
    pub text_storage_capacity: usize,
    pub text_storage_used: usize,
    pub entries: Vec<CompiledPlaybackDriverTextEntry>,
    pub typed_sources: BTreeMap<String, AssembledProgram>,
    pub unpacked_writes: Vec<UnpackedWriteReport>,
}

#[derive(Debug)]
pub(super) struct CompiledPlaybackDriverTextEntry {
    pub id: String,
    pub source_order: usize,
    pub original_file_offset: usize,
    pub file_offset: usize,
    pub com_address: u16,
    pub lines: Vec<String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub(super) struct TextStorageWrite {
    pub id: String,
    pub offset: usize,
    pub expected_original: Vec<u8>,
    pub replacement: Vec<u8>,
}
