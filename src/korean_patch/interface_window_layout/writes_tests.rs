use super::*;

#[test]
fn compiled_layout_write_owns_one_complete_typed_instruction() {
    let source = assemble_window_mov(0x200, Register16::DI, 0x3214).unwrap();
    let decoded = decode_bytes(source.bytes()).unwrap();

    assert_eq!(source.bytes().len(), 3);
    assert_eq!(
        decoded.instruction,
        Instruction::Mov {
            dest: Operand::Reg16(Register16::DI),
            src: Operand::Imm16(0x3214),
        }
    );
}
