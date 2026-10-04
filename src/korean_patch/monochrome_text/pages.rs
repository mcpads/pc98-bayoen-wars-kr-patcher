use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use crate::asset_bindings::MonochromeTextSequence;
use crate::translation_drafts::TranslationDraftEntry;

const COM_LOAD_BIAS: usize = 0x100;
const LINE_BREAK: u8 = 0xfe;
const PAGE_END: u8 = 0xff;
const MAXIMUM_LINE_COUNT: usize = 4;
const MAXIMUM_COLUMN_COUNT: usize = 16;

pub(super) struct CompiledPages {
    pub pointer_table: Vec<u8>,
    pub page_data: Vec<u8>,
    pub lines: Vec<Vec<Vec<usize>>>,
    pub page_byte_sizes: Vec<usize>,
    pub storage_capacity: usize,
}

pub(super) fn compile_pages(
    sequence: &MonochromeTextSequence,
    entries: &[TranslationDraftEntry],
    glyph_indices: &BTreeMap<char, u8>,
) -> Result<CompiledPages> {
    ensure!(
        entries.len() == sequence.pages.len(),
        "{} translation page count differs from the source consumer",
        sequence.id
    );
    let page_data_start = sequence
        .pages
        .first()
        .context("monochrome source sequence has no pages")?
        .file_offset;
    let mut expected_offset = page_data_start;
    for page in &sequence.pages {
        ensure!(
            page.file_offset == expected_offset,
            "{} source pages are not contiguous",
            sequence.id
        );
        expected_offset += page.byte_size;
    }
    let storage_capacity = expected_offset - page_data_start;
    let mut pointer_table = Vec::with_capacity(entries.len() * 2 + 2);
    let mut page_data = Vec::new();
    let mut compiled_lines = Vec::with_capacity(entries.len());
    let mut page_byte_sizes = Vec::with_capacity(entries.len());

    for (index, (entry, source_page)) in entries.iter().zip(&sequence.pages).enumerate() {
        let expected_id = format!("{}-page-{:02}", sequence.id, index + 1);
        ensure!(
            entry.id == expected_id && source_page.id == expected_id,
            "{} page order differs from the source consumer",
            sequence.id
        );
        let file_offset = page_data_start + page_data.len();
        let runtime_address = file_offset
            .checked_add(COM_LOAD_BIAS)
            .context("monochrome page runtime address overflow")?;
        pointer_table.extend_from_slice(
            &u16::try_from(runtime_address)
                .context("monochrome page lies above the COM address space")?
                .to_le_bytes(),
        );
        let (encoded, lines) = encode_page(&entry.korean_text, glyph_indices)
            .with_context(|| format!("failed to encode {}", entry.id))?;
        page_byte_sizes.push(encoded.len());
        page_data.extend_from_slice(&encoded);
        compiled_lines.push(lines);
    }
    pointer_table.extend_from_slice(&u16::MAX.to_le_bytes());
    ensure!(
        page_data.len() <= storage_capacity,
        "{} compiled pages need {} bytes but source storage has {storage_capacity}",
        sequence.id,
        page_data.len()
    );
    Ok(CompiledPages {
        pointer_table,
        page_data,
        lines: compiled_lines,
        page_byte_sizes,
        storage_capacity,
    })
}

fn encode_page(
    text_lines: &[String],
    glyph_indices: &BTreeMap<char, u8>,
) -> Result<(Vec<u8>, Vec<Vec<usize>>)> {
    ensure!(
        !text_lines.is_empty() && text_lines.len() <= MAXIMUM_LINE_COUNT,
        "page must contain 1..={MAXIMUM_LINE_COUNT} lines"
    );
    let mut encoded = Vec::new();
    let mut lines = Vec::with_capacity(text_lines.len());
    for (line_index, line) in text_lines.iter().enumerate() {
        let characters = line.chars().collect::<Vec<_>>();
        ensure!(
            !characters.is_empty() && characters.len() <= MAXIMUM_COLUMN_COUNT,
            "page line must contain 1..={MAXIMUM_COLUMN_COUNT} glyphs"
        );
        if line_index != 0 {
            encoded.push(LINE_BREAK);
        }
        let mut indices = Vec::with_capacity(characters.len());
        for character in characters {
            let index = *glyph_indices
                .get(&character)
                .with_context(|| format!("page character {character:?} has no atlas slot"))?;
            ensure!(
                index < LINE_BREAK,
                "page character {character:?} collides with a control byte"
            );
            encoded.push(index);
            indices.push(usize::from(index));
        }
        lines.push(indices);
    }
    encoded.push(PAGE_END);
    Ok((encoded, lines))
}

#[cfg(test)]
#[path = "pages_tests.rs"]
mod pages_tests;
