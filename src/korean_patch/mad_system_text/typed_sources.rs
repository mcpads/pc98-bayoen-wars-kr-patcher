use std::collections::BTreeMap;

use anyhow::{Context, Result};
use v30::{AssembledProgram, Register16};

use super::model::CompiledMadSystemText;
use crate::game_data::{
    MadSystemRuntimeCatalog, MadSystemRuntimeReference, MadSystemRuntimeReferenceKind,
};
use crate::korean_patch::typed_v30::assemble_relocated_mov_immediate;

pub(super) fn assemble_relocated_system_sources(
    mad_com: &[u8],
    runtime: &MadSystemRuntimeCatalog,
    compiled: &CompiledMadSystemText,
) -> Result<BTreeMap<String, AssembledProgram>> {
    let addresses = compiled
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry.com_address))
        .collect::<BTreeMap<_, _>>();
    runtime
        .references
        .iter()
        .filter(|reference| {
            reference.kind != MadSystemRuntimeReferenceKind::MetadataTableEntry
                && addresses[reference.target_entry_id.as_str()] != reference.original_com_address
        })
        .map(|reference| {
            let instruction_offset = reference
                .instruction_offset
                .context("MAD system machine reference has no instruction offset")?;
            let register = match reference.kind {
                MadSystemRuntimeReferenceKind::MachineCodeImmediate => Register16::DX,
                MadSystemRuntimeReferenceKind::RuntimeInsertMachineCode => Register16::BX,
                MadSystemRuntimeReferenceKind::MetadataTableEntry => unreachable!(),
            };
            let source_id = assembly_source_id(reference);
            let source = assemble_relocated_mov_immediate(
                mad_com,
                instruction_offset,
                addresses[reference.target_entry_id.as_str()],
                &[register],
                &format!("MAD system reference at {instruction_offset:#x}"),
            )?;
            Ok((source_id, source))
        })
        .collect()
}

pub(super) fn assembly_source_id(reference: &MadSystemRuntimeReference) -> String {
    format!(
        "mad-system-relocated-address-{:04x}",
        reference.storage_offset
    )
}

#[cfg(test)]
#[path = "typed_sources_tests.rs"]
mod typed_sources_tests;
