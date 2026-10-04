use std::collections::BTreeMap;
use std::ops::Range;

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::Instruction;

use super::monochrome_sprites::{MonochromeSpriteCatalog, decode_single};
use super::typed_v30::decode_complete_block;
use crate::byte_string::encode_lower_hex;
use crate::source_disk::sha256_hex;

const COM_LOAD_BIAS: usize = 0x100;
const GLYPH_RENDERER_RANGE: Range<usize> = 0x9cb4..0x9cc9;
const OPENING_POINTER_TABLE_RANGE: Range<usize> = 0x9bb9..0x9be1;
const OPENING_TABLE_CONSUMER_RANGE: Range<usize> = 0x9b1f..0x9b46;
const ENDING_POINTER_TABLE_RANGE: Range<usize> = 0x9ca0..0x9cb4;
const ENDING_TABLE_CONSUMER_RANGE: Range<usize> = 0x9be1..0x9c08;
const GLYPH_WIDTH: usize = 32;
const GLYPH_HEIGHT: usize = 32;
const GLYPH_SIZE: usize = GLYPH_WIDTH / 8 * GLYPH_HEIGHT;
const LINE_BREAK: u8 = 0xfe;
const PAGE_END: u8 = 0xff;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MonochromeTextCatalog {
    pub glyph_renderer_file_offset: usize,
    pub glyph_renderer_instruction_count: usize,
    pub glyph_renderer_call_count: usize,
    pub opening: MonochromeTextSequence,
    pub ending: MonochromeTextSequence,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MonochromeTextSequence {
    pub id: String,
    pub source_asset: String,
    pub pointer_table_file_offset: usize,
    pub table_consumer_file_offset: usize,
    pub table_consumer_instruction_count: usize,
    pub glyph_width: usize,
    pub glyph_height: usize,
    pub glyph_capacity: usize,
    pub glyphs: Vec<MonochromeGlyph>,
    pub pages: Vec<MonochromeTextPage>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MonochromeGlyph {
    pub id: String,
    pub index: usize,
    pub decoded_offset: usize,
    pub byte_size: usize,
    pub content_sha256: String,
    pub raw_hex: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MonochromeTextPage {
    pub id: String,
    pub pointer_entry_file_offset: usize,
    pub file_offset: usize,
    pub byte_size: usize,
    pub content_sha256: String,
    pub raw_hex: String,
    pub glyph_count: usize,
    pub lines: Vec<Vec<usize>>,
}

pub(crate) fn catalog_monochrome_text(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
    sprites: &MonochromeSpriteCatalog,
) -> Result<MonochromeTextCatalog> {
    let renderer =
        decode_complete_block(mad_com, GLYPH_RENDERER_RANGE, "monochrome glyph renderer")?;
    let renderer_call_count = renderer
        .iter()
        .filter(|instruction| matches!(instruction, Instruction::Call { .. }))
        .count();
    ensure!(
        renderer_call_count > 0,
        "monochrome glyph renderer contains no typed CALL"
    );

    Ok(MonochromeTextCatalog {
        glyph_renderer_file_offset: GLYPH_RENDERER_RANGE.start,
        glyph_renderer_instruction_count: renderer.len(),
        glyph_renderer_call_count: renderer_call_count,
        opening: catalog_sequence(
            mad_com,
            installer_payload,
            sprites,
            SequenceSpec {
                id: "opening",
                source_asset: "OPM.DAT",
                pointer_table_range: OPENING_POINTER_TABLE_RANGE,
                table_consumer_range: OPENING_TABLE_CONSUMER_RANGE,
            },
        )?,
        ending: catalog_sequence(
            mad_com,
            installer_payload,
            sprites,
            SequenceSpec {
                id: "ending",
                source_asset: "EDM.DAT",
                pointer_table_range: ENDING_POINTER_TABLE_RANGE,
                table_consumer_range: ENDING_TABLE_CONSUMER_RANGE,
            },
        )?,
    })
}

struct SequenceSpec {
    id: &'static str,
    source_asset: &'static str,
    pointer_table_range: Range<usize>,
    table_consumer_range: Range<usize>,
}

fn catalog_sequence(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
    sprites: &MonochromeSpriteCatalog,
    spec: SequenceSpec,
) -> Result<MonochromeTextSequence> {
    let expected_glyph_count = sprites
        .assets
        .iter()
        .find(|asset| asset.name == spec.source_asset)
        .with_context(|| format!("missing sprite binding for {}", spec.source_asset))?
        .sprite_count;
    let decoded = decode_single(installer_payload, spec.source_asset)?;
    ensure!(
        decoded.len() == expected_glyph_count * GLYPH_SIZE,
        "{} decoded size differs from its sprite binding",
        spec.source_asset
    );
    let glyphs = catalog_glyphs(spec.id, &decoded);
    let pages = parse_pages(
        mad_com,
        spec.id,
        spec.pointer_table_range.clone(),
        glyphs.len(),
    )?;
    let table_consumer = decode_complete_block(
        mad_com,
        spec.table_consumer_range.clone(),
        &format!("{} page-table consumer", spec.id),
    )?;

    Ok(MonochromeTextSequence {
        id: spec.id.to_owned(),
        source_asset: spec.source_asset.to_owned(),
        pointer_table_file_offset: spec.pointer_table_range.start,
        table_consumer_file_offset: spec.table_consumer_range.start,
        table_consumer_instruction_count: table_consumer.len(),
        glyph_width: GLYPH_WIDTH,
        glyph_height: GLYPH_HEIGHT,
        glyph_capacity: sprites.runtime.usable_glyph_count,
        glyphs,
        pages,
    })
}

fn catalog_glyphs(sequence_id: &str, decoded: &[u8]) -> Vec<MonochromeGlyph> {
    decoded
        .as_chunks::<GLYPH_SIZE>()
        .0
        .iter()
        .enumerate()
        .map(|(index, bytes)| MonochromeGlyph {
            id: format!("{sequence_id}-glyph-{index:03}"),
            index,
            decoded_offset: index * GLYPH_SIZE,
            byte_size: bytes.len(),
            content_sha256: sha256_hex(bytes),
            raw_hex: encode_lower_hex(bytes),
        })
        .collect()
}

fn parse_pages(
    program: &[u8],
    sequence_id: &str,
    pointer_table_range: Range<usize>,
    glyph_count: usize,
) -> Result<Vec<MonochromeTextPage>> {
    let table = program
        .get(pointer_table_range.clone())
        .with_context(|| format!("{sequence_id} pointer table lies outside MAD.COM"))?;
    ensure!(
        table.len().is_multiple_of(2),
        "{sequence_id} pointer table has a partial entry"
    );
    let mut pointers = Vec::new();
    for entry in table.as_chunks::<2>().0 {
        let pointer = u16::from_le_bytes([entry[0], entry[1]]) as usize;
        if pointer == 0xffff {
            break;
        }
        let file_offset = pointer
            .checked_sub(COM_LOAD_BIAS)
            .with_context(|| format!("{sequence_id} pointer precedes the COM load bias"))?;
        ensure!(
            pointers
                .last()
                .is_none_or(|previous| *previous < file_offset),
            "{sequence_id} page pointers are not strictly increasing"
        );
        pointers.push(file_offset);
    }
    ensure!(!pointers.is_empty(), "{sequence_id} pointer table is empty");
    ensure!(
        table.get(pointers.len() * 2..pointers.len() * 2 + 2) == Some(&[0xff, 0xff]),
        "{sequence_id} pointer table has no terminator"
    );
    ensure!(
        pointers.len() * 2 + 2 == table.len(),
        "{sequence_id} pointer table has trailing bytes"
    );

    pointers
        .iter()
        .enumerate()
        .map(|(index, &start)| {
            let end = if let Some(&next) = pointers.get(index + 1) {
                next
            } else {
                let rest = program
                    .get(start..)
                    .with_context(|| format!("{sequence_id} page lies outside MAD.COM"))?;
                start
                    + rest
                        .iter()
                        .position(|byte| *byte == PAGE_END)
                        .context("final monochrome text page has no terminator")?
                    + 1
            };
            let bytes = program
                .get(start..end)
                .with_context(|| format!("{sequence_id} page range lies outside MAD.COM"))?;
            let lines = parse_page(bytes, glyph_count)
                .with_context(|| format!("invalid {sequence_id} page {}", index + 1))?;
            Ok(MonochromeTextPage {
                id: format!("{sequence_id}-page-{:02}", index + 1),
                pointer_entry_file_offset: pointer_table_range.start + index * 2,
                file_offset: start,
                byte_size: bytes.len(),
                content_sha256: sha256_hex(bytes),
                raw_hex: encode_lower_hex(bytes),
                glyph_count: lines.iter().map(Vec::len).sum(),
                lines,
            })
        })
        .collect()
}

fn parse_page(bytes: &[u8], glyph_count: usize) -> Result<Vec<Vec<usize>>> {
    ensure!(
        bytes.last() == Some(&PAGE_END),
        "page has no trailing terminator"
    );
    ensure!(
        !bytes[..bytes.len() - 1].contains(&PAGE_END),
        "page has an early terminator"
    );
    let mut lines = vec![Vec::new()];
    for &byte in &bytes[..bytes.len() - 1] {
        if byte == LINE_BREAK {
            ensure!(!lines.last().unwrap().is_empty(), "page has an empty line");
            lines.push(Vec::new());
        } else {
            ensure!(
                usize::from(byte) < glyph_count,
                "glyph index {byte} exceeds atlas size {glyph_count}"
            );
            lines.last_mut().unwrap().push(usize::from(byte));
        }
    }
    ensure!(
        lines.iter().all(|line| !line.is_empty()),
        "page has an empty line"
    );
    Ok(lines)
}

#[cfg(test)]
#[path = "monochrome_text_tests.rs"]
mod monochrome_text_tests;
