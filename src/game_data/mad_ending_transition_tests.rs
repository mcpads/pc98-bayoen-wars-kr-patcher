use super::*;
use crate::game_data::{DialogueEntry, MadSceneTransitionCall, MadSceneTransitionCatalog};

fn fixture() -> Vec<u8> {
    let mut bytes = vec![0; DIALOGUE_GROUP_SCALE_FILE_OFFSET + 2];
    bytes[SELECTOR_REFRESH_CALL_FILE_OFFSET..SELECTOR_REFRESH_CALL_FILE_OFFSET + 3]
        .copy_from_slice(&[0xe8, 0xd0, 0x00]);
    bytes[ENDING_SELECTOR_COMPARE_FILE_OFFSET..ENDING_SELECTOR_COMPARE_FILE_OFFSET + 6]
        .copy_from_slice(&[0x2e, 0x80, 0x3e, 0x9e, 0xd7, 0x80]);
    bytes[ENDING_SELECTOR_SKIP_BRANCH_FILE_OFFSET..ENDING_SELECTOR_SKIP_BRANCH_FILE_OFFSET + 2]
        .copy_from_slice(&[0x75, 0x03]);
    bytes[ENDING_BRANCH_FILE_OFFSET..ENDING_BRANCH_FILE_OFFSET + 2].copy_from_slice(&[0xeb, 0x2a]);
    bytes[ENDING_STAGE_GROUP_WRITE_FILE_OFFSET..ENDING_STAGE_GROUP_WRITE_FILE_OFFSET + 6]
        .copy_from_slice(&[0xc7, 0x06, 0x8d, 0xdb, 0x0a, 0x00]);
    bytes[COMPLETION_STAGE_GROUP_COMPARE_FILE_OFFSET
        ..COMPLETION_STAGE_GROUP_COMPARE_FILE_OFFSET + 6]
        .copy_from_slice(&[0x2e, 0x83, 0x3e, 0x8d, 0xdb, 0x09]);
    bytes[COMPLETION_STAGE_GROUP_SKIP_BRANCH_FILE_OFFSET
        ..COMPLETION_STAGE_GROUP_SKIP_BRANCH_FILE_OFFSET + 2]
        .copy_from_slice(&[0x75, 0x06]);
    bytes[ENDING_SELECTOR_WRITE_FILE_OFFSET..ENDING_SELECTOR_WRITE_FILE_OFFSET + 7]
        .copy_from_slice(&[0x2e, 0xc6, 0x06, 0x9e, 0xd7, 0x80, 0xc3]);
    bytes[DIALOGUE_GROUP_LOAD_FILE_OFFSET..DIALOGUE_GROUP_LOAD_FILE_OFFSET + 5]
        .copy_from_slice(&[0xa1, 0x8d, 0xdb, 0xd1, 0xe0]);
    bytes
}

fn scene_catalog() -> MadSceneTransitionCatalog {
    MadSceneTransitionCatalog {
        startup_call: MadSceneTransitionCall {
            id: "startup-interface-bank".to_owned(),
            file_offset: 0x000d,
            com_address: 0x010d,
            byte_size: 3,
            target_com_address: 0x2fa6,
        },
        dialogue_routine_file_offset: 0xaf4b,
        dialogue_routine_com_address: 0xb04b,
        failure_exit_file_offset: 0x0490,
        failure_exit_com_address: 0x0590,
        call_site_count: 2,
        calls: vec![
            MadSceneTransitionCall {
                id: "new-game-dialogue".to_owned(),
                file_offset: 0x0290,
                com_address: 0x0390,
                byte_size: 3,
                target_com_address: 0xb04b,
            },
            MadSceneTransitionCall {
                id: "ending-dialogue".to_owned(),
                file_offset: ENDING_DIALOGUE_CALL_FILE_OFFSET,
                com_address: u16::try_from(ENDING_DIALOGUE_CALL_FILE_OFFSET + 0x100).unwrap(),
                byte_size: 3,
                target_com_address: 0xb04b,
            },
        ],
    }
}

fn dialogue_groups() -> Vec<DialogueGroup> {
    (1..=11)
        .map(|number| DialogueGroup {
            id: format!("dialogue-group-{number:02}"),
            pointer_table_offset: 0,
            record_table_offset: 0,
            entries: Vec::<DialogueEntry>::new(),
        })
        .collect()
}

#[test]
fn ending_branch_maps_stage_group_ten_to_dialogue_group_eleven() {
    let catalog =
        catalog_mad_ending_transition(&fixture(), &scene_catalog(), dialogue_groups().as_slice())
            .unwrap();

    assert_eq!(catalog.completion_stage_group, 9);
    assert_eq!(catalog.ending_stage_group, 10);
    assert_eq!(catalog.ending_selector_value, 0x80);
    assert_eq!(catalog.ending_dialogue_group_number, 11);
    assert_eq!(catalog.ending_dialogue_group_id, "dialogue-group-11");
}

#[test]
fn a_changed_ending_selector_fails_closed() {
    let mut bytes = fixture();
    bytes[ENDING_SELECTOR_COMPARE_FILE_OFFSET + 5] = 0x40;

    assert!(
        catalog_mad_ending_transition(&bytes, &scene_catalog(), dialogue_groups().as_slice(),)
            .is_err()
    );
}

#[test]
fn a_changed_completion_skip_target_fails_closed() {
    let mut bytes = fixture();
    bytes[COMPLETION_STAGE_GROUP_SKIP_BRANCH_FILE_OFFSET + 1] = 0x04;

    assert!(
        catalog_mad_ending_transition(&bytes, &scene_catalog(), dialogue_groups().as_slice(),)
            .is_err()
    );
}
