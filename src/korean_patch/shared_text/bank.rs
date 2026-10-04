use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use encoding_rs::SHIFT_JIS;

use super::model::{CompiledGaijiBank, GaijiBankSpan, SharedGaijiGlyph};
use crate::game_data::{
    GaijiCatalog, GaijiGlyph, GaijiGlyphMeaning, GaijiReadinessCatalog, jis_row_cell_to_shift_jis,
    parse_gaiji_program,
};
use crate::korean_patch::font_rasterizer::{FixedCellRasterizer, GLYPH_BYTES};
use crate::korean_patch::gaiji_record::{GAIJI_RECORD_SIZE, GaijiRecord};

pub(crate) fn compile_gaiji_bank(
    gaiji_program: &[u8],
    gaiji: &GaijiCatalog,
    readiness: &GaijiReadinessCatalog,
    lines: &[&str],
) -> Result<CompiledGaijiBank> {
    compile_gaiji_bank_with_glyph_overrides(
        gaiji_program,
        gaiji,
        readiness,
        lines,
        &BTreeMap::new(),
    )
}

pub(crate) fn compile_gaiji_bank_with_glyph_overrides(
    gaiji_program: &[u8],
    gaiji: &GaijiCatalog,
    readiness: &GaijiReadinessCatalog,
    lines: &[&str],
    glyph_overrides: &BTreeMap<char, [u8; GLYPH_BYTES]>,
) -> Result<CompiledGaijiBank> {
    compile_gaiji_bank_with_trailing_character_codes(
        gaiji_program,
        gaiji,
        readiness,
        lines,
        glyph_overrides,
        &[],
    )
}

pub(crate) fn compile_gaiji_bank_with_trailing_character_codes(
    gaiji_program: &[u8],
    gaiji: &GaijiCatalog,
    readiness: &GaijiReadinessCatalog,
    lines: &[&str],
    glyph_overrides: &BTreeMap<char, [u8; GLYPH_BYTES]>,
    trailing_character_codes: &[u16],
) -> Result<CompiledGaijiBank> {
    let characters = collect_unique_characters_with_overrides(lines, glyph_overrides)?;
    let parsed = parse_gaiji_program(gaiji_program)?;
    ensure!(
        parsed == *gaiji,
        "shared GAIJI source differs from the catalogued program"
    );
    let source_slots = available_slots(gaiji, readiness)?;
    let trailing_slots = validate_trailing_slots(gaiji, trailing_character_codes)?;
    let available_slot_count = source_slots.len() + trailing_slots.len();
    ensure!(
        characters.len() <= available_slot_count,
        "shared text needs {} GAIJI glyphs but only {} verified character slots are available",
        characters.len(),
        available_slot_count
    );

    let rasterizer = FixedCellRasterizer::load()?;
    let mut patched_program = gaiji_program.to_vec();
    let mut appended_records = Vec::new();
    let mut character_codes = BTreeMap::new();
    let mut glyphs = Vec::with_capacity(characters.len());
    let mut assigned_slots = Vec::with_capacity(characters.len());
    for (assignment_index, character) in characters.into_iter().enumerate() {
        let bitmap = if let Some(bitmap) = glyph_overrides.get(&character) {
            *bitmap
        } else {
            rasterizer
                .rasterize_visible_character(character)
                .with_context(|| format!("failed to rasterize shared GAIJI {character:?}"))?
        };
        let record = GaijiRecord::from_bitmap(bitmap).to_bytes();
        if let Some(slot) = source_slots.get(assignment_index).copied() {
            require_record(gaiji_program, slot)?;
            patched_program[slot.file_offset..slot.file_offset + GAIJI_RECORD_SIZE]
                .copy_from_slice(&record);
            character_codes.insert(character, slot.shift_jis_code);
            glyphs.push(SharedGaijiGlyph {
                character: character.to_string(),
                slot_index: slot.index,
                shift_jis_code: format!("0x{:04X}", slot.shift_jis_code),
                record_offset: slot.file_offset,
            });
            assigned_slots.push(slot);
        } else {
            let trailing_index = assignment_index - source_slots.len();
            let slot = trailing_slots
                .get(trailing_index)
                .context("verified trailing GAIJI assignment disappeared")?;
            appended_records.extend_from_slice(&record);
            character_codes.insert(character, slot.shift_jis_code);
            glyphs.push(SharedGaijiGlyph {
                character: character.to_string(),
                slot_index: slot.slot_index,
                shift_jis_code: format!("0x{:04X}", slot.shift_jis_code),
                record_offset: slot.record_offset,
            });
        }
    }
    let spans = contiguous_spans(gaiji_program, &patched_program, &assigned_slots);
    Ok(CompiledGaijiBank {
        patched_program,
        appended_records,
        character_codes,
        glyphs,
        spans,
        available_slot_count,
        reserved_slot_indexes: vec![readiness.reserved_glyph_index],
    })
}

