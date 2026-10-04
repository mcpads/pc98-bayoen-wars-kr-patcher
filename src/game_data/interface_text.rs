use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use encoding_rs::SHIFT_JIS;
use serde::Serialize;

use super::binary::ensure_prefix;
use super::gaiji::{GaijiCatalog, GaijiGlyph};
use super::gaiji_meaning::GaijiGlyphMeaning;
use crate::byte_string::encode_lower_hex;
use crate::source_disk::sha256_hex;

#[path = "interface_text/references.rs"]
mod references;

use references::catalog_interface_text_references;
pub use references::{
    InterfaceTextReference, InterfaceTextReferenceCatalog, InterfaceTextReferenceKind,
    InterfaceTextReferenceTable,
};

const DIRECT_RENDERER_OFFSET: usize = 0x3a48;
const BUFFERED_RENDERER_OFFSET: usize = 0x339b;
const TEXT_START_OFFSET: usize = 0x73be;
const TEXT_END_OFFSET: usize = 0x784c;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct InterfaceTextCatalog {
    pub direct_renderer_offset: usize,
    pub buffered_renderer_offset: usize,
    pub text_start_offset: usize,
    pub text_end_offset: usize,
    pub strings_are_contiguous: bool,
    pub entry_count: usize,
    pub gaiji_reference_count: usize,
    pub unique_gaiji_glyph_count: usize,
    pub reference_catalog: InterfaceTextReferenceCatalog,
    pub entries: Vec<InterfaceTextEntry>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct InterfaceTextEntry {
    pub id: String,
    pub file_offset: usize,
    pub com_address: usize,
    pub byte_size: usize,
    pub sha256: String,
    pub raw_hex: String,
    pub text: String,
    pub gaiji_glyph_indices: Vec<usize>,
    pub tokens: Vec<InterfaceTextToken>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InterfaceTextToken {
    Text {
        text: String,
    },
    Gaiji {
        glyph_index: usize,
        character_code: u16,
        shift_jis_code: u16,
        source: GaijiGlyphMeaning,
    },
    LineBreak,
    DisplayAttribute {
        code: u8,
    },
}

#[derive(Debug)]
pub(crate) struct ParsedInterfaceText {
    pub end_offset: usize,
    pub text: String,
    pub gaiji_glyph_indices: Vec<usize>,
    pub tokens: Vec<InterfaceTextToken>,
}

pub(super) fn parse_interface_text_catalog(
    bytes: &[u8],
    gaiji: &GaijiCatalog,
) -> Result<InterfaceTextCatalog> {
    require_consumers_and_boundaries(bytes)?;
    let gaiji_by_shift_jis: BTreeMap<u16, &GaijiGlyph> = gaiji
        .glyphs
        .iter()
        .map(|glyph| (glyph.shift_jis_code, glyph))
        .collect();

    let mut entries = Vec::new();
    let mut cursor = TEXT_START_OFFSET;
    while cursor < TEXT_END_OFFSET {
        let parsed = parse_interface_text(bytes, cursor, &gaiji_by_shift_jis)
            .with_context(|| format!("invalid MAD interface text record {}", entries.len() + 1))?;
        ensure!(
            parsed.end_offset <= TEXT_END_OFFSET,
            "MAD interface text record {} crosses the verified text boundary",
            entries.len() + 1
        );
        let raw = &bytes[cursor..parsed.end_offset];
        entries.push(InterfaceTextEntry {
            id: format!("interface-text-{:03}", entries.len() + 1),
            file_offset: cursor,
            com_address: cursor + 0x100,
            byte_size: raw.len(),
            sha256: sha256_hex(raw),
            raw_hex: encode_lower_hex(raw),
            text: parsed.text,
            gaiji_glyph_indices: parsed.gaiji_glyph_indices,
            tokens: parsed.tokens,
        });
        cursor = parsed.end_offset;
    }
    ensure!(
        cursor == TEXT_END_OFFSET,
        "MAD interface text records do not end at the verified boundary"
    );
    ensure!(!entries.is_empty(), "MAD interface text catalog is empty");

    let gaiji_reference_count = entries
        .iter()
        .map(|entry| {
            entry
                .tokens
                .iter()
                .filter(|token| matches!(token, InterfaceTextToken::Gaiji { .. }))
                .count()
        })
        .sum();
    let unique_gaiji_glyph_count = entries
        .iter()
        .flat_map(|entry| entry.gaiji_glyph_indices.iter().copied())
        .collect::<BTreeSet<_>>()
        .len();
    let reference_catalog = catalog_interface_text_references(bytes, &entries)?;

    Ok(InterfaceTextCatalog {
        direct_renderer_offset: DIRECT_RENDERER_OFFSET,
        buffered_renderer_offset: BUFFERED_RENDERER_OFFSET,
        text_start_offset: TEXT_START_OFFSET,
        text_end_offset: TEXT_END_OFFSET,
        strings_are_contiguous: true,
        entry_count: entries.len(),
        gaiji_reference_count,
        unique_gaiji_glyph_count,
        reference_catalog,
        entries,
    })
}

fn require_consumers_and_boundaries(bytes: &[u8]) -> Result<()> {
    ensure_prefix(
        bytes,
        DIRECT_RENDERER_OFFSET,
        &[
            0xfc, 0xb2, 0xe1, 0x8b, 0xf3, 0xb9, 0x00, 0xa0, 0x8e, 0xc1, 0x8b, 0xcf, 0xad, 0x3d,
            0x24, 0x24, 0x74, 0x3b, 0x3d, 0x24, 0x30,
        ],
        "MAD direct interface-text renderer",
    )?;
    ensure_prefix(
        bytes,
        BUFFERED_RENDERER_OFFSET,
        &[
            0x2e, 0x89, 0x16, 0xcb, 0xd4, 0x2e, 0x89, 0x3e, 0xcd, 0xd4, 0x2e, 0x89, 0x3e, 0xcf,
            0xd4, 0x2e, 0xa2, 0xd3, 0xd4,
        ],
        "MAD buffered interface-text renderer",
    )?;
    ensure_prefix(
        bytes,
        0x5813,
        &[
            0xbf, 0x08, 0x14, 0xba, 0xc6, 0x74, 0xb0, 0x07, 0xe8, 0x0a, 0xfe, 0xbf, 0x08, 0x19,
            0xba, 0xbe, 0x74, 0xb0, 0x07, 0xe8, 0xff, 0xfd,
        ],
        "MAD first interface-text record consumers",
    )?;
    ensure_prefix(
        bytes,
        0x573c,
        &[
            0xbf, 0x18, 0x32, 0xba, 0x40, 0x79, 0xb0, 0x0f, 0xe8, 0xe1, 0xfe,
        ],
        "MAD final interface-text record consumer",
    )?;
    ensure_prefix(
        bytes,
        TEXT_START_OFFSET,
        &[0x8d, 0x55, 0x81, 0x40, 0x8c, 0x82, 0x24, 0x24],
        "MAD interface-text region start",
    )?;
    ensure_prefix(
        bytes,
        TEXT_END_OFFSET - 12,
        &[
            0x81, 0x40, 0xeb, 0xa9, 0xeb, 0xa2, 0xeb, 0xaf, 0xeb, 0xe6, 0x24, 0x24, 0x00, 0x00,
            0xc6, 0x06,
        ],
        "MAD interface-text region end",
    )?;
    Ok(())
}

fn parse_interface_text(
    bytes: &[u8],
    start: usize,
    gaiji_by_shift_jis: &BTreeMap<u16, &GaijiGlyph>,
) -> Result<ParsedInterfaceText> {
    ensure!(start < bytes.len(), "interface text starts outside MAD.COM");
    let mut cursor = start;
    let mut text = String::new();
    let mut tokens = Vec::new();
    let mut standard_start = cursor;
    let mut used_gaiji = BTreeSet::new();

    loop {
        let first = *bytes.get(cursor).with_context(|| {
            format!("unterminated interface text starting at file offset {start:#x}")
        })?;
        if first == b'$' {
            flush_standard_text(bytes, standard_start, cursor, &mut text, &mut tokens)?;
            let control = *bytes
                .get(cursor + 1)
                .with_context(|| format!("truncated text control at file offset {cursor:#x}"))?;
            cursor += 2;
            match control {
                b'$' => {
                    return Ok(ParsedInterfaceText {
                        end_offset: cursor,
                        text,
                        gaiji_glyph_indices: used_gaiji.into_iter().collect(),
                        tokens,
                    });
                }
                b'0' => {
                    text.push('\n');
                    tokens.push(InterfaceTextToken::LineBreak);
                }
                b'1'..=b'8' => {
                    let code = control - b'0';
                    text.push_str(&format!("<DISPLAY:{code}>"));
                    tokens.push(InterfaceTextToken::DisplayAttribute { code });
                }
                _ => bail!(
                    "unsupported interface text control ${} at file offset {:#x}",
                    char::from(control),
                    cursor - 2
                ),
            }
            standard_start = cursor;
            continue;
        }

        let width = shift_jis_character_width(bytes, cursor)?;
        if width == 2 {
            let shift_jis_code = u16::from_be_bytes([bytes[cursor], bytes[cursor + 1]]);
            if let Some(glyph) = gaiji_by_shift_jis.get(&shift_jis_code) {
                flush_standard_text(bytes, standard_start, cursor, &mut text, &mut tokens)?;
                text.push_str(&glyph.meaning.source_text());
                tokens.push(InterfaceTextToken::Gaiji {
                    glyph_index: glyph.index,
                    character_code: glyph.character_code,
                    shift_jis_code,
                    source: glyph.meaning.clone(),
                });
                used_gaiji.insert(glyph.index);
                cursor += 2;
                standard_start = cursor;
                continue;
            }
        }
        cursor += width;
    }
}

pub(crate) fn parse_interface_text_record(
    bytes: &[u8],
    start: usize,
    gaiji: &GaijiCatalog,
) -> Result<ParsedInterfaceText> {
    let gaiji_by_shift_jis = gaiji
        .glyphs
        .iter()
        .map(|glyph| (glyph.shift_jis_code, glyph))
        .collect();
    parse_interface_text(bytes, start, &gaiji_by_shift_jis)
}

fn flush_standard_text(
    bytes: &[u8],
    start: usize,
    end: usize,
    text: &mut String,
    tokens: &mut Vec<InterfaceTextToken>,
) -> Result<()> {
    if start == end {
        return Ok(());
    }
    let decoded = SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(&bytes[start..end])
        .with_context(|| format!("invalid Shift_JIS interface text at file offset {start:#x}"))?
        .into_owned();
    text.push_str(&decoded);
    tokens.push(InterfaceTextToken::Text { text: decoded });
    Ok(())
}

fn shift_jis_character_width(bytes: &[u8], offset: usize) -> Result<usize> {
    let first = *bytes
        .get(offset)
        .with_context(|| format!("text character starts outside the file at {offset:#x}"))?;
    if (0x20..=0x7e).contains(&first) || (0xa1..=0xdf).contains(&first) {
        return Ok(1);
    }
    if (0x81..=0x9f).contains(&first) || (0xe0..=0xfc).contains(&first) {
        let second = *bytes
            .get(offset + 1)
            .with_context(|| format!("truncated Shift_JIS character at file offset {offset:#x}"))?;
        ensure!(
            ((0x40..=0x7e).contains(&second) || (0x80..=0xfc).contains(&second)) && second != 0x7f,
            "invalid Shift_JIS trailing byte {second:02X} at file offset {offset:#x}"
        );
        return Ok(2);
    }
    bail!("invalid Shift_JIS leading byte {first:02X} at file offset {offset:#x}")
}

#[cfg(test)]
#[path = "interface_text_tests.rs"]
mod interface_text_tests;
