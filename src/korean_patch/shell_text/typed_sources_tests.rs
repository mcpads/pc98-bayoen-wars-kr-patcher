use super::*;

#[test]
fn entry_jump_targets_the_appended_installer() {
    let program = assemble_entry_jump(0x03d5).unwrap();

    assert_eq!(program.bytes(), [0xe9, 0xd2, 0x03]);
}
