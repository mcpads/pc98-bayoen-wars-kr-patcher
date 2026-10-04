use anyhow::{Result, ensure};

use crate::title_screen::{TITLE_CONTENT_TRANSFERS, TITLE_SCREEN_HEIGHT, TITLE_SCREEN_WIDTH};

const MINIMUM_RELATIVE_SIZE_PERCENT: usize = 95;
const MAXIMUM_RELATIVE_SIZE_PERCENT: usize = 105;
const MAXIMUM_CENTER_DELTA_PIXELS: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct VisibleTitleBounds {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

pub(super) fn visible_title_bounds(
    indices: &[u8],
    background_palette_indices: &[u8],
) -> Result<VisibleTitleBounds> {
    ensure!(
        indices.len() == TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT,
        "title layout input must be a complete 640x400 color-index frame"
    );

    let mut minimum_x = TITLE_SCREEN_WIDTH;
    let mut minimum_y = TITLE_SCREEN_HEIGHT;
    let mut maximum_x = 0usize;
    let mut maximum_y = 0usize;
    let mut found = false;
    for y in 0..TITLE_SCREEN_HEIGHT {
        for x in 0..TITLE_SCREEN_WIDTH {
            let is_title_pixel = TITLE_CONTENT_TRANSFERS
                .iter()
                .any(|transfer| transfer.contains(x, y))
                && !background_palette_indices.contains(&indices[y * TITLE_SCREEN_WIDTH + x]);
            if is_title_pixel {
                minimum_x = minimum_x.min(x);
                minimum_y = minimum_y.min(y);
                maximum_x = maximum_x.max(x);
                maximum_y = maximum_y.max(y);
                found = true;
            }
        }
    }
    ensure!(found, "title layout has no visible content pixels");

    Ok(VisibleTitleBounds {
        x: minimum_x,
        y: minimum_y,
        width: maximum_x - minimum_x + 1,
        height: maximum_y - minimum_y + 1,
    })
}

pub(super) fn validate_replacement_title_bounds(
    source: VisibleTitleBounds,
    replacement: VisibleTitleBounds,
) -> Result<()> {
    ensure_relative_dimension("width", source.width, replacement.width)?;
    ensure_relative_dimension("height", source.height, replacement.height)?;

    let source_center_x_twice = source.x * 2 + source.width - 1;
    let source_center_y_twice = source.y * 2 + source.height - 1;
    let replacement_center_x_twice = replacement.x * 2 + replacement.width - 1;
    let replacement_center_y_twice = replacement.y * 2 + replacement.height - 1;
    ensure!(
        source_center_x_twice.abs_diff(replacement_center_x_twice)
            <= MAXIMUM_CENTER_DELTA_PIXELS * 2
            && source_center_y_twice.abs_diff(replacement_center_y_twice)
                <= MAXIMUM_CENTER_DELTA_PIXELS * 2,
        "replacement title visible center differs from the source by more than {MAXIMUM_CENTER_DELTA_PIXELS} pixels: source={source:?}, replacement={replacement:?}"
    );
    Ok(())
}

fn ensure_relative_dimension(role: &str, source: usize, replacement: usize) -> Result<()> {
    ensure!(
        replacement * 100 >= source * MINIMUM_RELATIVE_SIZE_PERCENT
            && replacement * 100 <= source * MAXIMUM_RELATIVE_SIZE_PERCENT,
        "replacement title visible {role} must remain within {MINIMUM_RELATIVE_SIZE_PERCENT}%..={MAXIMUM_RELATIVE_SIZE_PERCENT}% of the source"
    );
    Ok(())
}

#[cfg(test)]
#[path = "title_art_bounds_tests.rs"]
mod title_art_bounds_tests;
