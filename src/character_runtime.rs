use anyhow::{Context, Result};

use crate::pc98_graphics::{
    INDEXED_RGB4_PALETTE_TABLE_SIZE, Pc98PaletteRgb4, expand_rgb4_palette,
    read_indexed_rgb4_palette,
};

pub(crate) const CHARACTER_PALETTE_TABLE_FILE_OFFSET: usize = 0xd777;

pub(crate) const SOURCE_CHARACTER_PALETTE_RGB4: Pc98PaletteRgb4 = [
    [0, 0, 0],
    [2, 4, 11],
    [2, 8, 13],
    [2, 13, 15],
    [0, 9, 4],
    [15, 12, 10],
    [15, 9, 10],
    [12, 12, 12],
    [7, 12, 0],
    [11, 0, 1],
    [15, 0, 6],
    [14, 15, 1],
    [12, 7, 2],
    [8, 8, 8],
    [15, 10, 2],
    [15, 15, 15],
];

pub(crate) fn read_character_palette_rgb4(mad_com: &[u8]) -> Result<Pc98PaletteRgb4> {
    let table = mad_com
        .get(
            CHARACTER_PALETTE_TABLE_FILE_OFFSET
                ..CHARACTER_PALETTE_TABLE_FILE_OFFSET + INDEXED_RGB4_PALETTE_TABLE_SIZE,
        )
        .context("MAD.COM character palette table lies outside the program")?;
    read_indexed_rgb4_palette(table).context("read MAD.COM character runtime palette")
}

pub(crate) fn expand_character_palette(palette: &Pc98PaletteRgb4) -> [[u8; 3]; 16] {
    expand_rgb4_palette(palette)
}

#[cfg(test)]
#[path = "character_runtime_tests.rs"]
mod character_runtime_tests;
