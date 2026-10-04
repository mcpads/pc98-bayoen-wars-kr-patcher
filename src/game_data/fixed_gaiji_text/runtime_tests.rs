use super::*;

fn fixture() -> Vec<u8> {
    let mut bytes = vec![0; FIXED_TEXT_RENDER_CALL_FILE_OFFSET + 3];
    bytes[SELECTION_MODE_LOAD_FILE_OFFSET..SELECTION_ENTRY_CALL_FILE_OFFSET]
        .copy_from_slice(&[0xb0, 0x03]);
    bytes[SELECTION_ENTRY_CALL_FILE_OFFSET..SELECTION_ENTRY_CALL_FILE_OFFSET + 3]
        .copy_from_slice(&[0xe8, 0x2f, 0x8a]);
    bytes[FIXED_TEXT_RENDER_CALL_FILE_OFFSET..FIXED_TEXT_RENDER_CALL_FILE_OFFSET + 3]
        .copy_from_slice(&[0xe8, 0xce, 0x95]);
    bytes
}

#[test]
fn selection_entry_and_fixed_renderer_calls_are_typed() {
    let runtime = catalog_fixed_gaiji_text_runtime(&fixture()).unwrap();

    assert_eq!(runtime.selection_entry_call.target_com_address, 0x2b74);
    assert_eq!(runtime.render_call.target_com_address, 0x3b48);
    assert_eq!(runtime.renderer_file_offset, 0x3a48);
}

#[test]
fn selection_mode_precondition_is_fail_closed() {
    let mut bytes = fixture();
    bytes[SELECTION_MODE_LOAD_FILE_OFFSET + 1] = 2;

    assert!(catalog_fixed_gaiji_text_runtime(&bytes).is_err());
}
