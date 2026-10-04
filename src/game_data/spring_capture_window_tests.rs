use super::*;

#[test]
fn background_copy_width_is_owned_by_one_complete_mov_dx_instruction() {
    let mut program = vec![0; BACKGROUND_COPY_WIDTH_INSTRUCTION_OFFSET + 3];
    program[BACKGROUND_COPY_WIDTH_INSTRUCTION_OFFSET..].copy_from_slice(&[0xba, 0x0b, 0x00]);

    assert!(
        require_mov_reg_imm(
            &program,
            BACKGROUND_COPY_WIDTH_INSTRUCTION_OFFSET,
            Register16::DX,
            0x0b,
        )
        .is_ok()
    );
    assert!(
        require_mov_reg_imm(
            &program,
            BACKGROUND_COPY_WIDTH_INSTRUCTION_OFFSET,
            Register16::CX,
            0x0b,
        )
        .is_err()
    );
}

#[test]
fn near_call_target_uses_the_com_origin() {
    let mut program = vec![0; BACKGROUND_COPY_CALL_OFFSET + 3];
    let next = BACKGROUND_COPY_CALL_OFFSET + COM_ORIGIN + 3;
    let displacement = 0x5322u16.wrapping_sub(next as u16);
    program[BACKGROUND_COPY_CALL_OFFSET] = 0xe8;
    program[BACKGROUND_COPY_CALL_OFFSET + 1..BACKGROUND_COPY_CALL_OFFSET + 3]
        .copy_from_slice(&displacement.to_le_bytes());

    assert_eq!(
        require_near_call(&program, BACKGROUND_COPY_CALL_OFFSET).unwrap(),
        0x5322
    );
}

#[test]
fn state_table_word_uses_the_state_as_a_word_index() {
    let mut program = vec![0; WINDOW_STATE_TABLE_OFFSET + usize::from(STATE_VALUE) * 2 + 2];
    let offset = WINDOW_STATE_TABLE_OFFSET + usize::from(STATE_VALUE) * 2;
    program[offset..offset + 2].copy_from_slice(&0x6cf4_u16.to_le_bytes());

    assert_eq!(read_u16(&program, offset).unwrap(), 0x6cf4);
}
