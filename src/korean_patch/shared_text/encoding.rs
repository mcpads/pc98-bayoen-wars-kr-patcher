use anyhow::{Result, ensure};
use encoding_rs::SHIFT_JIS;

use super::CompiledGaijiBank;

const FULL_WIDTH_SPACE: [u8; 2] = [0x81, 0x40];

pub(crate) fn encode_shared_character(
    character: char,
    bank: &CompiledGaijiBank,
) -> Result<Vec<u8>> {
    if character == ' ' {
        return Ok(FULL_WIDTH_SPACE.to_vec());
    }
    encode_visible_character(character, bank)
}

pub(crate) fn encode_dos_character(character: char, bank: &CompiledGaijiBank) -> Result<Vec<u8>> {
    if character == ' ' {
        return Ok(vec![b' ']);
    }
    encode_visible_character(character, bank)
}

fn encode_visible_character(character: char, bank: &CompiledGaijiBank) -> Result<Vec<u8>> {
    ensure!(
        !character.is_control() && !character.is_whitespace(),
        "shared text contains unsupported whitespace {character:?}"
    );
    if let Some(code) = bank.character_codes.get(&character) {
        return Ok(code.to_be_bytes().to_vec());
    }
    let source = character.to_string();
    let (encoded, _, had_errors) = SHIFT_JIS.encode(&source);
    if !had_errors {
        ensure!(
            matches!(encoded.len(), 1 | 2),
            "shared text native character {character:?} has an unexpected byte width"
        );
        return Ok(encoded.into_owned());
    }
    Err(anyhow::anyhow!(
        "shared text has unassigned GAIJI {character:?}"
    ))
}

#[cfg(test)]
#[path = "encoding_tests.rs"]
mod encoding_tests;
