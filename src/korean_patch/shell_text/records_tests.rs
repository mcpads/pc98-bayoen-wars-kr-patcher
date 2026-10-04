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

fn source(text: &str) -> ExternalTextEntry {
    ExternalTextEntry {
        id: "external-dsh-text-fixture".to_owned(),
        text_offset: 10,
        runtime_address: 0x10a,
        terminator: ExternalTextTerminator::DosDollar,
        byte_size: text.len(),
        raw_sha256: String::new(),
        raw_hex: String::new(),
        text: text.to_owned(),
        references: vec![ExternalTextReference {
            role: "fixture".to_owned(),
            consumer_offset: 4,
            table_index: None,
        }],
    }
}

fn draft(lines: &[&str]) -> TranslationDraftEntry {
    TranslationDraftEntry {
        id: "external-dsh-text-fixture".to_owned(),
        korean_text: lines.iter().map(|line| (*line).to_owned()).collect(),
        status: DraftStatus::NeedsHumanReview,
        development_policy: DevelopmentPolicy::Translate,
        notes: None,
        questions: Vec::new(),
    }
}

#[test]
fn shell_record_preserves_crlf_and_ascii_spacing() {
    let compiled = compile_record(&source("source\r\n"), &draft(&["한 A"]), &bank()).unwrap();

    assert_eq!(compiled.bytes, [0xeb, 0x9f, b' ', b'A', b'\r', b'\n', b'$']);
    assert_eq!(compiled.consumer_offsets, [4]);
}

#[test]
fn shell_record_rejects_changed_line_structure_or_a_dos_terminator() {
    assert!(compile_record(&source("one\r\ntwo\r\n"), &draft(&["한"]), &bank()).is_err());
    assert!(compile_record(&source("one\r\n"), &draft(&["한$"]), &bank()).is_err());
}
