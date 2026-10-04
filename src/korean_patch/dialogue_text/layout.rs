use std::path::Path;

use anyhow::{Result, ensure};
use encoding_rs::SHIFT_JIS;

use crate::korean_patch::DIALOGUE_GAIJI_PUNCTUATION;
use crate::translation_analysis::load_translation_corpus;
use crate::translation_drafts::TranslationSurface;

const SOURCE_TILE_PIXELS: usize = 16;
// Runtime 0xB0D6 and 0xB113 prepare an 18x6-tile dialogue region. The flag set
// at runtime 0xB05A makes the text consumer draw each source glyph at 2x scale.
const DIALOGUE_REGION_TILE_COLUMNS: usize = 18;
const DIALOGUE_REGION_TILE_ROWS: usize = 6;
const DIALOGUE_CHARACTER_SCALE: usize = 2;

const DIALOGUE_LINE_PIXELS: usize = SOURCE_TILE_PIXELS * DIALOGUE_REGION_TILE_COLUMNS;
const DIALOGUE_LINE_COUNT: usize = DIALOGUE_REGION_TILE_ROWS / DIALOGUE_CHARACTER_SCALE;
const DIALOGUE_PIXELS_PER_ENCODED_BYTE: usize = SOURCE_TILE_PIXELS * DIALOGUE_CHARACTER_SCALE / 2;

pub(crate) fn validate_dialogue_rom_layout(translations: &Path) -> Result<()> {
    let corpus = load_translation_corpus(translations)?;
    for segment in corpus
        .segments
        .iter()
        .filter(|segment| segment.surface == TranslationSurface::Dialogue)
    {
        for entry in &segment.entries {
            validate_dialogue_entry_layout(&entry.id, &entry.korean_text)?;
        }
    }
    Ok(())
}

fn validate_dialogue_entry_layout(entry_id: &str, lines: &[String]) -> Result<()> {
    ensure!(
        lines.len() <= DIALOGUE_LINE_COUNT,
        "dialogue ROM layout {entry_id} uses {} lines but the verified consumer region holds {DIALOGUE_LINE_COUNT}",
        lines.len()
    );
    for (line_index, line) in lines.iter().enumerate() {
        let pixel_width = dialogue_line_pixel_width(line)?;
        ensure!(
            pixel_width <= DIALOGUE_LINE_PIXELS,
            "dialogue ROM layout {entry_id} line {} uses {pixel_width}px but the verified consumer region holds {DIALOGUE_LINE_PIXELS}px",
            line_index + 1
        );
    }
    Ok(())
}

fn dialogue_line_pixel_width(line: &str) -> Result<usize> {
    line.chars().try_fold(0, |width, character| {
        let encoded_byte_width = if character == ' ' {
            // The shared-text encoder emits Shift_JIS 0x8140 for spaces.
            2
        } else {
            ensure!(
                !character.is_control() && !character.is_whitespace(),
                "dialogue ROM layout contains unsupported whitespace {character:?}"
            );
            let source = character.to_string();
            let (encoded, _, had_errors) = SHIFT_JIS.encode(&source);
            if had_errors || DIALOGUE_GAIJI_PUNCTUATION.contains(&character) {
                // Every non-native translated character is a two-byte GAIJI code.
                2
            } else {
                ensure!(
                    matches!(encoded.len(), 1 | 2),
                    "dialogue ROM layout character {character:?} has an unexpected encoded width"
                );
                encoded.len()
            }
        };
        Ok(width + encoded_byte_width * DIALOGUE_PIXELS_PER_ENCODED_BYTE)
    })
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod layout_tests;
