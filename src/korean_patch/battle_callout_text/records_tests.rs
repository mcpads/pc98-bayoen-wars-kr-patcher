use super::*;
use crate::game_data::{GaijiGlyphMeaning, InterfaceTextToken};
use crate::translation_drafts::{DevelopmentPolicy, DraftStatus};

fn source(tokens: Vec<InterfaceTextToken>) -> InterfaceTextEntry {
    InterfaceTextEntry {
        id: "battle-callout-001".to_owned(),
        file_offset: 0x9280,
        com_address: 0x9380,
        byte_size: 2,
        sha256: String::new(),
        raw_hex: String::new(),
        text: String::new(),
        gaiji_glyph_indices: Vec::new(),
        tokens,
    }
}

fn draft(lines: &[&str]) -> TranslationDraftEntry {
    TranslationDraftEntry {
        id: "battle-callout-001".to_owned(),
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
        character_codes: [('가', 0xebe0), ('나', 0xebe1)].into_iter().collect(),
        glyphs: Vec::new(),
        spans: Vec::new(),
        available_slot_count: 2,
        reserved_slot_indexes: Vec::new(),
    }
}

#[test]
fn callout_lines_keep_the_runtime_break_and_terminator() {
    let source = source(vec![
        InterfaceTextToken::Gaiji {
            glyph_index: 0,
            character_code: 0,
            shift_jis_code: 0,
            source: GaijiGlyphMeaning::Character { character: '가' },
        },
        InterfaceTextToken::LineBreak,
        InterfaceTextToken::Text {
            text: "x".to_owned(),
        },
    ]);
    let compiled = compile_callout_record(&source, &draft(&["가", "나"]), &bank()).unwrap();

    assert_eq!(
        compiled.bytes,
        [0xeb, 0xe0, b'$', b'0', 0xeb, 0xe1, b'$', b'$']
    );
}

#[test]
fn callout_line_count_changes_fail_at_the_consumer_boundary() {
    let source = source(vec![InterfaceTextToken::Text {
        text: "x".to_owned(),
    }]);

    assert!(compile_callout_record(&source, &draft(&["가", "나"]), &bank()).is_err());
}
