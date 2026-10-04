use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, RegionKind, WriteIntent, WritePlan,
};
use v30::AssembledProgram;

use super::super::shared_alert_window::{
    add_shared_alert_window_plan, add_shared_alert_window_sources,
};
use super::super::spring_capture_window::{
    add_spring_capture_window_plan, add_spring_capture_window_sources,
};
use super::super::spring_recovery_window::{
    add_spring_recovery_window_plan, add_spring_recovery_window_sources,
};
use super::model::CompiledInterfaceText;
use super::typed_sources::{assemble_relocated_reference_sources, assembly_source_id};
use crate::game_data::{InterfaceTextReferenceCatalog, InterfaceTextReferenceKind};
use crate::korean_patch::payload_writes::PayloadFileWritePlan;

const MAD_FILE: &str = "MAD.COM";

pub(in crate::korean_patch) struct PlannedInterfaceTextWrites {
    pub plans: Vec<PayloadFileWritePlan>,
    pub typed_sources: BTreeMap<String, AssembledProgram>,
}

pub(in crate::korean_patch) fn interface_text_plans(
    mad_com: &[u8],
    references: &InterfaceTextReferenceCatalog,
    compiled: &CompiledInterfaceText,
) -> Result<PlannedInterfaceTextWrites> {
    let mut plan = WritePlan::new();
    plan = add_text_region(plan, mad_com, compiled)?;
    plan = add_metadata_tables(plan, mad_com, references, compiled)?;
    let mut typed_sources = assemble_relocated_reference_sources(mad_com, references, compiled)?;
    plan = add_machine_code_references(plan, mad_com, references, compiled, &typed_sources)?;
    plan = add_shared_alert_window_plan(plan, mad_com, &compiled.shared_alert_window)?;
    plan = add_shared_alert_window_plan(plan, mad_com, &compiled.stage_result_window)?;
    plan = add_spring_capture_window_plan(plan, mad_com, &compiled.spring_capture_window)?;
    plan = add_spring_recovery_window_plan(plan, mad_com, &compiled.spring_recovery_window)?;
    add_shared_alert_window_sources(&mut typed_sources, &compiled.shared_alert_window)?;
    add_shared_alert_window_sources(&mut typed_sources, &compiled.stage_result_window)?;
    add_spring_capture_window_sources(&mut typed_sources, &compiled.spring_capture_window)?;
    add_spring_recovery_window_sources(&mut typed_sources, &compiled.spring_recovery_window)?;
    Ok(PlannedInterfaceTextWrites {
        plans: vec![PayloadFileWritePlan {
            file_name: MAD_FILE,
            plan,
        }],
        typed_sources,
    })
}

fn add_text_region(
    plan: WritePlan,
    mad_com: &[u8],
    compiled: &CompiledInterfaceText,
) -> Result<WritePlan> {
    let range = compiled.text_region_start..compiled.text_region_end;
    let expected_original = mad_com
        .get(range.clone())
        .context("interface text storage lies outside MAD.COM")?
        .to_vec();
    Ok(plan
        .region(ImageRegion {
            id: "interface-text-storage".to_owned(),
            range,
            kind: RegionKind::Data,
            reason: "all 88 relocated interface records and explicitly owned runtime tail"
                .to_owned(),
        })
        .write(ExpectedWrite {
            id: "interface-text-storage".to_owned(),
            owner: "interface-text-packer".to_owned(),
            purpose:
                "pack reviewed interface text and bounded runtime data inside verified storage"
                    .to_owned(),
            offset: compiled.text_region_start,
            expected_original,
            replacement: compiled.text_region_replacement.clone(),
            intent: WriteIntent::Data,
        }))
}

