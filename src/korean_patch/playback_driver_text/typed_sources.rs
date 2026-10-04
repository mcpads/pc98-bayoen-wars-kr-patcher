use std::collections::BTreeMap;

use anyhow::{Context, Result};
use v30::{AssembledProgram, Assembler, CodeLocation, Instruction, JmpTarget, Register16};

use super::model::CompiledPlaybackDriverTextEntry;
use crate::external_text::{PackedSoundDriverReferenceKind, PackedSoundDriverRuntimeCatalog};
use crate::korean_patch::typed_v30::assemble_relocated_mov_immediate;

const COM_ORIGIN: usize = 0x100;

pub(super) fn entry_source_id(file_name: &str) -> String {
    format!("{}-hangul-entry-jump", source_stem(file_name))
}

pub(super) fn installer_source_id(file_name: &str) -> String {
    format!("{}-embedded-gaiji-installer", source_stem(file_name))
}

pub(super) fn reference_source_id(file_name: &str, instruction_offset: usize) -> String {
    format!(
        "{}-relocated-text-{instruction_offset:04x}",
        source_stem(file_name)
    )
}

pub(super) fn assemble_playback_driver_sources(
    source: &[u8],
    runtime: &PackedSoundDriverRuntimeCatalog,
    installer_offset: usize,
    installer: AssembledProgram,
    entries: &[CompiledPlaybackDriverTextEntry],
) -> Result<BTreeMap<String, AssembledProgram>> {
    let mut sources = BTreeMap::new();
    sources.insert(
        entry_source_id(&runtime.file_name),
        assemble_entry_jump(runtime.entry_jump_offset, installer_offset)?,
    );
    sources.insert(installer_source_id(&runtime.file_name), installer);
    for reference in runtime
        .references
        .iter()
        .filter(|reference| reference.kind == PackedSoundDriverReferenceKind::MachineCode)
    {
        let instruction_offset = reference
            .instruction_offset
            .context("machine-code playback reference lost its instruction offset")?;
        let address = entries
            .iter()
            .find(|entry| entry.id == reference.entry_id)
            .with_context(|| format!("missing translated record {}", reference.entry_id))?
            .com_address;
        let source_id = reference_source_id(&runtime.file_name, instruction_offset);
        let assembled = assemble_relocated_mov_immediate(
            source,
            instruction_offset,
            address,
            &[Register16::DX],
            &format!(
                "{} text reference at {instruction_offset:#x}",
                runtime.file_name
            ),
        )?;
        ensure_unique(&mut sources, source_id, assembled)?;
    }
    Ok(sources)
}

fn assemble_entry_jump(entry_offset: usize, installer_offset: usize) -> Result<AssembledProgram> {
    let origin = com_location(entry_offset, "playback-driver entry jump")?;
    let next = i32::from(origin.off) + 3;
    let target = i32::from(com_address(installer_offset, "playback-driver installer")?);
    let displacement = i16::try_from(target - next)
        .context("playback-driver entry hook is outside a near jump")?;
    let mut assembler = Assembler::new();
    assembler.emit(Instruction::Jmp {
        target: JmpTarget::Rel16(displacement),
    });
    Ok(assembler.assemble(origin)?)
}

fn ensure_unique(
    sources: &mut BTreeMap<String, AssembledProgram>,
    id: String,
    source: AssembledProgram,
) -> Result<()> {
    anyhow::ensure!(
        sources.insert(id.clone(), source).is_none(),
        "duplicate typed V30 source {id}"
    );
    Ok(())
}

fn source_stem(file_name: &str) -> String {
    file_name
        .split_once('.')
        .map_or(file_name, |(stem, _)| stem)
        .to_ascii_lowercase()
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
