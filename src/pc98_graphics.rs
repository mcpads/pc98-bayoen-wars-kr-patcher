use anyhow::{Result, ensure};

pub(crate) const DIGITAL_RGBI_COLOR_COUNT: usize = 16;
pub(crate) const INDEXED_RGB4_PALETTE_TABLE_SIZE: usize = DIGITAL_RGBI_COLOR_COUNT * 4 + 1;

pub(crate) type Pc98PaletteRgb4 = [[u8; 3]; DIGITAL_RGBI_COLOR_COUNT];

pub(crate) fn digital_rgbi_color(index: usize) -> [u8; 3] {
    let intensity = index & 8 != 0;
    let low = if intensity { 85 } else { 0 };
    let high = if intensity { 255 } else { 170 };
    let blue = if index & 1 != 0 { high } else { low };
    let red = if index & 2 != 0 { high } else { low };
    let green = if index & 4 != 0 { high } else { low };
    [red, green, blue]
}

pub(crate) fn nearest_16_color_palette_index(
    rgb: [u8; 3],
    palette: &[[u8; 3]; DIGITAL_RGBI_COLOR_COUNT],
) -> u8 {
    (0..DIGITAL_RGBI_COLOR_COUNT)
        .min_by_key(|index| {
            let color = palette[*index];
            color
                .into_iter()
                .zip(rgb)
                .map(|(candidate, input)| {
                    let difference = i32::from(candidate) - i32::from(input);
                    difference * difference
                })
                .sum::<i32>()
        })
        .expect("the digital RGBI palette is non-empty") as u8
}

pub(crate) fn digital_rgbi_palette() -> [[u8; 3]; DIGITAL_RGBI_COLOR_COUNT] {
    std::array::from_fn(digital_rgbi_color)
}

pub(crate) fn expand_rgb4_palette(
    palette: &Pc98PaletteRgb4,
) -> [[u8; 3]; DIGITAL_RGBI_COLOR_COUNT] {
    palette.map(|color| color.map(|component| component * 17))
}

pub(crate) fn encode_indexed_rgb4_palette(palette: &Pc98PaletteRgb4) -> Result<Vec<u8>> {
    ensure!(
        palette.iter().flatten().all(|component| *component <= 0x0f),
        "PC-98 RGB4 palette component exceeds 15"
    );
    let mut bytes = Vec::with_capacity(INDEXED_RGB4_PALETTE_TABLE_SIZE);
    for (index, color) in palette.iter().enumerate() {
        bytes.push(u8::try_from(index)?);
        bytes.extend_from_slice(color);
    }
    bytes.push(0xff);
    Ok(bytes)
}

pub(crate) fn read_indexed_rgb4_palette(table: &[u8]) -> Result<Pc98PaletteRgb4> {
    ensure!(
        table.len() == INDEXED_RGB4_PALETTE_TABLE_SIZE,
        "PC-98 indexed RGB4 palette table has an unexpected size"
    );
    ensure!(
        table[DIGITAL_RGBI_COLOR_COUNT * 4] == 0xff,
        "PC-98 indexed RGB4 palette table has no end marker"
    );
    let mut palette = [[0; 3]; DIGITAL_RGBI_COLOR_COUNT];
    for (index, record) in table[..DIGITAL_RGBI_COLOR_COUNT * 4]
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
    {
        ensure!(
            usize::from(record[0]) == index,
            "PC-98 indexed RGB4 palette index order changed"
        );
        ensure!(
            record[1..].iter().all(|component| *component <= 0x0f),
            "PC-98 RGB4 palette component exceeds 15"
        );
        palette[index].copy_from_slice(&record[1..]);
    }
    Ok(palette)
}

#[cfg(test)]
#[path = "pc98_graphics_tests.rs"]
mod pc98_graphics_tests;
