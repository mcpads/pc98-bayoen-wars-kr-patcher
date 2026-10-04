use super::*;

fn gaiji_fixture() -> Vec<u8> {
    let mut bytes = vec![0_u8; 0x1a5a];
    bytes[0x12..0x51].copy_from_slice(&[
        0xb9, 0xb8, 0x00, 0x2e, 0xc7, 0x06, 0x78, 0x01, 0x21, 0x76, 0xbb, 0x7a, 0x01, 0x2e, 0xa1,
        0x76, 0x01, 0xd1, 0xe0, 0x53, 0x03, 0xd8, 0x51, 0x2e, 0x8b, 0x0f, 0x2e, 0xff, 0x06, 0x76,
        0x01, 0x2e, 0x8b, 0x16, 0x78, 0x01, 0x81, 0xfa, 0x7e, 0x76, 0x75, 0x07, 0x2e, 0xc7, 0x06,
        0x78, 0x01, 0x20, 0x77, 0x2e, 0xff, 0x06, 0x78, 0x01, 0x53, 0xe8, 0x23, 0x00, 0x5b, 0x59,
        0x5b, 0xe2, 0xce,
    ]);
    bytes[0x6f..0x76].copy_from_slice(&[0x8c, 0xcb, 0xb4, 0x1a, 0xcd, 0x18, 0xc3]);
    for index in 0..184 {
        let file_offset = 0x1ea + index * GAIJI_RECORD_SIZE;
        let com_address = (file_offset + 0x100) as u16;
        let pointer_offset = 0x7a + index * 2;
        bytes[pointer_offset..pointer_offset + 2].copy_from_slice(&com_address.to_le_bytes());
    }
    bytes[DEMONSTRATION_GAIJI_RECORD_OFFSET..DEMONSTRATION_GAIJI_RECORD_OFFSET + GAIJI_RECORD_SIZE]
        .copy_from_slice(&EXPECTED_GAIJI_RECORD);
    bytes
}

fn payload_fixture() -> BTreeMap<String, Vec<u8>> {
    let mut mad = vec![0_u8; DEMONSTRATION_DIALOGUE_OFFSET + 2];
    mad[DEMONSTRATION_INTERFACE_OFFSET..DEMONSTRATION_INTERFACE_OFFSET + 2]
        .copy_from_slice(&EXPECTED_INTERFACE_CHARACTER);
    mad[DEMONSTRATION_DIALOGUE_OFFSET..].copy_from_slice(&EXPECTED_DIALOGUE_CHARACTER);
    BTreeMap::from([
        (GAIJI_FILE.to_owned(), gaiji_fixture()),
        (DIALOGUE_FILE.to_owned(), mad),
    ])
}

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn visibility_payload_changes_only_the_declared_record_and_character() {
    let original = payload_fixture();

    let patched =
        build_hangul_visibility_payload(&original, &BTreeMap::new(), &BTreeMap::new()).unwrap();

    assert_eq!(patched.report.character, "가");
    assert_eq!(patched.report.gaiji_slot, 104);
    assert_eq!(patched.report.writes.len(), 3);
    let mut expected_mad = original[DIALOGUE_FILE].clone();
    expected_mad[DEMONSTRATION_INTERFACE_OFFSET..DEMONSTRATION_INTERFACE_OFFSET + 2]
        .copy_from_slice(&[0xec, 0x4a]);
    expected_mad[DEMONSTRATION_DIALOGUE_OFFSET..DEMONSTRATION_DIALOGUE_OFFSET + 2]
        .copy_from_slice(&[0xec, 0x4a]);
    assert_eq!(patched.files[DIALOGUE_FILE], expected_mad);
    assert_eq!(
        &patched.files[DIALOGUE_FILE]
            [DEMONSTRATION_DIALOGUE_OFFSET..DEMONSTRATION_DIALOGUE_OFFSET + 2],
        &[0xec, 0x4a]
    );
    let mut expected_gaiji = original[GAIJI_FILE].clone();
    expected_gaiji
        [DEMONSTRATION_GAIJI_RECORD_OFFSET..DEMONSTRATION_GAIJI_RECORD_OFFSET + GAIJI_RECORD_SIZE]
        .copy_from_slice(
            &patched.files[GAIJI_FILE][DEMONSTRATION_GAIJI_RECORD_OFFSET
                ..DEMONSTRATION_GAIJI_RECORD_OFFSET + GAIJI_RECORD_SIZE],
        );
    assert_eq!(patched.files[GAIJI_FILE], expected_gaiji);
}

#[test]
fn a_new_consumer_reference_blocks_the_configured_slot() {
    let mut original = payload_fixture();
    original.insert("MENU.COM".to_owned(), vec![0xec, 0x4a]);

    let error =
        build_hangul_visibility_payload(&original, &BTreeMap::new(), &BTreeMap::new()).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("already present in installer payload MENU.COM")
    );
}

#[test]
fn changed_gaiji_preimage_is_rejected() {
    let mut original = payload_fixture();
    original.get_mut(GAIJI_FILE).unwrap()[DEMONSTRATION_GAIJI_RECORD_OFFSET + 2] ^= 0xff;

    let error =
        build_hangul_visibility_payload(&original, &BTreeMap::new(), &BTreeMap::new()).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("differs from the supported source preimage")
    );
}
