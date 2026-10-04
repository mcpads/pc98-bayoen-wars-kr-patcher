use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use v30::{
    AssembledProgram, Assembler, CallTarget, CodeLocation, Instruction, JmpTarget, Register16,
};

use super::model::CompiledMenuTextEntry;
use crate::game_data::{MenuRuntimeCatalog, MenuTextRuntimeReferenceKind};
use crate::korean_patch::typed_v30::{
    assemble_relocated_direct_ax_store, assemble_relocated_mov_immediate,
};

const COM_ORIGIN: usize = 0x100;
pub(super) const ENTRY_SOURCE_ID: &str = "menu-hangul-entry-jump";
pub(super) const INSTALLER_SOURCE_ID: &str = "menu-embedded-gaiji-installer";
pub(super) const TRAMPOLINE_SOURCE_ID: &str = "menu-original-entry-trampoline";

pub(super) fn reference_source_id(instruction_offset: usize) -> String {
    format!("menu-relocated-text-{instruction_offset:04x}")
}

pub(super) fn runtime_insert_source_id(id: &str) -> String {
    format!("{id}-relocated-store")
}

pub(super) fn assemble_menu_sources(
    source: &[u8],
    runtime: &MenuRuntimeCatalog,
    installer_offset: usize,
    installer: AssembledProgram,
    trampoline: AssembledProgram,
    entries: &[CompiledMenuTextEntry],
) -> Result<BTreeMap<String, AssembledProgram>> {
    let mut sources = BTreeMap::new();
    ensure_unique(
        &mut sources,
        ENTRY_SOURCE_ID.to_owned(),
        assemble_entry_jump(runtime.entry.hook_instruction_offset, installer_offset)?,
    )?;
    ensure_unique(&mut sources, INSTALLER_SOURCE_ID.to_owned(), installer)?;
    ensure_unique(&mut sources, TRAMPOLINE_SOURCE_ID.to_owned(), trampoline)?;

    for reference in runtime.references.iter().filter(|reference| {
        reference.storage_kind == MenuTextRuntimeReferenceKind::MachineCodeImmediate
    }) {
        let instruction_offset = reference
            .instruction_offset
            .context("MENU machine-code reference lost its instruction offset")?;
        let target = entry(entries, &reference.target_entry_id)?.com_address;
        let assembled = assemble_relocated_mov_immediate(
            source,
            instruction_offset,
            target,
            &[Register16::DX, Register16::SI],
            &format!("MENU text reference at {instruction_offset:#x}"),
        )?;
        ensure_unique(
            &mut sources,
            reference_source_id(instruction_offset),
            assembled,
        )?;
    }
    for reference in &runtime.runtime_insert_references {
        let entry = entry(entries, &reference.target_entry_id)?;
        let field_offset = *entry
            .runtime_field_offsets
            .get(&reference.id)
            .with_context(|| format!("{} compiled record lost its runtime field", reference.id))?;
        let target = u16::try_from(usize::from(entry.com_address) + field_offset)
            .context("MENU relocated runtime field exceeds 16 bits")?;
        let assembled = assemble_relocated_direct_ax_store(
            source,
            reference.instruction_offset,
            target,
            &reference.id,
        )?;
        ensure_unique(
            &mut sources,
            runtime_insert_source_id(&reference.id),
            assembled,
        )?;
    }
    Ok(sources)
}

pub(super) fn assemble_original_entry_trampoline(
    trampoline_offset: usize,
    original_call_target: u16,
    resume_address: u16,
) -> Result<AssembledProgram> {
    let origin = com_location(trampoline_offset, "MENU entry trampoline")?;
    let call_next = i32::from(origin.off) + 3;
    let call_displacement = i16::try_from(i32::from(original_call_target) - call_next)
        .context("MENU original entry call is outside the trampoline's near range")?;
    let jump_next = call_next + 3;
    let jump_displacement = i16::try_from(i32::from(resume_address) - jump_next)
        .context("MENU entry resume is outside the trampoline's near range")?;
    let mut assembler = Assembler::new();
    assembler
        .emit(Instruction::Call {
            target: CallTarget::Rel16(call_displacement),
        })
        .emit(Instruction::Jmp {
            target: JmpTarget::Rel16(jump_displacement),
        });
    Ok(assembler.assemble(origin)?)
}

fn assemble_entry_jump(entry_offset: usize, installer_offset: usize) -> Result<AssembledProgram> {
    let origin = com_location(entry_offset, "MENU entry hook")?;
    let next = i32::from(origin.off) + 3;
    let target = i32::from(com_address(installer_offset, "MENU GAIJI installer")?);
    let displacement = i16::try_from(target - next)
        .context("MENU GAIJI installer is outside the entry hook's near range")?;
    let mut assembler = Assembler::new();
    assembler.emit(Instruction::Jmp {
        target: JmpTarget::Rel16(displacement),
    });
    Ok(assembler.assemble(origin)?)
}

fn entry<'a>(entries: &'a [CompiledMenuTextEntry], id: &str) -> Result<&'a CompiledMenuTextEntry> {
    entries
        .iter()
        .find(|entry| entry.id == id)
        .with_context(|| format!("missing compiled MENU record {id}"))
}

fn ensure_unique(
    sources: &mut BTreeMap<String, AssembledProgram>,
    id: String,
    source: AssembledProgram,
) -> Result<()> {
    ensure!(
        sources.insert(id.clone(), source).is_none(),
        "duplicate typed V30 MENU source {id}"
    );
    Ok(())
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
