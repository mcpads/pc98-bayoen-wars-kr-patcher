use anyhow::{Context, Result, ensure};
use serde::Serialize;

use super::binary::{com_address_to_file_offset, read_u16};
use super::gaiji_installer::{GaijiInstallerCatalog, parse_gaiji_installer};
use super::gaiji_meaning::{GaijiGlyphMeaning, meaning_for_index};
use crate::source_disk::sha256_hex;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct GaijiCatalog {
    pub sha256: String,
    pub file_size: usize,
    pub installer: GaijiInstallerCatalog,
    pub pointer_table_offset: usize,
    pub first_character_code: u16,
    pub last_character_code: u16,
    pub first_shift_jis_code: u16,
    pub last_shift_jis_code: u16,
    pub uniform_record_size: Option<usize>,
    pub glyphs: Vec<GaijiGlyph>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct GaijiGlyph {
    pub index: usize,
    pub character_code: u16,
    pub shift_jis_code: u16,
    pub file_offset: usize,
    pub byte_size: usize,
    pub sha256: String,
    pub meaning: GaijiGlyphMeaning,
}

pub(crate) fn parse_gaiji_program(bytes: &[u8]) -> Result<GaijiCatalog> {
    let installer = parse_gaiji_installer(bytes)?;
    let glyph_count = installer.glyph_count;
    let first_character_code = installer.first_character_code;
    let pointer_table_address = usize::from(installer.pointer_table_com_address);
    let row_end = installer.row_end_character_code;
    let next_row_previous = installer.next_row_previous_character_code;
    let pointer_table_offset = com_address_to_file_offset(pointer_table_address)?;
    let pointer_table_size = glyph_count
        .checked_mul(2)
        .context("GAIJI pointer-table size overflow")?;
    let pointer_table_end = pointer_table_offset
        .checked_add(pointer_table_size)
        .context("GAIJI pointer-table boundary overflow")?;
    ensure!(
        pointer_table_end <= bytes.len(),
        "GAIJI pointer table ends outside the program"
    );

    let mut record_offsets = Vec::with_capacity(glyph_count);
    for index in 0..glyph_count {
        let pointer_offset = pointer_table_offset + index * 2;
        let record_address = read_u16(bytes, pointer_offset)? as usize;
        let record_offset = com_address_to_file_offset(record_address)?;
        ensure!(
            record_offset >= pointer_table_end,
            "GAIJI glyph {index} starts inside the pointer table"
        );
        ensure!(
            record_offset < bytes.len(),
            "GAIJI glyph {index} starts outside the program"
        );
        if let Some(previous) = record_offsets.last() {
            ensure!(
                record_offset > *previous,
                "GAIJI glyph pointers are not strictly increasing at index {index}"
            );
        }
        record_offsets.push(record_offset);
    }

    let mut character_codes = Vec::with_capacity(glyph_count);
    let mut character_code = first_character_code;
    for _ in 0..glyph_count {
        character_codes.push(character_code);
        character_code = if character_code == row_end {
            next_row_previous
                .checked_add(1)
                .context("GAIJI next-row character code overflow")?
        } else {
            character_code
                .checked_add(1)
                .context("GAIJI character code overflow")?
        };
    }

    let mut glyphs = Vec::with_capacity(glyph_count);
    for index in 0..glyph_count {
        let file_offset = record_offsets[index];
        let end = record_offsets
            .get(index + 1)
            .copied()
            .unwrap_or(bytes.len());
        let record = &bytes[file_offset..end];
        let character_code = character_codes[index];
        glyphs.push(GaijiGlyph {
            index,
            character_code,
            shift_jis_code: jis_row_cell_to_shift_jis(character_code)?,
            file_offset,
            byte_size: record.len(),
            sha256: sha256_hex(record),
            meaning: meaning_for_index(index)?,
        });
    }

    let uniform_record_size = glyphs
        .first()
        .map(|first| first.byte_size)
        .filter(|size| glyphs.iter().all(|glyph| glyph.byte_size == *size));
    let first = glyphs
        .first()
        .context("GAIJI glyph catalog unexpectedly became empty")?;
    let last = glyphs
        .last()
        .context("GAIJI glyph catalog unexpectedly became empty")?;

    Ok(GaijiCatalog {
        sha256: sha256_hex(bytes),
        file_size: bytes.len(),
        installer,
        pointer_table_offset,
        first_character_code: first.character_code,
        last_character_code: last.character_code,
        first_shift_jis_code: first.shift_jis_code,
        last_shift_jis_code: last.shift_jis_code,
        uniform_record_size,
        glyphs,
    })
}

pub(crate) fn jis_row_cell_to_shift_jis(character_code: u16) -> Result<u16> {
    let row = (character_code >> 8) as u8;
    let cell = character_code as u8;
    ensure!(
        (0x21..=0x7e).contains(&row) && (0x21..=0x7e).contains(&cell),
        "GAIJI character code {character_code:04X} is outside the JIS row-cell range"
    );

    let mut lead = ((row - 0x21) >> 1) + 0x81;
    if lead > 0x9f {
        lead = lead
            .checked_add(0x40)
            .context("Shift_JIS lead-byte overflow")?;
    }
    let trail = if row & 1 == 1 {
        let mut value = cell
            .checked_add(0x1f)
            .context("Shift_JIS trail-byte overflow")?;
        if value >= 0x7f {
            value = value
                .checked_add(1)
                .context("Shift_JIS trail-byte overflow")?;
        }
        value
    } else {
        cell.checked_add(0x7e)
            .context("Shift_JIS trail-byte overflow")?
    };
    Ok(u16::from_be_bytes([lead, trail]))
}

#[cfg(test)]
#[path = "gaiji_tests.rs"]
mod gaiji_tests;
