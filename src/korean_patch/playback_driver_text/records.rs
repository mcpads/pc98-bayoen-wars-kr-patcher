use anyhow::{Context, Result, ensure};

use super::model::CompiledPlaybackDriverTextEntry;
use crate::external_text::{ExternalProgramTextCatalog, ExternalTextTerminator};
use crate::korean_patch::shared_text::{CompiledGaijiBank, encode_ansi_dos_dollar_text};
use crate::translation_drafts::{DevelopmentPolicy, TranslationDraftSegment};

pub(super) fn compile_records(
    unpacked: &[u8],
    source: &ExternalProgramTextCatalog,
    draft: &TranslationDraftSegment,
    bank: &CompiledGaijiBank,
) -> Result<Vec<CompiledPlaybackDriverTextEntry>> {
    ensure!(
        source.entries.len() == draft.entries.len(),
        "{} source and draft populations differ",
        source.file_name
    );
    source
        .entries
        .iter()
        .zip(&draft.entries)
        .enumerate()
        .map(|(source_order, (source_entry, draft_entry))| {
            ensure!(
                source_entry.id == draft_entry.id,
                "{} source and draft order or identity changed",
                source.file_name
            );
            ensure!(
                source_entry.terminator == ExternalTextTerminator::DosDollar,
                "{} lost its DOS dollar terminator",
                source_entry.id
            );
            match draft_entry.development_policy {
                DevelopmentPolicy::Translate => ensure!(
                    !draft_entry.korean_text.is_empty(),
                    "{} translated draft is empty",
                    source_entry.id
                ),
                DevelopmentPolicy::PreserveSourceControl => ensure!(
                    draft_entry.korean_text.is_empty(),
                    "{} control-only draft gained visible text",
                    source_entry.id
                ),
                DevelopmentPolicy::RetainSourceAudio => {
                    anyhow::bail!("{} uses an audio-only development policy", source_entry.id)
                }
            }
            let source_bytes = unpacked
                .get(source_entry.text_offset..source_entry.text_offset + source_entry.byte_size)
                .with_context(|| {
                    format!(
                        "{} protected source body lies outside the image",
                        source_entry.id
                    )
                })?;
            let bytes = encode_ansi_dos_dollar_text(
                &source_entry.text,
                source_bytes,
                &draft_entry.korean_text,
                bank,
            )?;
            Ok(CompiledPlaybackDriverTextEntry {
                id: source_entry.id.clone(),
                source_order,
                original_file_offset: source_entry.text_offset,
                file_offset: 0,
                com_address: 0,
                lines: draft_entry.korean_text.clone(),
                bytes,
            })
        })
        .collect()
}
