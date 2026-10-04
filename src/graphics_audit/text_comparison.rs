use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use super::pc98_font::{Pc98Font, RasterGlyph};
use super::planar::RgbImage;
use crate::game_data::{FixedGaijiTextSlot, GaijiCatalog, GaijiGlyphMeaning, GaijiTextCell};
use crate::korean_patch::{GLYPH_BYTES, SharedGaijiGlyph};

pub(super) type GlyphOverrides = BTreeMap<char, [u8; GLYPH_BYTES]>;

pub(super) fn source_gaiji_overrides(
    gaiji_com: &[u8],
    catalog: &GaijiCatalog,
) -> Result<GlyphOverrides> {
    let mut overrides = BTreeMap::new();
    for glyph in &catalog.glyphs {
        let GaijiGlyphMeaning::Character { character } = glyph.meaning else {
            continue;
        };
        let bitmap = read_gaiji_bitmap(gaiji_com, glyph.file_offset, glyph.byte_size)?;
        ensure!(
            overrides.insert(character, bitmap).is_none(),
            "source GAIJI character {character:?} has duplicate records"
        );
    }
    Ok(overrides)
}

pub(super) fn replacement_gaiji_overrides(
    program: &[u8],
    glyphs: &[SharedGaijiGlyph],
) -> Result<GlyphOverrides> {
    let mut overrides = BTreeMap::new();
    for glyph in glyphs {
        let mut characters = glyph.character.chars();
        let character = characters
            .next()
            .context("replacement GAIJI report has an empty character")?;
        ensure!(
            characters.next().is_none(),
            "replacement GAIJI report character is not one Unicode scalar"
        );
        let bitmap = read_gaiji_bitmap(program, glyph.record_offset, GLYPH_BYTES + 2)?;
        ensure!(
            overrides.insert(character, bitmap).is_none(),
            "replacement GAIJI character {character:?} has duplicate records"
        );
    }
    Ok(overrides)
}

pub(super) fn render_text_lines(
    lines: &[String],
    font: &Pc98Font,
    overrides: &GlyphOverrides,
) -> Result<RgbImage> {
    ensure!(!lines.is_empty(), "text comparison has no lines");
    let rasterized = lines
        .iter()
        .map(|line| {
            line.chars()
                .map(|character| {
                    if let Some(bitmap) = overrides.get(&character) {
                        Ok(RasterGlyph {
                            width: 16,
                            bitmap: *bitmap,
                        })
                    } else {
                        font.rasterize(character)
                    }
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let width = rasterized
        .iter()
        .map(|line| line.iter().map(|glyph| glyph.width).sum())
        .max()
        .unwrap_or(0);
    ensure!(width > 0, "text comparison has no visible columns");
    let height = rasterized.len() * 16;
    let mut image = RgbImage {
        width,
        height,
        pixels: vec![0; width * height * 3],
    };
    for (row, line) in rasterized.iter().enumerate() {
        let mut x = 0usize;
        for glyph in line {
            draw_glyph(&mut image, x, row * 16, glyph);
            x += glyph.width;
        }
    }
    Ok(image)
}

pub(super) fn render_fixed_gaiji_slot(
    slot: &FixedGaijiTextSlot,
    gaiji_com: &[u8],
    gaiji: &GaijiCatalog,
    font: &Pc98Font,
) -> Result<RgbImage> {
    ensure!(
        !slot.lines.is_empty() && slot.lines.iter().all(|line| !line.is_empty()),
        "fixed GAIJI comparison slot has no cells"
    );
    let columns = slot.lines.iter().map(Vec::len).max().unwrap_or(0);
    let mut image = RgbImage {
        width: columns * 16,
        height: slot.lines.len() * 16,
        pixels: vec![0; columns * 16 * slot.lines.len() * 16 * 3],
    };
    for (row, line) in slot.lines.iter().enumerate() {
        for (column, cell) in line.iter().enumerate() {
            let glyph = match cell {
                GaijiTextCell::Space => continue,
                GaijiTextCell::Standard { text } => {
                    let mut characters = text.chars();
                    let character = characters
                        .next()
                        .context("fixed GAIJI standard cell is empty")?;
                    ensure!(
                        characters.next().is_none(),
                        "fixed GAIJI standard cell contains multiple characters"
                    );
                    font.rasterize(character)?
                }
                GaijiTextCell::Glyph { glyph_index, .. } => {
                    let source = gaiji
                        .glyphs
                        .get(*glyph_index)
                        .context("fixed GAIJI cell references a missing glyph")?;
                    RasterGlyph {
                        width: 16,
                        bitmap: read_gaiji_bitmap(gaiji_com, source.file_offset, source.byte_size)?,
                    }
                }
            };
            draw_glyph(&mut image, column * 16, row * 16, &glyph);
        }
    }
    Ok(image)
}

fn read_gaiji_bitmap(program: &[u8], offset: usize, byte_size: usize) -> Result<[u8; GLYPH_BYTES]> {
    ensure!(
        byte_size == GLYPH_BYTES + 2,
        "GAIJI comparison record has an unexpected size"
    );
    let record = program
        .get(offset..offset + byte_size)
        .context("GAIJI comparison record lies outside its program")?;
    ensure!(
        record[..2] == [0, 0],
        "GAIJI comparison record has an unexpected prefix"
    );
    let mut bitmap = [0u8; GLYPH_BYTES];
    bitmap.copy_from_slice(&record[2..]);
    Ok(bitmap)
}

fn draw_glyph(image: &mut RgbImage, origin_x: usize, origin_y: usize, glyph: &RasterGlyph) {
    for y in 0..16 {
        for x in 0..glyph.width {
            if glyph.bitmap[y * 2 + x / 8] & (0x80 >> (x % 8)) != 0 {
                let pixel = ((origin_y + y) * image.width + origin_x + x) * 3;
                image.pixels[pixel..pixel + 3].fill(255);
            }
        }
    }
}

#[cfg(test)]
#[path = "text_comparison_tests.rs"]
mod text_comparison_tests;
