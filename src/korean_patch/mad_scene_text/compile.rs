use anyhow::{Context, Result, ensure};

use super::super::battle_callout_text::{
    BattleCalloutHookLayout, CompiledBattleCalloutHook, CompiledBattleCalloutText,
    compile_battle_callout_hook, compile_battle_callouts,
};
use super::super::dialogue_text::{
    CompiledDialogueText, compile_scene_dialogue_text, dialogue_segments,
};
use super::super::fixed_gaiji_text::{CompiledFixedGaijiText, compile_fixed_gaiji_text};
use super::super::interface_text::compile_interface_text;
use super::super::mad_system_interface_text::{
    CompiledMadSystemInterfaceText, compile_mad_system_interface_text, require_segment,
};
use super::super::mad_system_text::preserve_mad_system_text_with_bank;
use super::bank_file::{CompiledGaijiBankFile, compile_gaiji_bank_file};
use super::model::MadSystemTextDisposition;
use super::transition::{CompiledSceneTransition, SceneTransitionLayout, compile_scene_transition};
use crate::game_data::{GaijiCatalog, GameDataCatalog};
use crate::translation_analysis::TranslationCorpus;
use crate::translation_drafts::TranslationSurface;

const SYSTEM_SEGMENT_ID: &str = "mad-system";
const INTERFACE_SEGMENT_ID: &str = "mad-interface";
const BATTLE_SEGMENT_ID: &str = "mad-battle-callouts";

pub(super) struct CompiledMadSceneText {
    pub system_text_disposition: MadSystemTextDisposition,
    pub shared: CompiledMadSystemInterfaceText,
    pub dialogue: CompiledDialogueText,
    pub fixed: CompiledFixedGaijiText,
    pub battle: CompiledBattleCalloutText,
    pub unit_list_status: super::super::unit_list_status::CompiledUnitListStatus,
    pub interface_extension_records: Vec<u8>,
    pub dialogue_bank_file: CompiledGaijiBankFile,
    pub fixed_bank_file: CompiledGaijiBankFile,
    pub battle_bank_file: CompiledGaijiBankFile,
    pub transition: CompiledSceneTransition,
    pub battle_hook: CompiledBattleCalloutHook,
}

