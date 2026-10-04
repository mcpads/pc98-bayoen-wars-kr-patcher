use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use super::Preview;
use super::contact_sheet::compose_contact_sheet;
use super::monochrome_sheet::render_monochrome_sheet;
use super::planar::RgbImage;
use crate::GaijiGlyph;
use crate::game_data::parse_gaiji_program;

const GAIJI_FILE: &str = "GAIJI.COM";
const RECORD_PREFIX_SIZE: usize = 2;
const GLYPH_WIDTH: usize = 16;
const GLYPH_HEIGHT: usize = 16;
const GLYPH_BITMAP_SIZE: usize = GLYPH_WIDTH / 8 * GLYPH_HEIGHT;
const RECORD_SIZE: usize = RECORD_PREFIX_SIZE + GLYPH_BITMAP_SIZE;
const SHEET_COLUMNS: usize = 16;
const CELL_GAP: usize = 2;

pub(super) fn render_gaiji_preview(
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Preview> {
    let gaiji_com = installer_payload
        .get(GAIJI_FILE)
        .context("verified installer payload is missing GAIJI.COM")?;
    let catalog = parse_gaiji_program(gaiji_com)?;
    let image = render_gaiji_sheet(gaiji_com, &catalog.glyphs)?;

    Ok(Preview {
        source_asset: GAIJI_FILE.to_owned(),
        output_file: "gaiji-com-installed-glyphs.png".to_owned(),
        evidence: format!(
            "GAIJI.COM installs {} ordered 16x16 records; the sheet is row-major with {} glyph indices per row from Shift_JIS {:04X} through {:04X}",
            catalog.glyphs.len(),
            SHEET_COLUMNS,
            catalog.first_shift_jis_code,
            catalog.last_shift_jis_code
        ),
        image,
    })
}

fn render_gaiji_sheet(program: &[u8], glyphs: &[GaijiGlyph]) -> Result<RgbImage> {
    ensure!(!glyphs.is_empty(), "GAIJI sheet has no glyphs");
    let mut cells = Vec::with_capacity(glyphs.len());
    for glyph in glyphs {
        ensure!(
            glyph.byte_size == RECORD_SIZE,
            "GAIJI glyph {} is not a {RECORD_SIZE}-byte record",
            glyph.index
        );
        let record_end = glyph
            .file_offset
            .checked_add(glyph.byte_size)
            .context("GAIJI record boundary overflow")?;
        let record = program
            .get(glyph.file_offset..record_end)
            .with_context(|| format!("GAIJI glyph {} lies outside GAIJI.COM", glyph.index))?;
        ensure!(
            record[..RECORD_PREFIX_SIZE] == [0, 0],
            "GAIJI glyph {} has an unexpected record prefix",
            glyph.index
        );
        cells.push(render_monochrome_sheet(
            &record[RECORD_PREFIX_SIZE..],
            GLYPH_WIDTH,
            GLYPH_HEIGHT,
            1,
        )?);
    }

    let maximum_width = SHEET_COLUMNS * GLYPH_WIDTH + (SHEET_COLUMNS - 1) * CELL_GAP;
    compose_contact_sheet(cells, maximum_width, CELL_GAP)
}

#[cfg(test)]
#[path = "gaiji_tests.rs"]
mod gaiji_tests;
