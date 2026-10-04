use super::*;
use crate::game_data::MadSystemRuntimeReferenceKind;

#[test]
fn assembly_source_identity_uses_the_owned_immediate_offset() {
    let reference = MadSystemRuntimeReference {
        id: "fixture".to_owned(),
        target_entry_id: "entry".to_owned(),
        consumer_role: "runtime insert".to_owned(),
        kind: MadSystemRuntimeReferenceKind::RuntimeInsertMachineCode,
        instruction_offset: Some(0x30dd),
        storage_offset: 0x30de,
        original_com_address: 0x2e59,
    };

    assert_eq!(
        assembly_source_id(&reference),
        "mad-system-relocated-address-30de"
    );
}
