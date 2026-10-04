use anyhow::{Context, Result, ensure};

use crate::korean_patch::font_catalog::FontTarget;
use crate::korean_patch::font_rasterizer::{FixedCellRasterizer, GLYPH_BYTES};

const FONT_CELL_SIZE: usize = 16;

#[derive(Debug, Eq, PartialEq)]
pub(super) struct TextCanvas {
    pub width: usize,
    pub height: usize,
    pixels: Vec<bool>,
}

impl TextCanvas {
    pub fn render(
        target: FontTarget,
        lines: &[String],
        width: usize,
        height: usize,
        scale: usize,
        border: bool,
    ) -> Result<Self> {
        ensure!(
            width > 0 && width.is_multiple_of(8) && height > 0 && scale > 0,
            "invalid baked-text canvas geometry"
        );
        ensure!(!lines.is_empty(), "baked-text canvas has no lines");
        let cell_size = FONT_CELL_SIZE * scale;
        let text_height = lines
            .len()
            .checked_mul(cell_size)
            .context("baked-text height overflow")?;
        ensure!(
            text_height <= height,
            "baked text needs {text_height} rows but the canvas has {height}"
        );
        let rasterizer = FixedCellRasterizer::load_for(target)?;
        let mut canvas = Self {
            width,
            height,
            pixels: vec![false; width * height],
        };
        if border {
            canvas.draw_border();
        }
        let origin_y = (height - text_height) / 2;
        for (line_index, line) in lines.iter().enumerate() {
            let characters = line.chars().collect::<Vec<_>>();
            ensure!(
                characters
                    .iter()
                    .all(|character| *character == ' ' || !character.is_whitespace()),
                "baked text contains unsupported whitespace"
            );
            let text_width = characters
                .len()
                .checked_mul(cell_size)
                .context("baked-text width overflow")?;
            ensure!(
                text_width <= width,
                "baked text needs {text_width} columns but the canvas has {width}"
            );
            let origin_x = (width - text_width) / 2;
            for (character_index, character) in characters.into_iter().enumerate() {
                if character == ' ' {
                    continue;
                }
                let glyph = rasterizer
                    .rasterize_visible_character(character)
                    .with_context(|| format!("failed to rasterize {character:?}"))?;
                canvas.draw_glyph(
                    &glyph,
                    origin_x + character_index * cell_size,
                    origin_y + line_index * cell_size,
                    scale,
                );
            }
        }
        Ok(canvas)
    }

    pub fn pixel(&self, x: usize, y: usize) -> bool {
        self.pixels[y * self.width + x]
    }

    fn draw_border(&mut self) {
        for x in 0..self.width {
            self.pixels[x] = true;
            self.pixels[(self.height - 1) * self.width + x] = true;
        }
        for y in 0..self.height {
            self.pixels[y * self.width] = true;
            self.pixels[y * self.width + self.width - 1] = true;
        }
    }

    fn draw_glyph(&mut self, glyph: &[u8; GLYPH_BYTES], x: usize, y: usize, scale: usize) {
        for source_y in 0..FONT_CELL_SIZE {
            for source_x in 0..FONT_CELL_SIZE {
                if glyph[source_y * 2 + source_x / 8] & (0x80 >> (source_x % 8)) == 0 {
                    continue;
                }
                for target_y in y + source_y * scale..y + (source_y + 1) * scale {
                    for target_x in x + source_x * scale..x + (source_x + 1) * scale {
                        self.pixels[target_y * self.width + target_x] = true;
                    }
                }
            }
        }
    }
}

pub(super) fn write_compact_brgi(
    target: &mut [u8],
    offset: usize,
    canvas: &TextCanvas,
    background_color: u8,
    foreground_color: u8,
) -> Result<()> {
    ensure!(
        background_color < 16 && foreground_color < 16,
        "B/R/G/I color index lies outside four planes"
    );
    let row_bytes = canvas.width / 8;
    let plane_size = row_bytes
        .checked_mul(canvas.height)
        .context("B/R/G/I plane size overflow")?;
    let end = offset
        .checked_add(plane_size * 4)
        .context("B/R/G/I target boundary overflow")?;
    let planes = target
        .get_mut(offset..end)
        .context("B/R/G/I target lies outside the decoded asset")?;
    planes.fill(0);
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let color = if canvas.pixel(x, y) {
                foreground_color
            } else {
                background_color
            };
            let byte = y * row_bytes + x / 8;
            let mask = 0x80 >> (x % 8);
            for plane in 0..4 {
                if color & (1 << plane) != 0 {
                    planes[plane * plane_size + byte] |= mask;
                }
            }
        }
    }
    Ok(())
}

pub(super) fn write_masked_brgi(
    target: &mut [u8],
    offset: usize,
    canvas: &TextCanvas,
    background_color: u8,
    foreground_color: u8,
) -> Result<()> {
    let plane_size = canvas.width / 8 * canvas.height;
    let end = offset
        .checked_add(plane_size * 5)
        .context("masked B/R/G/I target boundary overflow")?;
    let record = target
        .get_mut(offset..end)
        .context("masked B/R/G/I target lies outside the decoded asset")?;
    record[..plane_size].fill(0);
    write_compact_brgi(
        record,
        plane_size,
        canvas,
        background_color,
        foreground_color,
    )
}

#[cfg(test)]
#[path = "canvas_tests.rs"]
mod canvas_tests;
