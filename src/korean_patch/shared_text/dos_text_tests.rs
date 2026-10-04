use super::*;

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

#[test]
fn layout_encoder_preserves_internal_blank_lines_and_final_boundary() {
    let bank = bank();
    let lines = vec!["한".to_owned(), "글".to_owned()];

    let bytes = encode_dos_dollar_text_layout("source\r\n\r\ntext\r\n", &lines, &bank).unwrap();

    assert_eq!(
        bytes,
        [
            0xeb, 0x9f, b'\r', b'\n', b'\r', b'\n', 0xeb, 0xa0, b'\r', b'\n', b'$',
        ]
    );
}

#[test]
fn layout_encoder_preserves_a_fragment_without_final_crlf() {
    let bank = bank();
    let bytes = encode_dos_dollar_text_layout("source", &["한".to_owned()], &bank).unwrap();

    assert_eq!(bytes, [0xeb, 0x9f, b'$']);
}

#[test]
fn layout_encoder_rejects_a_non_crlf_source_boundary() {
    assert!(encode_dos_dollar_text_layout("one\ntwo", &["한".to_owned()], &bank()).is_err());
}

#[test]
fn dos_text_preserves_trailing_blank_lines_and_ascii_spaces() {
    let bytes = encode_dos_dollar_text("source\r\n\r\n", &["한 A".to_owned()], &bank()).unwrap();

    assert_eq!(
        bytes,
        [0xeb, 0x9f, b' ', b'A', b'\r', b'\n', b'\r', b'\n', b'$']
    );
}

#[test]
fn dos_text_rejects_changed_lines_or_an_internal_blank() {
    assert!(encode_dos_dollar_text("one\r\ntwo\r\n", &["한".to_owned()], &bank()).is_err());
    assert!(
        encode_dos_dollar_text(
            "one\r\n\r\ntwo\r\n",
            &["한".to_owned(), "한".to_owned()],
            &bank()
        )
        .is_err()
    );
}
