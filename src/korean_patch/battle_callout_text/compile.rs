use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use super::model::CompiledBattleCalloutText;
use super::records::compile_callout_record;
use crate::game_data::{BattleCalloutCatalog, GaijiCatalog, GaijiReadinessCatalog};
use crate::korean_patch::shared_text::compile_gaiji_bank;
use crate::translation_drafts::TranslationDraftSegment;

const COM_ORIGIN: usize = 0x100;

pub(in crate::korean_patch) fn compile_battle_callouts(
    mad_com: &[u8],
    gaiji_com: &[u8],
    gaiji: &GaijiCatalog,
    readiness: &GaijiReadinessCatalog,
    source: &BattleCalloutCatalog,
    draft: &TranslationDraftSegment,
    unit_list_draft: &TranslationDraftSegment,
) -> Result<CompiledBattleCalloutText> {
    let lines = unit_list_draft
        .entries
        .iter()
        .chain(&draft.entries)
        .flat_map(|entry| entry.korean_text.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let bank = compile_gaiji_bank(gaiji_com, gaiji, readiness, &lines)?;
    let mut entries = source
        .entries
        .iter()
        .zip(&draft.entries)
        .map(|(source, draft)| compile_callout_record(source, draft, &bank))
        .collect::<Result<Vec<_>>>()?;

    let storage_capacity = source.text_end_offset - source.text_start_offset;
    let packed_storage_bytes = entries.iter().map(|entry| entry.bytes.len()).sum::<usize>();
    ensure!(
        packed_storage_bytes < storage_capacity,
        "battle callouts need {packed_storage_bytes} bytes but the verified pool holds {storage_capacity} and its bank wrapper"
    );
    let mut text_region_replacement = vec![0; storage_capacity];
    let mut cursor = source.text_start_offset;
    for entry in &mut entries {
        let relative = cursor - source.text_start_offset;
        let end = relative + entry.bytes.len();
        text_region_replacement[relative..end].copy_from_slice(&entry.bytes);
        entry.file_offset = cursor;
        entry.com_address = u16::try_from(cursor + COM_ORIGIN)
            .context("relocated battle callout address exceeds 16 bits")?;
        cursor += entry.bytes.len();
    }
    ensure!(
        cursor == source.text_start_offset + packed_storage_bytes,
        "battle callout packer cursor changed"
    );

    let compiled_by_id = entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let pointer_table_end = source.pointer_table_offset + source.pointer_count * 2;
    let mut pointer_table_replacement = mad_com
        .get(source.pointer_table_offset..pointer_table_end)
        .context("battle callout pointer table lies outside MAD.COM")?
        .to_vec();
    for pointer in &source.pointers {
        let target = compiled_by_id
            .get(pointer.target_entry_id.as_str())
            .with_context(|| format!("missing compiled {}", pointer.target_entry_id))?;
        let relative = pointer.storage_offset - source.pointer_table_offset;
        pointer_table_replacement[relative..relative + 2]
            .copy_from_slice(&target.com_address.to_le_bytes());
    }

    Ok(CompiledBattleCalloutText {
        bank,
        pointer_table_offset: source.pointer_table_offset,
        pointer_table_replacement,
        text_region_start: source.text_start_offset,
        text_region_end: source.text_end_offset,
        packed_storage_bytes,
        text_region_replacement,
        entries,
    })
}
