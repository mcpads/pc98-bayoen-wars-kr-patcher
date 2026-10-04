use v30::{Instruction, JmpTarget, decode_bytes};

use super::assemble_entry_jump;

#[test]
fn entry_jump_reaches_the_first_nonresident_file_byte() {
    let assembled = assemble_entry_jump(0, 0x7f10).unwrap();
    let decoded = decode_bytes(assembled.bytes()).unwrap();
    let displacement = match decoded.instruction {
        Instruction::Jmp {
            target: JmpTarget::Rel16(displacement),
        } => displacement,
        instruction => panic!("unexpected entry instruction {instruction:?}"),
    };

    assert_eq!(decoded.byte_len, 3);
    assert_eq!(0x103_i32 + i32::from(displacement), 0x8010);
}

#[test]
fn entry_jump_rejects_a_target_outside_the_near_range() {
    assert!(assemble_entry_jump(0, 0x8003).is_err());
}
