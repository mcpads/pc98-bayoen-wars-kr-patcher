use anyhow::{Context, Result, ensure};

use super::model::CompiledDialogueText;
use super::records::compile_dialogue_record;
use crate::game_data::{DialogueCatalog, GaijiCatalog, GaijiReadinessCatalog};
use crate::korean_patch::shared_text::compile_gaiji_bank_with_trailing_character_codes;
use crate::translation_drafts::TranslationDraftSegment;

const COM_ORIGIN: usize = 0x100;
const FIRST_TRAILING_GAIJI_CHARACTER_CODE: u16 = 0x777b;
const LAST_TRAILING_GAIJI_CHARACTER_CODE: u16 = 0x777e;
pub(crate) const DIALOGUE_GAIJI_PHYSICAL_CAPACITY: usize = 188;

pub(in crate::korean_patch) fn compile_dialogue_text(
    mad_com: &[u8],
    gaiji_com: &[u8],
    gaiji: &GaijiCatalog,
    readiness: &GaijiReadinessCatalog,
    source: &DialogueCatalog,
    drafts: &[&TranslationDraftSegment],
) -> Result<CompiledDialogueText> {
    compile_dialogue_text_with_trailing_character_codes(
        mad_com,
        gaiji_com,
        gaiji,
        readiness,
        source,
        drafts,
        &[],
    )
}

pub(in crate::korean_patch) fn compile_scene_dialogue_text(
    mad_com: &[u8],
    gaiji_com: &[u8],
    gaiji: &GaijiCatalog,
    readiness: &GaijiReadinessCatalog,
    source: &DialogueCatalog,
    drafts: &[&TranslationDraftSegment],
) -> Result<CompiledDialogueText> {
    let trailing_character_codes = verified_unreferenced_trailing_character_codes(mad_com, gaiji)?;
    compile_dialogue_text_with_trailing_character_codes(
        mad_com,
        gaiji_com,
        gaiji,
        readiness,
        source,
        drafts,
        &trailing_character_codes,
    )
}

#[allow(clippy::too_many_arguments)]
fn compile_dialogue_text_with_trailing_character_codes(
    mad_com: &[u8],
    gaiji_com: &[u8],
    gaiji: &GaijiCatalog,
    readiness: &GaijiReadinessCatalog,
    source: &DialogueCatalog,
    drafts: &[&TranslationDraftSegment],
    trailing_character_codes: &[u16],
) -> Result<CompiledDialogueText> {
    let lines = drafts
        .iter()
        .flat_map(|segment| &segment.entries)
        .flat_map(|entry| entry.korean_text.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let punctuation = super::punctuation::dialogue_punctuation_overrides()?;
    let bank = compile_gaiji_bank_with_trailing_character_codes(
        gaiji_com,
        gaiji,
        readiness,
        &lines,
        &punctuation,
        trailing_character_codes,
    )?;
    let mut entries = Vec::with_capacity(source.entry_count);
    for (group, draft) in source.groups.iter().zip(drafts) {
        for (entry_index, (source_entry, draft_entry)) in
            group.entries.iter().zip(&draft.entries).enumerate()
        {
            entries.push(compile_dialogue_record(
                &group.id,
                group.record_table_offset + entry_index * 4 + 2,
                source_entry,
                draft_entry,
                &bank,
            )?);
        }
    }
    ensure!(
        entries.len() == source.entry_count,
        "compiled dialogue entry population changed"
    );

    let storage_capacity = source
        .text_end_offset
        .checked_sub(source.text_start_offset)
        .context("dialogue text storage boundaries are reversed")?;
    let packed_storage_bytes = entries.iter().map(|entry| entry.bytes.len()).sum::<usize>();
    ensure!(
        packed_storage_bytes <= storage_capacity,
        "dialogue text needs {packed_storage_bytes} bytes but the verified region holds {storage_capacity}"
    );
    let mut text_region_replacement = vec![0; storage_capacity];
    let mut cursor = source.text_start_offset;
    for entry in &mut entries {
        let relative = cursor - source.text_start_offset;
        let end = relative + entry.bytes.len();
        text_region_replacement[relative..end].copy_from_slice(&entry.bytes);
        entry.file_offset = cursor;
        entry.com_address = u16::try_from(cursor + COM_ORIGIN)
            .context("relocated dialogue text address exceeds 16 bits")?;
        cursor += entry.bytes.len();
    }
    ensure!(
        cursor == source.text_start_offset + packed_storage_bytes,
        "dialogue text packer cursor changed"
    );

    let record_table_start = source
        .groups
        .first()
        .context("dialogue source has no groups")?
        .record_table_offset;
    let record_table_end = source.text_start_offset;
    let mut record_table_replacement = mad_com
        .get(record_table_start..record_table_end)
        .context("dialogue record tables lie outside MAD.COM")?
        .to_vec();
    for entry in &entries {
        let relative = entry.text_pointer_offset - record_table_start;
        record_table_replacement[relative..relative + 2]
            .copy_from_slice(&entry.com_address.to_le_bytes());
    }

    Ok(CompiledDialogueText {
        bank,
        record_table_start,
        record_table_end,
        record_table_replacement,
        text_region_start: source.text_start_offset,
        text_region_end: source.text_end_offset,
        packed_storage_bytes,
        text_region_replacement,
        entries,
    })
}

fn verified_unreferenced_trailing_character_codes(
    mad_com: &[u8],
    gaiji: &GaijiCatalog,
) -> Result<Vec<u16>> {
    ensure!(
        gaiji.last_character_code + 1 == FIRST_TRAILING_GAIJI_CHARACTER_CODE,
        "MAD dialogue trailing GAIJI range no longer follows the source table"
    );
    let codes = (FIRST_TRAILING_GAIJI_CHARACTER_CODE..=LAST_TRAILING_GAIJI_CHARACTER_CODE)
        .collect::<Vec<_>>();
    for &character_code in &codes {
        let shift_jis = crate::game_data::jis_row_cell_to_shift_jis(character_code)?;
        let encoded = shift_jis.to_be_bytes();
        ensure!(
            !mad_com
                .windows(encoded.len())
                .any(|window| window == encoded),
            "MAD.COM already references uninstalled GAIJI code {character_code:04X}"
        );
    }
    Ok(codes)
}
