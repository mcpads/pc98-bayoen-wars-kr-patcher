use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::{Instruction, Operand, Register16, decode_bytes};

const COM_ORIGIN: usize = 0x100;
const STACK_SETUP_RANGE: std::ops::Range<usize> = 0x75..0x7a;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadMemoryCatalog {
    pub stack_setup_offset: usize,
    pub stack_setup_byte_size: usize,
    pub stack_setup_instruction_count: usize,
    pub initial_stack_pointer: u16,
    pub program_end_com_address: u16,
    pub last_nonzero_file_offset: usize,
    pub zero_tail_start: usize,
    pub zero_tail_byte_size: usize,
}

pub(super) fn parse_mad_memory(program: &[u8]) -> Result<MadMemoryCatalog> {
    let stack_setup = program
        .get(STACK_SETUP_RANGE.clone())
        .context("MAD stack setup lies outside the program")?;
    let load = decode_bytes(stack_setup).context("MAD stack base is not typed V30 code")?;
    let initial_stack_pointer = match load.instruction {
        Instruction::Mov {
            dest: Operand::Reg16(Register16::BX),
            src: Operand::Imm16(value),
        } if load.byte_len == 3 && load.prefixes.is_empty() => value,
        _ => anyhow::bail!("MAD stack base is not the verified typed MOV BX, imm16"),
    };
    let assign = decode_bytes(&stack_setup[load.byte_len..])
        .context("MAD stack assignment is not typed V30 code")?;
    ensure!(
        matches!(
            assign.instruction,
            Instruction::Mov {
                dest: Operand::Reg16(Register16::SP),
                src: Operand::Reg16(Register16::BX),
            }
        ) && assign.byte_len == 2
            && assign.prefixes.is_empty()
            && load.byte_len + assign.byte_len == stack_setup.len(),
        "MAD stack setup changed from the verified typed MOV SP, BX"
    );

    let program_end_com_address = u16::try_from(
        program
            .len()
            .checked_add(COM_ORIGIN)
            .context("MAD COM end address overflow")?,
    )
    .context("MAD COM end address exceeds 16 bits")?;
    ensure!(
        initial_stack_pointer == program_end_com_address,
        "MAD initial SP no longer equals the loaded COM file end"
    );

    let last_nonzero_file_offset = program
        .iter()
        .rposition(|byte| *byte != 0)
        .context("MAD program contains no nonzero bytes")?;
    let zero_tail_start = last_nonzero_file_offset + 1;
    let zero_tail_byte_size = program.len() - zero_tail_start;
    ensure!(
        zero_tail_byte_size > 0,
        "MAD program no longer has a zero-filled tail below its initial SP"
    );

    Ok(MadMemoryCatalog {
        stack_setup_offset: STACK_SETUP_RANGE.start,
        stack_setup_byte_size: STACK_SETUP_RANGE.len(),
        stack_setup_instruction_count: 2,
        initial_stack_pointer,
        program_end_com_address,
        last_nonzero_file_offset,
        zero_tail_start,
        zero_tail_byte_size,
    })
}

#[cfg(test)]
#[path = "mad_memory_tests.rs"]
mod mad_memory_tests;
