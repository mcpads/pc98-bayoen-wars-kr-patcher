use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::{CallTarget, Instruction, Operand, Register8, decode_bytes};

const COM_ORIGIN: usize = 0x100;
const SELECTION_MODE_LOAD_FILE_OFFSET: usize = 0xa040;
const SELECTION_ENTRY_CALL_FILE_OFFSET: usize = 0xa042;
const FIXED_TEXT_RENDER_CALL_FILE_OFFSET: usize = 0xa477;
pub(super) const FIXED_TEXT_RENDERER_FILE_OFFSET: usize = 0x3a48;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct FixedGaijiTextRuntimeCatalog {
    pub selection_mode_load_file_offset: usize,
    pub selection_entry_call: FixedGaijiTextRuntimeCall,
    pub render_call: FixedGaijiTextRuntimeCall,
    pub renderer_file_offset: usize,
    pub renderer_com_address: u16,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct FixedGaijiTextRuntimeCall {
    pub file_offset: usize,
    pub com_address: u16,
    pub byte_size: usize,
    pub target_com_address: u16,
}

pub(super) fn catalog_fixed_gaiji_text_runtime(
    bytes: &[u8],
) -> Result<FixedGaijiTextRuntimeCatalog> {
    let mode = decode_bytes(
        bytes
            .get(SELECTION_MODE_LOAD_FILE_OFFSET..)
            .context("MAD selection mode load lies outside MAD.COM")?,
    )?;
    ensure!(
        matches!(
            mode.instruction,
            Instruction::Mov {
                dest: Operand::Reg8(Register8::AL),
                src: Operand::Imm8(3),
            }
        ) && mode.byte_len == SELECTION_ENTRY_CALL_FILE_OFFSET - SELECTION_MODE_LOAD_FILE_OFFSET
            && mode.prefixes.is_empty(),
        "MAD selection entry no longer begins with the verified typed mode load"
    );
    let selection_entry_call = catalog_near_call(
        bytes,
        SELECTION_ENTRY_CALL_FILE_OFFSET,
        "MAD selection entry call",
    )?;
    let render_call = catalog_near_call(
        bytes,
        FIXED_TEXT_RENDER_CALL_FILE_OFFSET,
        "MAD fixed-text render call",
    )?;
    let renderer_com_address = com_address(FIXED_TEXT_RENDERER_FILE_OFFSET)?;
    ensure!(
        render_call.target_com_address == renderer_com_address,
        "MAD fixed-text render call target changed"
    );

    Ok(FixedGaijiTextRuntimeCatalog {
        selection_mode_load_file_offset: SELECTION_MODE_LOAD_FILE_OFFSET,
        selection_entry_call,
        render_call,
        renderer_file_offset: FIXED_TEXT_RENDERER_FILE_OFFSET,
        renderer_com_address,
    })
}

fn catalog_near_call(
    bytes: &[u8],
    file_offset: usize,
    role: &str,
) -> Result<FixedGaijiTextRuntimeCall> {
    let decoded = decode_bytes(
        bytes
            .get(file_offset..)
            .with_context(|| format!("{role} lies outside MAD.COM"))?,
    )?;
    let target_com_address = match decoded.instruction {
        Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } if decoded.byte_len == 3 && decoded.prefixes.is_empty() => com_address(file_offset)?
            .wrapping_add(u16::try_from(decoded.byte_len)?)
            .wrapping_add_signed(displacement),
        _ => anyhow::bail!("{role} is not the verified typed V30 near CALL"),
    };
    Ok(FixedGaijiTextRuntimeCall {
        file_offset,
        com_address: com_address(file_offset)?,
        byte_size: decoded.byte_len,
        target_com_address,
    })
}

fn com_address(file_offset: usize) -> Result<u16> {
    Ok(u16::try_from(file_offset + COM_ORIGIN)?)
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;
