use anyhow::{Context, Result};
use v30::Register16;

use super::model::{CompiledSpringRecoveryWindow, SpringRecoveryWindowPatchReport};
use crate::game_data::SpringRecoveryWindowCatalog;
use crate::korean_patch::interface_text::CompiledInterfaceTextEntry;
use crate::korean_patch::interface_window_layout::{
    CompiledWindowTypedWrite, assemble_window_mov, center_interface_window,
    compile_centered_frame_metadata_writes, required_inner_width_words,
};

pub(in crate::korean_patch) fn compile_spring_recovery_window(
    source: &SpringRecoveryWindowCatalog,
    entries: &[CompiledInterfaceTextEntry],
) -> Result<CompiledSpringRecoveryWindow> {
    let required_inner_width_words =
        required_inner_width_words(entries, &source.source_text_entry_ids)?;
    let layout = center_interface_window(
        source.source_frame_inner_width_words,
        required_inner_width_words,
        source.source_frame_origin,
        source.source_text_position,
    )?;
    let output_background_width_words = layout
        .output_inner_width_words
        .checked_add(2)
        .context("spring recovery background width overflowed")?;
    let mut typed_writes = Vec::new();
    for (source_id, write_id, purpose, file_offset, register, value) in [
        (
            "spring-recovery-background-save-origin",
            "spring-recovery-background-save-origin",
            "move the saved background with the centered recovery frame",
            source.background_save_origin_instruction_file_offset,
            Register16::SI,
            layout.output_frame_origin,
        ),
        (
            "spring-recovery-background-save-width",
            "spring-recovery-background-save-width",
            "save the full expanded recovery frame background",
            source.background_save_width_instruction_file_offset,
            Register16::DX,
            output_background_width_words,
        ),
        (
            "spring-recovery-background-restore-origin",
            "spring-recovery-background-restore-origin",
            "restore the background at the expanded recovery frame origin",
            source.background_restore_origin_instruction_file_offset,
            Register16::DI,
            layout.output_frame_origin,
        ),
        (
            "spring-recovery-background-restore-width",
            "spring-recovery-background-restore-width",
            "restore the full expanded recovery frame background",
            source.background_restore_width_instruction_file_offset,
            Register16::DX,
            output_background_width_words,
        ),
    ] {
        typed_writes.push(CompiledWindowTypedWrite {
            source_id: source_id.to_owned(),
            write_id: write_id.to_owned(),
            purpose: purpose.to_owned(),
            file_offset,
            register,
            output_value: value,
            source: assemble_window_mov(file_offset, register, value)?,
        });
    }
    for (index, file_offset) in source
        .text_position_instruction_file_offsets
        .iter()
        .copied()
        .enumerate()
    {
        let source_id = format!("spring-recovery-text-origin-{index}");
        typed_writes.push(CompiledWindowTypedWrite {
            write_id: source_id.clone(),
            source_id,
            purpose: "move recovery text with its centered frame".to_owned(),
            file_offset,
            register: Register16::DI,
            output_value: layout.output_text_origin,
            source: assemble_window_mov(file_offset, Register16::DI, layout.output_text_origin)?,
        });
    }
    let frame_width_file_offset = source.window_frame_record_file_offset;
    let frame_origin_file_offset = source.window_frame_record_file_offset + 4;
    let background_invalidation_x_file_offset = source.window_frame_record_file_offset + 6;
    let metadata_writes = compile_centered_frame_metadata_writes(
        "spring-recovery",
        "recovery",
        source.window_frame_record_file_offset,
        source.source_frame_inner_width_words,
        source.source_background_invalidation_x_pixels,
        &layout,
    )?;

    Ok(CompiledSpringRecoveryWindow {
        report: SpringRecoveryWindowPatchReport {
            source_text_entry_ids: source.source_text_entry_ids.clone(),
            required_inner_width_words,
            source_frame_inner_width_words: source.source_frame_inner_width_words,
            output_frame_inner_width_words: layout.output_inner_width_words,
            horizontal_shift_words: layout.horizontal_shift_words,
            source_frame_origin: source.source_frame_origin,
            output_frame_origin: layout.output_frame_origin,
            source_text_position: source.source_text_position,
            output_text_position: layout.output_text_origin,
            source_background_width_words: source.source_background_width_words,
            output_background_width_words,
            background_height_rows: source.background_height_rows,
            frame_width_file_offset,
            frame_origin_file_offset,
            source_background_invalidation_x_pixels: source.source_background_invalidation_x_pixels,
            output_background_invalidation_x_pixels: layout.output_frame_x_pixels,
            background_invalidation_x_file_offset,
            background_origin_instruction_file_offsets: vec![
                source.background_save_origin_instruction_file_offset,
                source.background_restore_origin_instruction_file_offset,
            ],
            background_width_instruction_file_offsets: vec![
                source.background_save_width_instruction_file_offset,
                source.background_restore_width_instruction_file_offset,
            ],
            text_position_instruction_file_offsets: source
                .text_position_instruction_file_offsets
                .clone(),
        },
        typed_writes,
        metadata_writes,
    })
}

#[cfg(test)]
#[path = "compile_tests.rs"]
mod compile_tests;
