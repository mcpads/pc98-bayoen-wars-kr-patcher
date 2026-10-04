use std::collections::BTreeMap;

use anyhow::{Context, Result};
use v30::{AssembledProgram, Register16};

use super::model::CompiledInterfaceText;
use crate::game_data::{
    InterfaceTextReference, InterfaceTextReferenceCatalog, InterfaceTextReferenceKind,
};
use crate::korean_patch::typed_v30::assemble_relocated_mov_immediate;

pub(super) fn assemble_relocated_reference_sources(
    mad_com: &[u8],
    references: &InterfaceTextReferenceCatalog,
    compiled: &CompiledInterfaceText,
) -> Result<BTreeMap<String, AssembledProgram>> {
    let addresses = compiled
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry.com_address))
        .collect::<BTreeMap<_, _>>();
    references
        .references
        .iter()
        .filter(|reference| {
            reference.storage_kind == InterfaceTextReferenceKind::MachineCodeImmediate
                && addresses[reference.target_entry_id.as_str()] != reference.target_com_address
        })
        .map(|reference| {
            let source_id = assembly_source_id(reference);
            let program = assemble_reference(
                mad_com,
                reference,
                addresses[reference.target_entry_id.as_str()],
            )?;
            Ok((source_id, program))
        })
        .collect()
}

fn assemble_reference(
    mad_com: &[u8],
    reference: &InterfaceTextReference,
    target_com_address: u16,
) -> Result<AssembledProgram> {
    let instruction_offset = reference
        .instruction_offset
        .context("machine-code interface reference has no instruction offset")?;
    assemble_relocated_mov_immediate(
        mad_com,
        instruction_offset,
        target_com_address,
        &[Register16::BX, Register16::DX],
        &format!("interface reference at {instruction_offset:#x}"),
    )
}

pub(super) fn assembly_source_id(reference: &InterfaceTextReference) -> String {
    format!(
        "interface-relocated-address-{:04x}",
        reference.storage_offset
    )
}

#[cfg(test)]
#[path = "typed_sources_tests.rs"]
mod typed_sources_tests;
