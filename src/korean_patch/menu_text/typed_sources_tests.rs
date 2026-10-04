use super::*;
use v30::{Instruction, JmpTarget, decode_bytes};

#[test]
fn entry_trampoline_calls_the_original_target_then_resumes() {
    let program = assemble_original_entry_trampoline(0x2000, 0x048d, 0x010c).unwrap();
    let call = &program.instruction_spans()[0];
    let jump = &program.instruction_spans()[1];

    assert!(matches!(
        call.instruction,
        Instruction::Call {
            target: CallTarget::Rel16(_)
        }
    ));
    assert!(matches!(
        jump.instruction,
        Instruction::Jmp {
            target: JmpTarget::Rel16(_)
        }
    ));
    assert_eq!(decode_bytes(program.bytes()).unwrap().byte_len, 3);
    assert_eq!(program.bytes().len(), 6);
}
