use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::{
    CallTarget, EffectiveAddressBase, EffectiveAddressDisplacement, Instruction, Operand,
    OperandSize, Prefix, Register8, Register16, SegmentRegister, ShiftCount, decode_bytes,
};

const GLYPH_BUFFER_SEGMENT_STORE_FILE_OFFSET: usize = 0x009a;
const GLYPH_BUFFER_PARAGRAPH_COUNT_FILE_OFFSET: usize = 0x009e;
const NEXT_BUFFER_SEGMENT_STORE_FILE_OFFSET: usize = 0x00a1;
const GLYPH_BUFFER_SEGMENT_ADDRESS: u16 = 0xd593;
const NEXT_BUFFER_SEGMENT_ADDRESS: u16 = 0xd595;
const GLYPH_INDEX_LOAD_FILE_OFFSET: usize = 0x9aa9;
const LINE_BREAK_COMPARE_FILE_OFFSET: usize = 0x9aab;
const PAGE_END_COMPARE_FILE_OFFSET: usize = 0x9ab0;
const GLYPH_RENDERER_CALL_FILE_OFFSET: usize = 0x9ab5;
const GLYPH_RENDERER_FILE_OFFSET: usize = 0x9cb4;
const GLYPH_ADDRESS_CLEAR_LOW_BYTE_FILE_OFFSET: usize = 0x9cb4;
const GLYPH_ADDRESS_SCALE_FILE_OFFSET: usize = 0x9cb6;
const LINE_BREAK: u8 = 0xfe;
const PAGE_END: u8 = 0xff;
const PARAGRAPH_BYTE_SIZE: usize = 16;
const GLYPH_BYTE_SIZE: usize = 128;

pub(crate) const MONOCHROME_USABLE_GLYPH_COUNT: usize = LINE_BREAK as usize;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MonochromeRuntimeBinding {
    pub glyph_buffer_segment_store_file_offset: usize,
    pub next_buffer_segment_store_file_offset: usize,
    pub decoded_buffer_paragraph_count: usize,
    pub decoded_buffer_byte_size: usize,
    pub usable_glyph_count: usize,
    pub highest_glyph_index: usize,
    pub highest_glyph_end_offset: usize,
    pub remaining_buffer_byte_count: usize,
}

pub(crate) fn catalog_monochrome_runtime(program: &[u8]) -> Result<MonochromeRuntimeBinding> {
    require_direct_cs_ax_store(
        program,
        GLYPH_BUFFER_SEGMENT_STORE_FILE_OFFSET,
        GLYPH_BUFFER_SEGMENT_ADDRESS,
        "monochrome decoded-buffer start",
    )?;
    let paragraph_count = require_add_ax_immediate(
        program,
        GLYPH_BUFFER_PARAGRAPH_COUNT_FILE_OFFSET,
        "monochrome decoded-buffer paragraph count",
    )?;
    require_direct_cs_ax_store(
        program,
        NEXT_BUFFER_SEGMENT_STORE_FILE_OFFSET,
        NEXT_BUFFER_SEGMENT_ADDRESS,
        "buffer following the monochrome decoded buffer",
    )?;
    require_glyph_index_consumer(program)?;

    let decoded_buffer_byte_size = usize::from(paragraph_count)
        .checked_mul(PARAGRAPH_BYTE_SIZE)
        .context("monochrome decoded-buffer size overflow")?;
    let usable_glyph_count = MONOCHROME_USABLE_GLYPH_COUNT;
    let highest_glyph_index = usable_glyph_count - 1;
    let highest_glyph_end_offset = usable_glyph_count
        .checked_mul(GLYPH_BYTE_SIZE)
        .context("monochrome highest-glyph end offset overflow")?;
    ensure!(
        highest_glyph_end_offset <= decoded_buffer_byte_size,
        "monochrome control-byte index range exceeds its decoded buffer"
    );

    Ok(MonochromeRuntimeBinding {
        glyph_buffer_segment_store_file_offset: GLYPH_BUFFER_SEGMENT_STORE_FILE_OFFSET,
        next_buffer_segment_store_file_offset: NEXT_BUFFER_SEGMENT_STORE_FILE_OFFSET,
        decoded_buffer_paragraph_count: usize::from(paragraph_count),
        decoded_buffer_byte_size,
        usable_glyph_count,
        highest_glyph_index,
        highest_glyph_end_offset,
        remaining_buffer_byte_count: decoded_buffer_byte_size - highest_glyph_end_offset,
    })
}

