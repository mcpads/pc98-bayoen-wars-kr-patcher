use super::*;

#[test]
fn direct_dos_print_scanner_keeps_referenced_japanese_and_ignores_ascii() {
    let mut bytes = vec![0; 0x80];
    bytes[0..7].copy_from_slice(&[0xba, 0x20, 0x01, 0xb4, 0x09, 0xcd, 0x21]);
    bytes[7..14].copy_from_slice(&[0xba, 0x30, 0x01, 0xb4, 0x09, 0xcd, 0x21]);
    bytes[0x20..0x27].copy_from_slice(&[0x83, 0x65, 0x83, 0x58, 0x83, 0x67, b'$']);
    bytes[0x30..0x36].copy_from_slice(b"ERROR$");
    let mut entries = BTreeMap::new();

    add_direct_dos_prints(&bytes, &mut entries).unwrap();

    assert_eq!(entries.len(), 1);
    let entry = entries.get(&0x20).unwrap();
    assert_eq!(decode_menu_text(&entry.bytes).unwrap(), "テスト");
    assert_eq!(entry.references[0].consumer_file_offset, 0);
}

#[test]
fn duplicate_pointer_targets_preserve_each_reference() {
    let mut bytes = vec![0; 0x80];
    bytes[0..4].copy_from_slice(&[0x20, 0x01, 0x20, 0x01]);
    bytes[0x20..0x27].copy_from_slice(&[0x83, 0x65, 0x83, 0x58, 0x83, 0x67, b'$']);
    let mut entries = BTreeMap::new();

    add_pointer_table(&bytes, &mut entries, 0x0100, 2, "table", 0x40).unwrap();

    let entry = entries.get(&0x20).unwrap();
    assert_eq!(entry.references.len(), 2);
    assert_eq!(entry.references[0].table_index, Some(0));
    assert_eq!(entry.references[1].table_index, Some(1));
}

#[test]
fn pointer_outside_the_program_is_rejected() {
    let mut bytes = vec![0; 0x20];
    bytes[0..2].copy_from_slice(&0x2000_u16.to_le_bytes());

    let error = add_pointer_table(&bytes, &mut BTreeMap::new(), 0x0100, 1, "table", 0).unwrap_err();

    assert!(error.to_string().contains("outside the program"));
}
