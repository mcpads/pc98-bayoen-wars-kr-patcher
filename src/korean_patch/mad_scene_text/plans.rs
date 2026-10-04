use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, RegionKind, ResizePlan, WriteIntent,
    WritePlan,
};
use v30::AssembledProgram;

use super::super::battle_callout_text::{
    BATTLE_WRAPPER_SOURCE_ID, call_source_id as battle_call_source_id,
};
use super::super::fixed_gaiji_text::fixed_slots_plan;
use super::super::interface_text::interface_text_plans;
use super::super::mad_system_interface_text::{
    PlannedMadSystemInterfaceTextWrites, combine_mad_system_interface_text_plans,
};
use super::super::mad_system_text::mad_system_text_plans;
use super::super::payload_writes::{PayloadFileWritePlan, combine_payload_write_plans};
use super::super::shared_text::gaiji_bank_plan;
use super::compile::CompiledMadSceneText;
use super::model::MadSystemTextDisposition;
use super::transition::{
    SELECTION_ENTRY_SOURCE_ID, STARTUP_ENTRY_SOURCE_ID, WRAPPER_SOURCE_ID, call_source_id,
};
use crate::game_data::GameDataCatalog;

const MAD_FILE: &str = "MAD.COM";
const GAIJI_FILE: &str = "GAIJI.COM";

pub(super) struct PlannedMadSceneTextWrites {
    pub plans: Vec<PayloadFileWritePlan>,
    pub typed_sources: BTreeMap<String, AssembledProgram>,
}

pub(super) fn mad_scene_text_plans(
    mad_com: &[u8],
    gaiji_com: &[u8],
    source: &GameDataCatalog,
    compiled: &CompiledMadSceneText,
) -> Result<PlannedMadSceneTextWrites> {
    let interface = interface_text_plans(
        mad_com,
        &source.mad_com.interface_text.reference_catalog,
        &compiled.shared.interface,
    )?;
    let shared = match compiled.system_text_disposition {
        MadSystemTextDisposition::TranslatedDevelopment => {
            let system = mad_system_text_plans(
                mad_com,
                &source.mad_com.system_runtime,
                &compiled.shared.system,
            )?;
            combine_mad_system_interface_text_plans(&compiled.shared.bank, system, interface)?
        }
        MadSystemTextDisposition::PreservedSource => {
            ensure!(
                interface.plans.len() == 1 && interface.plans[0].file_name == MAD_FILE,
                "MAD interface does not own exactly one MAD.COM plan"
            );
            PlannedMadSystemInterfaceTextWrites {
                plans: vec![
                    gaiji_bank_plan(
                        &compiled.shared.bank,
                        "mad-in-game-interface-text",
                        "mad-in-game-interface-gaiji-bank",
                        "install the game-interface bank while preserving DOS system text",
                    ),
                    interface
                        .plans
                        .into_iter()
                        .next()
                        .expect("one MAD interface plan was checked"),
                ],
                typed_sources: interface.typed_sources,
            }
        }
    };
    let mut planned = extend_scene_plans(
        mad_com,
        gaiji_com,
        &source.mad_com.scene_transition,
        &source.mad_com.fixed_gaiji_text.runtime,
        &source.mad_com.battle_callouts,
        compiled,
        shared,
    )?;
    planned
        .plans
        .push(fixed_slots_plan(mad_com, &compiled.fixed)?);
    planned.plans = combine_payload_write_plans(planned.plans)?;
    Ok(planned)
}

