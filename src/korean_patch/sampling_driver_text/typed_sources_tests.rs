use super::*;

#[test]
fn entry_hook_leaves_the_original_cli_in_front_of_the_installer_jump() {
    let program = assemble_entry_jump(1, 0x1721).unwrap();

    assert_eq!(program.origin(), CodeLocation { seg: 0, off: 0x101 });
    let decoded = v30::decode_bytes(program.bytes()).unwrap();
    assert!(matches!(
        decoded.instruction,
        Instruction::Jmp {
            target: JmpTarget::Rel16(_)
        }
    ));
}
