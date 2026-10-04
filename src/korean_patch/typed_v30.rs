use anyhow::{Context, Result, ensure};
use v30::{
    AssembledProgram, Assembler, CodeLocation, EffectiveAddress, EffectiveAddressBase,
    EffectiveAddressDisplacement, Instruction, Operand, OperandSize, Register16, decode_bytes,
};

pub(crate) fn assemble_relocated_mov_immediate(
    program: &[u8],
    instruction_offset: usize,
    target: u16,
    allowed_registers: &[Register16],
    role: &str,
) -> Result<AssembledProgram> {
    assemble_relocated_mov_immediate_at(
        program,
        instruction_offset,
        0x100,
        target,
        allowed_registers,
        role,
    )
}

pub(crate) fn assemble_relocated_segment_mov_immediate(
    program: &[u8],
    instruction_offset: usize,
    target: u16,
    allowed_registers: &[Register16],
    role: &str,
) -> Result<AssembledProgram> {
    assemble_relocated_mov_immediate_at(
        program,
        instruction_offset,
        0,
        target,
        allowed_registers,
        role,
    )
}

fn assemble_relocated_mov_immediate_at(
    program: &[u8],
    instruction_offset: usize,
    address_bias: usize,
    target: u16,
    allowed_registers: &[Register16],
    role: &str,
) -> Result<AssembledProgram> {
    let decoded = decode_bytes(
        program
            .get(instruction_offset..)
            .with_context(|| format!("{role} instruction lies outside the program"))?,
    )?;
    let register = match decoded.instruction {
        Instruction::Mov {
            dest: Operand::Reg16(register),
            src: Operand::Imm16(_),
        } => register,
        _ => anyhow::bail!("{role} changed typed V30 MOV reg16, imm16 instruction"),
    };
    ensure!(
        allowed_registers.contains(&register),
        "{role} changed its target register"
    );

    let mut assembler = Assembler::new();
    assembler.emit(Instruction::Mov {
        dest: Operand::Reg16(register),
        src: Operand::Imm16(target),
    });
    let origin = CodeLocation {
        seg: 0,
        off: u16::try_from(instruction_offset + address_bias)
            .with_context(|| format!("{role} segment address exceeds 16 bits"))?,
    };
    let assembled = assembler.assemble(origin)?;
    ensure!(
        assembled.bytes().len() == decoded.byte_len,
        "{role} typed V30 source changed instruction width"
    );
    Ok(assembled)
}

pub(crate) fn assemble_relocated_direct_ax_store(
    program: &[u8],
    instruction_offset: usize,
    target: u16,
    role: &str,
) -> Result<AssembledProgram> {
    let decoded = decode_bytes(
        program
            .get(instruction_offset..)
            .with_context(|| format!("{role} instruction lies outside the program"))?,
    )?;
    let source_memory = match decoded.instruction {
        Instruction::Mov {
            dest: Operand::Mem(memory),
            src: Operand::Reg16(Register16::AX),
        } => memory,
        _ => anyhow::bail!("{role} changed typed V30 MOV [absolute16], AX instruction"),
    };
    ensure!(
        decoded.prefixes.is_empty()
            && source_memory.base() == EffectiveAddressBase::Direct
            && source_memory.size() == OperandSize::Word,
        "{role} changed its direct word-addressing form"
    );

    let memory = EffectiveAddress::new(
        None,
        EffectiveAddressBase::Direct,
        EffectiveAddressDisplacement::Absolute(target),
        OperandSize::Word,
    )?;
    let mut assembler = Assembler::new();
    assembler.emit(Instruction::Mov {
        dest: Operand::Mem(memory),
        src: Operand::Reg16(Register16::AX),
    });
    let origin = CodeLocation {
        seg: 0,
        off: u16::try_from(instruction_offset + 0x100)
            .with_context(|| format!("{role} COM address exceeds 16 bits"))?,
    };
    let assembled = assembler.assemble(origin)?;
    ensure!(
        assembled.bytes().len() == decoded.byte_len,
        "{role} typed V30 source changed instruction width"
    );
    Ok(assembled)
}

#[cfg(test)]
#[path = "typed_v30_tests.rs"]
mod typed_v30_tests;
