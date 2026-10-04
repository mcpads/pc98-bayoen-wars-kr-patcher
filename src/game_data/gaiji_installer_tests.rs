use super::*;

fn installer_fixture() -> Vec<u8> {
    let mut bytes = vec![0_u8; INSTALL_ROUTINE_RANGE.end];
    bytes[INSTALL_LOOP_RANGE].copy_from_slice(&[
        0xb9, 0xb8, 0x00, 0x2e, 0xc7, 0x06, 0x78, 0x01, 0x21, 0x76, 0xbb, 0x7a, 0x01, 0x2e, 0xa1,
        0x76, 0x01, 0xd1, 0xe0, 0x53, 0x03, 0xd8, 0x51, 0x2e, 0x8b, 0x0f, 0x2e, 0xff, 0x06, 0x76,
        0x01, 0x2e, 0x8b, 0x16, 0x78, 0x01, 0x81, 0xfa, 0x7e, 0x76, 0x75, 0x07, 0x2e, 0xc7, 0x06,
        0x78, 0x01, 0x20, 0x77, 0x2e, 0xff, 0x06, 0x78, 0x01, 0x53, 0xe8, 0x23, 0x00, 0x5b, 0x59,
        0x5b, 0xe2, 0xce,
    ]);
    bytes[INSTALL_ROUTINE_RANGE].copy_from_slice(&[0x8c, 0xcb, 0xb4, 0x1a, 0xcd, 0x18, 0xc3]);
    bytes
}

#[test]
fn typed_installer_exposes_the_bios_register_abi() {
    let installer = parse_gaiji_installer(&installer_fixture()).unwrap();

    assert_eq!(installer.glyph_count, 184);
    assert_eq!(installer.first_character_code, 0x7621);
    assert_eq!(installer.pointer_table_com_address, 0x017a);
    assert_eq!(installer.row_end_character_code, 0x767e);
    assert_eq!(installer.next_row_previous_character_code, 0x7720);
    assert_eq!(installer.install_call_offset, 0x49);
    assert_eq!(installer.install_routine_offset, 0x6f);
    assert_eq!(installer.bios_interrupt_vector, 0x18);
    assert_eq!(installer.bios_function, 0x1a);
    assert_eq!(installer.glyph_segment_register, "bx");
    assert_eq!(installer.glyph_record_offset_register, "cx");
    assert_eq!(installer.character_code_register, "dx");
}

#[test]
fn changed_bios_function_is_rejected() {
    let mut bytes = installer_fixture();
    bytes[BIOS_FUNCTION_OFFSET + 1] = 0x1b;

    let error = parse_gaiji_installer(&bytes).unwrap_err();

    assert!(error.to_string().contains("function"));
}

#[test]
fn relative_target_outside_the_program_is_rejected() {
    let mut bytes = installer_fixture();
    bytes[INSTALL_CALL_OFFSET + 1..INSTALL_CALL_OFFSET + 3]
        .copy_from_slice(&i16::MIN.to_le_bytes());

    let error = parse_gaiji_installer(&bytes).unwrap_err();

    assert!(error.to_string().contains("outside"));
}
