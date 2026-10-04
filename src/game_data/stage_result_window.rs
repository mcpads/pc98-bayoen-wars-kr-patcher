use super::interface_window_binary::{
    require_frame_invalidation_consumer, require_frame_invalidation_geometry, require_frame_record,
    require_mov_reg_imm, require_near_call, require_state_check,
};
use super::{InterfaceTextCatalog, SharedAlertWindowCatalog, SharedAlertWindowState};
use anyhow::{Context, Result, ensure};
use v30::Register16;

pub(super) fn catalog_stage_result_window(
    program: &[u8],
    interface: &InterfaceTextCatalog,
) -> Result<SharedAlertWindowCatalog> {
    let words = [14, 2, 0x320e, 0x70, 0xa0, 0x1ca];
    let offset =
        require_frame_record(program, &[0x12, 0x13], 0x6ce8, words, "stage result window")?;
    require_frame_invalidation_geometry(words[2], words[3], words[4], "stage result window")?;
    require_frame_invalidation_consumer(program, "stage result window")?;
    let mut states = Vec::new();
    for (state, check, position, address, call, id) in [
        (0x12, 0x57cc, 0x57db, 0x57de, 0x57e3, "interface-text-009"),
        (0x13, 0x57ec, 0x57fb, 0x57fe, 0x5803, "interface-text-010"),
    ] {
        require_state_check(program, check, state, "stage result")?;
        ensure!(
            program.get(check + 7..check + 12) == Some(&[0xc6, 0x06, 0x03, 0x34, 0x01]),
            "stage result double-width renderer mode changed"
        );
        require_mov_reg_imm(
            program,
            position,
            Register16::DI,
            0x3710,
            "stage result origin",
        )?;
        let entry = interface
            .entries
            .iter()
            .find(|e| e.id == id)
            .context("stage result interface entry missing")?;
        require_mov_reg_imm(
            program,
            address,
            Register16::DX,
            u16::try_from(entry.com_address)?,
            "stage result text",
        )?;
        ensure!(
            require_near_call(program, call, "stage result renderer")? == 0x5728,
            "stage result renderer changed"
        );
        states.push(SharedAlertWindowState {
            state_value: state,
            source_text_entry_id: id.to_owned(),
            text_position_instruction_file_offset: position,
            text_address_instruction_file_offset: address,
        });
    }
    Ok(SharedAlertWindowCatalog {
        source_text_entry_ids: states
            .iter()
            .map(|s| s.source_text_entry_id.clone())
            .collect(),
        states,
        window_frame_record_file_offset: offset,
        window_frame_record_com_address: 0x6ce8,
        source_frame_inner_width_words: words[0],
        frame_inner_height_tiles: words[1],
        source_frame_origin: words[2],
        source_background_invalidation_x_pixels: words[3],
        source_background_invalidation_y_pixels: words[4],
        frame_interaction_table_offset: words[5],
        source_text_position: 0x3710,
        text_renderer_com_address: 0x5728,
    })
}
