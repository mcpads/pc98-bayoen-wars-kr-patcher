use super::*;
use crate::korean_patch::shared_text::model::GaijiBankSpan;

fn bank() -> CompiledGaijiBank {
    CompiledGaijiBank {
        patched_program: Vec::new(),
        appended_records: Vec::new(),
        character_codes: [('한', 0xeb9f), ('~', 0xeba0)].into_iter().collect(),
        glyphs: Vec::new(),
        spans: Vec::<GaijiBankSpan>::new(),
        available_slot_count: 0,
        reserved_slot_indexes: Vec::new(),
    }
}

#[test]
fn dos_encoding_keeps_ascii_spacing_while_interface_spacing_stays_full_width() {
    let bank = bank();

    assert_eq!(encode_dos_character(' ', &bank).unwrap(), [b' ']);
    assert_eq!(encode_shared_character(' ', &bank).unwrap(), [0x81, 0x40]);
}

#[test]
fn shared_character_encoder_distinguishes_native_and_gaiji_cells() {
    let bank = bank();

    assert_eq!(encode_shared_character('A', &bank).unwrap(), [b'A']);
    assert_eq!(encode_shared_character(' ', &bank).unwrap(), [0x81, 0x40]);
    assert_eq!(encode_shared_character('한', &bank).unwrap(), [0xeb, 0x9f]);
    assert_eq!(encode_shared_character('~', &bank).unwrap(), [0xeb, 0xa0]);
}

#[test]
fn shared_character_encoder_rejects_unassigned_or_unsupported_whitespace() {
    let bank = bank();

    assert!(encode_shared_character('글', &bank).is_err());
    assert!(encode_shared_character('\t', &bank).is_err());
}
