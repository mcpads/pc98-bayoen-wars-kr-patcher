use super::{catalog_mouse_driver, catalog_mouse_driver_runtime};

#[test]
fn startup_consumer_mismatch_fails_closed() {
    assert!(catalog_mouse_driver(&vec![0; 0x1000]).is_err());
}

#[test]
fn mouse_driver_lifetime_keeps_the_file_tail_outside_resident_memory() {
    let mut bytes = vec![0_u8; 0x0e15];
    bytes[..4].copy_from_slice(&[0xfa, 0xe9, 0x4a, 0x08]);
    bytes[0x096c..0x0974].copy_from_slice(&[0xba, 0x96, 0x00, 0xb8, 0x00, 0x31, 0xcd, 0x21]);

    let runtime = catalog_mouse_driver_runtime(&bytes).unwrap();

    assert_eq!(runtime.entry_jump_offset, 1);
    assert_eq!(runtime.transient_entry_com_address, 0x094e);
    assert_eq!(runtime.resident_paragraph_count, 0x0096);
    assert_eq!(runtime.frequency_pointer_table_offset, 0x0a6e);
    assert_eq!(runtime.frequency_pointer_count, 4);
    assert_eq!(runtime.command_mode_pointer_table_offset, 0x0a76);
    assert_eq!(runtime.command_mode_pointer_count, 2);
    assert!(
        0x100 + bytes.len() > usize::from(runtime.resident_paragraph_count) * 16,
        "the fixture must contain a non-resident file tail"
    );

    bytes[0x096c] = 0xbb;
    assert!(catalog_mouse_driver_runtime(&bytes).is_err());
}
