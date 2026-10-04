use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use super::MonochromeGlyphSlot;
use crate::korean_patch::font_catalog::FontTarget;
use crate::korean_patch::font_rasterizer::LargeCellRasterizer;

pub(super) const GLYPH_WIDTH: usize = 32;
pub(super) const GLYPH_HEIGHT: usize = 32;
pub(super) const GLYPH_BYTE_SIZE: usize = GLYPH_WIDTH / 8 * GLYPH_HEIGHT;

#[derive(Debug)]
pub(super) struct CompiledAtlas {
    pub glyph_indices: BTreeMap<char, u8>,
    pub glyph_slots: Vec<MonochromeGlyphSlot>,
    pub decoded: Vec<u8>,
}

pub(super) fn compile_atlas(lines: &[&str], capacity: usize) -> Result<CompiledAtlas> {
    ensure!(
        capacity > 0 && capacity <= 0xfe,
        "monochrome glyph capacity must fit below the control-byte range"
    );
    let mut characters = Vec::new();
    if lines.iter().any(|line| line.contains(' ')) {
        characters.push(' ');
    }
    for line in lines {
        for character in line.chars() {
            ensure!(
                character == ' ' || (!character.is_control() && !character.is_whitespace()),
                "monochrome text contains unsupported whitespace {character:?}"
            );
            if !characters.contains(&character) {
                characters.push(character);
            }
        }
    }
    ensure!(
        characters.len() <= capacity,
        "monochrome text needs {} glyphs but the atlas has {capacity} slots",
        characters.len()
    );

    let mut decoded = Vec::with_capacity(capacity * GLYPH_BYTE_SIZE);
    let mut glyph_indices = BTreeMap::new();
    let mut glyph_slots = Vec::with_capacity(characters.len());
    let rasterizer = LargeCellRasterizer::load_for(FontTarget::Narrative32)?;
    for (index, character) in characters.into_iter().enumerate() {
        let index = u8::try_from(index).expect("capacity is below the control-byte range");
        let glyph = if character == ' ' {
            [0; GLYPH_BYTE_SIZE]
        } else {
            rasterizer
                .rasterize_visible_character(character)
                .with_context(|| format!("failed to rasterize {character:?}"))?
        };
        decoded.extend_from_slice(&glyph);
        glyph_indices.insert(character, index);
        glyph_slots.push(MonochromeGlyphSlot {
            index: usize::from(index),
            character: character.to_string(),
        });
    }
    decoded.resize(capacity * GLYPH_BYTE_SIZE, 0);
    Ok(CompiledAtlas {
        glyph_indices,
        glyph_slots,
        decoded,
    })
}

#[cfg(test)]
#[path = "atlas_tests.rs"]
mod atlas_tests;
