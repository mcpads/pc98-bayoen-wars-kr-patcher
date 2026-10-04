use anyhow::{Context, Result, ensure};

use super::contact_sheet::compose_contact_sheet;
use super::planar::RgbImage;

pub(super) fn render_monochrome_sheet(
    source: &[u8],
    tile_width: usize,
    tile_height: usize,
    columns: usize,
) -> Result<RgbImage> {
    ensure!(
        tile_width > 0 && tile_width.is_multiple_of(8) && tile_height > 0 && columns > 0,
        "invalid monochrome tile-sheet geometry"
    );
    let row_bytes = tile_width / 8;
    let tile_size = row_bytes
        .checked_mul(tile_height)
        .context("monochrome tile size overflow")?;
    ensure!(
        !source.is_empty() && source.len().is_multiple_of(tile_size),
        "monochrome source ends inside a tile"
    );
    let tile_count = source.len() / tile_size;
    let used_columns = columns.min(tile_count);
    let rows = tile_count.div_ceil(columns);
    let width = used_columns * tile_width;
    let height = rows * tile_height;
    let mut pixels = vec![0; width * height * 3];

    for tile_index in 0..tile_count {
        let tile_x = (tile_index % columns) * tile_width;
        let tile_y = (tile_index / columns) * tile_height;
        let tile_start = tile_index * tile_size;
        for y in 0..tile_height {
            for x in 0..tile_width {
                let byte = source[tile_start + y * row_bytes + x / 8];
                if byte & (0x80 >> (x % 8)) == 0 {
                    continue;
                }
                let pixel = ((tile_y + y) * width + tile_x + x) * 3;
                pixels[pixel..pixel + 3].fill(255);
            }
        }
    }

    Ok(RgbImage {
        width,
        height,
        pixels,
    })
}

pub(super) fn render_monochrome_contact_sheet(
    source: &[u8],
    tile_width: usize,
    tile_height: usize,
    columns: usize,
    gap: usize,
) -> Result<RgbImage> {
    ensure!(columns > 0, "monochrome contact-sheet column count is zero");
    let row_bytes = tile_width / 8;
    let tile_size = row_bytes
        .checked_mul(tile_height)
        .context("monochrome contact-sheet tile size overflow")?;
    ensure!(
        tile_width > 0
            && tile_width.is_multiple_of(8)
            && tile_height > 0
            && !source.is_empty()
            && source.len().is_multiple_of(tile_size),
        "invalid monochrome contact-sheet source geometry"
    );
    let cells = source
        .chunks_exact(tile_size)
        .map(|tile| render_monochrome_sheet(tile, tile_width, tile_height, 1))
        .collect::<Result<Vec<_>>>()?;
    let maximum_width = columns * tile_width + (columns - 1) * gap;
    compose_contact_sheet(cells, maximum_width, gap)
}

#[cfg(test)]
#[path = "monochrome_sheet_tests.rs"]
mod monochrome_sheet_tests;
