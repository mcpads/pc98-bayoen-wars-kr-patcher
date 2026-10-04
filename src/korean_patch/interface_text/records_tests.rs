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

fn draft(lines: &[&str]) -> TranslationDraftEntry {
    TranslationDraftEntry {
        id: "interface-text-fixture".to_owned(),
        korean_text: lines.iter().map(|line| (*line).to_owned()).collect(),
        status: DraftStatus::NeedsHumanReview,
        development_policy: DevelopmentPolicy::Translate,
        notes: None,
        questions: Vec::new(),
    }
}

fn source(tokens: Vec<InterfaceTextToken>) -> InterfaceTextEntry {
    InterfaceTextEntry {
        id: "interface-text-fixture".to_owned(),
        file_offset: 0x100,
        com_address: 0x200,
        byte_size: 0,
        sha256: String::new(),
        raw_hex: String::new(),
        text: String::new(),
        gaiji_glyph_indices: Vec::new(),
        tokens,
    }
}

#[test]
fn interface_record_preserves_outer_attributes_and_line_breaks() {
    let source_record = source(vec![
        InterfaceTextToken::DisplayAttribute { code: 4 },
        InterfaceTextToken::Text {
            text: "source".to_owned(),
        },
        InterfaceTextToken::LineBreak,
        InterfaceTextToken::Text {
            text: "source".to_owned(),
        },
        InterfaceTextToken::DisplayAttribute { code: 8 },
    ]);

    let compiled = compile_record(&source_record, &draft(&["한 A", "한"]), &bank()).unwrap();

    assert_eq!(compiled.leading_attributes, [4]);
    assert_eq!(compiled.trailing_attributes, [8]);
    assert_eq!(compiled.line_screen_byte_widths, [5, 2]);
    assert_eq!(
        compiled.bytes,
        [
            b'$', b'4', 0xeb, 0x9f, 0x81, 0x40, b'A', b'$', b'0', 0xeb, 0x9f, b'$', b'8', b'$',
            b'$'
        ]
    );
}

#[test]
fn interface_record_rejects_a_line_count_or_embedded_attribute_change() {
    let source_record = source(vec![
        InterfaceTextToken::Text {
            text: "source".to_owned(),
        },
        InterfaceTextToken::DisplayAttribute { code: 3 },
        InterfaceTextToken::Text {
            text: "tail".to_owned(),
        },
    ]);

    assert!(compile_record(&source_record, &draft(&["한"]), &bank()).is_err());

    let one_line = source(vec![InterfaceTextToken::Text {
        text: "source".to_owned(),
    }]);
    assert!(compile_record(&one_line, &draft(&["한", "한"]), &bank()).is_err());
}
