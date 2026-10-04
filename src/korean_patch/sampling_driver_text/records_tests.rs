use super::*;
use crate::external_text::{ExternalTextReference, ExternalTextTerminator};
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

fn source(role: &str, text: &str) -> ExternalTextEntry {
    ExternalTextEntry {
        id: "external-bsamp-text-fixture".to_owned(),
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

fn draft(text: &str) -> TranslationDraftEntry {
    TranslationDraftEntry {
        id: "external-bsamp-text-fixture".to_owned(),
        korean_text: vec![text.to_owned()],
        status: DraftStatus::NeedsHumanReview,
        development_policy: DevelopmentPolicy::Translate,
        notes: None,
        questions: Vec::new(),
    }
}

#[test]
fn startup_banner_keeps_its_frame_and_trailing_blank_line() {
    let compiled = compile_record(
        &source("startup_banner", "～～ source ～～\r\n\r\n"),
        &draft("한"),
        &bank(),
    )
    .unwrap();

    assert_eq!(compiled.lines, ["～～ 한 ～～"]);
    assert_eq!(
        compiled.bytes,
        [
            0x81, 0x60, 0x81, 0x60, b' ', 0xeb, 0x9f, b' ', 0x81, 0x60, 0x81, 0x60, b'\r', b'\n',
            b'\r', b'\n', b'$',
        ]
    );
}

#[test]
fn status_text_is_not_given_the_banner_frame() {
    let compiled = compile_record(
        &source("resident_status", "source\r\n"),
        &draft("한"),
        &bank(),
    )
    .unwrap();

    assert_eq!(compiled.lines, ["한"]);
    assert_eq!(compiled.bytes, [0xeb, 0x9f, b'\r', b'\n', b'$']);
}