fn extend_scene_plans(
    mad_com: &[u8],
    gaiji_com: &[u8],
    scene: &crate::game_data::MadSceneTransitionCatalog,
    fixed_runtime: &crate::game_data::FixedGaijiTextRuntimeCatalog,
    battle_callouts: &crate::game_data::BattleCalloutCatalog,
    compiled: &CompiledMadSceneText,
    shared: PlannedMadSystemInterfaceTextWrites,
) -> Result<PlannedMadSceneTextWrites> {
    let PlannedMadSystemInterfaceTextWrites {
        plans: shared_plans,
        mut typed_sources,
    } = shared;
    let mut gaiji_plan = None;
    let mut mad_plan = None;
    for plan in shared_plans {
        match plan.file_name {
            GAIJI_FILE => ensure!(
                gaiji_plan.replace(plan.plan).is_none(),
                "duplicate shared GAIJI.COM plan"
            ),
            MAD_FILE => ensure!(
                mad_plan.replace(plan.plan).is_none(),
                "duplicate shared MAD.COM plan"
            ),
            name => anyhow::bail!("unexpected shared MAD scene plan for {name}"),
        }
    }
    let mut gaiji_plan = gaiji_plan.context("shared text plans are missing GAIJI.COM")?;
    let mut mad_plan = mad_plan.context("shared text plans are missing MAD.COM")?;
    gaiji_plan = append_scene_banks(gaiji_plan, gaiji_com, compiled)?;
    mad_plan = add_dialogue_storage(mad_plan, mad_com, compiled)?;
    mad_plan = add_battle_callout_storage(mad_plan, mad_com, battle_callouts, compiled)?;
    mad_plan = add_transition(mad_plan, mad_com, scene, fixed_runtime, compiled)?;
    mad_plan = super::super::unit_list_status::add_unit_list_status_plan(
        mad_plan,
        mad_com,
        &compiled.unit_list_status,
        &mut typed_sources,
    )?;

    insert_source(
        &mut typed_sources,
        WRAPPER_SOURCE_ID.to_owned(),
        compiled.transition.wrapper.clone(),
    )?;
    insert_source(
        &mut typed_sources,
        BATTLE_WRAPPER_SOURCE_ID.to_owned(),
        compiled.battle_hook.wrapper.clone(),
    )?;
    for call in &battle_callouts.calls {
        let id = battle_call_source_id(call);
        let source = compiled
            .battle_hook
            .call_sources
            .get(&id)
            .with_context(|| format!("missing battle callout source {id}"))?
            .clone();
        insert_source(&mut typed_sources, id, source)?;
    }
    insert_source(
        &mut typed_sources,
        STARTUP_ENTRY_SOURCE_ID.to_owned(),
        compiled.transition.startup_entry_source.clone(),
    )?;
    for call in &scene.calls {
        let id = call_source_id(call);
        let source = compiled
            .transition
            .call_sources
            .get(&id)
            .with_context(|| format!("missing MAD scene transition source {id}"))?
            .clone();
        insert_source(&mut typed_sources, id, source)?;
    }
    insert_source(
        &mut typed_sources,
        SELECTION_ENTRY_SOURCE_ID.to_owned(),
        compiled.transition.selection_entry_source.clone(),
    )?;
    let plans = vec![
        PayloadFileWritePlan {
            file_name: GAIJI_FILE,
            plan: gaiji_plan,
        },
        PayloadFileWritePlan {
            file_name: MAD_FILE,
            plan: mad_plan,
        },
    ];
    Ok(PlannedMadSceneTextWrites {
        plans,
        typed_sources,
    })
}

fn append_scene_banks(
    plan: WritePlan,
    source: &[u8],
    compiled: &CompiledMadSceneText,
) -> Result<WritePlan> {
    ensure!(
        compiled.dialogue_bank_file.file_offset
            == source.len() + compiled.interface_extension_records.len(),
        "dialogue bank no longer follows the explicit interface-bank extension"
    );
    ensure!(
        compiled.fixed_bank_file.file_offset == compiled.dialogue_bank_file.output_file_size,
        "fixed-text bank no longer immediately follows the dialogue bank"
    );
    ensure!(
        compiled.battle_bank_file.file_offset == compiled.fixed_bank_file.output_file_size,
        "battle-callout bank no longer immediately follows the fixed-text bank"
    );
    let interface_extension_range = source.len()..compiled.dialogue_bank_file.file_offset;
    let dialogue_range =
        compiled.dialogue_bank_file.file_offset..compiled.dialogue_bank_file.output_file_size;
    let fixed_range =
        compiled.fixed_bank_file.file_offset..compiled.fixed_bank_file.output_file_size;
    let battle_range =
        compiled.battle_bank_file.file_offset..compiled.battle_bank_file.output_file_size;
    Ok(plan
        .resize(ResizePlan {
            owner: "mad-scene-gaiji-bank-file".to_owned(),
            purpose: "append an explicit neutral interface extension plus complete dialogue and fixed-text GAIJI banks"
                .to_owned(),
            expected_input_len: source.len(),
            output_len: compiled.battle_bank_file.output_file_size,
        })
        .region(ImageRegion {
            id: "mad-scene-interface-gaiji-extension".to_owned(),
            range: interface_extension_range,
            kind: RegionKind::Data,
            reason: "neutral records make the startup bank match the shared installer count"
                .to_owned(),
        })
        .write(ExpectedWrite {
            id: "mad-scene-interface-gaiji-extension".to_owned(),
            owner: "mad-scene-gaiji-bank-file".to_owned(),
            purpose: "reserve unreferenced trailing codes without borrowing records from the dialogue bank"
                .to_owned(),
            offset: source.len(),
            expected_original: Vec::new(),
            replacement: compiled.interface_extension_records.clone(),
            intent: WriteIntent::Data,
        })
        .region(ImageRegion {
            id: "mad-scene-dialogue-gaiji-bank".to_owned(),
            range: dialogue_range,
            kind: RegionKind::Data,
            reason: "the complete target-format bank selected only during dialogue".to_owned(),
        })
        .write(ExpectedWrite {
            id: "mad-scene-dialogue-gaiji-bank".to_owned(),
            owner: "mad-scene-gaiji-bank-file".to_owned(),
            purpose: "store the complete dialogue bank without changing startup pointers"
                .to_owned(),
            offset: compiled.dialogue_bank_file.file_offset,
            expected_original: Vec::new(),
            replacement: compiled.dialogue_bank_file.records.clone(),
            intent: WriteIntent::Data,
        })
        .region(ImageRegion {
            id: "mad-scene-fixed-gaiji-bank".to_owned(),
            range: fixed_range,
            kind: RegionKind::Data,
            reason: "the complete target-format bank selected for the selection scene"
                .to_owned(),
        })
        .write(ExpectedWrite {
            id: "mad-scene-fixed-gaiji-bank".to_owned(),
            owner: "mad-scene-gaiji-bank-file".to_owned(),
            purpose: "store the fixed 6x7 text bank after the dialogue bank".to_owned(),
            offset: compiled.fixed_bank_file.file_offset,
            expected_original: Vec::new(),
            replacement: compiled.fixed_bank_file.records.clone(),
            intent: WriteIntent::Data,
        })
        .region(ImageRegion {
            id: "mad-scene-battle-callout-gaiji-bank".to_owned(),
            range: battle_range,
            kind: RegionKind::Data,
            reason: "the complete target-format bank selected only while battle callouts render"
                .to_owned(),
        })
        .write(ExpectedWrite {
            id: "mad-scene-battle-callout-gaiji-bank".to_owned(),
            owner: "mad-scene-gaiji-bank-file".to_owned(),
            purpose: "store the complete battle-callout bank after the fixed-text bank".to_owned(),
            offset: compiled.battle_bank_file.file_offset,
            expected_original: Vec::new(),
            replacement: compiled.battle_bank_file.records.clone(),
            intent: WriteIntent::Data,
        }))
}

