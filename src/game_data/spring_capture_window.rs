use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::{CallTarget, Instruction, Operand, Register16, decode_bytes};

use super::InterfaceTextCatalog;
use super::interface_window_binary::{
    require_frame_invalidation_consumer, require_frame_invalidation_geometry,
};

const COM_ORIGIN: usize = 0x100;
const BACKGROUND_COPY_SETUP_OFFSET: usize = 0x2035;
const BACKGROUND_COPY_HEIGHT_INSTRUCTION_OFFSET: usize = 0x203a;
const BACKGROUND_COPY_WIDTH_INSTRUCTION_OFFSET: usize = 0x203d;
const BACKGROUND_COPY_CALL_OFFSET: usize = 0x2040;
const BACKGROUND_RESTORE_ORIGIN_INSTRUCTION_OFFSET: usize = 0x517e;
const BACKGROUND_RESTORE_HEIGHT_INSTRUCTION_OFFSET: usize = 0x5183;
const BACKGROUND_RESTORE_WIDTH_INSTRUCTION_OFFSET: usize = 0x5186;
const BACKGROUND_RESTORE_CALL_OFFSET: usize = 0x5189;
const WINDOW_STATE_WRITE_OFFSET: usize = 0x2043;
const WINDOW_STATE_TABLE_OFFSET: usize = 0x6ac8;
const WINDOW_FRAME_RECORD_SIZE: usize = 12;
const TEXT_STATE_CHECK_OFFSET: usize = 0x593e;
const TEXT_POSITION_INSTRUCTION_OFFSET: usize = 0x5945;
const TEXT_ADDRESS_INSTRUCTION_OFFSET: usize = 0x5948;
const TEXT_RENDER_CALL_OFFSET: usize = 0x594d;
const SOURCE_TEXT_ENTRY_ID: &str = "interface-text-077";
const STATE_VALUE: u8 = 0x17;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SpringCaptureWindowCatalog {
    pub source_text_entry_id: String,
    pub state_value: u8,
    pub background_copy_setup_file_offset: usize,
    pub screen_word_offset: u16,
    pub background_copy_height_rows: u16,
    pub background_copy_width_instruction_file_offset: usize,
    pub background_copy_width_instruction_com_address: u16,
    pub background_copy_width_words: u16,
    pub background_copy_call_file_offset: usize,
    pub background_copy_target_com_address: u16,
    pub background_restore_origin_instruction_file_offset: usize,
    pub background_restore_width_instruction_file_offset: usize,
    pub background_restore_target_com_address: u16,
    pub window_state_table_file_offset: usize,
    pub window_frame_record_file_offset: usize,
    pub window_frame_record_com_address: u16,
    pub frame_inner_width_words: u16,
    pub frame_inner_height_tiles: u16,
    pub source_background_invalidation_x_pixels: u16,
    pub source_background_invalidation_y_pixels: u16,
    pub frame_interaction_table_offset: u16,
    pub text_position: u16,
    pub text_position_instruction_file_offset: usize,
    pub text_address_instruction_file_offset: usize,
    pub text_renderer_com_address: u16,
}

