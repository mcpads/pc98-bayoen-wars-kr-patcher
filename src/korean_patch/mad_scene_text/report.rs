use std::collections::BTreeMap;

use super::super::battle_callout_text::{BattleCalloutEntryPatchReport, BattleCalloutPatchReport};
use super::super::dialogue_text::dialogue_entry_report;
use super::super::fixed_gaiji_text::fixed_gaiji_text_report;
use super::super::font_rasterizer::FontProvenance;
use super::super::mad_system_interface_text::combined_report;
use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::PayloadWriteReport;
use super::compile::CompiledMadSceneText;
use super::model::{
    MadSceneTextPatchReport, MadSceneTransitionPatchReport, PatchedDialogueComponentReport,
};
use crate::game_data::GameDataCatalog;
use crate::source_disk::sha256_hex;

pub(super) fn mad_scene_report(
    supported_source_sha256: String,
    font: FontProvenance,
    source: &GameDataCatalog,
    source_gaiji_file_size: usize,
    compiled: &CompiledMadSceneText,
    writes: Vec<PayloadWriteReport>,
) -> MadSceneTextPatchReport {
    let shared = combined_report(
        supported_source_sha256.clone(),
        font.clone(),
        &source.mad_com,
        &source.mad_com.system_runtime,
        &compiled.shared.system,
        &compiled.shared.interface,
        Vec::new(),
    );
    let dialogue = &compiled.dialogue;
    let dialogue_storage_capacity = dialogue.text_region_end - dialogue.text_region_start;
    let wrapper_used = compiled.transition.wrapper.bytes().len();
    let fixed_writes = writes
        .iter()
        .filter(|write| {
            matches!(
                write.id.as_str(),
                "mad-scene-fixed-gaiji-bank" | "fixed-gaiji-text-slots"
            )
        })
        .cloned()
        .collect();
    let fixed = fixed_gaiji_text_report(
        supported_source_sha256.clone(),
        font.clone(),
        &compiled.fixed,
        fixed_writes,
    );
    MadSceneTextPatchReport {
        supported_source_sha256,
        translation_status: "needs_human_review".to_owned(),
        build_status: DevelopmentBuildStatus::DevelopmentOnly,
        font_profile: font.profile_id,
        font_sha256: font.font_sha256,
        shared_available_gaiji_slots: compiled.shared.bank.available_slot_count,
        shared_used_gaiji_slots: compiled.shared.bank.glyphs.len(),
        system_text_disposition: compiled.system_text_disposition,
        system: shared.system,
        interface: shared.interface,
        dialogue: PatchedDialogueComponentReport {
            available_gaiji_slots: dialogue.bank.available_slot_count,
            used_gaiji_slots: dialogue.bank.glyphs.len(),
            storage_capacity: dialogue_storage_capacity,
            packed_storage_bytes: dialogue.packed_storage_bytes,
            storage_headroom: dialogue_storage_capacity - dialogue.packed_storage_bytes,
            group_count: source.mad_com.dialogue.group_count,
            entry_count: source.mad_com.dialogue.entry_count,
            reference_count: source.mad_com.dialogue.reference_catalog.reference_count,
            machine_code_reference_count: source
                .mad_com
                .dialogue
                .reference_catalog
                .machine_code_reference_count,
            group_pointer_reference_count: source
                .mad_com
                .dialogue
                .reference_catalog
                .group_pointer_reference_count,
            text_pointer_reference_count: source
                .mad_com
                .dialogue
                .reference_catalog
                .text_pointer_reference_count,
            glyphs: dialogue.bank.glyphs.clone(),
            entries: dialogue.entries.iter().map(dialogue_entry_report).collect(),
        },
        battle_callouts: battle_callout_report(source, compiled),
        unit_list_status: super::super::unit_list_status::unit_list_status_report(
            &compiled.unit_list_status,
            compiled.battle_bank_file.file_offset,
        ),
        fixed,
        transition: MadSceneTransitionPatchReport {
            interface_bank_file_offset: source.gaiji.glyphs[0].file_offset,
            interface_extension_record_count: compiled.interface_extension_records.len()
                / super::transition::GAIJI_RECORD_BYTE_SIZE,
            interface_bank_record_count: compiled.dialogue_bank_file.record_count,
            unit_status_bank_record_count: compiled.transition.status_record_count,
            dialogue_bank_file_offset: compiled.dialogue_bank_file.file_offset,
            dialogue_bank_byte_size: compiled.dialogue_bank_file.records.len(),
            dialogue_bank_record_count: compiled.dialogue_bank_file.record_count,
            fixed_bank_file_offset: compiled.fixed_bank_file.file_offset,
            fixed_bank_byte_size: compiled.fixed_bank_file.records.len(),
            fixed_bank_record_count: compiled.fixed_bank_file.record_count,
            gaiji_record_count: compiled.dialogue_bank_file.record_count,
            gaiji_file_original_size: source_gaiji_file_size,
            gaiji_file_output_size: compiled.battle_bank_file.output_file_size,
            wrapper_file_offset: compiled.transition.wrapper_offset,
            wrapper_byte_size: compiled.transition.wrapper.bytes().len(),
            wrapper_storage_capacity: compiled.transition.wrapper_capacity,
            wrapper_storage_headroom: compiled.transition.wrapper_capacity - wrapper_used,
            file_name_file_offset: compiled.transition.file_name_offset,
            buffer_file_offset: compiled.transition.buffer_offset,
            buffer_byte_size: super::transition::GAIJI_RECORD_BYTE_SIZE,
            startup_entry_call_site_count: 1,
            call_site_count: source.mad_com.scene_transition.call_site_count,
            selection_entry_call_site_count: 1,
        },
        writes,
    }
}

