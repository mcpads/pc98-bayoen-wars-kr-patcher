use anyhow::{Result, ensure};

use super::model::CompiledDialogueTextEntry;
use crate::game_data::DialogueEntry;
use crate::korean_patch::shared_text::{CompiledGaijiBank, encode_shared_character};
use crate::translation_drafts::TranslationDraftEntry;

const CONTROL_PREFIX: u8 = b'$';
const LINE_BREAK: u8 = b'0';
const TERMINATOR: u8 = b'$';

pub(super) fn compile_dialogue_record(
    group_id: &str,
    text_pointer_offset: usize,
    source: &DialogueEntry,
    draft: &TranslationDraftEntry,
    bank: &CompiledGaijiBank,
) -> Result<CompiledDialogueTextEntry> {
    ensure!(source.id == draft.id, "dialogue text identity changed");
    ensure!(
        !draft.korean_text.is_empty(),
        "{} has no Korean development text",
        draft.id
    );
    let mut bytes = Vec::new();
    for (line_index, line) in draft.korean_text.iter().enumerate() {
        ensure!(!line.is_empty(), "{} has an empty dialogue line", draft.id);
        for character in line.chars() {
            ensure!(
                character != char::from(CONTROL_PREFIX),
                "{} contains the reserved dialogue control prefix",
                draft.id
            );
            bytes.extend(encode_shared_character(character, bank)?);
        }
        if line_index + 1 < draft.korean_text.len() {
            bytes.extend_from_slice(&[CONTROL_PREFIX, LINE_BREAK]);
        }
    }
    bytes.extend_from_slice(&[CONTROL_PREFIX, TERMINATOR]);

    Ok(CompiledDialogueTextEntry {
        id: draft.id.clone(),
        group_id: group_id.to_owned(),
        presentation_variant: source.presentation_variant,
        text_pointer_offset,
        original_file_offset: source.file_offset,
        file_offset: 0,
        com_address: 0,
        lines: draft.korean_text.clone(),
        bytes,
    })
}

#[cfg(test)]
#[path = "records_tests.rs"]
mod records_tests;
