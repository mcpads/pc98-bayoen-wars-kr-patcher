use super::*;

#[test]
fn interrupt_window_policy_wraps_only_the_bios_installation_path() {
    let program = assemble_installer(
        CodeLocation {
            seg: 0,
            off: 0x1900,
        },
        &[0x1a00],
        &[0x7621],
        0x094e,
        InstallerInterruptPolicy::EnableAndRestore,
        "fixture",
    )
    .unwrap();
    let instructions = program
        .instruction_spans()
        .iter()
        .map(|span| span.instruction.clone())
        .collect::<Vec<_>>();

    assert_eq!(instructions[0], Instruction::Pushf);
    assert_eq!(instructions[1], Instruction::Sti);
    assert_eq!(
        &instructions[6..11],
        &[
            Instruction::Mov {
                dest: Operand::Reg16(Register16::BX),
                src: Operand::Sreg(SegmentRegister::CS),
            },
            Instruction::Mov {
                dest: Operand::Reg16(Register16::CX),
                src: Operand::Imm16(0x1a00),
            },
            Instruction::Mov {
                dest: Operand::Reg16(Register16::DX),
                src: Operand::Imm16(0x7621),
            },
            Instruction::Mov {
                dest: Operand::Reg8(Register8::AH),
                src: Operand::Imm8(0x1a),
            },
            Instruction::Int { vector: 0x18 },
        ]
    );
    assert_eq!(instructions[instructions.len() - 2], Instruction::Popf);
    assert!(matches!(
        instructions.last(),
        Some(Instruction::Jmp {
            target: JmpTarget::Rel16(_)
        })
    ));
}

#[test]
fn inherited_interrupt_policy_does_not_change_flags() {
    let program = assemble_installer(
        CodeLocation {
            seg: 0,
            off: 0x1900,
        },
        &[0x1a00],
        &[0x7621],
        0x030b,
        InstallerInterruptPolicy::Inherit,
        "fixture",
    )
    .unwrap();
    let instructions = program
        .instruction_spans()
        .iter()
        .map(|span| &span.instruction)
        .collect::<Vec<_>>();

    assert!(!instructions.contains(&&Instruction::Pushf));
    assert!(!instructions.contains(&&Instruction::Sti));
    assert!(!instructions.contains(&&Instruction::Popf));
    assert_eq!(
        &instructions[..4],
        &[
            &Instruction::Push {
                src: Operand::Reg16(Register16::AX),
            },
            &Instruction::Push {
                src: Operand::Reg16(Register16::BX),
            },
            &Instruction::Push {
                src: Operand::Reg16(Register16::CX),
            },
            &Instruction::Push {
                src: Operand::Reg16(Register16::DX),
            },
        ]
    );
}
