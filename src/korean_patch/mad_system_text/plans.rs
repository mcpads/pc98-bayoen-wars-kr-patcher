use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, RegionKind, WriteIntent, WritePlan,
};
use v30::AssembledProgram;

use super::model::CompiledMadSystemText;
use super::typed_sources::{assemble_relocated_system_sources, assembly_source_id};
use crate::game_data::{MadSystemRuntimeCatalog, MadSystemRuntimeReferenceKind};
use crate::korean_patch::payload_writes::PayloadFileWritePlan;

const MAD_FILE: &str = "MAD.COM";

pub(in crate::korean_patch) struct PlannedMadSystemTextWrites {
    pub plans: Vec<PayloadFileWritePlan>,
    pub typed_sources: BTreeMap<String, AssembledProgram>,
}

pub(in crate::korean_patch) fn mad_system_text_plans(
    mad_com: &[u8],
    runtime: &MadSystemRuntimeCatalog,
    compiled: &CompiledMadSystemText,
) -> Result<PlannedMadSystemTextWrites> {
    let mut plan = WritePlan::new();
    plan = add_text_region(plan, mad_com, compiled)?;
    plan = add_metadata_table(plan, mad_com, runtime, compiled)?;
    let typed_sources = assemble_relocated_system_sources(mad_com, runtime, compiled)?;
    plan = add_machine_code_references(plan, mad_com, runtime, compiled, &typed_sources)?;
    Ok(PlannedMadSystemTextWrites {
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
    compiled: &CompiledMadSystemText,
) -> Result<WritePlan> {
    let range = compiled.text_region_start..compiled.text_region_end;
    let expected_original = mad_com
        .get(range.clone())
        .context("MAD system text storage lies outside MAD.COM")?
        .to_vec();
    Ok(plan
        .region(ImageRegion {
            id: "mad-system-text-storage".to_owned(),
            range,
            kind: RegionKind::Data,
            reason: "all ten relocated DOS system records and zeroed storage tail".to_owned(),
        })
        .write(ExpectedWrite {
            id: "mad-system-text-storage".to_owned(),
            owner: "mad-system-text-packer".to_owned(),
            purpose: "pack reviewed MAD system text inside its verified source storage".to_owned(),
            offset: compiled.text_region_start,
            expected_original,
            replacement: compiled.text_region_replacement.clone(),
            intent: WriteIntent::Data,
        }))
}

fn add_metadata_table(
    plan: WritePlan,
    mad_com: &[u8],
    runtime: &MadSystemRuntimeCatalog,
    compiled: &CompiledMadSystemText,
) -> Result<WritePlan> {
    let references = runtime
        .references
        .iter()
        .filter(|reference| reference.kind == MadSystemRuntimeReferenceKind::MetadataTableEntry)
        .collect::<Vec<_>>();
    ensure!(
        references.len() == runtime.metadata_reference_count,
        "MAD system metadata reference population changed"
    );
    let start = references
        .first()
        .context("MAD system metadata table is empty")?
        .storage_offset;
    let end = references
        .last()
        .context("MAD system metadata table is empty")?
        .storage_offset
        + 2;
    ensure!(
        references
            .iter()
            .enumerate()
            .all(|(index, reference)| reference.storage_offset == start + index * 2),
        "MAD system metadata table is not contiguous"
    );
    let expected_original = mad_com
        .get(start..end)
        .context("MAD system metadata table lies outside MAD.COM")?
        .to_vec();
    let addresses = compiled_addresses(compiled);
    let mut replacement = expected_original.clone();
    for reference in references {
        let relative = reference.storage_offset - start;
        replacement[relative..relative + 2]
            .copy_from_slice(&addresses[reference.target_entry_id.as_str()].to_le_bytes());
    }
    if replacement == expected_original {
        return Ok(plan);
    }
    Ok(plan
        .region(ImageRegion {
            id: "mad-system-disk-error-pointer-table".to_owned(),
            range: start..end,
            kind: RegionKind::Metadata,
            reason: "14 consumer-indexed DOS critical-error text addresses".to_owned(),
        })
        .write(ExpectedWrite {
            id: "mad-system-disk-error-pointer-table".to_owned(),
            owner: "mad-system-reference-relocator".to_owned(),
            purpose: "relocate every DOS critical-error pointer".to_owned(),
            offset: start,
            expected_original,
            replacement,
            intent: WriteIntent::Metadata,
        }))
}

fn add_machine_code_references(
    mut plan: WritePlan,
    mad_com: &[u8],
    runtime: &MadSystemRuntimeCatalog,
    compiled: &CompiledMadSystemText,
    typed_sources: &BTreeMap<String, AssembledProgram>,
) -> Result<WritePlan> {
    let addresses = compiled_addresses(compiled);
    let changed = runtime.references.iter().filter(|reference| {
        reference.kind != MadSystemRuntimeReferenceKind::MetadataTableEntry
            && addresses[reference.target_entry_id.as_str()] != reference.original_com_address
    });
    let mut count = 0;
    for reference in changed {
        count += 1;
        let instruction_offset = reference
            .instruction_offset
            .context("MAD system machine reference has no instruction offset")?;
        let source_id = assembly_source_id(reference);
        let source = typed_sources
            .get(&source_id)
            .with_context(|| format!("missing typed V30 source {source_id}"))?;
        let replacement = source.bytes().to_vec();
        let range = instruction_offset..instruction_offset + replacement.len();
        let expected_original = mad_com
            .get(range.clone())
            .context("MAD system machine reference lies outside MAD.COM")?
            .to_vec();
        let id = format!("mad-system-code-address-{instruction_offset:04x}");
        plan = plan
            .region(ImageRegion {
                id: id.clone(),
                range,
                kind: RegionKind::MachineCode,
                reason: "one complete typed V30 MOV reg16, imm16 instruction".to_owned(),
            })
            .write(ExpectedWrite {
                id,
                owner: "mad-system-typed-reference-relocator".to_owned(),
                purpose: format!(
                    "relocate {} to {:#06x}",
                    reference.target_entry_id,
                    addresses[reference.target_entry_id.as_str()]
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
        count == typed_sources.len(),
        "typed V30 MAD system source population changed"
    );
    Ok(plan)
}

fn compiled_addresses(compiled: &CompiledMadSystemText) -> BTreeMap<&str, u16> {
    compiled
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry.com_address))
        .collect()
}
