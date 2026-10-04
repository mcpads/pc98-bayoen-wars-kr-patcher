use std::ops::Range;

use anyhow::{Result, bail, ensure};
use serde::Serialize;
use v30::{
    CallTarget, Condition, EffectiveAddressBase, EffectiveAddressDisplacement, Instruction,
    LoopCondition, Operand, Register8, Register16, SegmentRegister,
};

use self::typed_v30::{
    decode_range, direct_cs_word_immediate, instruction_at, is_cs_word_memory, relative_target,
};

mod typed_v30;

const INSTALL_LOOP_RANGE: Range<usize> = 0x12..0x51;
const GLYPH_COUNT_OFFSET: usize = 0x12;
const FIRST_CODE_OFFSET: usize = 0x15;
const POINTER_TABLE_OFFSET: usize = 0x1c;
const POINTER_LOAD_OFFSET: usize = 0x29;
const CURRENT_CODE_LOAD_OFFSET: usize = 0x31;
const ROW_END_OFFSET: usize = 0x36;
const ROW_END_BRANCH_OFFSET: usize = 0x3a;
const NEXT_ROW_OFFSET: usize = 0x3c;
const INSTALL_CALL_OFFSET: usize = 0x49;
const LOOP_OFFSET: usize = 0x4f;
const GLYPH_ITERATION_OFFSET: usize = 0x1f;
const INSTALL_ROUTINE_RANGE: Range<usize> = 0x6f..0x76;
const SEGMENT_LOAD_OFFSET: usize = 0x6f;
const BIOS_FUNCTION_OFFSET: usize = 0x71;
const BIOS_INTERRUPT_OFFSET: usize = 0x73;
const INSTALL_RETURN_OFFSET: usize = 0x75;
const CURRENT_CODE_COM_ADDRESS: u16 = 0x0178;
const PC98_CHARACTER_BIOS_INTERRUPT: u8 = 0x18;
const PC98_CHARACTER_INSTALL_FUNCTION: u8 = 0x1a;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct GaijiInstallerCatalog {
    pub install_loop_offset: usize,
    pub install_loop_byte_size: usize,
    pub install_loop_instruction_count: usize,
    pub install_call_offset: usize,
    pub install_routine_offset: usize,
    pub install_routine_byte_size: usize,
    pub install_routine_instruction_count: usize,
    pub bios_interrupt_vector: u8,
    pub bios_function: u8,
    pub glyph_count: usize,
    pub first_character_code: u16,
    pub row_end_character_code: u16,
    pub next_row_previous_character_code: u16,
    pub pointer_table_com_address: u16,
    pub glyph_segment_register: String,
    pub glyph_record_offset_register: String,
    pub character_code_register: String,
}

