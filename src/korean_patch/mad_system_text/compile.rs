use anyhow::{Context, Result, ensure};

use super::model::CompiledMadSystemText;
use super::records::compile_record;
use crate::game_data::{GaijiCatalog, GaijiReadinessCatalog, MadSystemTextCatalog};
use crate::korean_patch::shared_text::{CompiledGaijiBank, compile_gaiji_bank};
use crate::translation_drafts::TranslationDraftSegment;

const COM_ORIGIN: usize = 0x100;

pub(super) fn compile_mad_system_text(
    gaiji_com: &[u8],
    gaiji: &GaijiCatalog,
    readiness: &GaijiReadinessCatalog,
    source: &MadSystemTextCatalog,
    draft: &TranslationDraftSegment,
) -> Result<CompiledMadSystemText> {
    let lines = draft
        .entries
        .iter()
        .flat_map(|entry| entry.korean_text.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let bank = compile_gaiji_bank(gaiji_com, gaiji, readiness, &lines)?;
    compile_mad_system_text_with_bank(source, draft, bank)
}

pub(in crate::korean_patch) fn compile_mad_system_text_with_bank(
    source: &MadSystemTextCatalog,
    draft: &TranslationDraftSegment,
    bank: CompiledGaijiBank,
) -> Result<CompiledMadSystemText> {
    let mut entries = source
        .entries
        .iter()
        .zip(&draft.entries)
        .map(|(source, draft)| compile_record(source, draft, &bank))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        source
            .entries
            .windows(2)
            .all(|pair| { pair[0].file_offset + pair[0].byte_size + 1 == pair[1].file_offset }),
        "MAD system source text storage is not contiguous"
    );
    let text_region_start = source
        .entries
        .first()
        .context("MAD system source text is empty")?
        .file_offset;
    let last = source
        .entries
        .last()
        .context("MAD system source text is empty")?;
    let text_region_end = last.file_offset + last.byte_size + 1;
    let storage_capacity = text_region_end - text_region_start;
    let packed_storage_bytes = entries.iter().map(|entry| entry.bytes.len()).sum::<usize>();
    ensure!(
        packed_storage_bytes <= storage_capacity,
        "MAD system text needs {packed_storage_bytes} bytes but the verified region holds {storage_capacity}"
    );

    let mut text_region_replacement = vec![0; storage_capacity];
    let mut cursor = text_region_start;
    for entry in &mut entries {
        let relative = cursor - text_region_start;
        let end = relative + entry.bytes.len();
        text_region_replacement[relative..end].copy_from_slice(&entry.bytes);
        entry.file_offset = cursor;
        entry.com_address = u16::try_from(cursor + COM_ORIGIN)
            .context("relocated MAD system text address exceeds 16 bits")?;
        cursor += entry.bytes.len();
    }
    ensure!(
        cursor == text_region_start + packed_storage_bytes,
        "MAD system text packer cursor changed"
    );

    Ok(CompiledMadSystemText {
        bank,
        text_region_start,
        text_region_end,
        packed_storage_bytes,
        populated_storage_bytes: packed_storage_bytes,
        text_region_replacement,
        entries,
    })
}

pub(in crate::korean_patch) fn preserve_mad_system_text_with_bank(
    mad_com: &[u8],
    source: &MadSystemTextCatalog,
    bank: CompiledGaijiBank,
) -> Result<CompiledMadSystemText> {
    let text_region_start = source
        .entries
        .first()
        .context("MAD system source text is empty")?
        .file_offset;
    let last = source
        .entries
        .last()
        .context("MAD system source text is empty")?;
    let text_region_end = last.file_offset + last.byte_size + 1;
    let text_region_replacement = mad_com
        .get(text_region_start..text_region_end)
        .context("MAD system source text storage lies outside MAD.COM")?
        .to_vec();
    let entries = source
        .entries
        .iter()
        .map(|entry| {
            let bytes = mad_com
                .get(entry.file_offset..entry.file_offset + entry.byte_size + 1)
                .with_context(|| format!("{} lies outside MAD.COM", entry.id))?
                .to_vec();
            ensure!(
                bytes.last() == Some(&b'$') && !bytes[..bytes.len() - 1].contains(&b'$'),
                "{} changed its source DOS terminator boundary",
                entry.id
            );
            let (runtime_insert_byte_offset, runtime_insert_byte_capacity) = entry
                .runtime_insert
                .as_ref()
                .map(|insert| (Some(insert.byte_offset), Some(insert.byte_capacity)))
                .unwrap_or((None, None));
            Ok(super::model::CompiledMadSystemTextEntry {
                id: entry.id.clone(),
                original_file_offset: entry.file_offset,
                file_offset: entry.file_offset,
                com_address: u16::try_from(entry.com_address)
                    .context("source MAD system address exceeds 16 bits")?,
                lines: vec![entry.text.clone()],
                runtime_insert_byte_offset,
                runtime_insert_byte_capacity,
                bytes,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let storage_capacity = text_region_end - text_region_start;
    ensure!(
        entries.iter().map(|entry| entry.bytes.len()).sum::<usize>() == storage_capacity,
        "MAD system source records no longer fill their verified storage"
    );
    Ok(CompiledMadSystemText {
        bank,
        text_region_start,
        text_region_end,
        packed_storage_bytes: storage_capacity,
        populated_storage_bytes: storage_capacity,
        text_region_replacement,
        entries,
    })
}
