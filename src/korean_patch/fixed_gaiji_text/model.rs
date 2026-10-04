use std::collections::BTreeMap;

use serde::Serialize;

use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::PayloadWriteReport;
use super::super::shared_text::{CompiledGaijiBank, SharedGaijiGlyph};

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct FixedGaijiTextSlotPatchReport {
    pub id: String,
    pub file_offset: usize,
    pub visible_line_count: usize,
    pub occupied_cell_count: usize,
    pub content_sha256: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct FixedGaijiTextPatchReport {
    pub supported_source_sha256: String,
    pub translation_status: String,
    pub build_status: DevelopmentBuildStatus,
    pub font_profile: String,
    pub font_sha256: String,
    pub available_gaiji_slots: usize,
    pub used_gaiji_slots: usize,
    pub glyphs: Vec<SharedGaijiGlyph>,
    pub slots: Vec<FixedGaijiTextSlotPatchReport>,
    pub writes: Vec<PayloadWriteReport>,
}

pub(crate) struct PatchedFixedGaijiTextPayload {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: FixedGaijiTextPatchReport,
}

pub(in crate::korean_patch) struct CompiledFixedGaijiText {
    pub bank: CompiledGaijiBank,
    pub slots: Vec<CompiledFixedGaijiSlot>,
}

#[derive(Debug)]
pub(in crate::korean_patch) struct CompiledFixedGaijiSlot {
    pub id: String,
    pub file_offset: usize,
    pub lines: Vec<String>,
    pub bytes: Vec<u8>,
}
