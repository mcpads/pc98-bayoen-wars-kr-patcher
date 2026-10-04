use encoding_rs::SHIFT_JIS;

use super::encode_ansi_dos_dollar_text;
use crate::korean_patch::shared_text::CompiledGaijiBank;

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

fn shift_jis(text: &str) -> Vec<u8> {
    let (encoded, _, errors) = SHIFT_JIS.encode(text);
    assert!(!errors);
    encoded.into_owned()
}

#[test]
fn ansi_crlf_decoration_and_indentation_are_preserved_exactly() {
    let source = "\u{1b}[33m\r\n ～～～～\u{1b}[32m\r\n　　source\u{1b}[37m\r\n";
    let source_bytes = shift_jis(source);

    let encoded =
        encode_ansi_dos_dollar_text(source, &source_bytes, &["한 글".to_owned()], &bank()).unwrap();

    let mut expected = shift_jis("\u{1b}[33m\r\n ～～～～\u{1b}[32m\r\n　　");
    expected.extend_from_slice(&[0xeb, 0x9f, b' ', 0xeb, 0xa0]);
    expected.extend_from_slice(b"\x1b[37m\r\n$");
    assert_eq!(encoded, expected);
}

#[test]
fn control_only_entry_is_preserved_without_a_draft_line() {
    let source = "\u{1b}[37m ";
    let source_bytes = source.as_bytes();
    let encoded = encode_ansi_dos_dollar_text(source, source_bytes, &[], &bank()).unwrap();

    assert_eq!(encoded, b"\x1b[37m $");
}

#[test]
fn line_population_and_protected_source_are_fail_closed() {
    let source = "\u{1b}[36m\r\nsource\r\n";
    let source_bytes = source.as_bytes();

    assert!(encode_ansi_dos_dollar_text(source, source_bytes, &[], &bank()).is_err());
    assert!(
        encode_ansi_dos_dollar_text(
            source,
            source_bytes,
            &["한".to_owned(), "글".to_owned()],
            &bank()
        )
        .is_err()
    );
    assert!(
        encode_ansi_dos_dollar_text(source, b"\x1b[36m\nsource\n", &["한".to_owned()], &bank())
            .is_err()
    );
    assert!(
        encode_ansi_dos_dollar_text(source, source_bytes, &["한\n글".to_owned()], &bank()).is_err()
    );
}
