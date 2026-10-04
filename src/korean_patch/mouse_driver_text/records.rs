use anyhow::{Result, ensure};

use super::model::CompiledMouseDriverTextEntry;
use crate::external_text::{ExternalTextEntry, ExternalTextTerminator};
use crate::korean_patch::shared_text::{CompiledGaijiBank, encode_dos_dollar_text_layout};
use crate::translation_drafts::TranslationDraftEntry;

const STARTUP_BANNER_ROLE: &str = "startup_banner";
const HELP_MESSAGE_ROLE: &str = "help_message";
const STARTUP_PREFIX: &str = "∮∮  ";
const STARTUP_SUFFIX: &str = "  ∮∮";
const HELP_PREFIX: &str = "∞∞  ";
const HELP_SUFFIX: &str = "  ∞∞";
const HELP_STATUS_PREFIX: &str = "∞  ";

pub(super) fn compile_record(
    source: &ExternalTextEntry,
    draft: &TranslationDraftEntry,
    bank: &CompiledGaijiBank,
) -> Result<CompiledMouseDriverTextEntry> {
    ensure!(source.id == draft.id, "mouse-driver text identity changed");
    ensure!(
        source.terminator == ExternalTextTerminator::DosDollar,
        "{} lost its DOS terminator",
        source.id
    );
    ensure!(
        source.references.len() == 1,
        "{} changed its single mouse-driver consumer",
        source.id
    );
    let reference = &source.references[0];
    let decorated = match reference.role.as_str() {
        STARTUP_BANNER_ROLE => decorate_startup_banner(source, draft)?,
        HELP_MESSAGE_ROLE => decorate_help_message(source, draft)?,
        _ => draft.korean_text.clone(),
    };
    let lines = preserve_leading_spaces(&source.text, &decorated)?;
    let bytes = encode_dos_dollar_text_layout(&source.text, &lines, bank)?;

    Ok(CompiledMouseDriverTextEntry {
        id: draft.id.clone(),
        original_file_offset: source.text_offset,
        file_offset: 0,
        com_address: 0,
        lines,
        bytes,
        role: reference.role.clone(),
        consumer_offset: reference.consumer_offset,
        table_index: reference.table_index,
    })
}

fn decorate_startup_banner(
    source: &ExternalTextEntry,
    draft: &TranslationDraftEntry,
) -> Result<Vec<String>> {
    let source_first = source.text.split("\r\n").next().unwrap_or_default();
    ensure!(
        source_first.starts_with(STARTUP_PREFIX) && source_first.ends_with(STARTUP_SUFFIX),
        "mouse-driver startup banner frame changed"
    );
    ensure!(
        !draft.korean_text.is_empty(),
        "mouse-driver startup banner has no visible line"
    );
    let mut lines = draft.korean_text.clone();
    lines[0] = format!("{STARTUP_PREFIX}{}{STARTUP_SUFFIX}", lines[0]);
    Ok(lines)
}

fn decorate_help_message(
    source: &ExternalTextEntry,
    draft: &TranslationDraftEntry,
) -> Result<Vec<String>> {
    let source_visible = source
        .text
        .split("\r\n")
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    ensure!(
        source_visible
            .first()
            .is_some_and(|line| { line.starts_with(HELP_PREFIX) && line.ends_with(HELP_SUFFIX) })
            && source_visible
                .last()
                .is_some_and(|line| line.starts_with(HELP_STATUS_PREFIX)),
        "mouse-driver help frame changed"
    );
    ensure!(
        draft.korean_text.len() >= 2,
        "mouse-driver help needs a title and status fragment"
    );
    let mut lines = draft.korean_text.clone();
    lines[0] = format!("{HELP_PREFIX}{}{HELP_SUFFIX}", lines[0]);
    let last = lines.len() - 1;
    lines[last] = format!("{HELP_STATUS_PREFIX}{}", lines[last]);
    Ok(lines)
}

fn preserve_leading_spaces(source_text: &str, korean_lines: &[String]) -> Result<Vec<String>> {
    let source_visible = source_text
        .split("\r\n")
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    ensure!(
        source_visible.len() == korean_lines.len(),
        "mouse-driver text changed its visible line count"
    );
    ensure!(
        korean_lines
            .iter()
            .all(|line| !line.starts_with(' ') && !line.ends_with(' ')),
        "mouse-driver draft must leave leading and trailing layout spaces to the source template"
    );
    Ok(source_visible
        .into_iter()
        .zip(korean_lines)
        .map(|(source, korean)| {
            let indentation = source.bytes().take_while(|byte| *byte == b' ').count();
            format!("{}{korean}", " ".repeat(indentation))
        })
        .collect())
}

#[cfg(test)]
#[path = "records_tests.rs"]
mod records_tests;