pub(super) fn catalog_spring_capture_window(
    program: &[u8],
    interface: &InterfaceTextCatalog,
) -> Result<SpringCaptureWindowCatalog> {
    require_mov_reg_imm(
        program,
        BACKGROUND_COPY_SETUP_OFFSET,
        Register16::SI,
        0x3714,
    )?;
    let clear_position = decode_at(program, BACKGROUND_COPY_SETUP_OFFSET + 3)?;
    ensure!(
        clear_position.instruction
            == Instruction::Xor {
                dest: Operand::Reg16(Register16::DI),
                src: Operand::Reg16(Register16::DI),
            }
            && clear_position.byte_len == 2
            && clear_position.prefixes.is_empty(),
        "spring-capture window position initializer changed"
    );
    require_mov_reg_imm(
        program,
        BACKGROUND_COPY_HEIGHT_INSTRUCTION_OFFSET,
        Register16::CX,
        0x0030,
    )?;
    require_mov_reg_imm(
        program,
        BACKGROUND_COPY_WIDTH_INSTRUCTION_OFFSET,
        Register16::DX,
        0x000b,
    )?;
    let background_copy_target_com_address =
        require_near_call(program, BACKGROUND_COPY_CALL_OFFSET)?;
    ensure!(
        background_copy_target_com_address == 0x5322,
        "spring-capture background-copy target changed"
    );
    ensure!(
        program.get(WINDOW_STATE_WRITE_OFFSET..WINDOW_STATE_WRITE_OFFSET + 5)
            == Some([0xc6, 0x06, 0x11, 0xd8, STATE_VALUE].as_slice()),
        "spring-capture state write changed"
    );
    require_mov_reg_imm(
        program,
        BACKGROUND_RESTORE_ORIGIN_INSTRUCTION_OFFSET,
        Register16::DI,
        0x3714,
    )?;
    let clear_restore_source =
        decode_at(program, BACKGROUND_RESTORE_ORIGIN_INSTRUCTION_OFFSET + 3)?;
    ensure!(
        clear_restore_source.instruction
            == Instruction::Xor {
                dest: Operand::Reg16(Register16::SI),
                src: Operand::Reg16(Register16::SI),
            }
            && clear_restore_source.byte_len == 2
            && clear_restore_source.prefixes.is_empty(),
        "spring-capture restore buffer initializer changed"
    );
    require_mov_reg_imm(
        program,
        BACKGROUND_RESTORE_HEIGHT_INSTRUCTION_OFFSET,
        Register16::CX,
        0x0030,
    )?;
    require_mov_reg_imm(
        program,
        BACKGROUND_RESTORE_WIDTH_INSTRUCTION_OFFSET,
        Register16::DX,
        0x000b,
    )?;
    let background_restore_target_com_address =
        require_near_call(program, BACKGROUND_RESTORE_CALL_OFFSET)?;
    ensure!(
        background_restore_target_com_address == 0x535e,
        "spring-capture background-restore target changed"
    );
    let state_table_entry_offset = WINDOW_STATE_TABLE_OFFSET + usize::from(STATE_VALUE) * 2;
    let window_frame_record_com_address = read_u16(program, state_table_entry_offset)?;
    ensure!(
        window_frame_record_com_address == 0x6cf4,
        "spring-capture window-state table target changed"
    );
    let window_frame_record_file_offset = usize::from(window_frame_record_com_address)
        .checked_sub(COM_ORIGIN)
        .context("spring-capture window-frame record precedes the COM origin")?;
    let frame_record = program
        .get(
            window_frame_record_file_offset
                ..window_frame_record_file_offset + WINDOW_FRAME_RECORD_SIZE,
        )
        .context("spring-capture window-frame record lies outside MAD.COM")?;
    let frame_words: [u16; WINDOW_FRAME_RECORD_SIZE / 2] = std::array::from_fn(|index| {
        let offset = index * 2;
        u16::from_le_bytes([frame_record[offset], frame_record[offset + 1]])
    });
    ensure!(
        frame_words == [0x0009, 0x0001, 0x3714, 0x00a0, 0x00b0, 0x01d6],
        "spring-capture window-frame record changed"
    );
    ensure!(
        frame_words[0] + 2 == 0x000b
            && (frame_words[1] + 2) * 16 == 0x0030
            && frame_words[2] == 0x3714,
        "spring-capture frame and background-copy geometry diverged"
    );
    require_frame_invalidation_geometry(
        frame_words[2],
        frame_words[3],
        frame_words[4],
        "spring capture window",
    )?;
    require_frame_invalidation_consumer(program, "spring capture window")?;
    ensure!(
        program.get(TEXT_STATE_CHECK_OFFSET..TEXT_STATE_CHECK_OFFSET + 5)
            == Some([0x80, 0x3e, 0x11, 0xd8, STATE_VALUE].as_slice()),
        "spring-capture text state check changed"
    );
    require_mov_reg_imm(
        program,
        TEXT_POSITION_INSTRUCTION_OFFSET,
        Register16::DI,
        0x3c16,
    )?;
    let text_entry = interface
        .entries
        .iter()
        .find(|entry| entry.id == SOURCE_TEXT_ENTRY_ID)
        .context("spring-capture source text entry is missing")?;
    require_mov_reg_imm(
        program,
        TEXT_ADDRESS_INSTRUCTION_OFFSET,
        Register16::DX,
        u16::try_from(text_entry.com_address)?,
    )?;
    let text_renderer_com_address = require_near_call(program, TEXT_RENDER_CALL_OFFSET)?;
    ensure!(
        text_renderer_com_address == 0x5728,
        "spring-capture text renderer changed"
    );

    Ok(SpringCaptureWindowCatalog {
        source_text_entry_id: SOURCE_TEXT_ENTRY_ID.to_owned(),
        state_value: STATE_VALUE,
        background_copy_setup_file_offset: BACKGROUND_COPY_SETUP_OFFSET,
        screen_word_offset: 0x3714,
        background_copy_height_rows: 0x0030,
        background_copy_width_instruction_file_offset: BACKGROUND_COPY_WIDTH_INSTRUCTION_OFFSET,
        background_copy_width_instruction_com_address: u16::try_from(
            BACKGROUND_COPY_WIDTH_INSTRUCTION_OFFSET + COM_ORIGIN,
        )?,
        background_copy_width_words: 0x000b,
        background_copy_call_file_offset: BACKGROUND_COPY_CALL_OFFSET,
        background_copy_target_com_address,
        background_restore_origin_instruction_file_offset:
            BACKGROUND_RESTORE_ORIGIN_INSTRUCTION_OFFSET,
        background_restore_width_instruction_file_offset:
            BACKGROUND_RESTORE_WIDTH_INSTRUCTION_OFFSET,
        background_restore_target_com_address,
        window_state_table_file_offset: WINDOW_STATE_TABLE_OFFSET,
        window_frame_record_file_offset,
        window_frame_record_com_address,
        frame_inner_width_words: frame_words[0],
        frame_inner_height_tiles: frame_words[1],
        source_background_invalidation_x_pixels: frame_words[3],
        source_background_invalidation_y_pixels: frame_words[4],
        frame_interaction_table_offset: frame_words[5],
        text_position: 0x3c16,
        text_position_instruction_file_offset: TEXT_POSITION_INSTRUCTION_OFFSET,
        text_address_instruction_file_offset: TEXT_ADDRESS_INSTRUCTION_OFFSET,
        text_renderer_com_address,
    })
}