fn add_dialogue_storage(
    mut plan: WritePlan,
    source: &[u8],
    compiled: &CompiledMadSceneText,
) -> Result<WritePlan> {
    let dialogue = &compiled.dialogue;
    plan = add_existing_write(
        plan,
        source,
        dialogue.record_table_start,
        dialogue.record_table_replacement.clone(),
        "mad-scene-dialogue-record-tables",
        "dialogue-reference-relocator",
        "relocate every dialogue text pointer while preserving presentation fields",
        RegionKind::Metadata,
        WriteIntent::Metadata,
    )?;
    plan = add_existing_write(
        plan,
        source,
        dialogue.text_region_start,
        dialogue.text_region_replacement[..dialogue.packed_storage_bytes].to_vec(),
        "mad-scene-dialogue-text",
        "dialogue-text-packer",
        "pack all reviewed dialogue records before the bank wrapper",
        RegionKind::Data,
        WriteIntent::Data,
    )?;
    Ok(plan)
}

fn add_battle_callout_storage(
    mut plan: WritePlan,
    source: &[u8],
    catalog: &crate::game_data::BattleCalloutCatalog,
    compiled: &CompiledMadSceneText,
) -> Result<WritePlan> {
    let battle = &compiled.battle;
    plan = add_existing_write(
        plan,
        source,
        battle.pointer_table_offset,
        battle.pointer_table_replacement.clone(),
        "mad-battle-callout-pointer-table",
        "battle-callout-pointer-relocator",
        "relocate all 18-unit by 4-callout table entries",
        RegionKind::Metadata,
        WriteIntent::Metadata,
    )?;
    plan = add_existing_write(
        plan,
        source,
        battle.text_region_start,
        battle.text_region_replacement[..battle.packed_storage_bytes].to_vec(),
        "mad-battle-callout-text",
        "battle-callout-text-packer",
        "pack all target battle callouts before their bank wrapper",
        RegionKind::Data,
        WriteIntent::Data,
    )?;
    let hook = &compiled.battle_hook;
    plan = add_existing_write(
        plan,
        source,
        hook.wrapper_offset,
        hook.wrapper.bytes().to_vec(),
        BATTLE_WRAPPER_SOURCE_ID,
        "battle-callout-typed-bank-transition",
        "install the battle bank, call the original renderer, then restore the interface bank",
        RegionKind::MachineCode,
        machine_intent(BATTLE_WRAPPER_SOURCE_ID),
    )?;
    let tail_start = hook.wrapper_offset + hook.wrapper.bytes().len();
    let tail_end = hook.wrapper_offset + hook.wrapper_capacity;
    if tail_start < tail_end {
        plan = add_existing_write(
            plan,
            source,
            tail_start,
            vec![0; tail_end - tail_start],
            "mad-battle-callout-tail-padding",
            "battle-callout-text-packer",
            "neutralize the obsolete source callout tail after the typed wrapper",
            RegionKind::Data,
            WriteIntent::Data,
        )?;
    }
    for source_call in &catalog.calls {
        let id = battle_call_source_id(source_call);
        let source_program = compiled
            .battle_hook
            .call_sources
            .get(&id)
            .with_context(|| format!("missing battle callout redirect {id}"))?;
        plan = add_existing_write(
            plan,
            source,
            source_call.file_offset,
            source_program.bytes().to_vec(),
            &id,
            "battle-callout-typed-bank-transition",
            &format!("route {} through the battle bank wrapper", source_call.id),
            RegionKind::MachineCode,
            machine_intent(&id),
        )?;
    }
    Ok(plan)
}

