use std::collections::BTreeMap;

use super::*;
use crate::external_text::{ExternalTextReference, ExternalTextTerminator};
use crate::source_disk::sha256_hex;
use crate::translation_drafts::{DevelopmentPolicy, DraftStatus};

fn bank() -> CompiledGaijiBank {
    CompiledGaijiBank {
        patched_program: Vec::new(),
        appended_records: Vec::new(),
        character_codes: BTreeMap::new(),
        glyphs: Vec::new(),
        spans: Vec::new(),
        available_slot_count: 0,
        reserved_slot_indexes: Vec::new(),
    }
}

fn source(text: &str) -> ExternalTextEntry {
    ExternalTextEntry {
        id: "external-megdos-fixture".to_owned(),
        text_offset: 0x800,
        runtime_address: 0x800,
        terminator: ExternalTextTerminator::Null,
        byte_size: text.len(),
        raw_sha256: sha256_hex(text.as_bytes()),
        raw_hex: String::new(),
        text: text.to_owned(),
        references: vec![ExternalTextReference {
            role: "fixture".to_owned(),
            consumer_offset: 0x200,
            table_index: None,
        }],
    }
}

fn draft(text: &str) -> TranslationDraftEntry {
    TranslationDraftEntry {
        id: "external-megdos-fixture".to_owned(),
        korean_text: vec![text.to_owned()],
        development_policy: DevelopmentPolicy::Translate,
        status: DraftStatus::NeedsHumanReview,
        notes: None,
        questions: Vec::new(),
    }
}

#[test]
fn null_record_preserves_the_runtime_fragment_space() {
    let compiled = compile_record(&source("ERROR: "), &draft("FAIL:"), &bank()).unwrap();

    assert_eq!(compiled.bytes, b"FAIL: \0");
}

#[test]
fn null_record_rejects_a_changed_fragment_colon() {
    assert!(compile_record(&source("ERROR: "), &draft("FAIL"), &bank()).is_err());
}
