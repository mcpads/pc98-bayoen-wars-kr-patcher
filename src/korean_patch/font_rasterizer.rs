use anyhow::{Result, bail, ensure};
use fontdue::{Font, FontSettings};

use super::font_catalog::{FontTarget, font_provenance_for, load_font_profile};

pub(crate) use super::font_catalog::FontProvenance;

pub(crate) const GLYPH_WIDTH: usize = 16;
pub(crate) const GLYPH_HEIGHT: usize = 16;
pub(crate) const GLYPH_BYTES: usize = 32;
pub(crate) const LARGE_GLYPH_WIDTH: usize = 32;
pub(crate) const LARGE_GLYPH_HEIGHT: usize = 32;
pub(crate) const LARGE_GLYPH_BYTES: usize = 128;

pub(crate) struct CellRasterizer<const WIDTH: usize, const HEIGHT: usize, const BYTE_SIZE: usize> {
    font: Font,
    font_size: u16,
    baseline_y: u8,
    threshold: u8,
}

pub(crate) type FixedCellRasterizer = CellRasterizer<GLYPH_WIDTH, GLYPH_HEIGHT, GLYPH_BYTES>;
pub(crate) type LargeCellRasterizer =
    CellRasterizer<LARGE_GLYPH_WIDTH, LARGE_GLYPH_HEIGHT, LARGE_GLYPH_BYTES>;

pub(crate) fn font_provenance() -> Result<FontProvenance> {
    font_provenance_for(FontTarget::Body16)
}

pub(crate) fn rasterize_hangul_syllable(character: char) -> Result<[u8; GLYPH_BYTES]> {
    ensure!(
        ('\u{ac00}'..='\u{d7a3}').contains(&character),
        "font input {character:?} is not a modern Hangul syllable"
    );
    FixedCellRasterizer::load()?.rasterize_visible_character(character)
}

impl CellRasterizer<GLYPH_WIDTH, GLYPH_HEIGHT, GLYPH_BYTES> {
    pub(crate) fn load() -> Result<Self> {
        Self::load_for(FontTarget::Body16)
    }
}

impl<const WIDTH: usize, const HEIGHT: usize, const BYTE_SIZE: usize>
    CellRasterizer<WIDTH, HEIGHT, BYTE_SIZE>
{
    pub(crate) fn load_for(target: FontTarget) -> Result<Self> {
        ensure!(
            WIDTH == HEIGHT
                && WIDTH.is_multiple_of(8)
                && BYTE_SIZE == WIDTH / 8 * HEIGHT
                && target.cell_size() == WIDTH,
            "{} font target does not match the {WIDTH}x{HEIGHT} raster cell",
            target.id()
        );
        let loaded = load_font_profile(target)?;
        let font = Font::from_bytes(loaded.font_bytes, FontSettings::default())
            .map_err(|error| anyhow::anyhow!("failed to parse embedded font: {error}"))?;
        Ok(Self {
            font,
            font_size: loaded.profile.font_size,
            baseline_y: loaded.profile.baseline_y,
            threshold: loaded.profile.threshold,
        })
    }

    pub(crate) fn rasterize_visible_character(&self, character: char) -> Result<[u8; BYTE_SIZE]> {
        ensure!(
            !character.is_control() && !character.is_whitespace(),
            "font input {character:?} is not a visible character"
        );
        ensure!(
            self.font.lookup_glyph_index(character) != 0,
            "embedded font has no glyph for {character:?}"
        );
        let (metrics, coverage) = self.font.rasterize(character, f32::from(self.font_size));
        if metrics.width == 0 || metrics.height == 0 || coverage.is_empty() {
            bail!(
                "font rendered no pixels for {character:?} (U+{:04X})",
                character as u32
            );
        }
        ensure!(
            metrics.width <= WIDTH && metrics.height <= HEIGHT,
            "glyph {character:?} is {}x{} and does not fit {WIDTH}x{HEIGHT}",
            metrics.width,
            metrics.height
        );

        let left = (WIDTH - metrics.width) / 2;
        let baseline_top = i32::from(self.baseline_y) - (metrics.ymin + metrics.height as i32);
        let top = baseline_top.clamp(0, (HEIGHT - metrics.height) as i32);
        let bottom = top + metrics.height as i32;
        ensure!(
            top >= 0 && bottom <= HEIGHT as i32,
            "glyph {character:?} target rows {top}..{bottom} do not fit the fixed cell at baseline {}",
            self.baseline_y
        );

        let mut bitmap = [0_u8; BYTE_SIZE];
        for source_y in 0..metrics.height {
            for source_x in 0..metrics.width {
                if coverage[source_y * metrics.width + source_x] < self.threshold {
                    continue;
                }
                let x = left + source_x;
                let y = top as usize + source_y;
                bitmap[y * (WIDTH / 8) + x / 8] |= 1 << (7 - (x % 8));
            }
        }
        ensure!(
            bitmap.iter().any(|byte| *byte != 0),
            "font rendered an empty 1bpp glyph for {character:?}"
        );
        Ok(bitmap)
    }
}

#[cfg(test)]
#[path = "font_rasterizer_tests.rs"]
mod font_rasterizer_tests;
