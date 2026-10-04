use super::*;
use crate::byte_string::encode_lower_hex;
use crate::game_data::{MadSystemTextReference, RuntimeTextInsert};
use crate::source_disk::sha256_hex;
use crate::translation_drafts::{DevelopmentPolicy, DraftStatus};

fn bank() -> CompiledGaijiBank {
    CompiledGaijiBank {
        patched_program: Vec::new(),
        appended_records: Vec::new(),
        character_codes: [('한', 0xeb9f), ('글', 0xeba0)].into_iter().collect(),
        glyphs: Vec::new(),
        spans: Vec::new(),
        available_slot_count: 0,
        reserved_slot_indexes: Vec::new(),
    }
}

fn source() -> MadSystemTextEntry {
    let bytes = b"\x1b[24;0H            .DAT source";
    MadSystemTextEntry {
        id: "mad-system-fixture".to_owned(),
        file_offset: 0x200,
        com_address: 0x300,
        byte_size: bytes.len(),
        raw_sha256: sha256_hex(bytes),
        raw_hex: encode_lower_hex(bytes),
        text: String::from_utf8(bytes.to_vec()).unwrap(),
        runtime_insert: Some(RuntimeTextInsert {
            role: "data_file_stem".to_owned(),
            byte_offset: 11,
            byte_capacity: 8,
            consumer_file_offset: 0x400,
        }),
        references: vec![MadSystemTextReference {
            role: "fixture".to_owned(),
            consumer_file_offset: 0x500,
            table_index: None,
        }],
    }
}

fn draft(text: &str) -> TranslationDraftEntry {
    TranslationDraftEntry {
        id: "mad-system-fixture".to_owned(),
        korean_text: vec![text.to_owned()],
        development_policy: DevelopmentPolicy::Translate,
        status: DraftStatus::NeedsHumanReview,
        notes: None,
        questions: Vec::new(),
    }
}

#[test]
fn runtime_file_name_field_keeps_its_exact_offset_and_suffix() {
    let compiled = compile_record(&source(), &draft(".DAT 한글"), &bank()).unwrap();

    assert_eq!(&compiled.bytes[11..19], &[b' '; 8]);
    assert_eq!(&compiled.bytes[19..23], b".DAT");
    assert_eq!(compiled.runtime_insert_byte_offset, Some(11));
    assert_eq!(compiled.runtime_insert_byte_capacity, Some(8));
    assert_eq!(compiled.bytes.last(), Some(&b'$'));
}