pub(super) fn compile_mad_scene_text(
    mad_com: &[u8],
    gaiji_com: &[u8],
    source: &GameDataCatalog,
    corpus: &TranslationCorpus,
    system_text_disposition: MadSystemTextDisposition,
) -> Result<CompiledMadSceneText> {
    let interface_segment = require_segment(
        corpus,
        INTERFACE_SEGMENT_ID,
        TranslationSurface::InterfaceText,
        source.mad_com.interface_text.entries.len(),
    )?;
    let unit_list_segment = require_segment(
        corpus,
        "mad-unit-list-status",
        TranslationSurface::InterfaceText,
        source.mad_com.unit_list_status.entries.len(),
    )?;
    let battle_segment = require_segment(
        corpus,
        BATTLE_SEGMENT_ID,
        TranslationSurface::BattleCallout,
        source.mad_com.battle_callouts.entries.len(),
    )?;
    let mut shared = match system_text_disposition {
        MadSystemTextDisposition::TranslatedDevelopment => {
            let system_segment = require_segment(
                corpus,
                SYSTEM_SEGMENT_ID,
                TranslationSurface::DosSystemText,
                source.mad_com.system_text.entries.len(),
            )?;
            compile_mad_system_interface_text(
                gaiji_com,
                &source.mad_com,
                &source.gaiji,
                system_segment,
                interface_segment,
            )?
        }
        MadSystemTextDisposition::PreservedSource => {
            let interface = compile_interface_text(
                gaiji_com,
                &source.gaiji,
                &source.mad_com,
                interface_segment,
            )?;
            let system = preserve_mad_system_text_with_bank(
                mad_com,
                &source.mad_com.system_text,
                interface.bank.clone(),
            )?;
            CompiledMadSystemInterfaceText {
                bank: interface.bank.clone(),
                system,
                interface,
            }
        }
    };
    let dialogue_drafts = dialogue_segments(&source.mad_com.dialogue, &corpus.segments)?;
    let dialogue = compile_scene_dialogue_text(
        mad_com,
        gaiji_com,
        &source.gaiji,
        &source.mad_com.gaiji_readiness,
        &source.mad_com.dialogue,
        &dialogue_drafts,
    )?;
    let fixed = compile_fixed_gaiji_text(gaiji_com, &source.mad_com, &source.gaiji, corpus)?;
    let battle = compile_battle_callouts(
        mad_com,
        gaiji_com,
        &source.gaiji,
        &source.mad_com.gaiji_readiness,
        &source.mad_com.battle_callouts,
        battle_segment,
        unit_list_segment,
    )?;
    let dialogue_record_count = source.gaiji.glyphs.len()
        + dialogue.bank.appended_records.len() / super::transition::GAIJI_RECORD_BYTE_SIZE;
    let interface_extension_record_count = dialogue_record_count - source.gaiji.glyphs.len();
    let blank_record = crate::korean_patch::gaiji_record::GaijiRecord::from_bitmap(
        [0; crate::korean_patch::font_rasterizer::GLYPH_BYTES],
    )
    .to_bytes();
    let interface_extension_records = blank_record.repeat(interface_extension_record_count);
    let dialogue_bank_file = compile_gaiji_bank_file(
        gaiji_com,
        &source.gaiji,
        &dialogue.bank,
        gaiji_com.len() + interface_extension_records.len(),
        dialogue_record_count,
    )?;
    let fixed_bank_file = compile_gaiji_bank_file(
        gaiji_com,
        &source.gaiji,
        &fixed.bank,
        dialogue_bank_file.output_file_size,
        dialogue_record_count,
    )?;
    let battle_bank_file = compile_gaiji_bank_file(
        gaiji_com,
        &source.gaiji,
        &battle.bank,
        fixed_bank_file.output_file_size,
        dialogue_record_count,
    )?;
    let interface_bank_file_offset = first_gaiji_record_offset(&source.gaiji)?;
    let wrapper_offset = dialogue.text_region_start + dialogue.packed_storage_bytes;
    let (buffer_offset, buffer_capacity) = match system_text_disposition {
        MadSystemTextDisposition::TranslatedDevelopment => {
            let offset = shared.system.text_region_start + shared.system.packed_storage_bytes;
            (offset, shared.system.text_region_end - offset)
        }
        MadSystemTextDisposition::PreservedSource => {
            let offset = shared.interface.text_region_start + shared.interface.packed_storage_bytes;
            (offset, shared.interface.text_region_end - offset)
        }
    };
    let status_characters = unit_list_segment
        .entries
        .iter()
        .flat_map(|entry| entry.korean_text.iter())
        .flat_map(|line| line.chars())
        .collect::<std::collections::BTreeSet<_>>();
    let status_record_count = battle
        .bank
        .glyphs
        .iter()
        .filter(|glyph| {
            glyph
                .character
                .chars()
                .any(|c| status_characters.contains(&c))
        })
        .map(|glyph| glyph.slot_index + 1)
        .max()
        .context("unit status has no bank glyphs")?;
    ensure!(
        status_record_count < 139,
        "unit status prefix crosses reserved graphic slots"
    );
    let transition = compile_scene_transition(
        &source.mad_com.scene_transition,
        &source.mad_com.fixed_gaiji_text.runtime,
        SceneTransitionLayout {
            wrapper_offset,
            wrapper_capacity: dialogue.text_region_end - wrapper_offset,
            buffer_offset,
            buffer_capacity,
            interface_bank_file_offset,
            dialogue_bank_file_offset: dialogue_bank_file.file_offset,
            fixed_bank_file_offset: fixed_bank_file.file_offset,
            gaiji_record_count: dialogue_bank_file.record_count,
            status_record_count,
        },
    )?;
    let battle_wrapper_offset = battle.text_region_start + battle.packed_storage_bytes;
    let battle_hook = compile_battle_callout_hook(
        &source.mad_com.battle_callouts,
        BattleCalloutHookLayout {
            wrapper_offset: battle_wrapper_offset,
            wrapper_capacity: battle.text_region_end - battle_wrapper_offset,
            switch_bank_com_address: transition.switch_bank_com_address,
            interface_bank_file_offset,
            battle_bank_file_offset: battle_bank_file.file_offset,
        },
    )?;
    let unit_list_status = super::super::unit_list_status::compile_unit_list_status(
        &source.mad_com.unit_list_status,
        unit_list_segment,
        &battle.bank,
        transition.switch_status_bank_com_address,
        battle_bank_file.file_offset,
        interface_bank_file_offset,
    )?;
    match system_text_disposition {
        MadSystemTextDisposition::TranslatedDevelopment => {
            install_persistent_system_bank_storage(&mut shared.system, &transition)?
        }
        MadSystemTextDisposition::PreservedSource => {
            install_persistent_interface_bank_storage(&mut shared.interface, &transition)?
        }
    }
    Ok(CompiledMadSceneText {
        system_text_disposition,
        shared,
        dialogue,
        fixed,
        battle,
        unit_list_status,
        interface_extension_records,
        dialogue_bank_file,
        fixed_bank_file,
        battle_bank_file,
        transition,
        battle_hook,
    })
}

