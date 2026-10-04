use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use v30::{
    AssembledProgram, Assembler, CodeLocation, EffectiveAddress, EffectiveAddressBase,
    EffectiveAddressDisplacement, Instruction, JmpTarget, OperandSize, Register16, SegmentRegister,
};

use super::model::CompiledSystemLoaderTextEntry;
use crate::external_text::SystemLoaderRuntimeCatalog;
use crate::game_data::GaijiCatalog;
use crate::korean_patch::shared_text::{
    CompiledGaijiBank, InstallerInterruptPolicy, assemble_segment_gaiji_installer,
};
use crate::korean_patch::typed_v30::assemble_relocated_segment_mov_immediate;

pub(super) const RESIDENT_SIZE_SOURCE_ID: &str = "megdos-resident-size";
pub(super) const RESIDENT_COPY_SOURCE_ID: &str = "megdos-resident-copy-count";
pub(super) const POST_STACK_HOOK_SOURCE_ID: &str = "megdos-post-stack-installer-jump";
pub(super) const INSTALLER_SOURCE_ID: &str = "megdos-resident-gaiji-installer";
pub(super) const TRAMPOLINE_SOURCE_ID: &str = "megdos-post-stack-lds-trampoline";

pub(super) fn reference_source_id(instruction_offset: usize) -> String {
    format!("megdos-relocated-text-{instruction_offset:04x}")
}

pub(super) fn assemble_system_loader_installer(
    source_gaiji: &GaijiCatalog,
    bank: &CompiledGaijiBank,
    installer_offset: usize,
    trampoline_offset: u16,
) -> Result<(AssembledProgram, Vec<u8>)> {
    assemble_segment_gaiji_installer(
        source_gaiji,
        bank,
        installer_offset,
        trampoline_offset,
        InstallerInterruptPolicy::Inherit,
        "MEGDOS.SYS",
    )
}

pub(super) fn assemble_post_stack_trampoline(
    source: &[u8],
    runtime: &SystemLoaderRuntimeCatalog,
    trampoline_offset: usize,
) -> Result<AssembledProgram> {
    let source_instruction = source
        .get(
            runtime.post_stack_hook_offset
                ..runtime.post_stack_hook_offset + runtime.post_stack_hook_byte_count,
        )
        .context("MEGDOS.SYS original post-stack load lies outside the source")?;
    let memory = EffectiveAddress::new(
        Some(SegmentRegister::SS),
        EffectiveAddressBase::Direct,
        EffectiveAddressDisplacement::Absolute(0x0005),
        OperandSize::Word,
    )?;
    let origin = segment_location(trampoline_offset, "MEGDOS.SYS trampoline")?;
    let mut placeholder = Assembler::new();
    placeholder
        .emit(Instruction::Lds {
            dest: Register16::SI,
            src: memory,
        })
        .emit(Instruction::Jmp {
            target: JmpTarget::Rel16(0),
        });
    let placeholder = placeholder.assemble(origin)?;
    ensure!(
        placeholder.bytes().starts_with(source_instruction),
        "MEGDOS.SYS trampoline did not reproduce the exact displaced LDS"
    );
    let next = i32::from(origin.off) + i32::try_from(placeholder.bytes().len())?;
    let displacement = i16::try_from(i32::from(runtime.post_stack_resume_address) - next)
        .context("MEGDOS.SYS trampoline resume is outside a near jump")?;
    let mut assembler = Assembler::new();
    assembler
        .emit(Instruction::Lds {
            dest: Register16::SI,
            src: memory,
        })
        .emit(Instruction::Jmp {
            target: JmpTarget::Rel16(displacement),
        });
    Ok(assembler.assemble(origin)?)
}

pub(super) fn assemble_system_loader_sources(
    source: &[u8],
    runtime: &SystemLoaderRuntimeCatalog,
    output_resident_byte_count: u16,
    installer_offset: usize,
    installer: AssembledProgram,
    entries: &[CompiledSystemLoaderTextEntry],
    trampoline: AssembledProgram,
) -> Result<BTreeMap<String, AssembledProgram>> {
    let mut sources = BTreeMap::new();
    ensure_unique(
        &mut sources,
        RESIDENT_SIZE_SOURCE_ID.to_owned(),
        assemble_relocated_segment_mov_immediate(
            source,
            runtime.resident_size_load_offset,
            output_resident_byte_count,
            &[Register16::AX],
            "MEGDOS.SYS resident size",
        )?,
    )?;
    ensure_unique(
        &mut sources,
        RESIDENT_COPY_SOURCE_ID.to_owned(),
        assemble_relocated_segment_mov_immediate(
            source,
            runtime.resident_copy_count_load_offset,
            output_resident_byte_count,
            &[Register16::CX],
            "MEGDOS.SYS resident copy count",
        )?,
    )?;
    ensure_unique(
        &mut sources,
        POST_STACK_HOOK_SOURCE_ID.to_owned(),
        assemble_post_stack_hook(runtime, installer_offset)?,
    )?;
    ensure_unique(&mut sources, INSTALLER_SOURCE_ID.to_owned(), installer)?;
    ensure_unique(&mut sources, TRAMPOLINE_SOURCE_ID.to_owned(), trampoline)?;
    for reference in &runtime.references {
        let address = entries
            .iter()
            .find(|entry| entry.id == reference.entry_id)
            .with_context(|| format!("missing translated record {}", reference.entry_id))?
            .resident_offset;
        ensure_unique(
            &mut sources,
            reference_source_id(reference.instruction_offset),
            assemble_relocated_segment_mov_immediate(
                source,
                reference.instruction_offset,
                address,
                &[Register16::DX],
                &format!(
                    "MEGDOS.SYS text reference at {:#x}",
                    reference.instruction_offset
                ),
            )?,
        )?;
    }
    Ok(sources)
}

fn assemble_post_stack_hook(
    runtime: &SystemLoaderRuntimeCatalog,
    installer_offset: usize,
) -> Result<AssembledProgram> {
    let origin = segment_location(runtime.post_stack_hook_offset, "MEGDOS.SYS post-stack hook")?;
    let next = i32::from(origin.off) + 3;
    let displacement = i16::try_from(i32::try_from(installer_offset)? - next)
        .context("MEGDOS.SYS resident installer is outside a near jump")?;
    let mut assembler = Assembler::new();
    assembler
        .emit(Instruction::Jmp {
            target: JmpTarget::Rel16(displacement),
        })
        .emit(Instruction::Nop)
        .emit(Instruction::Nop);
    let assembled = assembler.assemble(origin)?;
    ensure!(
        assembled.bytes().len() == runtime.post_stack_hook_byte_count,
        "MEGDOS.SYS post-stack hook changed the displaced instruction width"
    );
    Ok(assembled)
}

fn ensure_unique(
    sources: &mut BTreeMap<String, AssembledProgram>,
    id: String,
    source: AssembledProgram,
) -> Result<()> {
    ensure!(
        sources.insert(id.clone(), source).is_none(),
        "duplicate typed V30 MEGDOS.SYS source {id}"
    );
    Ok(())
}

fn segment_location(offset: usize, role: &str) -> Result<CodeLocation> {
    Ok(CodeLocation {
        seg: 0,
        off: u16::try_from(offset).with_context(|| format!("{role} offset exceeds one segment"))?,
    })
}

#[cfg(test)]
#[path = "typed_sources_tests.rs"]
mod typed_sources_tests;