fn read_u16(program: &[u8], offset: usize) -> Result<u16> {
    let raw: [u8; 2] = program
        .get(offset..offset + 2)
        .with_context(|| format!("spring-capture word at {offset:#x} lies outside MAD.COM"))?
        .try_into()
        .expect("two bytes convert to one word");
    Ok(u16::from_le_bytes(raw))
}

fn require_mov_reg_imm(
    program: &[u8],
    offset: usize,
    expected_register: Register16,
    expected_value: u16,
) -> Result<()> {
    let decoded = decode_at(program, offset)?;
    ensure!(
        decoded.instruction
            == Instruction::Mov {
                dest: Operand::Reg16(expected_register),
                src: Operand::Imm16(expected_value),
            }
            && decoded.byte_len == 3
            && decoded.prefixes.is_empty(),
        "spring-capture instruction at {offset:#x} changed"
    );
    Ok(())
}

fn require_near_call(program: &[u8], offset: usize) -> Result<u16> {
    let decoded = decode_at(program, offset)?;
    match decoded.instruction {
        Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } if decoded.byte_len == 3 && decoded.prefixes.is_empty() => {
            Ok(u16::try_from(offset + COM_ORIGIN + decoded.byte_len)?
                .wrapping_add_signed(displacement))
        }
        _ => anyhow::bail!("spring-capture call at {offset:#x} changed"),
    }
}

fn decode_at(program: &[u8], offset: usize) -> Result<v30::DecodedInstruction> {
    decode_bytes(
        program.get(offset..).with_context(|| {
            format!("spring-capture instruction at {offset:#x} is outside MAD.COM")
        })?,
    )
    .with_context(|| format!("spring-capture instruction at {offset:#x} is not typed V30 code"))
}

#[cfg(test)]
#[path = "spring_capture_window_tests.rs"]
mod spring_capture_window_tests;
