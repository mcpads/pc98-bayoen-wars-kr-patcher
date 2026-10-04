use std::collections::BTreeMap;

use super::references::{merge_reference, scan_target_occurrences};
use super::{MenuTextRuntimeReference, MenuTextRuntimeReferenceKind};

#[test]
fn target_scanner_keeps_overlapping_occurrences_and_target_identity() {
    let targets = [(0x1234, "first"), (0x3412, "second")]
        .into_iter()
        .collect();

    let occurrences = scan_target_occurrences(&[0x34, 0x12, 0x34], &targets);

    assert_eq!(occurrences, [(0, 0x1234, "first"), (1, 0x3412, "second")]);
}

#[test]
fn duplicate_semantic_roles_share_one_physical_reference() {
    let mut references = BTreeMap::new();
    let first = MenuTextRuntimeReference {
        id: "menu-code-reference-0010".to_owned(),
        storage_kind: MenuTextRuntimeReferenceKind::MachineCodeImmediate,
        storage_offset: 0x11,
        instruction_offset: Some(0x10),
        target_com_address: 0x1234,
        target_entry_id: "menu-text-fixture".to_owned(),
        consumer_roles: vec!["first".to_owned()],
    };
    let mut second = MenuTextRuntimeReference {
        consumer_roles: vec!["second".to_owned()],
        ..first
    };

    merge_reference(&mut references, second).unwrap();
    second = MenuTextRuntimeReference {
        id: "menu-code-reference-0010".to_owned(),
        storage_kind: MenuTextRuntimeReferenceKind::MachineCodeImmediate,
        storage_offset: 0x11,
        instruction_offset: Some(0x10),
        target_com_address: 0x1234,
        target_entry_id: "menu-text-fixture".to_owned(),
        consumer_roles: vec!["third".to_owned()],
    };
    merge_reference(&mut references, second).unwrap();

    assert_eq!(references[&0x11].consumer_roles, ["second", "third"]);
}
