use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use super::Preview;
use super::decoded::decode_single_asset;
use super::planar::RgbImage;
use crate::asset_bindings::{
    MonochromeTextPage, catalog_monochrome_sprites, catalog_monochrome_text,
};

const GLYPH_WIDTH: usize = 32;
const GLYPH_HEIGHT: usize = 32;
const GLYPH_ROW_BYTES: usize = GLYPH_WIDTH / 8;
const GLYPH_SIZE: usize = GLYPH_ROW_BYTES * GLYPH_HEIGHT;

pub(super) fn render_monochrome_page_previews(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    let sprites = catalog_monochrome_sprites(mad_com, installer_payload)?;
    let catalog = catalog_monochrome_text(mad_com, installer_payload, &sprites)?;
    let mut previews = Vec::with_capacity(catalog.opening.pages.len() + catalog.ending.pages.len());
    for sequence in [catalog.opening, catalog.ending] {
        let glyphs = decode_single_asset(installer_payload, &sequence.source_asset)?;
        for page in sequence.pages {
            previews.push(Preview {
                source_asset: format!("{} + MAD.COM", sequence.source_asset),
                output_file: format!("{}.png", page.id),
                evidence: format!(
                    "MAD.COM pointer entry {:#X} selects {} glyph-index bytes rendered through the typed V30 monochrome consumer",
                    page.pointer_entry_file_offset, page.byte_size
                ),
                image: render_page(&glyphs, &page)?,
            });
        }
    }
    Ok(previews)
}

fn render_page(glyphs: &[u8], page: &MonochromeTextPage) -> Result<RgbImage> {
    ensure!(
        !page.lines.is_empty() && page.lines.iter().all(|line| !line.is_empty()),
        "monochrome text page has no visible glyphs"
    );
    ensure!(
        glyphs.len().is_multiple_of(GLYPH_SIZE),
        "monochrome glyph atlas ends inside a record"
    );
    let column_count = page.lines.iter().map(Vec::len).max().unwrap();
    let width = column_count * GLYPH_WIDTH;
    let height = page.lines.len() * GLYPH_HEIGHT;
    let mut pixels = vec![0; width * height * 3];

    for (line_index, line) in page.lines.iter().enumerate() {
        for (column, glyph_index) in line.iter().copied().enumerate() {
            let glyph = glyphs
                .get(glyph_index * GLYPH_SIZE..(glyph_index + 1) * GLYPH_SIZE)
                .with_context(|| format!("{} references missing glyph {glyph_index}", page.id))?;
            for y in 0..GLYPH_HEIGHT {
                for x in 0..GLYPH_WIDTH {
                    if glyph[y * GLYPH_ROW_BYTES + x / 8] & (0x80 >> (x % 8)) == 0 {
                        continue;
                    }
                    let output_x = column * GLYPH_WIDTH + x;
                    let output_y = line_index * GLYPH_HEIGHT + y;
                    pixels
                        [(output_y * width + output_x) * 3..(output_y * width + output_x + 1) * 3]
                        .fill(255);
                }
            }
        }
    }
    Ok(RgbImage {
        width,
        height,
        pixels,
    })
}

#[cfg(test)]
#[path = "monochrome_pages_tests.rs"]
mod monochrome_pages_tests;
