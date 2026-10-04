use anyhow::Result;
use v30::Register16;

use super::model::{CompiledSharedAlertWindow, SharedAlertWindowPatchReport};
use crate::game_data::SharedAlertWindowCatalog;
use crate::korean_patch::interface_text::CompiledInterfaceTextEntry;
use crate::korean_patch::interface_window_layout::{
    CompiledWindowTypedWrite, assemble_window_mov, center_interface_window,
    compile_centered_frame_metadata_writes, required_inner_width_words,
};

pub(in crate::korean_patch) fn compile_shared_alert_window(
    source: &SharedAlertWindowCatalog,
    entries: &[CompiledInterfaceTextEntry],
) -> Result<CompiledSharedAlertWindow> {
    compile_centered_text_window(source, entries, "shared-alert", 1)
}

pub(in crate::korean_patch) fn compile_centered_text_window(
    source: &SharedAlertWindowCatalog,
    entries: &[CompiledInterfaceTextEntry],
    role: &str,
    scale: u16,
) -> Result<CompiledSharedAlertWindow> {
    let required_inner_width_words =
        required_inner_width_words(entries, &source.source_text_entry_ids)? * scale;
    let layout = center_interface_window(
        source.source_frame_inner_width_words,
        required_inner_width_words,
        source.source_frame_origin,
        source.source_text_position,
    )?;
    let typed_writes = source
        .states
        .iter()
        .map(|state| {
            let source_id = format!("{role}-text-origin-state-{:02x}", state.state_value);
            Ok(CompiledWindowTypedWrite {
                write_id: source_id.clone(),
                source_id,
                purpose: format!(
                    "move state {:#04x} text with the centered shared alert frame",
                    state.state_value
                ),
                file_offset: state.text_position_instruction_file_offset,
                register: Register16::DI,
                output_value: layout.output_text_origin,
                source: assemble_window_mov(
                    state.text_position_instruction_file_offset,
                    Register16::DI,
                    layout.output_text_origin,
                )?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let frame_width_file_offset = source.window_frame_record_file_offset;
    let frame_origin_file_offset = source.window_frame_record_file_offset + 4;
    let background_invalidation_x_file_offset = source.window_frame_record_file_offset + 6;
    let metadata_writes = compile_centered_frame_metadata_writes(
        role,
        role,
        source.window_frame_record_file_offset,
        source.source_frame_inner_width_words,
        source.source_background_invalidation_x_pixels,
        &layout,
    )?;

    Ok(CompiledSharedAlertWindow {
        report: SharedAlertWindowPatchReport {
            source_text_entry_ids: source.source_text_entry_ids.clone(),
            required_inner_width_words,
            source_frame_inner_width_words: source.source_frame_inner_width_words,
            output_frame_inner_width_words: layout.output_inner_width_words,
            horizontal_shift_words: layout.horizontal_shift_words,
            source_frame_origin: source.source_frame_origin,
            output_frame_origin: layout.output_frame_origin,
            source_text_position: source.source_text_position,
            output_text_position: layout.output_text_origin,
            frame_width_file_offset,
            frame_origin_file_offset,
            source_background_invalidation_x_pixels: source.source_background_invalidation_x_pixels,
            output_background_invalidation_x_pixels: layout.output_frame_x_pixels,
            background_invalidation_x_file_offset,
            text_position_instruction_file_offsets: source
                .states
                .iter()
                .map(|state| state.text_position_instruction_file_offset)
                .collect(),
        },
        typed_writes,
        metadata_writes,
    })
}

#[cfg(test)]
#[path = "compile_tests.rs"]
mod compile_tests;
