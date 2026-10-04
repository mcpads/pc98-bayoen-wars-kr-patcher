use anyhow::{Context, Result, ensure};
use encoding_rs::SHIFT_JIS;

use super::model::CompiledFixedGaijiSlot;
use crate::game_data::FixedGaijiTextSlot;
use crate::korean_patch::shared_text::CompiledGaijiBank;
use crate::translation_drafts::TranslationDraftEntry;

const LINE_COUNT: usize = 6;
const LINE_CELLS: usize = 7;
const SLOT_SIZE: usize = 0x60;
const FULL_WIDTH_SPACE: [u8; 2] = [0x81, 0x40];

pub(super) fn compile_slot(
    source: &FixedGaijiTextSlot,
    draft: &TranslationDraftEntry,
    bank: &CompiledGaijiBank,
) -> Result<CompiledFixedGaijiSlot> {
    ensure!(source.id == draft.id, "fixed GAIJI slot identity changed");
    ensure!(
        source.byte_size == SLOT_SIZE,
        "{} fixed GAIJI slot size changed",
        source.id
    );
    ensure!(
        !draft.korean_text.is_empty() && draft.korean_text.len() <= LINE_COUNT,
        "{} needs one to {LINE_COUNT} visible lines",
        draft.id
    );
    let mut bytes = Vec::with_capacity(SLOT_SIZE);
    for line_index in 0..LINE_COUNT {
        let line = draft
            .korean_text
            .get(line_index)
            .map(String::as_str)
            .unwrap_or("");
        let characters = line.chars().collect::<Vec<_>>();
        ensure!(
            characters.len() <= LINE_CELLS,
            "{} line {} needs {} cells but only {LINE_CELLS} are available",
            draft.id,
            line_index + 1,
            characters.len()
        );
        for character in characters.iter().copied() {
            if character == ' ' {
                bytes.extend_from_slice(&FULL_WIDTH_SPACE);
            } else if let Some(cell) = fixed_shift_jis_cell(character)? {
                bytes.extend_from_slice(&cell);
            } else {
                let code = bank
                    .character_codes
                    .get(&character)
                    .with_context(|| format!("{} has unassigned GAIJI {character:?}", draft.id))?;
                bytes.extend_from_slice(&code.to_be_bytes());
            }
        }
        for _ in characters.len()..LINE_CELLS {
            bytes.extend_from_slice(&FULL_WIDTH_SPACE);
        }
        bytes.extend_from_slice(if line_index + 1 == LINE_COUNT {
            b"$$"
        } else {
            b"$0"
        });
    }
    ensure!(
        bytes.len() == SLOT_SIZE,
        "{} fixed GAIJI encoder produced the wrong size",
        draft.id
    );
    Ok(CompiledFixedGaijiSlot {
        id: draft.id.clone(),
        file_offset: source.file_offset,
        lines: draft.korean_text.clone(),
        bytes,
    })
}

fn fixed_shift_jis_cell(character: char) -> Result<Option<[u8; 2]>> {
    let fixed_character = match character {
        '!' => '！',
        '?' => '？',
        '~' => '～',
        '.' => '。',
        ',' => '、',
        '!'..='~' => char::from_u32(character as u32 + 0xfee0)
            .context("ASCII full-width cell conversion failed")?,
        _ => character,
    };
    let fixed_text = fixed_character.to_string();
    let (encoded, _, had_errors) = SHIFT_JIS.encode(&fixed_text);
    if had_errors {
        return Ok(None);
    }
    ensure!(
        encoded.len() == 2,
        "fixed text native character {character:?} is not one 2-byte Shift_JIS cell"
    );
    Ok(Some([encoded[0], encoded[1]]))
}

#[cfg(test)]
#[path = "pages_tests.rs"]
mod pages_tests;
