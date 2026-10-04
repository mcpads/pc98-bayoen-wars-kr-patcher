use anyhow::{Context, Result, ensure};

use crate::pc98_graphics::{
    INDEXED_RGB4_PALETTE_TABLE_SIZE, Pc98PaletteRgb4, encode_indexed_rgb4_palette,
    expand_rgb4_palette, read_indexed_rgb4_palette,
};

pub(crate) const TITLE_ANIMATION_FRAME_COUNT: usize = 16;
pub(crate) const TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET: usize = 0xac48;
pub(crate) const TITLE_PALETTE_TABLE_FILE_OFFSET: usize = 0xacf0;
pub(crate) const TITLE_ANIMATION_SKIP_SOURCE: u16 = 0xfffe;

const TITLE_ANIMATION_END_SOURCE: u16 = 0xffff;
pub(crate) type TitlePaletteRgb4 = Pc98PaletteRgb4;

pub(crate) const SOURCE_TITLE_ANIMATION_FRAME_SOURCES: [u16; TITLE_ANIMATION_FRAME_COUNT] = [
    0x0c00, 0x0c00, 0x0000, 0x0000, 0x0600, 0x0600, 0x0000, 0x0000, 0x0c00, 0x0c00, 0x1800, 0x1800,
    0x1200, 0x1200, 0x1800, 0x1800,
];

pub(crate) const SOURCE_TITLE_PALETTE_RGB4: TitlePaletteRgb4 = [
    [0, 0, 0],
    [6, 0, 0],
    [15, 15, 15],
    [5, 5, 6],
    [8, 8, 9],
    [11, 11, 12],
    [13, 13, 14],
    [5, 14, 9],
    [15, 15, 2],
    [10, 6, 0],
    [14, 9, 0],
    [13, 10, 0],
    [14, 11, 4],
    [15, 13, 6],
    [15, 15, 8],
    [15, 15, 10],
];

pub(crate) fn expand_title_palette(palette: &TitlePaletteRgb4) -> [[u8; 3]; 16] {
    expand_rgb4_palette(palette)
}

pub(crate) fn encode_title_palette_table(palette: &TitlePaletteRgb4) -> Result<Vec<u8>> {
    encode_indexed_rgb4_palette(palette).context("encode title runtime palette")
}

pub(crate) fn read_title_palette_rgb4(mad_com: &[u8]) -> Result<TitlePaletteRgb4> {
    let table = mad_com
        .get(
            TITLE_PALETTE_TABLE_FILE_OFFSET
                ..TITLE_PALETTE_TABLE_FILE_OFFSET + INDEXED_RGB4_PALETTE_TABLE_SIZE,
        )
        .context("MAD.COM title palette table lies outside the program")?;
    read_indexed_rgb4_palette(table).context("read MAD.COM title runtime palette")
}

pub(crate) fn encode_title_animation_frame_table(
    sources: &[u16; TITLE_ANIMATION_FRAME_COUNT],
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((TITLE_ANIMATION_FRAME_COUNT + 1) * 2);
    for source in sources {
        bytes.extend_from_slice(&source.to_le_bytes());
    }
    bytes.extend_from_slice(&TITLE_ANIMATION_END_SOURCE.to_le_bytes());
    bytes
}

pub(crate) fn disabled_title_animation_frame_sources() -> [u16; TITLE_ANIMATION_FRAME_COUNT] {
    [TITLE_ANIMATION_SKIP_SOURCE; TITLE_ANIMATION_FRAME_COUNT]
}

pub(crate) fn read_title_animation_frame_sources(
    mad_com: &[u8],
) -> Result<[u16; TITLE_ANIMATION_FRAME_COUNT]> {
    let table = mad_com
        .get(
            TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET
                ..TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET + (TITLE_ANIMATION_FRAME_COUNT + 1) * 2,
        )
        .context("MAD.COM title animation table lies outside the program")?;
    ensure!(
        u16::from_le_bytes([
            table[TITLE_ANIMATION_FRAME_COUNT * 2],
            table[TITLE_ANIMATION_FRAME_COUNT * 2 + 1],
        ]) == TITLE_ANIMATION_END_SOURCE,
        "MAD.COM title animation table has no end marker"
    );
    let mut sources = [0; TITLE_ANIMATION_FRAME_COUNT];
    for (source, bytes) in sources.iter_mut().zip(table.as_chunks::<2>().0) {
        *source = u16::from_le_bytes([bytes[0], bytes[1]]);
    }
    Ok(sources)
}

pub(crate) fn title_animation_is_disabled(mad_com: &[u8]) -> Result<bool> {
    Ok(read_title_animation_frame_sources(mad_com)?
        .iter()
        .all(|source| *source == TITLE_ANIMATION_SKIP_SOURCE))
}

#[cfg(test)]
#[path = "title_runtime_tests.rs"]
mod title_runtime_tests;
