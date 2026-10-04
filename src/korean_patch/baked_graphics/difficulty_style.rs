use anyhow::{Context, Result, ensure};

use crate::korean_patch::font_catalog::FontTarget;
use crate::korean_patch::font_rasterizer::{
    LARGE_GLYPH_BYTES, LARGE_GLYPH_HEIGHT, LARGE_GLYPH_WIDTH, LargeCellRasterizer,
};

const BACKGROUND_COLOR: u8 = 15;
const OUTLINE_COLOR: u8 = 4;
const SHADOW_COLOR: u8 = 8;
const ACCENT_COLOR: u8 = 11;
const FILL_COLOR: u8 = 15;
const REQUIRED_LABEL_COLORS: [u8; 4] = [FILL_COLOR, OUTLINE_COLOR, SHADOW_COLOR, ACCENT_COLOR];

#[derive(Debug)]
pub(super) struct DifficultyLabelCanvas {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl DifficultyLabelCanvas {
    pub(super) fn render(text: &str, width: usize) -> Result<Self> {
        ensure!(
            !text.is_empty() && width > 0 && width.is_multiple_of(16),
            "difficulty label canvas needs nonempty text and a tile-aligned width"
        );
        let characters = text.chars().collect::<Vec<_>>();
        let text_width = characters
            .len()
            .checked_mul(LARGE_GLYPH_WIDTH)
            .context("difficulty label width overflow")?;
        ensure!(
            text_width <= width,
            "difficulty label needs {text_width} columns but its writable tiles provide {width}"
        );

        let height = LARGE_GLYPH_HEIGHT;
        let mut mask = vec![false; width * height];
        let rasterizer = LargeCellRasterizer::load_for(FontTarget::Difficulty32)?;
        let origin_x = (width - text_width) / 2;
        for (character_index, character) in characters.into_iter().enumerate() {
            ensure!(
                !character.is_whitespace() && !character.is_control(),
                "difficulty label contains unsupported whitespace or control text"
            );
            let glyph = rasterizer
                .rasterize_visible_character(character)
                .with_context(|| {
                    format!("failed to rasterize difficulty character {character:?}")
                })?;
            draw_glyph_mask(
                &mut mask,
                width,
                origin_x + character_index * LARGE_GLYPH_WIDTH,
                &glyph,
            );
        }

        let mut pixels = vec![BACKGROUND_COLOR; width * height];
        paint_offset(&mut pixels, &mask, width, height, 2, 2, SHADOW_COLOR);
        paint_dilation(&mut pixels, &mask, width, height, 1, OUTLINE_COLOR);
        paint_offset(&mut pixels, &mask, width, height, -1, -1, ACCENT_COLOR);
        paint_offset(&mut pixels, &mask, width, height, 0, 0, FILL_COLOR);
        ensure!(
            REQUIRED_LABEL_COLORS
                .iter()
                .all(|color| pixels.contains(color)),
            "difficulty label did not retain its source letter palette roles"
        );
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    pub(super) fn write_tile(
        &self,
        target: &mut [u8],
        offset: usize,
        source_x: usize,
        source_y: usize,
    ) -> Result<()> {
        const TILE_SIZE: usize = 16;
        const TILE_ROW_BYTES: usize = TILE_SIZE / 8;
        const TILE_PLANE_BYTES: usize = TILE_ROW_BYTES * TILE_SIZE;
        ensure!(
            source_x + TILE_SIZE <= self.width && source_y + TILE_SIZE <= self.height,
            "difficulty label tile lies outside its styled canvas"
        );
        let record = target
            .get_mut(offset..offset + TILE_PLANE_BYTES * 4)
            .context("difficulty label tile lies outside SEL1.DAT")?;
        record.fill(0);
        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                let color = self.pixels[(source_y + y) * self.width + source_x + x];
                let byte = y * TILE_ROW_BYTES + x / 8;
                let mask = 0x80 >> (x % 8);
                for plane in 0..4 {
                    if color & (1 << plane) != 0 {
                        record[plane * TILE_PLANE_BYTES + byte] |= mask;
                    }
                }
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn colors(&self) -> std::collections::BTreeSet<u8> {
        self.pixels.iter().copied().collect()
    }
}

fn draw_glyph_mask(
    target: &mut [bool],
    width: usize,
    origin_x: usize,
    glyph: &[u8; LARGE_GLYPH_BYTES],
) {
    for y in 0..LARGE_GLYPH_HEIGHT {
        for x in 0..LARGE_GLYPH_WIDTH {
            if glyph[y * (LARGE_GLYPH_WIDTH / 8) + x / 8] & (0x80 >> (x % 8)) != 0 {
                target[y * width + origin_x + x] = true;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_dilation(
    target: &mut [u8],
    mask: &[bool],
    width: usize,
    height: usize,
    radius: isize,
    color: u8,
) {
    for y in 0..height {
        for x in 0..width {
            if !mask[y * width + x] {
                continue;
            }
            for offset_y in -radius..=radius {
                for offset_x in -radius..=radius {
                    paint_pixel(target, width, height, x, y, offset_x, offset_y, color);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_offset(
    target: &mut [u8],
    mask: &[bool],
    width: usize,
    height: usize,
    offset_x: isize,
    offset_y: isize,
    color: u8,
) {
    for y in 0..height {
        for x in 0..width {
            if mask[y * width + x] {
                paint_pixel(target, width, height, x, y, offset_x, offset_y, color);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_pixel(
    target: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    offset_x: isize,
    offset_y: isize,
    color: u8,
) {
    let Some(target_x) = x.checked_add_signed(offset_x) else {
        return;
    };
    let Some(target_y) = y.checked_add_signed(offset_y) else {
        return;
    };
    if target_x < width && target_y < height {
        target[target_y * width + target_x] = color;
    }
}

#[cfg(test)]
#[path = "difficulty_style_tests.rs"]
mod difficulty_style_tests;