fn add_metadata_tables(
    mut plan: WritePlan,
    mad_com: &[u8],
    references: &InterfaceTextReferenceCatalog,
    compiled: &CompiledInterfaceText,
) -> Result<WritePlan> {
    let addresses = compiled
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry.com_address))
        .collect::<BTreeMap<_, _>>();
    for table in &references.metadata_tables {
        let range = table.offset..table.offset + table.entry_count * table.stride;
        let expected_original = mad_com
            .get(range.clone())
            .with_context(|| format!("interface metadata table {} lies outside MAD.COM", table.id))?
            .to_vec();
        let mut replacement = expected_original.clone();
        let table_references = references
            .references
            .iter()
            .filter(|reference| {
                reference.storage_kind == InterfaceTextReferenceKind::MetadataTableEntry
                    && reference.consumer_role == table.id
            })
            .collect::<Vec<_>>();
        ensure!(
            table_references.len() == table.entry_count,
            "interface metadata table {} reference population changed",
            table.id
        );
        for reference in table_references {
            let relative = reference.storage_offset - table.offset;
            replacement[relative..relative + 2]
                .copy_from_slice(&addresses[reference.target_entry_id.as_str()].to_le_bytes());
        }
        if replacement == expected_original {
            continue;
        }
        plan = plan
            .region(ImageRegion {
                id: table.id.clone(),
                range,
                kind: RegionKind::Metadata,
                reason: "consumer-indexed interface text addresses and preserved record fields"
                    .to_owned(),
            })
            .write(ExpectedWrite {
                id: table.id.clone(),
                owner: "interface-reference-relocator".to_owned(),
                purpose: format!("relocate every address in {}", table.id),
                offset: table.offset,
                expected_original,
                replacement,
                intent: WriteIntent::Metadata,
            });
    }
    Ok(plan)
}

fn add_machine_code_references(
    mut plan: WritePlan,
    mad_com: &[u8],
    references: &InterfaceTextReferenceCatalog,
    compiled: &CompiledInterfaceText,
    typed_sources: &BTreeMap<String, AssembledProgram>,
) -> Result<WritePlan> {
    let addresses = compiled
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry.com_address))
        .collect::<BTreeMap<_, _>>();
    for reference in references.references.iter().filter(|reference| {
        reference.storage_kind == InterfaceTextReferenceKind::MachineCodeImmediate
    }) {
        let new_address = addresses[reference.target_entry_id.as_str()];
        if new_address == reference.target_com_address {
            continue;
        }
        let instruction_offset = reference
            .instruction_offset
            .context("interface machine reference has no instruction offset")?;
        let source_id = assembly_source_id(reference);
        let source = typed_sources
            .get(&source_id)
            .with_context(|| format!("missing typed V30 source {source_id}"))?;
        let replacement = source.bytes().to_vec();
        let range = instruction_offset..instruction_offset + replacement.len();
        let expected_original = mad_com
            .get(range.clone())
            .context("interface machine reference lies outside MAD.COM")?
            .to_vec();
        let id = format!("interface-code-address-{instruction_offset:04x}");
        plan = plan
            .region(ImageRegion {
                id: id.clone(),
                range,
                kind: RegionKind::MachineCode,
                reason: "one complete typed V30 MOV reg16, imm16 instruction".to_owned(),
            })
            .write(ExpectedWrite {
                id,
                owner: "interface-typed-reference-relocator".to_owned(),
                purpose: format!(
                    "relocate {} to {new_address:#06x}",
                    reference.target_entry_id
                ),
                offset: instruction_offset,
                expected_original,
                replacement,
                intent: WriteIntent::MachineCode(MachineCodeProvenance {
                    assembly_source_id: source_id,
                    isa_profile_id: v30::PROFILE_ID.to_owned(),
                }),
            });
    }
    ensure!(
        typed_sources.len()
            == references
                .references
                .iter()
                .filter(|reference| {
                    reference.storage_kind == InterfaceTextReferenceKind::MachineCodeImmediate
                        && addresses[reference.target_entry_id.as_str()]
                            != reference.target_com_address
                })
                .count(),
        "typed V30 interface source population changed"
    );
    Ok(plan)
}
