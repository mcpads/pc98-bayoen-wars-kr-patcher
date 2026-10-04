use super::*;

#[test]
fn duplicate_targets_keep_every_consumer_reference() {
    let mut bytes = vec![0; 0x40];
    bytes[0x20..0x27].copy_from_slice(&[0x83, 0x65, 0x83, 0x58, 0x83, 0x67, b'$']);
    let mut entries = BTreeMap::new();

    for table_index in 0..2 {
        add_reference(
            &bytes,
            &mut entries,
            0x0120,
            MadSystemTextReference {
                role: "test_table".to_owned(),
                consumer_file_offset: 0,
                table_index: Some(table_index),
            },
        )
        .unwrap();
    }

    let entry = entries.get(&0x20).unwrap();
    assert_eq!(decode_system_text(&entry.bytes, 0x20).unwrap(), "テスト");
    assert_eq!(entry.references.len(), 2);
    assert_eq!(entry.references[1].table_index, Some(1));
}

#[test]
fn runtime_file_name_field_requires_the_verified_capacity_and_suffix() {
    let mut text = b"12345678901        .DAT suffix".to_vec();

    let insert = validate_dynamic_file_name_insert(&text).unwrap();

    assert_eq!(insert.byte_offset, 11);
    assert_eq!(insert.byte_capacity, 8);
    text[18] = b'X';
    assert!(
        validate_dynamic_file_name_insert(&text)
            .unwrap_err()
            .to_string()
            .contains("8-byte runtime insertion field")
    );
}

#[test]
fn unterminated_dos_text_is_rejected() {
    let error = read_dos_text(b"ERROR", 0).unwrap_err();

    assert!(error.to_string().contains("unterminated MAD system text"));
}
