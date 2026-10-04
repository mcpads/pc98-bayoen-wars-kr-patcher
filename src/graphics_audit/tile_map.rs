use anyhow::{Context, Result, ensure};

use super::planar::{RgbImage, render_strided_brgi};

pub(super) fn render_compact_brgi_tile_map(
    tiles: &[u8],
    tile_width: usize,
    tile_height: usize,
    map: &[u8],
    map_width_tiles: usize,
    map_height_tiles: usize,
) -> Result<RgbImage> {
    ensure!(
        tile_width > 0 && tile_width.is_multiple_of(8) && tile_height > 0,
        "invalid compact tile dimensions"
    );
    let entry_count = map_width_tiles
        .checked_mul(map_height_tiles)
        .context("tile-map entry count overflow")?;
    ensure!(
        map.len() == entry_count && entry_count > 0,
        "tile-map entry count does not match its dimensions"
    );
    let width_bytes = tile_width / 8;
    let plane_size = width_bytes
        .checked_mul(tile_height)
        .context("tile plane size overflow")?;
    let tile_size = plane_size * 4;
    ensure!(
        tiles.len().is_multiple_of(tile_size),
        "compact tile set ends inside a tile"
    );
    let tile_count = tiles.len() / tile_size;
    let width = map_width_tiles * tile_width;
    let height = map_height_tiles * tile_height;
    let mut pixels = vec![0; width * height * 3];

    for (map_index, tile_index) in map.iter().copied().map(usize::from).enumerate() {
        ensure!(
            tile_index < tile_count,
            "tile map references missing tile {tile_index}"
        );
        let source_start = tile_index * tile_size;
        let tile = render_strided_brgi(
            &tiles[source_start..source_start + tile_size],
            tile_width,
            tile_height,
            width_bytes,
            plane_size,
        )?;
        let destination_x = (map_index % map_width_tiles) * tile_width;
        let destination_y = (map_index / map_width_tiles) * tile_height;
        for row in 0..tile_height {
            let source = row * tile_width * 3;
            let destination = ((destination_y + row) * width + destination_x) * 3;
            pixels[destination..destination + tile_width * 3]
                .copy_from_slice(&tile.pixels[source..source + tile_width * 3]);
        }
    }

    Ok(RgbImage {
        width,
        height,
        pixels,
    })
}

#[cfg(test)]
#[path = "tile_map_tests.rs"]
mod tile_map_tests;
