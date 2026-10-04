use super::*;

fn mad_fixture() -> Vec<u8> {
    let mut program = vec![0_u8; 0x90];
    let program_end_com_address = (program.len() + COM_ORIGIN) as u16;
    program[STACK_SETUP_RANGE].copy_from_slice(&[
        0xbb,
        program_end_com_address as u8,
        (program_end_com_address >> 8) as u8,
        0x8b,
        0xe3,
    ]);
    program
}

#[test]
fn zero_tail_below_initial_stack_pointer_is_not_generic_file_slack() {
    let memory = parse_mad_memory(&mad_fixture()).unwrap();

    assert_eq!(memory.initial_stack_pointer, 0x190);
    assert_eq!(memory.program_end_com_address, 0x190);
    assert_eq!(memory.last_nonzero_file_offset, 0x79);
    assert_eq!(memory.zero_tail_start, 0x7a);
    assert_eq!(memory.zero_tail_byte_size, 0x16);
}

#[test]
fn stack_pointer_detached_from_program_end_is_rejected() {
    let mut program = mad_fixture();
    program[STACK_SETUP_RANGE.start + 1] -= 1;

    let error = parse_mad_memory(&program).unwrap_err();

    assert!(error.to_string().contains("file end"));
}
