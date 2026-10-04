use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::Register16;

use super::InterfaceTextCatalog;
use super::interface_window_binary::{
    require_frame_invalidation_consumer, require_frame_invalidation_geometry, require_frame_record,
    require_mov_reg_imm, require_near_call, require_state_check,
};

const FRAME_RECORD_COM_ADDRESS: u16 = 0x6cb8;
const SOURCE_FRAME_WORDS: [u16; 6] = [0x0007, 0x0002, 0x2d18, 0x00c0, 0x0090, 0x019a];
const SOURCE_TEXT_POSITION: u16 = 0x321a;
const TEXT_RENDERER_COM_ADDRESS: u16 = 0x5728;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SharedAlertWindowCatalog {
    pub source_text_entry_ids: Vec<String>,
    pub states: Vec<SharedAlertWindowState>,
    pub window_frame_record_file_offset: usize,
    pub window_frame_record_com_address: u16,
    pub source_frame_inner_width_words: u16,
    pub frame_inner_height_tiles: u16,
    pub source_frame_origin: u16,
    pub source_background_invalidation_x_pixels: u16,
    pub source_background_invalidation_y_pixels: u16,
    pub frame_interaction_table_offset: u16,
    pub source_text_position: u16,
    pub text_renderer_com_address: u16,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SharedAlertWindowState {
    pub state_value: u8,
    pub source_text_entry_id: String,
    pub text_position_instruction_file_offset: usize,
    pub text_address_instruction_file_offset: usize,
}

pub(super) fn catalog_shared_alert_window(
    program: &[u8],
    interface: &InterfaceTextCatalog,
) -> Result<SharedAlertWindowCatalog> {
    let sites = shared_alert_sites();
    let state_values = sites
        .iter()
        .map(|site| site.state_value)
        .collect::<Vec<_>>();
    let window_frame_record_file_offset = require_frame_record(
        program,
        &state_values,
        FRAME_RECORD_COM_ADDRESS,
        SOURCE_FRAME_WORDS,
        "shared alert window",
    )?;
    require_frame_invalidation_geometry(
        SOURCE_FRAME_WORDS[2],
        SOURCE_FRAME_WORDS[3],
        SOURCE_FRAME_WORDS[4],
        "shared alert window",
    )?;
    require_frame_invalidation_consumer(program, "shared alert window")?;
    for site in &sites {
        require_state_check(
            program,
            site.state_check_file_offset,
            site.state_value,
            "shared alert window",
        )?;
        require_mov_reg_imm(
            program,
            site.text_position_instruction_file_offset,
            Register16::DI,
            SOURCE_TEXT_POSITION,
            "shared alert text position",
        )?;
        let entry = interface
            .entries
            .iter()
            .find(|entry| entry.id == site.source_text_entry_id)
            .with_context(|| {
                format!(
                    "shared alert entry {} is missing",
                    site.source_text_entry_id
                )
            })?;
        require_mov_reg_imm(
            program,
            site.text_address_instruction_file_offset,
            Register16::DX,
            u16::try_from(entry.com_address)?,
            "shared alert text address",
        )?;
        ensure!(
            require_near_call(
                program,
                site.text_renderer_call_file_offset,
                "shared alert renderer"
            )? == TEXT_RENDERER_COM_ADDRESS,
            "shared alert renderer target changed"
        );
    }

    Ok(SharedAlertWindowCatalog {
        source_text_entry_ids: sites
            .iter()
            .map(|site| site.source_text_entry_id.to_owned())
            .collect(),
        states: sites
            .into_iter()
            .map(|site| SharedAlertWindowState {
                state_value: site.state_value,
                source_text_entry_id: site.source_text_entry_id.to_owned(),
                text_position_instruction_file_offset: site.text_position_instruction_file_offset,
                text_address_instruction_file_offset: site.text_address_instruction_file_offset,
            })
            .collect(),
        window_frame_record_file_offset,
        window_frame_record_com_address: FRAME_RECORD_COM_ADDRESS,
        source_frame_inner_width_words: SOURCE_FRAME_WORDS[0],
        frame_inner_height_tiles: SOURCE_FRAME_WORDS[1],
        source_frame_origin: SOURCE_FRAME_WORDS[2],
        source_background_invalidation_x_pixels: SOURCE_FRAME_WORDS[3],
        source_background_invalidation_y_pixels: SOURCE_FRAME_WORDS[4],
        frame_interaction_table_offset: SOURCE_FRAME_WORDS[5],
        source_text_position: SOURCE_TEXT_POSITION,
        text_renderer_com_address: TEXT_RENDERER_COM_ADDRESS,
    })
}

#[derive(Clone, Copy)]
struct SharedAlertSite {
    state_value: u8,
    source_text_entry_id: &'static str,
    state_check_file_offset: usize,
    text_position_instruction_file_offset: usize,
    text_address_instruction_file_offset: usize,
    text_renderer_call_file_offset: usize,
}

fn shared_alert_sites() -> Vec<SharedAlertSite> {
    vec![
        SharedAlertSite {
            state_value: 0x0e,
            source_text_entry_id: "interface-text-003",
            state_check_file_offset: 0x5793,
            text_position_instruction_file_offset: 0x579a,
            text_address_instruction_file_offset: 0x579d,
            text_renderer_call_file_offset: 0x57a2,
        },
        SharedAlertSite {
            state_value: 0x10,
            source_text_entry_id: "interface-text-007",
            state_check_file_offset: 0x57a6,
            text_position_instruction_file_offset: 0x57ad,
            text_address_instruction_file_offset: 0x57b0,
            text_renderer_call_file_offset: 0x57b5,
        },
        SharedAlertSite {
            state_value: 0x11,
            source_text_entry_id: "interface-text-008",
            state_check_file_offset: 0x57b9,
            text_position_instruction_file_offset: 0x57c0,
            text_address_instruction_file_offset: 0x57c3,
            text_renderer_call_file_offset: 0x57c8,
        },
        SharedAlertSite {
            state_value: 0x1f,
            source_text_entry_id: "interface-text-012",
            state_check_file_offset: 0x582a,
            text_position_instruction_file_offset: 0x5831,
            text_address_instruction_file_offset: 0x5834,
            text_renderer_call_file_offset: 0x5839,
        },
        SharedAlertSite {
            state_value: 0x16,
            source_text_entry_id: "interface-text-011",
            state_check_file_offset: 0x592b,
            text_position_instruction_file_offset: 0x5932,
            text_address_instruction_file_offset: 0x5935,
            text_renderer_call_file_offset: 0x593a,
        },
    ]
}

#[cfg(test)]
#[path = "shared_alert_window_tests.rs"]
mod shared_alert_window_tests;
