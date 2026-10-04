use std::collections::VecDeque;

use anyhow::{Result, ensure};

use crate::pc98_graphics::DIGITAL_RGBI_COLOR_COUNT;
use crate::title_screen::{TITLE_CONTENT_TRANSFERS, TITLE_SCREEN_HEIGHT, TITLE_SCREEN_WIDTH};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TitleBackgroundTextureStats {
    pub(super) edge_connected_background_pixels_preserved: usize,
    pub(super) edge_connected_background_pixel_indices_restored: usize,
    pub(super) artwork_owned_background_pixels_retained: usize,
    pub(super) source_logo_pixels_replaced_with_background: usize,
}

pub(super) fn restore_edge_connected_background_texture(
    source_indices: &[u8],
    target_indices: &mut [u8],
    background_palette_indices: &[u8],
) -> Result<TitleBackgroundTextureStats> {
    let screen_pixel_count = TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT;
    ensure!(
        source_indices.len() == screen_pixel_count && target_indices.len() == screen_pixel_count,
        "title background preservation requires two complete 640x400 indexed frames"
    );
    ensure!(
        !background_palette_indices.is_empty()
            && background_palette_indices
                .iter()
                .all(|index| usize::from(*index) < DIGITAL_RGBI_COLOR_COUNT),
        "title background palette indices must be a nonempty subset of the 16-color palette"
    );
    ensure!(
        source_indices
            .iter()
            .chain(target_indices.iter())
            .all(|index| usize::from(*index) < DIGITAL_RGBI_COLOR_COUNT),
        "title background preservation received a palette index above 15"
    );

    // Palette role 0 is shared by the outer backdrop and enclosed logo shadows.
    // Connectivity keeps those two ownership domains distinct after quantization.
    let edge_connected_background =
        edge_connected_background_mask(target_indices, background_palette_indices);
    let mut edge_connected_background_pixels_preserved = 0usize;
    let mut edge_connected_background_pixel_indices_restored = 0usize;
    let mut artwork_owned_background_pixels_retained = 0usize;
    let mut source_logo_pixels_replaced_with_background = 0usize;
    for transfer in TITLE_CONTENT_TRANSFERS {
        for y in transfer.y()..transfer.y() + transfer.height {
            for x in transfer.x()..transfer.x() + transfer.width_bytes * 8 {
                let offset = y * TITLE_SCREEN_WIDTH + x;
                let source = source_indices[offset];
                let target = &mut target_indices[offset];
                let source_is_background = background_palette_indices.contains(&source);
                let target_is_background = background_palette_indices.contains(target);
                if source_is_background && target_is_background && edge_connected_background[offset]
                {
                    edge_connected_background_pixels_preserved += 1;
                    if source != *target {
                        *target = source;
                        edge_connected_background_pixel_indices_restored += 1;
                    }
                } else if source_is_background && target_is_background {
                    artwork_owned_background_pixels_retained += 1;
                } else if !source_is_background && target_is_background {
                    source_logo_pixels_replaced_with_background += 1;
                }
            }
        }
    }

    Ok(TitleBackgroundTextureStats {
        edge_connected_background_pixels_preserved,
        edge_connected_background_pixel_indices_restored,
        artwork_owned_background_pixels_retained,
        source_logo_pixels_replaced_with_background,
    })
}

fn edge_connected_background_mask(indices: &[u8], background_palette_indices: &[u8]) -> Vec<bool> {
    let mut connected = vec![false; indices.len()];
    let mut pending = VecDeque::new();
    for x in 0..TITLE_SCREEN_WIDTH {
        enqueue_background_pixel(
            x,
            0,
            indices,
            background_palette_indices,
            &mut connected,
            &mut pending,
        );
        enqueue_background_pixel(
            x,
            TITLE_SCREEN_HEIGHT - 1,
            indices,
            background_palette_indices,
            &mut connected,
            &mut pending,
        );
    }
    for y in 1..TITLE_SCREEN_HEIGHT - 1 {
        enqueue_background_pixel(
            0,
            y,
            indices,
            background_palette_indices,
            &mut connected,
            &mut pending,
        );
        enqueue_background_pixel(
            TITLE_SCREEN_WIDTH - 1,
            y,
            indices,
            background_palette_indices,
            &mut connected,
            &mut pending,
        );
    }

    while let Some((x, y)) = pending.pop_front() {
        if x > 0 {
            enqueue_background_pixel(
                x - 1,
                y,
                indices,
                background_palette_indices,
                &mut connected,
                &mut pending,
            );
        }
        if x + 1 < TITLE_SCREEN_WIDTH {
            enqueue_background_pixel(
                x + 1,
                y,
                indices,
                background_palette_indices,
                &mut connected,
                &mut pending,
            );
        }
        if y > 0 {
            enqueue_background_pixel(
                x,
                y - 1,
                indices,
                background_palette_indices,
                &mut connected,
                &mut pending,
            );
        }
        if y + 1 < TITLE_SCREEN_HEIGHT {
            enqueue_background_pixel(
                x,
                y + 1,
                indices,
                background_palette_indices,
                &mut connected,
                &mut pending,
            );
        }
    }
    connected
}

fn enqueue_background_pixel(
    x: usize,
    y: usize,
    indices: &[u8],
    background_palette_indices: &[u8],
    connected: &mut [bool],
    pending: &mut VecDeque<(usize, usize)>,
) {
    let offset = y * TITLE_SCREEN_WIDTH + x;
    if !connected[offset] && background_palette_indices.contains(&indices[offset]) {
        connected[offset] = true;
        pending.push_back((x, y));
    }
}

#[cfg(test)]
#[path = "title_background_texture_tests.rs"]
mod title_background_texture_tests;