fn require_direct_cs_ax_store(
    program: &[u8],
    offset: usize,
    expected_address: u16,
    role: &str,
) -> Result<()> {
    let decoded = instruction_at(program, offset, role)?;
    ensure!(
        matches!(
            decoded.instruction,
            Instruction::Mov {
                dest: Operand::Mem(memory),
                src: Operand::Reg16(Register16::AX),
            } if memory.segment() == Some(SegmentRegister::CS)
                && memory.base() == EffectiveAddressBase::Direct
                && memory.displacement()
                    == EffectiveAddressDisplacement::Absolute(expected_address)
                && memory.size() == OperandSize::Word
        ) && decoded.prefixes.as_slice() == [Prefix::Segment(SegmentRegister::CS)],
        "MAD.COM {role} is not the verified typed V30 segment store"
    );
    Ok(())
}

fn require_add_ax_immediate(program: &[u8], offset: usize, role: &str) -> Result<u16> {
    let decoded = instruction_at(program, offset, role)?;
    match decoded.instruction {
        Instruction::Add {
            dest: Operand::Reg16(Register16::AX),
            src: Operand::Imm16(value),
        } if decoded.prefixes.is_empty() => Ok(value),
        _ => anyhow::bail!("MAD.COM {role} is not the verified typed V30 ADD AX, imm16"),
    }
}

fn require_glyph_index_consumer(program: &[u8]) -> Result<()> {
    let load = instruction_at(
        program,
        GLYPH_INDEX_LOAD_FILE_OFFSET,
        "monochrome glyph-index load",
    )?;
    ensure!(
        matches!(
            load.instruction,
            Instruction::Mov {
                dest: Operand::Reg8(Register8::AH),
                src: Operand::Mem(memory),
            } if memory.base() == EffectiveAddressBase::Bx
                && memory.displacement() == EffectiveAddressDisplacement::Signed(0)
                && memory.size() == OperandSize::Byte
        ) && load.prefixes.is_empty(),
        "MAD.COM monochrome consumer no longer loads one glyph-index byte into AH"
    );
    require_compare_ah_immediate(
        program,
        LINE_BREAK_COMPARE_FILE_OFFSET,
        LINE_BREAK,
        "line-break control",
    )?;
    require_compare_ah_immediate(
        program,
        PAGE_END_COMPARE_FILE_OFFSET,
        PAGE_END,
        "page-end control",
    )?;

    let call = instruction_at(
        program,
        GLYPH_RENDERER_CALL_FILE_OFFSET,
        "monochrome glyph-renderer call",
    )?;
    let displacement = match call.instruction {
        Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } if call.prefixes.is_empty() => displacement,
        _ => anyhow::bail!("MAD.COM monochrome glyph-renderer call changed"),
    };
    let target = GLYPH_RENDERER_CALL_FILE_OFFSET
        .checked_add(call.byte_len)
        .and_then(|next| next.checked_add_signed(isize::from(displacement)))
        .context("monochrome glyph-renderer call target overflow")?;
    ensure!(
        target == GLYPH_RENDERER_FILE_OFFSET,
        "MAD.COM monochrome glyph-renderer call target changed"
    );

    let clear = instruction_at(
        program,
        GLYPH_ADDRESS_CLEAR_LOW_BYTE_FILE_OFFSET,
        "monochrome glyph address low-byte clear",
    )?;
    ensure!(
        matches!(
            clear.instruction,
            Instruction::Xor {
                dest: Operand::Reg8(Register8::AL),
                src: Operand::Reg8(Register8::AL),
            }
        ) && clear.prefixes.is_empty(),
        "MAD.COM monochrome renderer no longer clears AL before scaling the AH index"
    );
    let scale = instruction_at(
        program,
        GLYPH_ADDRESS_SCALE_FILE_OFFSET,
        "monochrome glyph address scale",
    )?;
    ensure!(
        matches!(
            scale.instruction,
            Instruction::Shr {
                dest: Operand::Reg16(Register16::AX),
                count: ShiftCount::One,
            }
        ) && scale.prefixes.is_empty(),
        "MAD.COM monochrome renderer no longer scales the AH index to a 128-byte record"
    );
    Ok(())
}

fn require_compare_ah_immediate(
    program: &[u8],
    offset: usize,
    value: u8,
    role: &str,
) -> Result<()> {
    let decoded = instruction_at(program, offset, role)?;
    ensure!(
        matches!(
            decoded.instruction,
            Instruction::Cmp {
                a: Operand::Reg8(Register8::AH),
                b: Operand::Imm8(actual),
            } if actual == value
        ) && decoded.prefixes.is_empty(),
        "MAD.COM monochrome {role} comparison changed"
    );
    Ok(())
}

fn instruction_at(program: &[u8], offset: usize, role: &str) -> Result<v30::DecodedInstruction> {
    decode_bytes(
        program
            .get(offset..)
            .with_context(|| format!("MAD.COM {role} lies outside the file"))?,
    )
    .with_context(|| format!("MAD.COM {role} is not typed V30 code at file offset {offset:#x}"))
}

#[cfg(test)]
#[path = "monochrome_runtime_tests.rs"]
mod monochrome_runtime_tests;
