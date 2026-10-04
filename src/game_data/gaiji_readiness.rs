use anyhow::{Context, Result, ensure};
use serde::Serialize;

use super::binary::{com_address_to_file_offset, ensure_prefix, read_u16};
use super::{GaijiCatalog, GaijiGlyphMeaning};

const CHECK_OFFSET: usize = 0x3157;
const EXPECTED_TABLE_ADDRESS_OFFSET: usize = 0x3165;
const PROBE_RENDERER_OFFSET: usize = 0x31a4;
const PROBE_TEXT_ADDRESS_OFFSET: usize = 0x31a6;
const ROW_COUNT: usize = 16;

pub(crate) const GAIJI_READINESS_GLYPH_COUNT: usize = 1;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct GaijiReadinessCatalog {
    pub check_offset: usize,
    pub expected_bitmap_table_offset: usize,
    pub probe_renderer_offset: usize,
    pub probe_text_offset: usize,
    pub probe_shift_jis_code: u16,
    pub reserved_glyph_index: usize,
    pub row_count: usize,
}

pub(super) fn parse_gaiji_readiness(
    mad_com: &[u8],
    gaiji_com: &[u8],
    gaiji: &GaijiCatalog,
) -> Result<GaijiReadinessCatalog> {
    ensure_prefix(
        mad_com,
        CHECK_OFFSET,
        &[
            0x8c, 0xc8, 0x8e, 0xd8, 0xb8, 0x00, 0xa8, 0x8e, 0xc0, 0x33, 0xff, 0x33, 0xc9, 0xbe,
            0x84, 0x32, 0xad, 0xfe, 0xc5, 0x26, 0x39, 0x05, 0x75, 0x02, 0xfe, 0xc1, 0x83, 0xc7,
            0x50, 0x80, 0xfd, 0x10, 0x75, 0xee, 0x80, 0xf9, 0x10, 0x74, 0x05, 0xb4, 0x08, 0xe9,
            0xcf, 0xfe, 0xc3,
        ],
        "MAD GAIJI readiness check",
    )?;
    ensure_prefix(
        mad_com,
        PROBE_RENDERER_OFFSET,
        &[
            0x1e, 0xba, 0xb1, 0x32, 0x33, 0xff, 0xb0, 0x0f, 0xe8, 0xec, 0x01, 0x1f, 0xc3,
        ],
        "MAD GAIJI readiness probe renderer",
    )?;

    let expected_bitmap_table_offset =
        com_address_to_file_offset(read_u16(mad_com, EXPECTED_TABLE_ADDRESS_OFFSET)? as usize)?;
    let probe_text_offset =
        com_address_to_file_offset(read_u16(mad_com, PROBE_TEXT_ADDRESS_OFFSET)? as usize)?;
    let probe = mad_com
        .get(probe_text_offset..probe_text_offset + 4)
        .context("MAD GAIJI readiness probe text lies outside MAD.COM")?;
    ensure!(
        probe[2..] == *b"$$",
        "MAD GAIJI readiness probe text lost its terminator"
    );
    let probe_shift_jis_code = u16::from_be_bytes([probe[0], probe[1]]);
    let glyph = gaiji
        .glyphs
        .iter()
        .find(|glyph| glyph.shift_jis_code == probe_shift_jis_code)
        .context("MAD GAIJI readiness probe does not target an installed glyph")?;
    ensure!(
        matches!(glyph.meaning, GaijiGlyphMeaning::Character { .. }),
        "MAD GAIJI readiness probe targets a non-text graphic slot"
    );
    let bitmap_start = glyph.file_offset + 2;
    let bitmap = gaiji_com
        .get(bitmap_start..bitmap_start + ROW_COUNT * 2)
        .context("MAD GAIJI readiness glyph bitmap lies outside GAIJI.COM")?;
    let expected_bitmap = rendered_probe_bitmap(bitmap)?;
    ensure!(
        mad_com.get(expected_bitmap_table_offset..expected_bitmap_table_offset + ROW_COUNT * 2)
            == Some(expected_bitmap.as_slice()),
        "MAD GAIJI readiness expected bitmap no longer matches its probe glyph"
    );

    Ok(GaijiReadinessCatalog {
        check_offset: CHECK_OFFSET,
        expected_bitmap_table_offset,
        probe_renderer_offset: PROBE_RENDERER_OFFSET,
        probe_text_offset,
        probe_shift_jis_code,
        reserved_glyph_index: glyph.index,
        row_count: ROW_COUNT,
    })
}

fn rendered_probe_bitmap(bitmap: &[u8]) -> Result<Vec<u8>> {
    ensure!(
        bitmap.len() == ROW_COUNT * 2,
        "GAIJI readiness bitmap must contain {ROW_COUNT} rows"
    );
    Ok(bitmap
        .as_chunks::<2>()
        .0
        .iter()
        .flat_map(|row| {
            let bits = u16::from_be_bytes([row[0], row[1]]);
            (bits | bits.wrapping_shl(1)).to_be_bytes()
        })
        .collect())
}

#[cfg(test)]
#[path = "gaiji_readiness_tests.rs"]
mod gaiji_readiness_tests;