fn add_transition(
    mut plan: WritePlan,
    source: &[u8],
    scene: &crate::game_data::MadSceneTransitionCatalog,
    fixed_runtime: &crate::game_data::FixedGaijiTextRuntimeCatalog,
    compiled: &CompiledMadSceneText,
) -> Result<WritePlan> {
    let transition = &compiled.transition;
    plan = add_existing_write(
        plan,
        source,
        scene.startup_call.file_offset,
        transition.startup_entry_source.bytes().to_vec(),
        STARTUP_ENTRY_SOURCE_ID,
        "mad-scene-typed-bank-transition",
        "restore the interface bank before the original MAD startup initializer",
        RegionKind::MachineCode,
        machine_intent(STARTUP_ENTRY_SOURCE_ID),
    )?;
    plan = add_existing_write(
        plan,
        source,
        transition.wrapper_offset,
        transition.wrapper.bytes().to_vec(),
        WRAPPER_SOURCE_ID,
        "mad-scene-typed-bank-transition",
        "install the dialogue bank, call the original consumer, then restore the interface bank",
        RegionKind::MachineCode,
        machine_intent(WRAPPER_SOURCE_ID),
    )?;
    let tail_start = transition.wrapper_offset + transition.wrapper.bytes().len();
    let tail_end = transition.wrapper_offset + transition.wrapper_capacity;
    if tail_start < tail_end {
        plan = add_existing_write(
            plan,
            source,
            tail_start,
            vec![0; tail_end - tail_start],
            "mad-scene-dialogue-tail-padding",
            "mad-scene-bank-storage",
            "zero the unused remainder of the verified dialogue storage tail",
            RegionKind::Data,
            WriteIntent::Data,
        )?;
    }
    for call in &scene.calls {
        let id = call_source_id(call);
        let replacement = transition
            .call_sources
            .get(&id)
            .with_context(|| format!("missing MAD scene call source {id}"))?
            .bytes()
            .to_vec();
        plan = add_existing_write(
            plan,
            source,
            call.file_offset,
            replacement,
            &id,
            "mad-scene-typed-bank-transition",
            &format!("route {} through the bank-switch wrapper", call.id),
            RegionKind::MachineCode,
            machine_intent(&id),
        )?;
    }
    plan = add_existing_write(
        plan,
        source,
        fixed_runtime.selection_entry_call.file_offset,
        transition.selection_entry_source.bytes().to_vec(),
        SELECTION_ENTRY_SOURCE_ID,
        "mad-scene-typed-bank-transition",
        "install the fixed-text bank before entering the selection scene",
        RegionKind::MachineCode,
        machine_intent(SELECTION_ENTRY_SOURCE_ID),
    )?;
    Ok(plan)
}

#[allow(clippy::too_many_arguments)]
fn add_existing_write(
    plan: WritePlan,
    source: &[u8],
    offset: usize,
    replacement: Vec<u8>,
    id: &str,
    owner: &str,
    purpose: &str,
    kind: RegionKind,
    intent: WriteIntent,
) -> Result<WritePlan> {
    let range = offset..offset + replacement.len();
    let expected_original = source
        .get(range.clone())
        .with_context(|| format!("MAD scene write {id} lies outside immutable input"))?
        .to_vec();
    Ok(plan
        .region(ImageRegion {
            id: id.to_owned(),
            range,
            kind,
            reason: purpose.to_owned(),
        })
        .write(ExpectedWrite {
            id: id.to_owned(),
            owner: owner.to_owned(),
            purpose: purpose.to_owned(),
            offset,
            expected_original,
            replacement,
            intent,
        }))
}

fn machine_intent(source_id: &str) -> WriteIntent {
    WriteIntent::MachineCode(MachineCodeProvenance {
        assembly_source_id: source_id.to_owned(),
        isa_profile_id: v30::PROFILE_ID.to_owned(),
    })
}

fn insert_source(
    sources: &mut BTreeMap<String, AssembledProgram>,
    id: String,
    source: AssembledProgram,
) -> Result<()> {
    ensure!(
        sources.insert(id.clone(), source).is_none(),
        "duplicate typed V30 MAD scene source {id}"
    );
    Ok(())
}