struct TrailingGaijiSlot {
    slot_index: usize,
    shift_jis_code: u16,
    record_offset: usize,
}

fn validate_trailing_slots(
    gaiji: &GaijiCatalog,
    character_codes: &[u16],
) -> Result<Vec<TrailingGaijiSlot>> {
    let mut expected = gaiji
        .last_character_code
        .checked_add(1)
        .context("GAIJI trailing character-code range overflow")?;
    let mut slots = Vec::with_capacity(character_codes.len());
    for (index, &character_code) in character_codes.iter().enumerate() {
        ensure!(
            character_code == expected,
            "trailing GAIJI code {character_code:04X} is not contiguous after {expected:04X}"
        );
        slots.push(TrailingGaijiSlot {
            slot_index: gaiji.glyphs.len() + index,
            shift_jis_code: jis_row_cell_to_shift_jis(character_code)?,
            record_offset: gaiji.file_size + index * GAIJI_RECORD_SIZE,
        });
        expected = if character_code & 0xff == 0x7e {
            (character_code & 0xff00)
                .checked_add(0x100)
                .and_then(|row| row.checked_add(0x21))
                .context("GAIJI trailing row transition overflow")?
        } else {
            character_code
                .checked_add(1)
                .context("GAIJI trailing character-code overflow")?
        };
    }
    Ok(slots)
}

fn available_slots<'a>(
    gaiji: &'a GaijiCatalog,
    readiness: &GaijiReadinessCatalog,
) -> Result<Vec<&'a GaijiGlyph>> {
    let reserved = gaiji
        .glyphs
        .get(readiness.reserved_glyph_index)
        .context("GAIJI readiness reservation lies outside the glyph catalog")?;
    ensure!(
        reserved.shift_jis_code == readiness.probe_shift_jis_code
            && matches!(reserved.meaning, GaijiGlyphMeaning::Character { .. }),
        "GAIJI readiness reservation no longer identifies its probe character"
    );
    Ok(gaiji
        .glyphs
        .iter()
        .filter(|glyph| {
            matches!(glyph.meaning, GaijiGlyphMeaning::Character { .. })
                && glyph.index != readiness.reserved_glyph_index
        })
        .collect())
}

#[cfg(test)]
fn collect_unique_characters(lines: &[&str]) -> Result<Vec<char>> {
    collect_unique_characters_with_overrides(lines, &BTreeMap::new())
}

fn collect_unique_characters_with_overrides(
    lines: &[&str],
    glyph_overrides: &BTreeMap<char, [u8; GLYPH_BYTES]>,
) -> Result<Vec<char>> {
    let mut seen = BTreeSet::new();
    let mut characters = Vec::new();
    for line in lines {
        for character in line.chars() {
            if character == ' ' {
                continue;
            }
            ensure!(
                !character.is_control() && !character.is_whitespace(),
                "shared text contains unsupported whitespace {character:?}"
            );
            let (_, _, had_errors) = SHIFT_JIS.encode(&character.to_string());
            if (had_errors || glyph_overrides.contains_key(&character)) && seen.insert(character) {
                characters.push(character);
            }
        }
    }
    Ok(characters)
}

fn require_record(program: &[u8], slot: &GaijiGlyph) -> Result<()> {
    ensure!(
        slot.byte_size == GAIJI_RECORD_SIZE,
        "GAIJI slot {} record size changed",
        slot.index
    );
    let end = slot.file_offset + slot.byte_size;
    let record = GaijiRecord::parse(
        program
            .get(slot.file_offset..end)
            .context("GAIJI bank slot lies outside the program")?,
    )?;
    record.require_target_format()
}

fn contiguous_spans(
    original: &[u8],
    replacement: &[u8],
    slots: &[&GaijiGlyph],
) -> Vec<GaijiBankSpan> {
    let mut spans = Vec::new();
    let mut start = 0;
    while start < slots.len() {
        let mut end = start + 1;
        while end < slots.len()
            && slots[end].file_offset == slots[end - 1].file_offset + GAIJI_RECORD_SIZE
        {
            end += 1;
        }
        let offset = slots[start].file_offset;
        let byte_end = slots[end - 1].file_offset + GAIJI_RECORD_SIZE;
        spans.push(GaijiBankSpan {
            first_slot_index: slots[start].index,
            offset,
            expected_original: original[offset..byte_end].to_vec(),
            replacement: replacement[offset..byte_end].to_vec(),
        });
        start = end;
    }
    spans
}

#[cfg(test)]
#[path = "bank_tests.rs"]
mod bank_tests;
