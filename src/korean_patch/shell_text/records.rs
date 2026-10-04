use anyhow::{Result, ensure};

use super::model::CompiledShellTextEntry;
use crate::external_text::{ExternalTextEntry, ExternalTextTerminator};
use crate::korean_patch::shared_text::{CompiledGaijiBank, encode_dos_dollar_text};
use crate::translation_drafts::TranslationDraftEntry;

pub(super) fn compile_record(
    source: &ExternalTextEntry,
    draft: &TranslationDraftEntry,
    bank: &CompiledGaijiBank,
) -> Result<CompiledShellTextEntry> {
    ensure!(source.id == draft.id, "DSH text identity changed");
    ensure!(
        source.terminator == ExternalTextTerminator::DosDollar,
        "{} lost its DOS terminator",
        source.id
    );
    ensure!(
        !draft.korean_text.is_empty(),
        "{} has no Korean development text",
        draft.id
    );
    let bytes = encode_dos_dollar_text(&source.text, &draft.korean_text, bank)?;

    Ok(CompiledShellTextEntry {
        id: draft.id.clone(),
        original_file_offset: source.text_offset,
        file_offset: 0,
        com_address: 0,
        lines: draft.korean_text.clone(),
        bytes,
        consumer_offsets: source
            .references
            .iter()
            .map(|reference| reference.consumer_offset)
            .collect(),
    })
}

#[cfg(test)]
#[path = "records_tests.rs"]
mod records_tests;
