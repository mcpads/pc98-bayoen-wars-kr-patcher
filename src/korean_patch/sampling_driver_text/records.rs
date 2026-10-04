use anyhow::{Result, ensure};

use super::model::CompiledSamplingDriverTextEntry;
use crate::external_text::{ExternalTextEntry, ExternalTextTerminator};
use crate::korean_patch::shared_text::{CompiledGaijiBank, encode_dos_dollar_text};
use crate::translation_drafts::TranslationDraftEntry;

const STARTUP_BANNER_ROLE: &str = "startup_banner";
const BANNER_PREFIX: &str = "～～ ";
const BANNER_SUFFIX: &str = " ～～";

pub(super) fn compile_record(
    source: &ExternalTextEntry,
    draft: &TranslationDraftEntry,
    bank: &CompiledGaijiBank,
) -> Result<CompiledSamplingDriverTextEntry> {
    ensure!(
        source.id == draft.id,
        "sampling-driver text identity changed"
    );
    ensure!(
        source.terminator == ExternalTextTerminator::DosDollar,
        "{} lost its DOS terminator",
        source.id
    );
    ensure!(
        source.references.len() == 1,
        "{} changed its single sampling-driver consumer",
        source.id
    );

    let lines = if source.references[0].role == STARTUP_BANNER_ROLE {
        preserve_banner_frame(source, draft)?
    } else {
        draft.korean_text.clone()
    };
    let bytes = encode_dos_dollar_text(&source.text, &lines, bank)?;

    Ok(CompiledSamplingDriverTextEntry {
        id: draft.id.clone(),
        original_file_offset: source.text_offset,
        file_offset: 0,
        com_address: 0,
        lines,
        bytes,
        consumer_offsets: vec![source.references[0].consumer_offset],
    })
}

fn preserve_banner_frame(
    source: &ExternalTextEntry,
    draft: &TranslationDraftEntry,
) -> Result<Vec<String>> {
    ensure!(
        source.text.starts_with(BANNER_PREFIX)
            && source.text.ends_with(&format!("{BANNER_SUFFIX}\r\n\r\n")),
        "sampling-driver startup banner frame changed"
    );
    ensure!(
        draft.korean_text.len() == 1,
        "sampling-driver startup banner needs one visible Korean line"
    );
    Ok(vec![format!(
        "{BANNER_PREFIX}{}{BANNER_SUFFIX}",
        draft.korean_text[0]
    )])
}

#[cfg(test)]
#[path = "records_tests.rs"]
mod records_tests;
