use std::collections::BTreeMap;

use serde::Serialize;
use v30::AssembledProgram;

use super::super::shared_text::{CompiledGaijiBank, SharedGaijiGlyph};

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BattleCalloutEntryPatchReport {
    pub id: String,
    pub original_file_offset: usize,
    pub file_offset: usize,
    pub com_address: u16,
    pub byte_size: usize,
    pub line_count: usize,
    pub pointer_count: usize,
    pub content_sha256: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BattleCalloutPatchReport {
    pub available_gaiji_slots: usize,
    pub used_gaiji_slots: usize,
    pub storage_capacity: usize,
    pub packed_storage_bytes: usize,
    pub storage_headroom: usize,
    pub entry_count: usize,
    pub pointer_count: usize,
    pub call_site_count: usize,
    pub bank_file_offset: usize,
    pub bank_byte_size: usize,
    pub wrapper_file_offset: usize,
    pub wrapper_byte_size: usize,
    pub wrapper_storage_capacity: usize,
    pub glyphs: Vec<SharedGaijiGlyph>,
    pub entries: Vec<BattleCalloutEntryPatchReport>,
}

pub(in crate::korean_patch) struct CompiledBattleCalloutText {
    pub bank: CompiledGaijiBank,
    pub pointer_table_offset: usize,
    pub pointer_table_replacement: Vec<u8>,
    pub text_region_start: usize,
    pub text_region_end: usize,
    pub packed_storage_bytes: usize,
    pub text_region_replacement: Vec<u8>,
    pub entries: Vec<CompiledBattleCalloutEntry>,
}

pub(in crate::korean_patch) struct CompiledBattleCalloutEntry {
    pub id: String,
    pub original_file_offset: usize,
    pub file_offset: usize,
    pub com_address: u16,
    pub lines: Vec<String>,
    pub bytes: Vec<u8>,
}

pub(in crate::korean_patch) struct CompiledBattleCalloutHook {
    pub wrapper_offset: usize,
    pub wrapper_capacity: usize,
    pub wrapper: AssembledProgram,
    pub call_sources: BTreeMap<String, AssembledProgram>,
}
