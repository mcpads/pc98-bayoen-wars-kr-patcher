use anyhow::{Context, Result, ensure};

use super::interface_text::CompiledInterfaceTextEntry;

mod frame_record;
mod writes;

pub(in crate::korean_patch) use frame_record::compile_centered_frame_metadata_writes;
pub(in crate::korean_patch) use writes::{
    CompiledWindowMetadataWrite, CompiledWindowTypedWrite, add_window_layout_sources,
    add_window_layout_writes, assemble_window_mov, verify_window_layout_writes,
};

const GRAPHICS_ROW_BYTES: usize = 80;
const SCREEN_WORD_BYTES: usize = 2;
const FRAME_BORDER_WORDS: usize = 2;

#[derive(Debug, Eq, PartialEq)]
pub(in crate::korean_patch) struct CenteredInterfaceWindowLayout {
    pub required_inner_width_words: u16,
    pub output_inner_width_words: u16,
    pub horizontal_shift_words: u16,
    pub output_frame_origin: u16,
    pub output_text_origin: u16,
    pub output_frame_x_pixels: u16,
}

pub(in crate::korean_patch) fn required_inner_width_words(
    entries: &[CompiledInterfaceTextEntry],
    source_text_entry_ids: &[String],
) -> Result<u16> {
    let mut required_screen_bytes = 0;
    for id in source_text_entry_ids {
        let entry = entries
            .iter()
            .find(|entry| entry.id == *id)
            .with_context(|| format!("interface window is missing compiled entry {id}"))?;
        let entry_width = entry
            .line_screen_byte_widths
            .iter()
            .copied()
            .max()
            .context("interface window entry has no compiled lines")?;
        required_screen_bytes = required_screen_bytes.max(entry_width);
    }
    ensure!(
        required_screen_bytes > 0,
        "interface window has no visible compiled text"
    );
    u16::try_from(required_screen_bytes.div_ceil(SCREEN_WORD_BYTES))
        .context("interface window text width exceeds 16 bits")
}

pub(in crate::korean_patch) fn center_interface_window(
    source_inner_width_words: u16,
    required_inner_width_words: u16,
    source_frame_origin: u16,
    source_text_origin: u16,
) -> Result<CenteredInterfaceWindowLayout> {
    let mut output_inner_width_words = source_inner_width_words.max(required_inner_width_words);
    if !(output_inner_width_words - source_inner_width_words).is_multiple_of(2) {
        output_inner_width_words = output_inner_width_words
            .checked_add(1)
            .context("interface window symmetric width overflowed")?;
    }
    let horizontal_shift_words = (output_inner_width_words - source_inner_width_words) / 2;
    let horizontal_shift_bytes = usize::from(horizontal_shift_words)
        .checked_mul(SCREEN_WORD_BYTES)
        .context("interface window horizontal shift overflowed")?;
    let output_frame_origin = usize::from(source_frame_origin)
        .checked_sub(horizontal_shift_bytes)
        .context("interface window frame cannot remain centered")?;
    let output_text_origin = usize::from(source_text_origin)
        .checked_sub(horizontal_shift_bytes)
        .context("interface window text cannot remain centered")?;
    ensure!(
        output_frame_origin / GRAPHICS_ROW_BYTES
            == usize::from(source_frame_origin) / GRAPHICS_ROW_BYTES
            && output_text_origin / GRAPHICS_ROW_BYTES
                == usize::from(source_text_origin) / GRAPHICS_ROW_BYTES,
        "interface window horizontal shift crossed a graphics row"
    );
    let frame_left = output_frame_origin % GRAPHICS_ROW_BYTES;
    let text_left = output_text_origin % GRAPHICS_ROW_BYTES;
    let frame_width_bytes = (usize::from(output_inner_width_words) + FRAME_BORDER_WORDS)
        .checked_mul(SCREEN_WORD_BYTES)
        .context("interface window frame width overflowed")?;
    ensure!(
        frame_left + frame_width_bytes <= GRAPHICS_ROW_BYTES,
        "interface window exceeds the graphics row"
    );
    ensure!(
        text_left == frame_left + SCREEN_WORD_BYTES,
        "interface text and frame left edges diverged"
    );

    Ok(CenteredInterfaceWindowLayout {
        required_inner_width_words,
        output_inner_width_words,
        horizontal_shift_words,
        output_frame_origin: u16::try_from(output_frame_origin)?,
        output_text_origin: u16::try_from(output_text_origin)?,
        output_frame_x_pixels: u16::try_from(
            frame_left
                .checked_mul(8)
                .context("interface window frame X coordinate overflowed")?,
        )?,
    })
}

#[cfg(test)]
#[path = "interface_window_layout_tests.rs"]
mod interface_window_layout_tests;
