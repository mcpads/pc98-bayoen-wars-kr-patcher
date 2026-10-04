use anyhow::{Result, ensure};

use crate::pc98_graphics::DIGITAL_RGBI_COLOR_COUNT;
use crate::title_screen::{TITLE_CONTENT_TRANSFERS, TITLE_SCREEN_HEIGHT, TITLE_SCREEN_WIDTH};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TitleTransferFitStats {
    pub(super) source_pixels_restored_outside_transfers: usize,
}

pub(super) fn preserve_source_outside_title_transfers(
    source_indices: &[u8],
    target_indices: &mut [u8],
    background_palette_indices: &[u8],
    trimmable_palette_indices: &[u8],
) -> Result<TitleTransferFitStats> {
    let screen_pixel_count = TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT;
    ensure!(
        source_indices.len() == screen_pixel_count && target_indices.len() == screen_pixel_count,
        "title transfer fitting requires two complete 640x400 indexed frames"
    );
    ensure!(
        !background_palette_indices.is_empty()
            && !trimmable_palette_indices.is_empty()
            && background_palette_indices
                .iter()
                .chain(trimmable_palette_indices)
                .all(|index| usize::from(*index) < DIGITAL_RGBI_COLOR_COUNT),
        "title transfer fitting palette roles must be nonempty subsets of the 16-color palette"
    );
    ensure!(
        background_palette_indices
            .iter()
            .all(|index| !trimmable_palette_indices.contains(index)),
        "title transfer fitting background and trimmable palette roles must be disjoint"
    );
    ensure!(
        source_indices
            .iter()
            .chain(target_indices.iter())
            .all(|index| usize::from(*index) < DIGITAL_RGBI_COLOR_COUNT),
        "title transfer fitting received a palette index above 15"
    );

    let mut offsets_to_restore = Vec::new();
    for y in 0..TITLE_SCREEN_HEIGHT {
        for x in 0..TITLE_SCREEN_WIDTH {
            if TITLE_CONTENT_TRANSFERS
                .iter()
                .any(|transfer| transfer.contains(x, y))
            {
                continue;
            }
            let offset = y * TITLE_SCREEN_WIDTH + x;
            let target = target_indices[offset];
            if background_palette_indices.contains(&target) {
                continue;
            }
            ensure!(
                trimmable_palette_indices.contains(&target),
                "title artwork has an essential palette index {target} outside its consumer transfers at ({x}, {y})"
            );
            offsets_to_restore.push(offset);
        }
    }

    for offset in &offsets_to_restore {
        target_indices[*offset] = source_indices[*offset];
    }

    Ok(TitleTransferFitStats {
        source_pixels_restored_outside_transfers: offsets_to_restore.len(),
    })
}

#[cfg(test)]
#[path = "title_art_transfer_fit_tests.rs"]
mod title_art_transfer_fit_tests;
