use super::*;

#[test]
fn entry_hook_targets_the_appended_installer() {
    let program = assemble_entry_jump(1, 0x0e15).unwrap();

    assert_eq!(program.origin(), CodeLocation { seg: 0, off: 0x101 });
    let decoded = v30::decode_bytes(program.bytes()).unwrap();
    let displacement = match decoded.instruction {
        Instruction::Jmp {
            target: JmpTarget::Rel16(value),
        } => i32::from(value),
        _ => panic!("mouse-driver entry hook must be a near jump"),
    };
    assert_eq!(0x104 + displacement, 0x0f15);
}
