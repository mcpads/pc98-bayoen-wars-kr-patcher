use super::*;

fn fixture() -> Vec<u8> {
    let mut bytes = vec![0; DIALOGUE_ROUTINE_FILE_OFFSET + 1];
    bytes[DOS_TERMINATE_FILE_OFFSET..DOS_TERMINATE_FILE_OFFSET + 5]
        .copy_from_slice(&[0xb8, 0x00, 0x4c, 0xcd, 0x21]);
    let startup_next = com_address(STARTUP_CALL_FILE_OFFSET + 3).unwrap();
    let startup_displacement = STARTUP_CALL_TARGET_COM_ADDRESS.wrapping_sub(startup_next) as i16;
    bytes[STARTUP_CALL_FILE_OFFSET] = 0xe8;
    bytes[STARTUP_CALL_FILE_OFFSET + 1..STARTUP_CALL_FILE_OFFSET + 3]
        .copy_from_slice(&startup_displacement.to_le_bytes());
    let target = com_address(DIALOGUE_ROUTINE_FILE_OFFSET).unwrap();
    for &(offset, _) in &CALL_SITES {
        let next = com_address(offset + 3).unwrap();
        let displacement = target.wrapping_sub(next) as i16;
        bytes[offset] = 0xe8;
        bytes[offset + 1..offset + 3].copy_from_slice(&displacement.to_le_bytes());
    }
    bytes
}

#[test]
fn new_game_and_ending_paths_call_the_same_dialogue_entry() {
    let catalog = catalog_mad_scene_transition(&fixture()).unwrap();

    assert_eq!(catalog.call_site_count, 2);
    assert_eq!(catalog.startup_call.file_offset, STARTUP_CALL_FILE_OFFSET);
    assert_eq!(
        catalog.startup_call.target_com_address,
        STARTUP_CALL_TARGET_COM_ADDRESS
    );
    assert_eq!(catalog.dialogue_routine_file_offset, 0xaf4b);
    assert_eq!(catalog.failure_exit_com_address, 0x0590);
    assert_eq!(
        catalog
            .calls
            .iter()
            .map(|call| (call.id.as_str(), call.file_offset))
            .collect::<Vec<_>>(),
        [("new-game-dialogue", 0x0290), ("ending-dialogue", 0x03cc)]
    );
}

#[test]
fn a_retargeted_startup_call_fails_closed() {
    let mut bytes = fixture();
    bytes[STARTUP_CALL_FILE_OFFSET + 1] ^= 1;

    assert!(catalog_mad_scene_transition(&bytes).is_err());
}

#[test]
fn a_new_or_retargeted_dialogue_call_fails_closed() {
    let mut bytes = fixture();
    bytes[CALL_SITES[0].0 + 1] ^= 1;

    assert!(catalog_mad_scene_transition(&bytes).is_err());
}
