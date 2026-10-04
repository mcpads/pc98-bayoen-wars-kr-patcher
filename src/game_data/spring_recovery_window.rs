use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::Register16;

use super::InterfaceTextCatalog;
use super::interface_window_binary::{
    require_frame_invalidation_consumer, require_frame_invalidation_geometry, require_frame_record,
    require_mov_reg_imm, require_near_call, require_state_check,
};

const STATE_VALUE: u8 = 0x1b;
const FRAME_RECORD_COM_ADDRESS: u16 = 0x6d0c;
const SOURCE_FRAME_WORDS: [u16; 6] = [0x000b, 0x0002, 0x3212, 0x0090, 0x00a0, 0x01ee];
const SOURCE_TEXT_POSITION: u16 = 0x3714;
const BACKGROUND_HEIGHT_ROWS: u16 = 0x0050;
const BACKGROUND_WIDTH_WORDS: u16 = 0x000d;
const TEXT_RENDERER_COM_ADDRESS: u16 = 0x5728;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SpringRecoveryWindowCatalog {
    pub source_text_entry_ids: Vec<String>,
    pub state_value: u8,
    pub window_frame_record_file_offset: usize,
    pub window_frame_record_com_address: u16,
    pub source_frame_inner_width_words: u16,
    pub frame_inner_height_tiles: u16,
    pub source_frame_origin: u16,
    pub source_background_invalidation_x_pixels: u16,
    pub source_background_invalidation_y_pixels: u16,
    pub frame_interaction_table_offset: u16,
    pub background_height_rows: u16,
    pub source_background_width_words: u16,
    pub background_save_origin_instruction_file_offset: usize,
    pub background_save_width_instruction_file_offset: usize,
    pub background_restore_origin_instruction_file_offset: usize,
    pub background_restore_width_instruction_file_offset: usize,
    pub source_text_position: u16,
    pub text_position_instruction_file_offsets: Vec<usize>,
    pub text_renderer_com_address: u16,
}

pub(super) fn catalog_spring_recovery_window(
    program: &[u8],
    interface: &InterfaceTextCatalog,
) -> Result<SpringRecoveryWindowCatalog> {
    let window_frame_record_file_offset = require_frame_record(
        program,
        &[STATE_VALUE],
        FRAME_RECORD_COM_ADDRESS,
        SOURCE_FRAME_WORDS,
        "spring recovery window",
    )?;
    require_frame_invalidation_geometry(
        SOURCE_FRAME_WORDS[2],
        SOURCE_FRAME_WORDS[3],
        SOURCE_FRAME_WORDS[4],
        "spring recovery window",
    )?;
    require_frame_invalidation_consumer(program, "spring recovery window")?;
    require_background_transfer(
        program,
        0x51f4,
        Register16::SI,
        0x51f7,
        [0x33, 0xff],
        0x51f9,
        0x51fc,
        0x51ff,
        0x5322,
        "spring recovery background save",
    )?;
    require_background_transfer(
        program,
        0x516c,
        Register16::DI,
        0x516f,
        [0x33, 0xf6],
        0x5171,
        0x5174,
        0x5177,
        0x535e,
        "spring recovery background restore",
    )?;
    ensure!(
        SOURCE_FRAME_WORDS[0] + 2 == BACKGROUND_WIDTH_WORDS,
        "spring recovery frame and background widths diverged"
    );
    require_state_check(program, 0x5980, STATE_VALUE, "spring recovery window")?;

    let text_sites = [
        ("interface-text-079", 0x598e, 0x5991, 0x5996),
        ("interface-text-080", 0x599a, 0x599d, 0x59a2),
    ];
    for (entry_id, position_offset, address_offset, call_offset) in text_sites {
        require_mov_reg_imm(
            program,
            position_offset,
            Register16::DI,
            SOURCE_TEXT_POSITION,
            "spring recovery text position",
        )?;
        let entry = interface
            .entries
            .iter()
            .find(|entry| entry.id == entry_id)
            .with_context(|| format!("spring recovery entry {entry_id} is missing"))?;
        require_mov_reg_imm(
            program,
            address_offset,
            Register16::DX,
            u16::try_from(entry.com_address)?,
            "spring recovery text address",
        )?;
        ensure!(
            require_near_call(program, call_offset, "spring recovery renderer")?
                == TEXT_RENDERER_COM_ADDRESS,
            "spring recovery renderer target changed"
        );
    }

    Ok(SpringRecoveryWindowCatalog {
        source_text_entry_ids: text_sites
            .iter()
            .map(|(entry_id, ..)| (*entry_id).to_owned())
            .collect(),
        state_value: STATE_VALUE,
        window_frame_record_file_offset,
        window_frame_record_com_address: FRAME_RECORD_COM_ADDRESS,
        source_frame_inner_width_words: SOURCE_FRAME_WORDS[0],
        frame_inner_height_tiles: SOURCE_FRAME_WORDS[1],
        source_frame_origin: SOURCE_FRAME_WORDS[2],
        source_background_invalidation_x_pixels: SOURCE_FRAME_WORDS[3],
        source_background_invalidation_y_pixels: SOURCE_FRAME_WORDS[4],
        frame_interaction_table_offset: SOURCE_FRAME_WORDS[5],
        background_height_rows: BACKGROUND_HEIGHT_ROWS,
        source_background_width_words: BACKGROUND_WIDTH_WORDS,
        background_save_origin_instruction_file_offset: 0x51f4,
        background_save_width_instruction_file_offset: 0x51fc,
        background_restore_origin_instruction_file_offset: 0x516c,
        background_restore_width_instruction_file_offset: 0x5174,
        source_text_position: SOURCE_TEXT_POSITION,
        text_position_instruction_file_offsets: text_sites
            .iter()
            .map(|(_, position_offset, ..)| *position_offset)
            .collect(),
        text_renderer_com_address: TEXT_RENDERER_COM_ADDRESS,
    })
}

#[allow(clippy::too_many_arguments)]
fn require_background_transfer(
    program: &[u8],
    origin_offset: usize,
    origin_register: Register16,
    clear_offset: usize,
    expected_clear: [u8; 2],
    height_offset: usize,
    width_offset: usize,
    call_offset: usize,
    call_target: u16,
    role: &str,
) -> Result<()> {
    require_mov_reg_imm(
        program,
        origin_offset,
        origin_register,
        SOURCE_FRAME_WORDS[2],
        role,
    )?;
    ensure!(
        program.get(clear_offset..clear_offset + 2) == Some(expected_clear.as_slice()),
        "{role} buffer-origin clear changed"
    );
    require_mov_reg_imm(
        program,
        height_offset,
        Register16::CX,
        BACKGROUND_HEIGHT_ROWS,
        role,
    )?;
    require_mov_reg_imm(
        program,
        width_offset,
        Register16::DX,
        BACKGROUND_WIDTH_WORDS,
        role,
    )?;
    ensure!(
        require_near_call(program, call_offset, role)? == call_target,
        "{role} target changed"
    );
    Ok(())
}

#[cfg(test)]
#[path = "spring_recovery_window_tests.rs"]
mod spring_recovery_window_tests;