fn battle_callout_report(
    source: &GameDataCatalog,
    compiled: &CompiledMadSceneText,
) -> BattleCalloutPatchReport {
    let battle = &compiled.battle;
    let storage_capacity = battle.text_region_end - battle.text_region_start;
    let pointer_counts = source.mad_com.battle_callouts.pointers.iter().fold(
        BTreeMap::<&str, usize>::new(),
        |mut counts, pointer| {
            *counts.entry(pointer.target_entry_id.as_str()).or_default() += 1;
            counts
        },
    );
    BattleCalloutPatchReport {
        available_gaiji_slots: battle.bank.available_slot_count,
        used_gaiji_slots: battle.bank.glyphs.len(),
        storage_capacity,
        packed_storage_bytes: battle.packed_storage_bytes,
        storage_headroom: storage_capacity - battle.packed_storage_bytes,
        entry_count: battle.entries.len(),
        pointer_count: source.mad_com.battle_callouts.pointer_count,
        call_site_count: source.mad_com.battle_callouts.call_site_count,
        bank_file_offset: compiled.battle_bank_file.file_offset,
        bank_byte_size: compiled.battle_bank_file.records.len(),
        wrapper_file_offset: compiled.battle_hook.wrapper_offset,
        wrapper_byte_size: compiled.battle_hook.wrapper.bytes().len(),
        wrapper_storage_capacity: compiled.battle_hook.wrapper_capacity,
        glyphs: battle.bank.glyphs.clone(),
        entries: battle
            .entries
            .iter()
            .map(|entry| BattleCalloutEntryPatchReport {
                id: entry.id.clone(),
                original_file_offset: entry.original_file_offset,
                file_offset: entry.file_offset,
                com_address: entry.com_address,
                byte_size: entry.bytes.len(),
                line_count: entry.lines.len(),
                pointer_count: pointer_counts.get(entry.id.as_str()).copied().unwrap_or(0),
                content_sha256: sha256_hex(&entry.bytes),
                lines: entry.lines.clone(),
            })
            .collect(),
    }
}
