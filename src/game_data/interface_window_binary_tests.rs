use super::*;

#[test]
fn near_call_target_uses_the_com_origin() {
    let offset = 0x200;
    let mut program = vec![0; offset + 3];
    let next = offset + COM_ORIGIN + 3;
    let displacement = 0x5322u16.wrapping_sub(next as u16);
    program[offset] = 0xe8;
    program[offset + 1..offset + 3].copy_from_slice(&displacement.to_le_bytes());

    assert_eq!(
        require_near_call(&program, offset, "fixture").unwrap(),
        0x5322
    );
}

#[test]
fn frame_record_requires_every_shared_state_to_name_the_same_record() {
    let states = [0x0e, 0x10];
    let record_address: u16 = 0x6cb8;
    let record_offset = usize::from(record_address) - COM_ORIGIN;
    let mut program = vec![0; record_offset + WINDOW_FRAME_RECORD_SIZE];
    for state in states {
        let offset = WINDOW_STATE_TABLE_OFFSET + usize::from(state) * 2;
        program[offset..offset + 2].copy_from_slice(&record_address.to_le_bytes());
    }
    let words: [u16; 6] = [7, 2, 0x2d18, 0x00c0, 0x0090, 0x019a];
    for (index, word) in words.iter().enumerate() {
        program[record_offset + index * 2..record_offset + index * 2 + 2]
            .copy_from_slice(&word.to_le_bytes());
    }

    assert_eq!(
        require_frame_record(&program, &states, record_address, words, "fixture").unwrap(),
        record_offset
    );
    let second_state_offset = WINDOW_STATE_TABLE_OFFSET + usize::from(states[1]) * 2;
    program[second_state_offset..second_state_offset + 2]
        .copy_from_slice(&0x6cf4_u16.to_le_bytes());
    assert!(require_frame_record(&program, &states, record_address, words, "fixture").is_err());
}

#[test]
fn frame_invalidation_coordinates_match_the_frame_origin() {
    require_frame_invalidation_geometry(0x2d18, 0x00c0, 0x0090, "fixture").unwrap();

    assert!(require_frame_invalidation_geometry(0x2d12, 0x00c0, 0x0090, "fixture").is_err());
    require_frame_invalidation_geometry(0x2d12, 0x0090, 0x0090, "fixture").unwrap();
}
