use super::*;
use crate::external_text::{ExternalTextReference, ExternalTextTerminator};
use crate::translation_drafts::{DevelopmentPolicy, DraftStatus};

fn bank() -> CompiledGaijiBank {
    CompiledGaijiBank {
        patched_program: Vec::new(),
        appended_records: Vec::new(),
        character_codes: [('한', 0xeb9f), ('글', 0xeba0), ('값', 0xeba1)]
            .into_iter()
            .collect(),
        glyphs: Vec::new(),
        spans: Vec::new(),
        available_slot_count: 0,
        reserved_slot_indexes: Vec::new(),
    }
}

fn source(role: &str, text: &str) -> ExternalTextEntry {
    ExternalTextEntry {
        id: "external-nmouse-text-fixture".to_owned(),
        text_offset: 10,
        runtime_address: 0x10a,
        terminator: ExternalTextTerminator::DosDollar,
        byte_size: text.len(),
        raw_sha256: String::new(),
        raw_hex: String::new(),
        text: text.to_owned(),
        references: vec![ExternalTextReference {
            role: role.to_owned(),
            consumer_offset: 4,
            table_index: None,
        }],
    }
}

fn draft(lines: &[&str]) -> TranslationDraftEntry {
    TranslationDraftEntry {
        id: "external-nmouse-text-fixture".to_owned(),
        korean_text: lines.iter().map(|line| (*line).to_owned()).collect(),
        status: DraftStatus::NeedsHumanReview,
        development_policy: DevelopmentPolicy::Translate,
        notes: None,
        questions: Vec::new(),
    }
}

#[test]
fn startup_banner_keeps_frame_indentation_and_fragment_boundary() {
    let compiled = compile_record(
        &source(
            "startup_banner",
            "∮∮  source  ∮∮\r\n    body\r\n      label：",
        ),
        &draft(&["한", "글", "값"]),
        &bank(),
    )
    .unwrap();

    assert_eq!(compiled.lines, ["∮∮  한  ∮∮", "    글", "      값"]);
    assert_eq!(compiled.bytes.last(), Some(&b'$'));
    assert_eq!(
        compiled
            .bytes
            .windows(2)
            .filter(|bytes| *bytes == b"\r\n")
            .count(),
        2
    );
}

#[test]
fn help_message_keeps_internal_blank_lines_and_both_frames() {
    let compiled = compile_record(
        &source(
            "help_message",
            "∞∞  source  ∞∞\r\n\r\n  body\r\n\r\n∞  current",
        ),
        &draft(&["한", "글", "값"]),
        &bank(),
    )
    .unwrap();

    assert_eq!(compiled.lines, ["∞∞  한  ∞∞", "  글", "∞  값"]);
    assert_eq!(
        compiled
            .bytes
            .windows(2)
            .filter(|bytes| *bytes == b"\r\n")
            .count(),
        4
    );
}
