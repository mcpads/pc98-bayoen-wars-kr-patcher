use std::collections::BTreeMap;

use anyhow::{Result, ensure};

use crate::korean_patch::font_rasterizer::{FixedCellRasterizer, GLYPH_BYTES, GLYPH_HEIGHT};

pub(crate) const DIALOGUE_GAIJI_PUNCTUATION: [char; 2] = ['~', '…'];

const ELLIPSIS_BASELINE_ROW: usize = 14;

pub(crate) fn dialogue_punctuation_overrides() -> Result<BTreeMap<char, [u8; GLYPH_BYTES]>> {
    let rasterizer = FixedCellRasterizer::load()?;
    let wave = rasterizer.rasterize_visible_character('~')?;
    let ellipsis = align_ink_bottom(
        rasterizer.rasterize_visible_character('…')?,
        ELLIPSIS_BASELINE_ROW,
    )?;
    Ok([('~', wave), ('…', ellipsis)].into_iter().collect())
}

fn align_ink_bottom(
    bitmap: [u8; GLYPH_BYTES],
    target_bottom_row: usize,
) -> Result<[u8; GLYPH_BYTES]> {
    let occupied_rows = (0..GLYPH_HEIGHT)
        .filter(|row| row_has_ink(&bitmap, *row))
        .collect::<Vec<_>>();
    let source_top = *occupied_rows
        .first()
        .expect("rasterizer rejects empty glyphs");
    let source_bottom = *occupied_rows
        .last()
        .expect("rasterizer rejects empty glyphs");
    ensure!(
        target_bottom_row < GLYPH_HEIGHT && target_bottom_row >= source_bottom,
        "punctuation baseline would move ink outside the fixed cell"
    );
    let shift = target_bottom_row - source_bottom;
    let mut aligned = [0_u8; GLYPH_BYTES];
    for source_row in source_top..=source_bottom {
        let target_row = source_row + shift;
        aligned[target_row * 2..target_row * 2 + 2]
            .copy_from_slice(&bitmap[source_row * 2..source_row * 2 + 2]);
    }
    Ok(aligned)
}

fn row_has_ink(bitmap: &[u8; GLYPH_BYTES], row: usize) -> bool {
    bitmap[row * 2..row * 2 + 2].iter().any(|byte| *byte != 0)
}

#[cfg(test)]
#[path = "punctuation_tests.rs"]
mod punctuation_tests;
