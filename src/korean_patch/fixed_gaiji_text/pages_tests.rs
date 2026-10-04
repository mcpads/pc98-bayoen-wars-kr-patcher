use std::collections::BTreeMap;

use super::*;
use crate::korean_patch::shared_text::CompiledGaijiBank;
use crate::korean_patch::shared_text::SharedGaijiGlyph;
use crate::translation_drafts::{DevelopmentPolicy, DraftStatus};

fn source() -> FixedGaijiTextSlot {
    FixedGaijiTextSlot {
        id: "fixed-gaiji-text-01".to_owned(),
        pointer_instruction_offset: 0,
        file_offset: 10,
        byte_size: SLOT_SIZE,
        sha256: String::new(),
        raw_hex: String::new(),
        text: String::new(),
        lines: Vec::new(),
        used_glyph_indices: Vec::new(),
    }
}

fn draft(lines: &[&str]) -> TranslationDraftEntry {
    TranslationDraftEntry {
        id: "fixed-gaiji-text-01".to_owned(),
        korean_text: lines.iter().map(|line| (*line).to_owned()).collect(),
        development_policy: DevelopmentPolicy::Translate,
        status: DraftStatus::NeedsHumanReview,
        notes: None,
        questions: Vec::new(),
    }
}

fn bank() -> CompiledGaijiBank {
    CompiledGaijiBank {
        patched_program: Vec::new(),
        appended_records: Vec::new(),
        character_codes: BTreeMap::from([('가', 0xeb9f)]),
        glyphs: Vec::<SharedGaijiGlyph>::new(),
        spans: Vec::new(),
        available_slot_count: 0,
        reserved_slot_indexes: Vec::new(),
    }
}

#[test]
fn fixed_page_pads_every_line_and_terminates_at_the_slot_boundary() {
    let compiled = compile_slot(&source(), &draft(&["가 가"]), &bank()).unwrap();

    assert_eq!(compiled.bytes.len(), SLOT_SIZE);
    assert_eq!(&compiled.bytes[..6], &[0xeb, 0x9f, 0x81, 0x40, 0xeb, 0x9f]);
    assert_eq!(&compiled.bytes[14..16], b"$0");
    assert_eq!(&compiled.bytes[SLOT_SIZE - 2..], b"$$");
}

#[test]
fn fixed_page_rejects_a_line_wider_than_seven_cells() {
    let error = compile_slot(&source(), &draft(&["가가가가가가가가"]), &bank()).unwrap_err();

    assert!(error.to_string().contains("only 7 are available"));
}

#[test]
fn fixed_page_uses_two_byte_native_punctuation_cells() {
    let compiled = compile_slot(&source(), &draft(&["가!?"]), &bank()).unwrap();

    assert_eq!(&compiled.bytes[..2], &[0xeb, 0x9f]);
    assert_eq!(fixed_shift_jis_cell('!').unwrap(), Some([0x81, 0x49]));
    assert_eq!(fixed_shift_jis_cell('?').unwrap(), Some([0x81, 0x48]));
}
