use anyhow::{Result, ensure};

use super::model::CompiledBattleCalloutEntry;
use crate::game_data::{InterfaceTextEntry, InterfaceTextToken};
use crate::korean_patch::shared_text::{CompiledGaijiBank, encode_shared_character};
use crate::translation_drafts::TranslationDraftEntry;

const CONTROL_PREFIX: u8 = b'$';

pub(super) fn compile_callout_record(
    source: &InterfaceTextEntry,
    draft: &TranslationDraftEntry,
    bank: &CompiledGaijiBank,
) -> Result<CompiledBattleCalloutEntry> {
    ensure!(source.id == draft.id, "battle callout identity changed");
    ensure!(
        !draft.korean_text.is_empty(),
        "{} has no Korean battle callout text",
        draft.id
    );
    let source_line_count = source_line_count(source)?;
    ensure!(
        draft.korean_text.len() == source_line_count,
        "{} has {} Korean lines but its battle consumer record has {source_line_count}",
        draft.id,
        draft.korean_text.len()
    );

    let mut bytes = Vec::new();
    for (line_index, line) in draft.korean_text.iter().enumerate() {
        for character in line.chars() {
            ensure!(
                character != char::from(CONTROL_PREFIX),
                "{} contains the reserved battle control prefix",
                draft.id
            );
            bytes.extend(encode_shared_character(character, bank)?);
        }
        if line_index + 1 < draft.korean_text.len() {
            bytes.extend_from_slice(b"$0");
        }
    }
    bytes.extend_from_slice(b"$$");
    Ok(CompiledBattleCalloutEntry {
        id: draft.id.clone(),
        original_file_offset: source.file_offset,
        file_offset: 0,
        com_address: 0,
        lines: draft.korean_text.clone(),
        bytes,
    })
}

fn source_line_count(source: &InterfaceTextEntry) -> Result<usize> {
    let mut visible = false;
    let mut line_breaks = 0;
    for token in &source.tokens {
        match token {
            InterfaceTextToken::Text { .. } | InterfaceTextToken::Gaiji { .. } => visible = true,
            InterfaceTextToken::LineBreak => {
                ensure!(visible, "{} starts with a line break", source.id);
                line_breaks += 1;
            }
            InterfaceTextToken::DisplayAttribute { .. } => {
                anyhow::bail!("{} unexpectedly contains a display attribute", source.id)
            }
        }
    }
    ensure!(visible, "{} has no visible source callout", source.id);
    Ok(line_breaks + 1)
}

#[cfg(test)]
#[path = "records_tests.rs"]
mod records_tests;
