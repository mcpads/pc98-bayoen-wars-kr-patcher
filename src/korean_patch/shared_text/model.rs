use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct SharedGaijiGlyph {
    pub character: String,
    pub slot_index: usize,
    pub shift_jis_code: String,
    pub record_offset: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct CompiledGaijiBank {
    pub patched_program: Vec<u8>,
    pub appended_records: Vec<u8>,
    pub character_codes: BTreeMap<char, u16>,
    pub glyphs: Vec<SharedGaijiGlyph>,
    pub spans: Vec<GaijiBankSpan>,
    pub available_slot_count: usize,
    pub reserved_slot_indexes: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct GaijiBankSpan {
    pub first_slot_index: usize,
    pub offset: usize,
    pub expected_original: Vec<u8>,
    pub replacement: Vec<u8>,
}
