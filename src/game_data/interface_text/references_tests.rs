use super::*;

#[test]
fn machine_reference_scanner_only_accepts_register_immediates_to_known_entries() {
    let entries = [InterfaceTextEntry {
        id: "interface-text-fixture".to_owned(),
        file_offset: 0,
        com_address: 0x1234,
        byte_size: 2,
        sha256: String::new(),
        raw_hex: String::new(),
        text: String::new(),
        gaiji_glyph_indices: Vec::new(),
        tokens: Vec::new(),
    }];
    let entries_by_address = BTreeMap::from([(0x1234, &entries[0])]);
    let program = [0xbb, 0x34, 0x12, 0x90, 0xba, 0x34, 0x12, 0x34, 0x12];

    assert_eq!(
        scan_machine_code_immediates(&program, &entries_by_address),
        BTreeSet::from([0, 4])
    );
}

#[test]
fn typed_machine_reference_rejects_an_immediate_in_the_wrong_instruction_form() {
    let entry = InterfaceTextEntry {
        id: "interface-text-fixture".to_owned(),
        file_offset: 0,
        com_address: 0x1234,
        byte_size: 2,
        sha256: String::new(),
        raw_hex: String::new(),
        text: String::new(),
        gaiji_glyph_indices: Vec::new(),
        tokens: Vec::new(),
    };
    let entries_by_address = BTreeMap::from([(0x1234, &entry)]);

    let error = machine_code_reference(&[0x68, 0x34, 0x12], 0, &entries_by_address).unwrap_err();

    assert!(error.to_string().contains("not MOV reg16, imm16"));
}
