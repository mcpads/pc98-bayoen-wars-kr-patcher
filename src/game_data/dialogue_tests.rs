use super::*;

fn two_group_dialogue_fixture() -> Vec<u8> {
    let mut bytes = vec![0_u8; 0xb100];
    bytes[CONSUMER_OFFSET..CONSUMER_OFFSET + 6]
        .copy_from_slice(&[0xa1, 0x8d, 0xdb, 0xd1, 0xe0, 0xbb]);
    bytes[TABLE_INSTRUCTION_OFFSET + 1..TABLE_INSTRUCTION_OFFSET + 3]
        .copy_from_slice(&0xb100_u16.to_le_bytes());
    bytes[TABLE_INSTRUCTION_OFFSET + 3..TABLE_INSTRUCTION_OFFSET + 26].copy_from_slice(&[
        0x03, 0xd8, 0x2e, 0x8b, 0x1f, 0x2e, 0x8b, 0x07, 0x3d, 0xff, 0xff, 0x74, 0x3c, 0x2e, 0x8b,
        0x57, 0x02, 0x83, 0xc3, 0x04, 0x3d, 0x00, 0x00,
    ]);

    bytes[0xb000..0xb004].copy_from_slice(&[0x04, 0xb1, 0x0a, 0xb1]);
    bytes[0xb004..0xb00a].copy_from_slice(&[0x00, 0x00, 0x20, 0xb1, 0xff, 0xff]);
    bytes[0xb00a..0xb010].copy_from_slice(&[0x01, 0x00, 0x30, 0xb1, 0xff, 0xff]);
    bytes[0xb020..0xb023].copy_from_slice(b"A$$");
    bytes[0xb030..0xb033].copy_from_slice(b"B$$");
    bytes
}

#[test]
fn two_byte_group_terminator_allows_the_next_group_to_follow_immediately() {
    let catalog = parse_dialogue_structure(&two_group_dialogue_fixture()).unwrap();

    assert_eq!(catalog.group_count, 2);
    assert_eq!(catalog.entry_count, 2);
    assert_eq!(catalog.groups[0].record_table_offset, 0xb004);
    assert_eq!(catalog.groups[1].record_table_offset, 0xb00a);
}

#[test]
fn shift_jis_text_preserves_lines_and_requires_a_terminator() {
    let bytes = b"\x82\xa0\x82\xa2$0\x83\x41$$";

    let parsed = parse_dialogue_text_record(bytes, 0).unwrap();

    assert_eq!(parsed.end_offset, bytes.len());
    assert_eq!(parsed.lines, ["あい", "ア"]);
}

#[test]
fn unknown_text_controls_are_rejected() {
    let error = parse_dialogue_text_record(b"text$x$$", 0).unwrap_err();

    assert!(error.to_string().contains("unsupported text control $x"));
}

#[test]
fn control_parser_accepts_unmapped_gaiji_cells_without_decoding_them() {
    let parsed = parse_dialogue_text_controls(&[0xec, 0x4a, b'$', b'$'], 0).unwrap();

    assert_eq!(parsed.end_offset, 4);
    assert_eq!(parsed.line_count, 1);
}
