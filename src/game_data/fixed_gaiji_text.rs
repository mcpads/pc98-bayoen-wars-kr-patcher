use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use encoding_rs::SHIFT_JIS;
use serde::Serialize;

use super::binary::{com_address_to_file_offset, ensure_prefix, read_u16};
use super::gaiji::{GaijiCatalog, GaijiGlyph};
use super::gaiji_meaning::GaijiGlyphMeaning;
use crate::byte_string::encode_lower_hex;
use crate::source_disk::sha256_hex;

mod runtime;

pub use runtime::{FixedGaijiTextRuntimeCall, FixedGaijiTextRuntimeCatalog};

const POINTER_INSTRUCTION_OFFSETS: [usize; 10] = [
    0xa3e8, 0xa3f4, 0xa400, 0xa40c, 0xa418, 0xa424, 0xa430, 0xa43c, 0xa448, 0xa460,
];
const SLOT_SIZE: usize = 0x60;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct FixedGaijiTextCatalog {
    pub slot_size: usize,
    pub slots: Vec<FixedGaijiTextSlot>,
    pub runtime: FixedGaijiTextRuntimeCatalog,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct FixedGaijiTextSlot {
    pub id: String,
    pub pointer_instruction_offset: usize,
    pub file_offset: usize,
    pub byte_size: usize,
    pub sha256: String,
    pub raw_hex: String,
    pub text: String,
    pub lines: Vec<Vec<GaijiTextCell>>,
    pub used_glyph_indices: Vec<usize>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GaijiTextCell {
    Space,
    Standard {
        text: String,
    },
    Glyph {
        glyph_index: usize,
        character_code: u16,
        shift_jis_code: u16,
        source: GaijiGlyphMeaning,
    },
}

pub(super) fn parse_fixed_gaiji_text(
    bytes: &[u8],
    gaiji: &GaijiCatalog,
) -> Result<FixedGaijiTextCatalog> {
    let runtime = runtime::catalog_fixed_gaiji_text_runtime(bytes)?;
    let gaiji_by_shift_jis: BTreeMap<u16, &GaijiGlyph> = gaiji
        .glyphs
        .iter()
        .map(|glyph| (glyph.shift_jis_code, glyph))
        .collect();
    let mut slots = Vec::with_capacity(POINTER_INSTRUCTION_OFFSETS.len());
    let mut previous_offset = None;

    for (slot_index, instruction_offset) in POINTER_INSTRUCTION_OFFSETS.iter().copied().enumerate()
    {
        ensure_prefix(
            bytes,
            instruction_offset,
            &[0xbb],
            "MAD fixed-text pointer instruction",
        )?;
        let address = read_u16(bytes, instruction_offset + 1)? as usize;
        let file_offset = com_address_to_file_offset(address)?;
        if let Some(previous) = previous_offset {
            ensure!(
                file_offset == previous + SLOT_SIZE,
                "MAD fixed-text slots do not use the verified {SLOT_SIZE:#x}-byte stride"
            );
        }
        let end = file_offset
            .checked_add(SLOT_SIZE)
            .context("MAD fixed-text slot boundary overflow")?;
        let raw = bytes.get(file_offset..end).with_context(|| {
            format!(
                "MAD fixed-text slot {} lies outside MAD.COM",
                slot_index + 1
            )
        })?;
        let (lines, used_glyph_indices) = parse_fixed_gaiji_slot(raw, &gaiji_by_shift_jis)
            .with_context(|| format!("invalid MAD fixed-text slot {}", slot_index + 1))?;
        let text = format_fixed_gaiji_text(&lines);

        slots.push(FixedGaijiTextSlot {
            id: format!("fixed-gaiji-text-{:02}", slot_index + 1),
            pointer_instruction_offset: instruction_offset,
            file_offset,
            byte_size: raw.len(),
            sha256: sha256_hex(raw),
            raw_hex: encode_lower_hex(raw),
            text,
            lines,
            used_glyph_indices,
        });
        previous_offset = Some(file_offset);
    }

    Ok(FixedGaijiTextCatalog {
        slot_size: SLOT_SIZE,
        slots,
        runtime,
    })
}

fn format_fixed_gaiji_text(lines: &[Vec<GaijiTextCell>]) -> String {
    lines
        .iter()
        .map(|line| {
            line.iter()
                .map(|cell| match cell {
                    GaijiTextCell::Space => "　".to_owned(),
                    GaijiTextCell::Standard { text } => text.clone(),
                    GaijiTextCell::Glyph { source, .. } => source.source_text(),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_fixed_gaiji_slot(
    raw: &[u8],
    gaiji_by_shift_jis: &BTreeMap<u16, &GaijiGlyph>,
) -> Result<(Vec<Vec<GaijiTextCell>>, Vec<usize>)> {
    ensure!(
        raw.len() == SLOT_SIZE,
        "fixed GAIJI text slot must be exactly {SLOT_SIZE:#x} bytes"
    );
    ensure!(
        raw.len().is_multiple_of(2),
        "fixed GAIJI text slot has an odd byte length"
    );

    let mut lines = vec![Vec::new()];
    let mut used_glyph_indices = BTreeSet::new();
    let pair_count = raw.len() / 2;
    for pair_index in 0..pair_count {
        let pair = &raw[pair_index * 2..pair_index * 2 + 2];
        if pair == b"$$" {
            ensure!(
                pair_index + 1 == pair_count,
                "fixed GAIJI text terminator appears before the slot boundary"
            );
            break;
        }
        if pair == b"$0" {
            lines.push(Vec::new());
            continue;
        }
        if pair == [0x81, 0x40] {
            lines
                .last_mut()
                .expect("fixed-text parser always has one line")
                .push(GaijiTextCell::Space);
            continue;
        }

        let shift_jis_code = u16::from_be_bytes([pair[0], pair[1]]);
        if let Some(glyph) = gaiji_by_shift_jis.get(&shift_jis_code) {
            used_glyph_indices.insert(glyph.index);
            lines
                .last_mut()
                .expect("fixed-text parser always has one line")
                .push(GaijiTextCell::Glyph {
                    glyph_index: glyph.index,
                    character_code: glyph.character_code,
                    shift_jis_code,
                    source: glyph.meaning.clone(),
                });
            continue;
        }
        ensure!(
            !is_target_gaiji_code(shift_jis_code),
            "fixed text references uninstalled Shift_JIS code {shift_jis_code:04X}"
        );
        let decoded = SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(pair)
            .with_context(|| {
                format!("fixed text has invalid standard Shift_JIS cell {shift_jis_code:04X}")
            })?;
        ensure!(
            decoded.chars().count() == 1
                && decoded.chars().all(|character| !character.is_control()),
            "fixed text standard cell {shift_jis_code:04X} is not one visible character"
        );
        lines
            .last_mut()
            .expect("fixed-text parser always has one line")
            .push(GaijiTextCell::Standard {
                text: decoded.into_owned(),
            });
    }

    ensure!(
        raw.ends_with(b"$$"),
        "fixed GAIJI text slot has no boundary terminator"
    );
    ensure!(
        lines.len() == 6 && lines.iter().all(|line| line.len() == 7),
        "fixed GAIJI text slot does not have the verified 6-line by 7-cell layout"
    );

    Ok((lines, used_glyph_indices.into_iter().collect()))
}

fn is_target_gaiji_code(code: u16) -> bool {
    (0xeb9f..=0xebfc).contains(&code) || (0xec40..=0xec9a).contains(&code)
}

#[cfg(test)]
#[path = "fixed_gaiji_text_tests.rs"]
mod fixed_gaiji_text_tests;
