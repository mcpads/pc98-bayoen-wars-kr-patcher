use std::collections::BTreeMap;

use serde::Serialize;

use super::atlas::CompiledAtlas;
use super::pages::CompiledPages;
use crate::korean_patch::payload_writes::PayloadWriteReport;

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DevelopmentBuildStatus {
    DevelopmentOnly,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct MonochromeGlyphSlot {
    pub index: usize,
    pub character: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MonochromeSequencePatchReport {
    pub id: String,
    pub source_asset: String,
    pub glyph_capacity: usize,
    pub used_glyph_count: usize,
    pub unused_glyph_count: usize,
    pub glyph_slots: Vec<MonochromeGlyphSlot>,
    pub decoded_atlas_sha256: String,
    pub original_packed_size: usize,
    pub replacement_packed_size: usize,
    pub replacement_packed_sha256: String,
    pub page_count: usize,
    pub page_data_size: usize,
    pub page_storage_capacity: usize,
    pub page_byte_sizes: Vec<usize>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MonochromeTextPatchReport {
    pub supported_source_sha256: String,
    pub translation_status: String,
    pub build_status: DevelopmentBuildStatus,
    pub font_profile: String,
    pub font_sha256: String,
    pub font_version: String,
    pub font_source: String,
    pub font_upstream_revision: String,
    pub sequences: Vec<MonochromeSequencePatchReport>,
    pub writes: Vec<PayloadWriteReport>,
}

pub(crate) struct PatchedMonochromeTextPayload {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: MonochromeTextPatchReport,
}

pub(super) struct SequencePatch {
    pub id: String,
    pub file_name: &'static str,
    pub atlas: CompiledAtlas,
    pub pages: CompiledPages,
    pub packed_atlas: Vec<u8>,
}
