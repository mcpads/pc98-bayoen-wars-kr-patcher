use anyhow::{Result, ensure};

use super::model::CompiledSystemLoaderTextEntry;
use crate::external_text::{ExternalTextEntry, ExternalTextTerminator};
use crate::korean_patch::shared_text::{CompiledGaijiBank, encode_dos_character};
use crate::translation_drafts::TranslationDraftEntry;

pub(super) fn compile_record(
    source: &ExternalTextEntry,
    draft: &TranslationDraftEntry,
    bank: &CompiledGaijiBank,
) -> Result<CompiledSystemLoaderTextEntry> {
    ensure!(source.id == draft.id, "MEGDOS.SYS text identity changed");
    ensure!(
        source.terminator == ExternalTextTerminator::Null,
        "{} lost its null terminator",
        source.id
    );
    ensure!(
        draft.korean_text.len() == 1
            && !draft.korean_text[0].is_empty()
            && !draft.korean_text[0].contains(['\0', '\r', '\n']),
        "{} requires one visible null-terminated line",
        draft.id
    );
    let line = &draft.korean_text[0];
    ensure!(
        !line.starts_with(' ') && !line.ends_with(' '),
        "{} leaves outer layout spaces to its protected source structure",
        draft.id
    );
    let trailing_spaces = source
        .text
        .bytes()
        .rev()
        .take_while(|byte| *byte == b' ')
        .count();
    let source_visible = source.text.trim_end_matches(' ');
    ensure!(
        source_visible.ends_with(':') == line.ends_with(':'),
        "{} changed its runtime-fragment colon boundary",
        draft.id
    );

    let mut bytes = Vec::new();
    for character in line.chars() {
        bytes.extend(encode_dos_character(character, bank)?);
    }
    bytes.resize(bytes.len() + trailing_spaces, b' ');
    bytes.push(0);

    Ok(CompiledSystemLoaderTextEntry {
        id: draft.id.clone(),
        original_file_offset: source.text_offset,
        resident_offset: 0,
        lines: draft.korean_text.clone(),
        bytes,
    })
}

#[cfg(test)]
#[path = "records_tests.rs"]
mod records_tests;
