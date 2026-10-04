use anyhow::{Context, Result};
use v30::Register16;

use super::model::{CompiledSpringCaptureWindow, SpringCaptureWindowPatchReport};
use crate::game_data::SpringCaptureWindowCatalog;
use crate::korean_patch::interface_text::CompiledInterfaceTextEntry;
use crate::korean_patch::interface_window_layout::{
    CompiledWindowTypedWrite, assemble_window_mov, center_interface_window,
    compile_centered_frame_metadata_writes, required_inner_width_words,
};

pub(in crate::korean_patch) fn compile_spring_capture_window(
    source: &SpringCaptureWindowCatalog,
    entries: &[CompiledInterfaceTextEntry],
) -> Result<CompiledSpringCaptureWindow> {
    let source_text_entry_ids = [source.source_text_entry_id.clone()];
    let required_inner_width_words = required_inner_width_words(entries, &source_text_entry_ids)?;
    let layout = center_interface_window(
        source.frame_inner_width_words,
        required_inner_width_words,
        source.screen_word_offset,
        source.text_position,
    )?;
    let output_background_width_words = layout
        .output_inner_width_words
        .checked_add(2)
        .context("spring capture background width overflowed")?;
    let typed_specs = [
        (
            "spring-capture-background-save-origin",
            "move the saved background with the centered capture frame",
            source.background_copy_setup_file_offset,
            Register16::SI,
            layout.output_frame_origin,
        ),
        (
            "spring-capture-background-save-width",
            "save the full expanded capture frame background",
            source.background_copy_width_instruction_file_offset,
            Register16::DX,
            output_background_width_words,
        ),
        (
            "spring-capture-background-restore-origin",
            "restore the background at the centered capture frame origin",
            source.background_restore_origin_instruction_file_offset,
            Register16::DI,
            layout.output_frame_origin,
        ),
        (
            "spring-capture-background-restore-width",
            "restore the full expanded capture frame background",
            source.background_restore_width_instruction_file_offset,
            Register16::DX,
            output_background_width_words,
        ),
        (
            "spring-capture-text-origin",
            "move capture text with its centered frame",
            source.text_position_instruction_file_offset,
            Register16::DI,
            layout.output_text_origin,
        ),
    ];
    let typed_writes = typed_specs
        .into_iter()
        .map(|(id, purpose, file_offset, register, output_value)| {
            Ok(CompiledWindowTypedWrite {
                source_id: id.to_owned(),
                write_id: id.to_owned(),
                purpose: purpose.to_owned(),
                file_offset,
                register,
                output_value,
                source: assemble_window_mov(file_offset, register, output_value)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let frame_width_file_offset = source.window_frame_record_file_offset;
    let frame_origin_file_offset = source.window_frame_record_file_offset + 4;
    let background_invalidation_x_file_offset = source.window_frame_record_file_offset + 6;
    let metadata_writes = compile_centered_frame_metadata_writes(
        "spring-capture",
        "capture",
        source.window_frame_record_file_offset,
        source.frame_inner_width_words,
        source.source_background_invalidation_x_pixels,
        &layout,
    )?;

    Ok(CompiledSpringCaptureWindow {
        report: SpringCaptureWindowPatchReport {
            source_text_entry_id: source.source_text_entry_id.clone(),
            required_inner_width_words,
            source_frame_inner_width_words: source.frame_inner_width_words,
            output_frame_inner_width_words: layout.output_inner_width_words,
            horizontal_shift_words: layout.horizontal_shift_words,
            source_frame_origin: source.screen_word_offset,
            output_frame_origin: layout.output_frame_origin,
            source_text_position: source.text_position,
            output_text_position: layout.output_text_origin,
            source_background_width_words: source.background_copy_width_words,
            output_background_width_words,
            background_height_rows: source.background_copy_height_rows,
            frame_width_file_offset,
            frame_origin_file_offset,
            source_background_invalidation_x_pixels: source.source_background_invalidation_x_pixels,
            output_background_invalidation_x_pixels: layout.output_frame_x_pixels,
            background_invalidation_x_file_offset,
            background_origin_instruction_file_offsets: vec![
                source.background_copy_setup_file_offset,
                source.background_restore_origin_instruction_file_offset,
            ],
            background_width_instruction_file_offsets: vec![
                source.background_copy_width_instruction_file_offset,
                source.background_restore_width_instruction_file_offset,
            ],
            text_position_instruction_file_offset: source.text_position_instruction_file_offset,
        },
        typed_writes,
        metadata_writes,
    })
}

#[cfg(test)]
#[path = "compile_tests.rs"]
mod compile_tests;
