use std::collections::BTreeMap;

use anyhow::{Context, Result};
use v30::{AssembledProgram, Assembler, CodeLocation, Instruction, JmpTarget, Register16};

use super::model::CompiledSamplingDriverTextEntry;
use crate::external_text::SamplingDriverLifetimeCatalog;
use crate::game_data::GaijiCatalog;
use crate::korean_patch::shared_text::{
    CompiledGaijiBank, InstallerInterruptPolicy, assemble_embedded_gaiji_installer,
};
use crate::korean_patch::typed_v30::assemble_relocated_mov_immediate;

pub(super) const ENTRY_SOURCE_ID: &str = "sampling-driver-hangul-entry-jump";
pub(super) const INSTALLER_SOURCE_ID: &str = "sampling-driver-embedded-gaiji-installer";

const COM_ORIGIN: usize = 0x100;
pub(super) fn assemble_embedded_installer(
    source_gaiji: &GaijiCatalog,
    bank: &CompiledGaijiBank,
    installer_offset: usize,
    handoff_com_address: u16,
) -> Result<(AssembledProgram, Vec<u8>)> {
    assemble_embedded_gaiji_installer(
        source_gaiji,
        bank,
        installer_offset,
        handoff_com_address,
        InstallerInterruptPolicy::EnableAndRestore,
        "sampling-driver",
    )
}

pub(super) fn assemble_sampling_driver_sources(
    source: &[u8],
    lifetime: SamplingDriverLifetimeCatalog,
    installer_offset: usize,
    installer: AssembledProgram,
    entries: &[CompiledSamplingDriverTextEntry],
) -> Result<BTreeMap<String, AssembledProgram>> {
    let mut sources = BTreeMap::new();
    sources.insert(
        ENTRY_SOURCE_ID.to_owned(),
        assemble_entry_jump(lifetime.entry_jump_offset, installer_offset)?,
    );
    sources.insert(INSTALLER_SOURCE_ID.to_owned(), installer);
    for entry in entries {
        for &consumer_offset in &entry.consumer_offsets {
            sources.insert(
                reference_source_id(consumer_offset),
                assemble_relocated_mov_immediate(
                    source,
                    consumer_offset,
                    entry.com_address,
                    &[Register16::DX],
                    &format!("sampling-driver text reference at {consumer_offset:#x}"),
                )?,
            );
        }
    }
    Ok(sources)
}

fn assemble_entry_jump(
    entry_jump_offset: usize,
    installer_file_offset: usize,
) -> Result<AssembledProgram> {
    let origin = com_location(entry_jump_offset, "sampling-driver entry jump")?;
    let next = i32::from(origin.off) + 3;
    let target = i32::from(com_address(
        installer_file_offset,
        "sampling-driver installer",
    )?);
    let displacement = i16::try_from(target - next)
        .context("sampling-driver entry hook is outside a near jump")?;
    let mut assembler = Assembler::new();
    assembler.emit(Instruction::Jmp {
        target: JmpTarget::Rel16(displacement),
    });
    Ok(assembler.assemble(origin)?)
}

pub(super) fn reference_source_id(consumer_offset: usize) -> String {
    format!("sampling-driver-relocated-text-{consumer_offset:04x}")
}

fn com_address(file_offset: usize, role: &str) -> Result<u16> {
    u16::try_from(file_offset + COM_ORIGIN)
        .with_context(|| format!("{role} COM address exceeds 16 bits"))
}

fn com_location(file_offset: usize, role: &str) -> Result<CodeLocation> {
    Ok(CodeLocation {
        seg: 0,
        off: com_address(file_offset, role)?,
    })
}

#[cfg(test)]
#[path = "typed_sources_tests.rs"]
mod typed_sources_tests;
