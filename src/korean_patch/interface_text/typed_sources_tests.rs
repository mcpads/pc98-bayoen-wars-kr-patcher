use super::*;
use crate::game_data::InterfaceTextReferenceKind;
use v30::CodeLocation;

#[test]
fn relocated_reference_is_assembled_as_the_original_register_form() {
    let reference = InterfaceTextReference {
        id: "fixture".to_owned(),
        storage_kind: InterfaceTextReferenceKind::MachineCodeImmediate,
        storage_offset: 1,
        instruction_offset: Some(0),
        target_com_address: 0x1234,
        target_entry_id: "entry".to_owned(),
        consumer_role: "fixture".to_owned(),
    };

    let program = assemble_reference(&[0xba, 0x34, 0x12], &reference, 0x5678).unwrap();

    assert_eq!(program.bytes(), [0xba, 0x78, 0x56]);
    assert_eq!(program.origin(), CodeLocation { seg: 0, off: 0x100 });
}