pub(super) fn parse_gaiji_installer(bytes: &[u8]) -> Result<GaijiInstallerCatalog> {
    let install_loop = decode_range(bytes, INSTALL_LOOP_RANGE.clone(), "install loop")?;
    let install_routine = decode_range(bytes, INSTALL_ROUTINE_RANGE.clone(), "BIOS routine")?;

    let glyph_count = match instruction_at(&install_loop, GLYPH_COUNT_OFFSET)?.instruction {
        Instruction::Mov {
            dest: Operand::Reg16(Register16::CX),
            src: Operand::Imm16(value),
        } => usize::from(value),
        _ => bail!("GAIJI install loop no longer loads its glyph count into CX"),
    };
    ensure!(glyph_count > 0, "GAIJI program declares no glyph records");

    let (current_code_address, first_character_code) = direct_cs_word_immediate(
        instruction_at(&install_loop, FIRST_CODE_OFFSET)?,
        "first character code",
    )?;
    ensure!(
        current_code_address == CURRENT_CODE_COM_ADDRESS,
        "GAIJI first character code moved to an unknown state word"
    );

    let pointer_table_com_address =
        match instruction_at(&install_loop, POINTER_TABLE_OFFSET)?.instruction {
            Instruction::Mov {
                dest: Operand::Reg16(Register16::BX),
                src: Operand::Imm16(value),
            } => value,
            _ => bail!("GAIJI install loop no longer loads the pointer-table base into BX"),
        };

    ensure!(
        matches!(
            instruction_at(&install_loop, POINTER_LOAD_OFFSET)?.instruction,
            Instruction::Mov {
                dest: Operand::Reg16(Register16::CX),
                src: Operand::Mem(memory),
            } if is_cs_word_memory(
                memory,
                EffectiveAddressBase::Bx,
                EffectiveAddressDisplacement::Signed(0),
            )
        ),
        "GAIJI install loop no longer loads a CS-relative glyph pointer into CX"
    );
    ensure!(
        matches!(
            instruction_at(&install_loop, CURRENT_CODE_LOAD_OFFSET)?.instruction,
            Instruction::Mov {
                dest: Operand::Reg16(Register16::DX),
                src: Operand::Mem(memory),
            } if is_cs_word_memory(
                memory,
                EffectiveAddressBase::Direct,
                EffectiveAddressDisplacement::Absolute(CURRENT_CODE_COM_ADDRESS),
            )
        ),
        "GAIJI install loop no longer loads the character code into DX"
    );

    let row_end_character_code = match instruction_at(&install_loop, ROW_END_OFFSET)?.instruction {
        Instruction::Cmp {
            a: Operand::Reg16(Register16::DX),
            b: Operand::Imm16(value),
        } => value,
        _ => bail!("GAIJI install loop no longer compares the row end through DX"),
    };
    ensure!(
        matches!(
            instruction_at(&install_loop, ROW_END_BRANCH_OFFSET)?.instruction,
            Instruction::Jcc {
                cond: Condition::Ne,
                ..
            }
        ),
        "GAIJI row transition no longer uses the verified conditional branch"
    );
    let (next_row_address, next_row_previous_character_code) = direct_cs_word_immediate(
        instruction_at(&install_loop, NEXT_ROW_OFFSET)?,
        "next row character code",
    )?;
    ensure!(
        next_row_address == CURRENT_CODE_COM_ADDRESS,
        "GAIJI next-row character code moved to an unknown state word"
    );

    let install_call = instruction_at(&install_loop, INSTALL_CALL_OFFSET)?;
    let install_displacement = match install_call.instruction {
        Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } => displacement,
        _ => bail!("GAIJI install loop no longer calls a typed near routine"),
    };
    ensure!(
        relative_target(
            INSTALL_CALL_OFFSET,
            install_call.byte_len,
            install_displacement
        )? == INSTALL_ROUTINE_RANGE.start,
        "GAIJI install call no longer targets the verified BIOS routine"
    );

    let loop_instruction = instruction_at(&install_loop, LOOP_OFFSET)?;
    let loop_displacement = match loop_instruction.instruction {
        Instruction::Loop {
            condition: LoopCondition::Always,
            target,
        } => i16::from(target),
        _ => bail!("GAIJI install loop no longer closes with typed LOOP"),
    };
    ensure!(
        relative_target(LOOP_OFFSET, loop_instruction.byte_len, loop_displacement)?
            == GLYPH_ITERATION_OFFSET,
        "GAIJI LOOP no longer returns to the verified glyph iteration boundary"
    );

    ensure!(
        matches!(
            instruction_at(&install_routine, SEGMENT_LOAD_OFFSET)?.instruction,
            Instruction::Mov {
                dest: Operand::Reg16(Register16::BX),
                src: Operand::Sreg(SegmentRegister::CS),
            }
        ),
        "GAIJI BIOS routine no longer passes the glyph segment in BX"
    );
    let bios_function = match instruction_at(&install_routine, BIOS_FUNCTION_OFFSET)?.instruction {
        Instruction::Mov {
            dest: Operand::Reg8(Register8::AH),
            src: Operand::Imm8(value),
        } => value,
        _ => bail!("GAIJI BIOS routine no longer loads its function into AH"),
    };
    let bios_interrupt_vector =
        match instruction_at(&install_routine, BIOS_INTERRUPT_OFFSET)?.instruction {
            Instruction::Int { vector } => vector,
            _ => bail!("GAIJI BIOS routine no longer invokes a typed interrupt"),
        };
    ensure!(
        bios_function == PC98_CHARACTER_INSTALL_FUNCTION
            && bios_interrupt_vector == PC98_CHARACTER_BIOS_INTERRUPT,
        "GAIJI BIOS install function or interrupt vector changed"
    );
    ensure!(
        matches!(
            instruction_at(&install_routine, INSTALL_RETURN_OFFSET)?.instruction,
            Instruction::Ret { pop: 0 }
        ),
        "GAIJI BIOS routine no longer ends in a near RET"
    );

    Ok(GaijiInstallerCatalog {
        install_loop_offset: INSTALL_LOOP_RANGE.start,
        install_loop_byte_size: INSTALL_LOOP_RANGE.len(),
        install_loop_instruction_count: install_loop.len(),
        install_call_offset: INSTALL_CALL_OFFSET,
        install_routine_offset: INSTALL_ROUTINE_RANGE.start,
        install_routine_byte_size: INSTALL_ROUTINE_RANGE.len(),
        install_routine_instruction_count: install_routine.len(),
        bios_interrupt_vector,
        bios_function,
        glyph_count,
        first_character_code,
        row_end_character_code,
        next_row_previous_character_code,
        pointer_table_com_address,
        glyph_segment_register: Register16::BX.name().to_owned(),
        glyph_record_offset_register: Register16::CX.name().to_owned(),
        character_code_register: Register16::DX.name().to_owned(),
    })
}

#[cfg(test)]
#[path = "gaiji_installer_tests.rs"]
mod gaiji_installer_tests;
