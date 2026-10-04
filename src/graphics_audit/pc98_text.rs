use std::collections::BTreeMap;

use anyhow::{Result, ensure};

use super::planar::RgbImage;
use crate::korean_patch::{FixedCellRasterizer, GLYPH_BYTES};

pub(super) const CELL_SIZE: usize = 16;

pub(super) fn render_fixed_cell_lines(lines: &[String]) -> Result<RgbImage> {
    render_fixed_cell_lines_with_overrides(lines, &BTreeMap::new())
}

pub(super) fn render_fixed_cell_lines_with_overrides(
    lines: &[String],
    glyph_overrides: &BTreeMap<char, [u8; GLYPH_BYTES]>,
) -> Result<RgbImage> {
    ensure!(!lines.is_empty(), "fixed-cell preview has no lines");
    let column_count = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    ensure!(column_count > 0, "fixed-cell preview has no visible cells");
    let mut image = RgbImage {
        width: column_count * CELL_SIZE,
        height: lines.len() * CELL_SIZE,
        pixels: vec![0; column_count * CELL_SIZE * lines.len() * CELL_SIZE * 3],
    };
    let rasterizer = FixedCellRasterizer::load()?;
    for (row, line) in lines.iter().enumerate() {
        for (column, character) in line.chars().enumerate() {
            if character == ' ' {
                continue;
            }
            let bitmap = if let Some(bitmap) = glyph_overrides.get(&character) {
                *bitmap
            } else {
                rasterizer.rasterize_visible_character(character)?
            };
            draw_bitmap(&mut image, column * CELL_SIZE, row * CELL_SIZE, &bitmap);
        }
    }
    Ok(image)
}

pub(super) fn draw_bitmap(
    image: &mut RgbImage,
    origin_x: usize,
    origin_y: usize,
    bitmap: &[u8; GLYPH_BYTES],
) {
    for y in 0..CELL_SIZE {
        for x in 0..CELL_SIZE {
            if bitmap[y * 2 + x / 8] & (0x80 >> (x % 8)) != 0 {
                let pixel = ((origin_y + y) * image.width + origin_x + x) * 3;
                image.pixels[pixel..pixel + 3].fill(255);
            }
        }
    }
}

#[cfg(test)]
#[path = "pc98_text_tests.rs"]
mod pc98_text_tests;
