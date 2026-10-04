use anyhow::{Result, ensure};

use super::model::CompiledInterfaceTextEntry;
use crate::game_data::{InterfaceTextEntry, InterfaceTextToken};
use crate::korean_patch::shared_text::{CompiledGaijiBank, encode_shared_character};
use crate::translation_drafts::TranslationDraftEntry;

const CONTROL_PREFIX: u8 = b'$';
const LINE_BREAK: u8 = b'0';
const TERMINATOR: u8 = b'$';

pub(in crate::korean_patch) fn compile_record(
    source: &InterfaceTextEntry,
    draft: &TranslationDraftEntry,
    bank: &CompiledGaijiBank,
) -> Result<CompiledInterfaceTextEntry> {
    ensure!(source.id == draft.id, "interface text identity changed");
    ensure!(
        !draft.korean_text.is_empty(),
        "{} has no Korean development text",
        draft.id
    );
    let (leading_attributes, trailing_attributes, source_line_count) = preserved_controls(source)?;
    ensure!(
        draft.korean_text.len() == source_line_count,
        "{} has {} Korean lines but its consumer record has {source_line_count}",
        draft.id,
        draft.korean_text.len()
    );

    let mut bytes = Vec::new();
    let mut line_screen_byte_widths = Vec::with_capacity(draft.korean_text.len());
    append_attributes(&mut bytes, &leading_attributes);
    for (line_index, line) in draft.korean_text.iter().enumerate() {
        let mut screen_byte_width = 0;
        for character in line.chars() {
            ensure!(
                character != char::from(CONTROL_PREFIX),
                "{} contains the reserved interface control prefix",
                draft.id
            );
            let encoded = encode_shared_character(character, bank)?;
            screen_byte_width += encoded.len();
            bytes.extend(encoded);
        }
        line_screen_byte_widths.push(screen_byte_width);
        if line_index + 1 < draft.korean_text.len() {
            bytes.extend_from_slice(&[CONTROL_PREFIX, LINE_BREAK]);
        }
    }
    append_attributes(&mut bytes, &trailing_attributes);
    bytes.extend_from_slice(&[CONTROL_PREFIX, TERMINATOR]);

    Ok(CompiledInterfaceTextEntry {
        id: draft.id.clone(),
        original_file_offset: source.file_offset,
        file_offset: 0,
        com_address: 0,
        lines: draft.korean_text.clone(),
        line_screen_byte_widths,
        leading_attributes,
        trailing_attributes,
        bytes,
    })
}

fn preserved_controls(source: &InterfaceTextEntry) -> Result<(Vec<u8>, Vec<u8>, usize)> {
    let mut leading = Vec::new();
    let mut trailing = Vec::new();
    let mut saw_visible = false;
    let mut saw_trailing_attribute = false;
    let mut line_break_count = 0;
    for token in &source.tokens {
        match token {
            InterfaceTextToken::Text { .. } | InterfaceTextToken::Gaiji { .. } => {
                ensure!(
                    !saw_trailing_attribute,
                    "{} has a display attribute embedded inside visible text",
                    source.id
                );
                saw_visible = true;
            }
            InterfaceTextToken::LineBreak => {
                ensure!(
                    saw_visible && !saw_trailing_attribute,
                    "{} has a line break outside visible text",
                    source.id
                );
                line_break_count += 1;
            }
            InterfaceTextToken::DisplayAttribute { code } => {
                ensure!(
                    (1..=8).contains(code),
                    "{} has an unsupported display attribute",
                    source.id
                );
                if saw_visible {
                    saw_trailing_attribute = true;
                    trailing.push(*code);
                } else {
                    leading.push(*code);
                }
            }
        }
    }
    ensure!(saw_visible, "{} has no visible source text", source.id);
    Ok((leading, trailing, line_break_count + 1))
}

fn append_attributes(bytes: &mut Vec<u8>, attributes: &[u8]) {
    for attribute in attributes {
        bytes.extend_from_slice(&[CONTROL_PREFIX, b'0' + *attribute]);
    }
}

#[cfg(test)]
#[path = "records_tests.rs"]
mod records_tests;
