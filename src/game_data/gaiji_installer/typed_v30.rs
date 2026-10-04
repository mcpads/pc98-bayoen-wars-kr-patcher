use std::ops::Range;

use anyhow::{Context, Result, bail, ensure};
use v30::{
    DecodedInstruction, EffectiveAddressBase, EffectiveAddressDisplacement, Instruction, Operand,
    OperandSize, Prefix, SegmentRegister, decode_bytes,
};

pub(super) struct PlacedInstruction {
    offset: usize,
    instruction: DecodedInstruction,
}

pub(super) fn decode_range(
    bytes: &[u8],
    range: Range<usize>,
    role: &str,
) -> Result<Vec<PlacedInstruction>> {
    let block = bytes
        .get(range.clone())
        .with_context(|| format!("GAIJI {role} lies outside the program"))?;
    let mut instructions = Vec::new();
    let mut relative_offset = 0;
    while relative_offset < block.len() {
        let instruction = decode_bytes(&block[relative_offset..]).with_context(|| {
            format!(
                "GAIJI {role} is not typed V30 code at file offset {:#x}",
                range.start + relative_offset
            )
        })?;
        ensure!(
            instruction.byte_len > 0,
            "typed V30 decoder made no progress"
        );
        relative_offset += instruction.byte_len;
        instructions.push(PlacedInstruction {
            offset: range.start + relative_offset - instruction.byte_len,
            instruction,
        });
    }
    ensure!(
        relative_offset == block.len(),
        "GAIJI {role} ends inside a typed V30 instruction"
    );
    Ok(instructions)
}

pub(super) fn instruction_at(
    instructions: &[PlacedInstruction],
    offset: usize,
) -> Result<&DecodedInstruction> {
    instructions
        .iter()
        .find(|instruction| instruction.offset == offset)
        .map(|instruction| &instruction.instruction)
        .with_context(|| format!("GAIJI typed V30 boundary is missing at {offset:#x}"))
}

pub(super) fn direct_cs_word_immediate(
    decoded: &DecodedInstruction,
    role: &str,
) -> Result<(u16, u16)> {
    match decoded.instruction {
        Instruction::Mov {
            dest: Operand::Mem(memory),
            src: Operand::Imm16(value),
        } if memory.segment() == Some(SegmentRegister::CS)
            && memory.base() == EffectiveAddressBase::Direct
            && memory.size() == OperandSize::Word
            && decoded.prefixes.as_slice() == [Prefix::Segment(SegmentRegister::CS)] =>
        {
            let EffectiveAddressDisplacement::Absolute(address) = memory.displacement() else {
                unreachable!("direct V30 memory has an absolute displacement")
            };
            Ok((address, value))
        }
        _ => bail!("GAIJI {role} is not the verified typed V30 memory write"),
    }
}

pub(super) fn is_cs_word_memory(
    memory: v30::EffectiveAddress,
    base: EffectiveAddressBase,
    displacement: EffectiveAddressDisplacement,
) -> bool {
    memory.segment() == Some(SegmentRegister::CS)
        && memory.base() == base
        && memory.displacement() == displacement
        && memory.size() == OperandSize::Word
}

pub(super) fn relative_target(offset: usize, byte_len: usize, displacement: i16) -> Result<usize> {
    (offset + byte_len)
        .checked_add_signed(isize::from(displacement))
        .context("GAIJI typed relative target lies outside the program")
}
