use std::ops::Range;

use anyhow::{Context, Result, bail, ensure};
use encoding_rs::SHIFT_JIS;
use serde::Serialize;

use super::binary::{com_address_to_file_offset, ensure_prefix, read_u16};
use crate::byte_string::encode_lower_hex;
use crate::source_disk::sha256_hex;

#[path = "dialogue/references.rs"]
mod references;

use references::catalog_dialogue_references;
pub use references::{
    DialogueNonReferenceOccurrence, DialogueReference, DialogueReferenceCatalog,
    DialogueReferenceKind,
};

const CONSUMER_OFFSET: usize = 0xaf5f;
const TABLE_INSTRUCTION_OFFSET: usize = 0xaf64;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct DialogueCatalog {
    pub consumer_offset: usize,
    pub group_pointer_table_offset: usize,
    pub group_count: usize,
    pub entry_count: usize,
    pub text_start_offset: usize,
    pub text_end_offset: usize,
    pub strings_are_contiguous: bool,
    pub reference_catalog: DialogueReferenceCatalog,
    pub groups: Vec<DialogueGroup>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct DialogueGroup {
    pub id: String,
    pub pointer_table_offset: usize,
    pub record_table_offset: usize,
    pub entries: Vec<DialogueEntry>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct DialogueEntry {
    pub id: String,
    pub presentation_variant: u16,
    pub file_offset: usize,
    pub byte_size: usize,
    pub sha256: String,
    pub raw_hex: String,
    pub text: String,
    pub lines: Vec<String>,
}

struct ParsedDialogueStructure {
    group_pointer_table_offset: usize,
    group_count: usize,
    entry_count: usize,
    text_start_offset: usize,
    text_end_offset: usize,
    strings_are_contiguous: bool,
    groups: Vec<DialogueGroup>,
}

pub(super) fn parse_dialogue_catalog(bytes: &[u8]) -> Result<DialogueCatalog> {
    let structure = parse_dialogue_structure(bytes)?;
    let reference_catalog = catalog_dialogue_references(
        bytes,
        structure.group_pointer_table_offset,
        &structure.groups,
    )?;

    Ok(DialogueCatalog {
        consumer_offset: CONSUMER_OFFSET,
        group_pointer_table_offset: structure.group_pointer_table_offset,
        group_count: structure.group_count,
        entry_count: structure.entry_count,
        text_start_offset: structure.text_start_offset,
        text_end_offset: structure.text_end_offset,
        strings_are_contiguous: structure.strings_are_contiguous,
        reference_catalog,
        groups: structure.groups,
    })
}

fn parse_dialogue_structure(bytes: &[u8]) -> Result<ParsedDialogueStructure> {
    ensure_prefix(
        bytes,
        CONSUMER_OFFSET,
        &[0xa1, 0x8d, 0xdb, 0xd1, 0xe0, 0xbb],
        "MAD dialogue group selector",
    )?;
    ensure_prefix(
        bytes,
        TABLE_INSTRUCTION_OFFSET + 3,
        &[
            0x03, 0xd8, 0x2e, 0x8b, 0x1f, 0x2e, 0x8b, 0x07, 0x3d, 0xff, 0xff, 0x74, 0x3c, 0x2e,
            0x8b, 0x57, 0x02, 0x83, 0xc3, 0x04, 0x3d, 0x00, 0x00,
        ],
        "MAD dialogue record consumer",
    )?;

    let group_pointer_table_address = read_u16(bytes, TABLE_INSTRUCTION_OFFSET + 1)? as usize;
    let group_pointer_table_offset = com_address_to_file_offset(group_pointer_table_address)?;
    let first_group_address = read_u16(bytes, group_pointer_table_offset)? as usize;
    let first_group_offset = com_address_to_file_offset(first_group_address)?;
    ensure!(
        first_group_offset > group_pointer_table_offset,
        "MAD first dialogue record table does not follow its pointer table"
    );
    let pointer_table_size = first_group_offset - group_pointer_table_offset;
    ensure!(
        pointer_table_size.is_multiple_of(2),
        "MAD dialogue group pointer table has an odd byte size"
    );
    let group_count = pointer_table_size / 2;
    ensure!(group_count > 0, "MAD dialogue group table is empty");

    let mut groups = Vec::with_capacity(group_count);
    let mut all_entries = Vec::new();
    let mut previous_record_table_offset = None;
    for group_index in 0..group_count {
        let pointer_table_offset = group_pointer_table_offset + group_index * 2;
        let record_table_address = read_u16(bytes, pointer_table_offset)? as usize;
        let record_table_offset = com_address_to_file_offset(record_table_address)?;
        if let Some(previous) = previous_record_table_offset {
            ensure!(
                record_table_offset > previous,
                "MAD dialogue record tables are not strictly increasing"
            );
        }

        let mut cursor = record_table_offset;
        let mut entries = Vec::new();
        loop {
            let presentation_variant = read_u16(bytes, cursor)?;
            if presentation_variant == 0xffff {
                cursor = cursor
                    .checked_add(2)
                    .context("MAD dialogue record cursor overflow")?;
                break;
            }
            let text_address = read_u16(bytes, cursor + 2)? as usize;
            cursor = cursor
                .checked_add(4)
                .context("MAD dialogue record cursor overflow")?;
            ensure!(
                matches!(presentation_variant, 0 | 1),
                "MAD dialogue record has unsupported presentation variant {presentation_variant}"
            );
            let file_offset = com_address_to_file_offset(text_address)?;
            let parsed = parse_dialogue_text_record(bytes, file_offset).with_context(|| {
                format!(
                    "invalid dialogue text for group {} record {}",
                    group_index + 1,
                    entries.len() + 1
                )
            })?;
            let raw = &bytes[file_offset..parsed.end_offset];
            entries.push(DialogueEntry {
                id: format!("dialogue-g{:02}-r{:02}", group_index + 1, entries.len() + 1),
                presentation_variant,
                file_offset,
                byte_size: raw.len(),
                sha256: sha256_hex(raw),
                raw_hex: encode_lower_hex(raw),
                text: parsed.lines.join("\n"),
                lines: parsed.lines,
            });
            all_entries.push((file_offset, parsed.end_offset));
        }
        ensure!(
            !entries.is_empty(),
            "MAD dialogue group {} has no entries",
            group_index + 1
        );
        if let Some(next_group_pointer_offset) = (group_index + 1 < group_count)
            .then_some(group_pointer_table_offset + (group_index + 1) * 2)
        {
            let next_group_address = read_u16(bytes, next_group_pointer_offset)? as usize;
            let next_group_offset = com_address_to_file_offset(next_group_address)?;
            ensure!(
                cursor == next_group_offset,
                "MAD dialogue record table {} does not end at the next group",
                group_index + 1
            );
        }

        groups.push(DialogueGroup {
            id: format!("dialogue-group-{:02}", group_index + 1),
            pointer_table_offset,
            record_table_offset,
            entries,
        });
        previous_record_table_offset = Some(record_table_offset);
    }

    all_entries.sort_unstable();
    for ranges in all_entries.windows(2) {
        ensure!(
            ranges[0].1 <= ranges[1].0,
            "MAD dialogue strings overlap at file offsets {:#x} and {:#x}",
            ranges[0].0,
            ranges[1].0
        );
    }
    let strings_are_contiguous = all_entries
        .windows(2)
        .all(|ranges| ranges[0].1 == ranges[1].0);
    let (text_start_offset, text_end_offset) = match (all_entries.first(), all_entries.last()) {
        (Some(first), Some(last)) => (first.0, last.1),
        _ => bail!("MAD dialogue catalog has no text ranges"),
    };

    Ok(ParsedDialogueStructure {
        group_pointer_table_offset,
        group_count,
        entry_count: all_entries.len(),
        text_start_offset,
        text_end_offset,
        strings_are_contiguous,
        groups,
    })
}

#[derive(Debug)]
pub(crate) struct ParsedDialogueTextRecord {
    pub(crate) end_offset: usize,
    pub(crate) lines: Vec<String>,
}

pub(crate) struct ParsedDialogueTextControls {
    pub(crate) end_offset: usize,
    pub(crate) line_count: usize,
}

pub(crate) fn parse_dialogue_text_record(
    bytes: &[u8],
    start: usize,
) -> Result<ParsedDialogueTextRecord> {
    let boundaries = scan_dialogue_text(bytes, start)?;
    let lines = boundaries
        .line_ranges
        .iter()
        .map(|range| decode_shift_jis_line(&bytes[range.clone()], range.start))
        .collect::<Result<Vec<_>>>()?;
    Ok(ParsedDialogueTextRecord {
        end_offset: boundaries.end_offset,
        lines,
    })
}

pub(crate) fn parse_dialogue_text_controls(
    bytes: &[u8],
    start: usize,
) -> Result<ParsedDialogueTextControls> {
    let boundaries = scan_dialogue_text(bytes, start)?;
    Ok(ParsedDialogueTextControls {
        end_offset: boundaries.end_offset,
        line_count: boundaries.line_ranges.len(),
    })
}

struct DialogueTextBoundaries {
    end_offset: usize,
    line_ranges: Vec<Range<usize>>,
}

fn scan_dialogue_text(bytes: &[u8], start: usize) -> Result<DialogueTextBoundaries> {
    ensure!(start < bytes.len(), "text starts outside MAD.COM");
    let mut cursor = start;
    let mut line_start = start;
    let mut line_ranges = Vec::new();

    loop {
        let byte = *bytes
            .get(cursor)
            .with_context(|| format!("unterminated text starting at file offset {start:#x}"))?;
        if byte == b'$' {
            let control = *bytes
                .get(cursor + 1)
                .with_context(|| format!("truncated text control at file offset {cursor:#x}"))?;
            line_ranges.push(line_start..cursor);
            match control {
                b'0' => {
                    cursor += 2;
                    line_start = cursor;
                }
                b'$' => {
                    return Ok(DialogueTextBoundaries {
                        end_offset: cursor + 2,
                        line_ranges,
                    });
                }
                _ => {
                    bail!(
                        "unsupported text control ${} at file offset {cursor:#x}",
                        char::from(control)
                    );
                }
            }
            continue;
        }

        cursor += shift_jis_character_width(bytes, cursor)?;
    }
}

fn decode_shift_jis_line(bytes: &[u8], offset: usize) -> Result<String> {
    let (decoded, _, had_errors) = SHIFT_JIS.decode(bytes);
    ensure!(
        !had_errors,
        "invalid Shift_JIS text at file offset {offset:#x}"
    );
    Ok(decoded.into_owned())
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
#[path = "dialogue_tests.rs"]
mod dialogue_tests;
