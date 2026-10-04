use anyhow::{Result, ensure};

use super::planar::{PlanarScreen, RgbImage};

pub(super) fn render_compact_brgi_tiles(
    source: &[u8],
    width_bytes: usize,
    height: usize,
    columns: usize,
) -> Result<RgbImage> {
    let tile_size = width_bytes
        .checked_mul(height)
        .and_then(|plane_size| plane_size.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("B/R/G/I tile size overflow"))?;
    ensure!(
        !source.is_empty() && tile_size > 0 && source.len().is_multiple_of(tile_size),
        "B/R/G/I tile source is not a whole number of compact tiles"
    );
    let tile_width = width_bytes * 8;
    ensure!(
        columns > 0 && columns * tile_width <= 640,
        "invalid B/R/G/I tile sheet columns"
    );
    let tile_count = source.len() / tile_size;
    let rows = tile_count.div_ceil(columns);
    ensure!(
        rows * height <= 400,
        "B/R/G/I tile sheet exceeds one PC-98 screen"
    );
    let mut screen = PlanarScreen::new();

    for tile_index in 0..tile_count {
        let destination =
            (tile_index / columns) * height * 80 + (tile_index % columns) * width_bytes;
        screen.copy_compact_brgi(
            source,
            tile_index * tile_size,
            destination,
            width_bytes,
            height,
        )?;
    }
    Ok(screen.into_rgb())
}

#[cfg(test)]
#[path = "tile_sheet_tests.rs"]
mod tile_sheet_tests;
