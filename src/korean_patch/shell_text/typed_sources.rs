use std::collections::BTreeMap;

use anyhow::{Context, Result};
use v30::{AssembledProgram, Assembler, CodeLocation, Instruction, JmpTarget, Register16};

use super::model::CompiledShellTextEntry;
use crate::game_data::GaijiCatalog;
use crate::korean_patch::shared_text::{
    CompiledGaijiBank, InstallerInterruptPolicy, assemble_embedded_gaiji_installer,
};
use crate::korean_patch::typed_v30::assemble_relocated_mov_immediate;

pub(super) const ORIGINAL_ENTRY_COM_ADDRESS: u16 = 0x030b;
pub(super) const MEMORY_END_INSTRUCTION_OFFSET: usize = 0x0236;
pub(super) const ENTRY_SOURCE_ID: &str = "dsh-hangul-entry-jump";
pub(super) const MEMORY_END_SOURCE_ID: &str = "dsh-relocated-memory-end";
pub(super) const INSTALLER_SOURCE_ID: &str = "dsh-embedded-gaiji-installer";

const COM_ORIGIN: usize = 0x100;
pub(super) fn assemble_embedded_installer(
    source_gaiji: &GaijiCatalog,
    bank: &CompiledGaijiBank,
    installer_offset: usize,
) -> Result<(AssembledProgram, Vec<u8>)> {
    assemble_embedded_gaiji_installer(
        source_gaiji,
        bank,
        installer_offset,
        ORIGINAL_ENTRY_COM_ADDRESS,
        InstallerInterruptPolicy::Inherit,
        "DSH",
    )
}

pub(super) fn assemble_shell_sources(
    source: &[u8],
    installer_offset: usize,
    installer: AssembledProgram,
    output_file_size: usize,
    entries: &[CompiledShellTextEntry],
) -> Result<BTreeMap<String, AssembledProgram>> {
    let mut sources = BTreeMap::new();
    sources.insert(
        ENTRY_SOURCE_ID.to_owned(),
        assemble_entry_jump(installer_offset)?,
    );
    sources.insert(
        MEMORY_END_SOURCE_ID.to_owned(),
        assemble_relocated_mov_immediate(
            source,
            MEMORY_END_INSTRUCTION_OFFSET,
            com_address(output_file_size, "DSH output end")?,
            &[Register16::BX],
            "DSH memory resize end",
        )?,
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
                    &format!("DSH text reference at {consumer_offset:#x}"),
                )?,
            );
        }
    }
    Ok(sources)
}

fn assemble_entry_jump(installer_offset: usize) -> Result<AssembledProgram> {
    let target = i32::from(com_address(installer_offset, "DSH installer")?);
    let displacement =
        i16::try_from(target - 0x0103).context("DSH entry hook is outside a near jump")?;
    let mut assembler = Assembler::new();
    assembler.emit(Instruction::Jmp {
        target: JmpTarget::Rel16(displacement),
    });
    Ok(assembler.assemble(CodeLocation { seg: 0, off: 0x100 })?)
}

pub(super) fn reference_source_id(consumer_offset: usize) -> String {
    format!("dsh-relocated-text-{consumer_offset:04x}")
}

fn com_address(file_offset: usize, role: &str) -> Result<u16> {
    u16::try_from(file_offset + COM_ORIGIN)
        .with_context(|| format!("{role} COM address exceeds 16 bits"))
}

#[cfg(test)]
#[path = "typed_sources_tests.rs"]
mod typed_sources_tests;
