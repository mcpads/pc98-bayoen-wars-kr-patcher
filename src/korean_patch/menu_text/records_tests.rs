use super::*;
use crate::game_data::MenuTextReference;
use crate::korean_patch::shared_text::CompiledGaijiBank;
use crate::translation_drafts::{DraftStatus, TranslationDraftEntry};

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

fn source(id: &str, body: &[u8], terminator: u8) -> MenuTextEntry {
    MenuTextEntry {
        id: id.to_owned(),
        file_offset: 0,
        com_address: 0x100,
        byte_size: body.len(),
        terminator_hex: format!("{terminator:02X}"),
        raw_sha256: sha256_hex(body),
        raw_hex: String::new(),
        text: SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(body)
            .unwrap()
            .replace('@', "\n"),
        references: Vec::<MenuTextReference>::new(),
    }
}

fn draft(id: &str, lines: &[&str]) -> TranslationDraftEntry {
    TranslationDraftEntry {
        id: id.to_owned(),
        korean_text: lines.iter().map(|line| (*line).to_owned()).collect(),
        development_policy: DevelopmentPolicy::Translate,
        status: DraftStatus::NeedsHumanReview,
        notes: None,
        questions: Vec::new(),
    }
}

#[test]
fn structured_record_preserves_crlf_bell_and_at_boundaries() {
    let body = b"ONE\r\n\x07TWO@";
    let mut program = body.to_vec();
    program.push(b'$');

    let compiled = compile_record(
        &program,
        &source("menu-text-fixture", body, b'$'),
        &draft("menu-text-fixture", &["A", "B"]),
        &bank(),
    )
    .unwrap();

    assert_eq!(compiled.bytes, b"A\r\n\x07B@$".to_vec());
}

#[test]
fn dynamic_record_preserves_commands_and_reports_both_mutable_fields() {
    let body = b"\\P\0\0\\C6DRIVE ?? \\C7";
    let mut program = body.to_vec();
    program.push(b'$');

    let compiled = compile_record(
        &program,
        &source(DYNAMIC_ENTRY_ID, body, b'$'),
        &draft(DYNAMIC_ENTRY_ID, &["D ??"]),
        &bank(),
    )
    .unwrap();

    assert_eq!(compiled.bytes, b"\\P\0\0\\C6D ?? \\C7$".to_vec());
    assert_eq!(compiled.runtime_field_offsets[POSITION_FIELD_ID], 2);
    assert_eq!(compiled.runtime_field_offsets[DRIVE_LABEL_FIELD_ID], 9);
}
