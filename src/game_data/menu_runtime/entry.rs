use anyhow::{Context, Result, ensure};
use v30::{CallTarget, Instruction, Operand, Register16, SegmentRegister, decode_bytes};

use super::MenuEntryRuntimeCatalog;

const COM_ORIGIN: usize = 0x100;
const ENTRY_HOOK_INSTRUCTION_OFFSET: usize = 0x0009;
const ORIGINAL_ENTRY_CALL_TARGET: u16 = 0x048d;
const ENTRY_RESUME_COM_ADDRESS: u16 = 0x010c;
const INITIAL_STACK_POINTER: u16 = 0x181e;

pub(super) fn catalog_entry(program: &[u8]) -> Result<MenuEntryRuntimeCatalog> {
    require_instruction(
        program,
        0,
        Instruction::Mov {
            dest: Operand::Reg16(Register16::AX),
            src: Operand::Sreg(SegmentRegister::CS),
        },
        2,
        "copy CS into AX",
    )?;
    require_instruction(
        program,
        2,
        Instruction::Mov {
            dest: Operand::Sreg(SegmentRegister::DS),
            src: Operand::Reg16(Register16::AX),
        },
        2,
        "initialize DS",
    )?;
    require_instruction(
        program,
        4,
        Instruction::Mov {
            dest: Operand::Sreg(SegmentRegister::SS),
            src: Operand::Reg16(Register16::AX),
        },
        2,
        "initialize SS",
    )?;
    require_instruction(
        program,
        6,
        Instruction::Mov {
            dest: Operand::Reg16(Register16::SP),
            src: Operand::Imm16(INITIAL_STACK_POINTER),
        },
        3,
        "initialize SP",
    )?;
    let entry_call = decode_bytes(
        program
            .get(ENTRY_HOOK_INSTRUCTION_OFFSET..)
            .context("MENU entry call lies outside the file")?,
    )?;
    let displacement = match entry_call.instruction {
        Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } if entry_call.byte_len == 3 && entry_call.prefixes.is_empty() => displacement,
        _ => anyhow::bail!("MENU entry hook is not the verified typed V30 near CALL"),
    };
    let next_address = ENTRY_HOOK_INSTRUCTION_OFFSET + COM_ORIGIN + entry_call.byte_len;
    ensure!(
        i32::try_from(next_address)? + i32::from(displacement)
            == i32::from(ORIGINAL_ENTRY_CALL_TARGET),
        "MENU original entry call target changed"
    );
    ensure!(
        u16::try_from(next_address).context("MENU entry resume address exceeds 16 bits")?
            == ENTRY_RESUME_COM_ADDRESS,
        "MENU entry resume address changed"
    );
    let program_end_com_address =
        u16::try_from(program.len() + COM_ORIGIN).context("MENU program end exceeds 16 bits")?;
    ensure!(
        program_end_com_address == INITIAL_STACK_POINTER,
        "MENU file end and initial stack ceiling no longer coincide"
    );

    Ok(MenuEntryRuntimeCatalog {
        hook_instruction_offset: ENTRY_HOOK_INSTRUCTION_OFFSET,
        hook_com_address: u16::try_from(ENTRY_HOOK_INSTRUCTION_OFFSET + COM_ORIGIN)?,
        original_call_target_com_address: ORIGINAL_ENTRY_CALL_TARGET,
        resume_com_address: ENTRY_RESUME_COM_ADDRESS,
        initial_stack_pointer: INITIAL_STACK_POINTER,
        program_end_com_address,
    })
}

fn require_instruction(
    program: &[u8],
    offset: usize,
    expected: Instruction,
    expected_byte_len: usize,
    role: &str,
) -> Result<()> {
    let decoded = decode_bytes(
        program
            .get(offset..)
            .with_context(|| format!("MENU {role} instruction lies outside the file"))?,
    )?;
    ensure!(
        decoded.instruction == expected
            && decoded.byte_len == expected_byte_len
            && decoded.prefixes.is_empty(),
        "MENU typed V30 instruction changed while trying to {role}"
    );
    Ok(())
}