fn install_persistent_system_bank_storage(
    system: &mut super::super::mad_system_text::CompiledMadSystemText,
    transition: &CompiledSceneTransition,
) -> Result<()> {
    let buffer_end = transition
        .buffer_offset
        .checked_add(super::transition::GAIJI_RECORD_BYTE_SIZE)
        .context("MAD scene record buffer end exceeds system-text storage")?;
    ensure!(
        transition.file_name_offset >= buffer_end,
        "MAD scene bank file name overlaps its reusable record buffer"
    );
    let relative = transition
        .file_name_offset
        .checked_sub(system.text_region_start)
        .context("MAD scene record buffer begins before system-text storage")?;
    let end = relative
        .checked_add(transition.file_name.len())
        .context("MAD scene file name end exceeds system-text storage")?;
    ensure!(
        relative >= system.packed_storage_bytes && end <= system.text_region_replacement.len(),
        "MAD scene file name lies outside the verified system-text tail"
    );
    system.text_region_replacement[relative..end].copy_from_slice(&transition.file_name);
    system.populated_storage_bytes = end;
    Ok(())
}

fn install_persistent_interface_bank_storage(
    interface: &mut super::super::interface_text::CompiledInterfaceText,
    transition: &CompiledSceneTransition,
) -> Result<()> {
    let buffer_end = transition
        .buffer_offset
        .checked_add(super::transition::GAIJI_RECORD_BYTE_SIZE)
        .context("MAD scene record buffer end exceeds interface-text storage")?;
    ensure!(
        transition.file_name_offset >= buffer_end,
        "MAD scene bank file name overlaps its reusable record buffer"
    );
    let relative = transition
        .file_name_offset
        .checked_sub(interface.text_region_start)
        .context("MAD scene record buffer begins before interface-text storage")?;
    let end = relative
        .checked_add(transition.file_name.len())
        .context("MAD scene file name end exceeds interface-text storage")?;
    ensure!(
        relative >= interface.packed_storage_bytes
            && end <= interface.text_region_replacement.len(),
        "MAD scene file name lies outside the verified interface-text tail"
    );
    interface.text_region_replacement[relative..end].copy_from_slice(&transition.file_name);
    interface.populated_storage_bytes = end;
    Ok(())
}

fn first_gaiji_record_offset(source: &GaijiCatalog) -> Result<usize> {
    Ok(source
        .glyphs
        .first()
        .context("GAIJI source record table is empty")?
        .file_offset)
}
