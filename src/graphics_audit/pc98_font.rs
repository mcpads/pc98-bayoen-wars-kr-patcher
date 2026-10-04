use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use encoding_rs::SHIFT_JIS;

use crate::korean_patch::GLYPH_BYTES;
use crate::source_disk::sha256_hex;

pub(super) const SUPPORTED_PC98_FONT_BMP_SHA256: &str =
    "41535bd7242c07d69ec5427584d31f068ca4eb996ca95246caed4e3573a494e6";

const BMP_WIDTH: usize = 2048;
const BMP_HEIGHT: usize = 2048;
const BMP_ROW_BYTES: usize = BMP_WIDTH / 8;
const BMP_DATA_OFFSET: usize = 62;
const BMP_DATA_SIZE: usize = BMP_ROW_BYTES * BMP_HEIGHT;

#[derive(Clone, Copy)]
pub(super) struct RasterGlyph {
    pub(super) width: usize,
    pub(super) bitmap: [u8; GLYPH_BYTES],
}

pub(super) struct Pc98Font {
    bmp: Vec<u8>,
}

impl Pc98Font {
    pub(super) fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("failed to read PC-98 font BMP {}", path.display()))?;
        ensure!(
            sha256_hex(&bytes) == SUPPORTED_PC98_FONT_BMP_SHA256,
            "PC-98 font BMP SHA-256 differs from the supported retrobios asset"
        );
        Self::parse(bytes)
    }

    fn parse(bytes: Vec<u8>) -> Result<Self> {
        ensure!(
            bytes.len() == BMP_DATA_OFFSET + BMP_DATA_SIZE,
            "PC-98 font BMP has an unexpected byte size"
        );
        ensure!(&bytes[..2] == b"BM", "PC-98 font file is not a BMP");
        ensure!(
            read_u32(&bytes, 10)? == BMP_DATA_OFFSET as u32
                && read_u32(&bytes, 14)? == 40
                && read_u32(&bytes, 18)? == BMP_WIDTH as u32
                && read_u32(&bytes, 22)? == BMP_HEIGHT as u32
                && read_u16(&bytes, 26)? == 1
                && read_u16(&bytes, 28)? == 1
                && read_u32(&bytes, 30)? == 0
                && read_u32(&bytes, 34)? == BMP_DATA_SIZE as u32,
            "PC-98 font BMP layout differs from the 2048x2048 1bpp consumer format"
        );
        Ok(Self { bmp: bytes })
    }

    pub(super) fn rasterize(&self, character: char) -> Result<RasterGlyph> {
        let text = character.to_string();
        let (encoded, _, had_errors) = SHIFT_JIS.encode(&text);
        ensure!(
            !had_errors && matches!(encoded.len(), 1 | 2),
            "character {character:?} is absent from the PC-98 Shift_JIS font"
        );
        match encoded.as_ref() {
            [ank] => Ok(RasterGlyph {
                width: 8,
                bitmap: self.copy_glyph(usize::from(*ank) * 8, 0, 8),
            }),
            [lead, trail] => {
                let (row, cell) = shift_jis_to_jis(*lead, *trail)?;
                Ok(RasterGlyph {
                    width: 16,
                    bitmap: self.copy_glyph(usize::from(row) * 16, usize::from(cell) * 16, 16),
                })
            }
            _ => unreachable!(),
        }
    }

    fn copy_glyph(&self, source_x: usize, source_y: usize, width: usize) -> [u8; GLYPH_BYTES] {
        let mut bitmap = [0u8; GLYPH_BYTES];
        let output_row_bytes = width / 8;
        for y in 0..16 {
            let bmp_row = BMP_HEIGHT - 1 - (source_y + y);
            let source = BMP_DATA_OFFSET + bmp_row * BMP_ROW_BYTES + source_x / 8;
            for byte in 0..output_row_bytes {
                bitmap[y * 2 + byte] = !self.bmp[source + byte];
            }
        }
        bitmap
    }
}

fn shift_jis_to_jis(lead: u8, trail: u8) -> Result<(u8, u8)> {
    ensure!(
        (0x81..=0x9f).contains(&lead) || (0xe0..=0xef).contains(&lead),
        "Shift_JIS lead byte {lead:#04x} is outside the PC-98 JIS mapping"
    );
    ensure!(
        (0x40..=0xfc).contains(&trail) && trail != 0x7f,
        "Shift_JIS trail byte {trail:#04x} is outside the PC-98 JIS mapping"
    );
    let mut row = if lead <= 0x9f {
        (lead - 0x81) * 2 + 0x21
    } else {
        (lead - 0xe0) * 2 + 0x5f
    };
    let cell = if trail >= 0x9f {
        row += 1;
        trail - 0x7e
    } else if trail < 0x7f {
        trail - 0x1f
    } else {
        trail - 0x20
    };
    Ok((row, cell))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let value = bytes
        .get(offset..offset + 2)
        .context("PC-98 font BMP header is truncated")?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let value = bytes
        .get(offset..offset + 4)
        .context("PC-98 font BMP header is truncated")?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

#[cfg(test)]
#[path = "pc98_font_tests.rs"]
mod pc98_font_tests;
