use super::*;
use crate::translation_drafts::{DevelopmentPolicy, DraftStatus};

fn bank() -> CompiledGaijiBank {
    CompiledGaijiBank {
        patched_program: Vec::new(),
        appended_records: Vec::new(),
        character_codes: [('한', 0xeb9f)].into_iter().collect(),
        glyphs: Vec::new(),
        spans: Vec::new(),
        available_slot_count: 0,
        reserved_slot_indexes: Vec::new(),
    }
}

fn source() -> DialogueEntry {
    DialogueEntry {
        id: "dialogue-g01-r01".to_owned(),
        presentation_variant: 1,
        file_offset: 0x200,
        byte_size: 0,
        sha256: String::new(),
        raw_hex: String::new(),
        text: String::new(),
        lines: vec!["source".to_owned(), "source".to_owned()],
    }
}

fn draft(lines: &[&str]) -> TranslationDraftEntry {
    TranslationDraftEntry {
        id: "dialogue-g01-r01".to_owned(),
        korean_text: lines.iter().map(|line| (*line).to_owned()).collect(),
        development_policy: DevelopmentPolicy::Translate,
        status: DraftStatus::NeedsHumanReview,
        notes: None,
        questions: Vec::new(),
    }
}

#[test]
fn dialogue_record_encodes_lines_and_carries_presentation_variant() {
    let compiled = compile_dialogue_record(
        "dialogue-group-01",
        0x102,
        &source(),
        &draft(&["한 A", "한"]),
        &bank(),
    )
    .unwrap();

    assert_eq!(compiled.presentation_variant, 1);
    assert_eq!(compiled.text_pointer_offset, 0x102);
    assert_eq!(
        compiled.bytes,
        [
            0xeb, 0x9f, 0x81, 0x40, b'A', b'$', b'0', 0xeb, 0x9f, b'$', b'$'
        ]
    );
}

#[test]
fn dialogue_record_encodes_explicit_reflow_and_rejects_reserved_controls() {
    let reflowed = compile_dialogue_record(
        "dialogue-group-01",
        0x102,
        &source(),
        &draft(&["한", "한", "한"]),
        &bank(),
    )
    .unwrap();
    assert_eq!(reflowed.lines, ["한", "한", "한"]);

    assert!(
        compile_dialogue_record(
            "dialogue-group-01",
            0x102,
            &source(),
            &draft(&["한$", "한"]),
            &bank()
        )
        .is_err()
    );
}
