use anyhow::{Result, ensure};
use encoding_rs::SHIFT_JIS;

use super::model::CompiledMadSystemTextEntry;
use crate::byte_string::encode_lower_hex;
use crate::game_data::MadSystemTextEntry;
use crate::korean_patch::shared_text::{CompiledGaijiBank, encode_ansi_dos_dollar_text};
use crate::translation_drafts::TranslationDraftEntry;

pub(super) fn compile_record(
    source: &MadSystemTextEntry,
    draft: &TranslationDraftEntry,
    bank: &CompiledGaijiBank,
) -> Result<CompiledMadSystemTextEntry> {
    ensure!(source.id == draft.id, "MAD system text identity changed");
    let (source_bytes, _, encoding_errors) = SHIFT_JIS.encode(&source.text);
    ensure!(
        !encoding_errors && encode_lower_hex(&source_bytes) == source.raw_hex,
        "{} protected text no longer reproduces its source bytes",
        source.id
    );
    let bytes = encode_ansi_dos_dollar_text(&source.text, &source_bytes, &draft.korean_text, bank)?;
    let (runtime_insert_byte_offset, runtime_insert_byte_capacity) = source
        .runtime_insert
        .as_ref()
        .map(|insert| {
            let end = insert.byte_offset + insert.byte_capacity;
            ensure!(
                bytes
                    .get(insert.byte_offset..end)
                    .is_some_and(|field| field.iter().all(|byte| *byte == b' '))
                    && bytes.get(end..end + 4) == Some(b".DAT"),
                "{} changed its runtime file-name field or suffix",
                source.id
            );
            Ok((Some(insert.byte_offset), Some(insert.byte_capacity)))
        })
        .transpose()?
        .unwrap_or((None, None));

    Ok(CompiledMadSystemTextEntry {
        id: draft.id.clone(),
        original_file_offset: source.file_offset,
        file_offset: 0,
        com_address: 0,
        lines: draft.korean_text.clone(),
        runtime_insert_byte_offset,
        runtime_insert_byte_capacity,
        bytes,
    })
}

#[cfg(test)]
#[path = "records_tests.rs"]
mod records_tests;
