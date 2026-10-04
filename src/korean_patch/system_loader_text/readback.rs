use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use v30::{Instruction, JmpTarget, Operand, Register16, decode_bytes};

use super::layout::reconstruct_resident_extension;
use super::model::CompiledSystemLoaderText;
use super::typed_sources::{
    INSTALLER_SOURCE_ID, POST_STACK_HOOK_SOURCE_ID, RESIDENT_COPY_SOURCE_ID,
    RESIDENT_SIZE_SOURCE_ID, TRAMPOLINE_SOURCE_ID, reference_source_id,
};
use crate::korean_patch::gaiji_record::{GAIJI_RECORD_SIZE, GaijiRecord};

const SYSTEM_LOADER_FILE: &str = "MEGDOS.SYS";

pub(super) fn verify_system_loader_text_files(
    files: &BTreeMap<String, Vec<u8>>,
    compiled: &CompiledSystemLoaderText,
) -> Result<()> {
    let output = files
        .get(SYSTEM_LOADER_FILE)
        .context("patched boot files are missing MEGDOS.SYS")?;
    ensure!(
        output == &compiled.output_bytes
            && output.len() == compiled.source_bytes.len() + compiled.resident_extension_byte_count,
        "MEGDOS.SYS final container failed exact readback"
    );
    let reconstructed = reconstruct_resident_extension(
        &compiled.source_bytes,
        compiled.runtime.tail_offset,
        usize::from(compiled.output_resident_byte_count),
    )?;
    ensure!(
        reconstructed == compiled.reconstructed_source,
        "MEGDOS.SYS reconstructed baseline is not repeatable"
    );
    ensure!(
        output[usize::from(compiled.output_resident_byte_count)..]
            == compiled.source_bytes[compiled.runtime.tail_offset..],
        "MEGDOS.SYS shifted loader tail changed"
    );

    for (offset, id, expected) in [
        (
            compiled.runtime.resident_size_load_offset,
            RESIDENT_SIZE_SOURCE_ID,
            compiled.output_resident_byte_count,
        ),
        (
            compiled.runtime.resident_copy_count_load_offset,
            RESIDENT_COPY_SOURCE_ID,
            compiled.output_resident_byte_count,
        ),
    ] {
        ensure_typed_source(output, offset, id, compiled)?;
        let decoded = decode_bytes(&output[offset..])?;
        ensure!(
            matches!(
                decoded.instruction,
                Instruction::Mov {
                    src: Operand::Imm16(value),
                    ..
                } if value == expected
            ),
            "MEGDOS.SYS {id} failed typed immediate readback"
        );
    }

    ensure_typed_source(
        output,
        compiled.runtime.post_stack_hook_offset,
        POST_STACK_HOOK_SOURCE_ID,
        compiled,
    )?;
    let hook = decode_bytes(&output[compiled.runtime.post_stack_hook_offset..])?;
    ensure!(
        relative_jump_target(
            compiled.runtime.post_stack_hook_offset,
            &hook.instruction,
            hook.byte_len,
        )? == i32::try_from(compiled.installer_offset)?,
        "MEGDOS.SYS post-stack hook misses the resident installer"
    );

    ensure_typed_source(
        output,
        compiled.installer_offset,
        INSTALLER_SOURCE_ID,
        compiled,
    )?;
    let installer_handoff = compiled
        .installer
        .instruction_spans()
        .last()
        .context("MEGDOS.SYS installer has no typed instructions")?;
    ensure!(
        relative_jump_target(
            usize::from(installer_handoff.location.off),
            &installer_handoff.instruction,
            installer_handoff.bytes.len().get(),
        )? == i32::try_from(compiled.trampoline_offset)?,
        "MEGDOS.SYS installer misses the displaced-instruction trampoline"
    );

    let glyph_end = compiled.glyph_records_offset + compiled.glyph_records.len();
    ensure!(
        output.get(compiled.glyph_records_offset..glyph_end)
            == Some(compiled.glyph_records.as_slice())
            && compiled
                .glyph_records
                .len()
                .is_multiple_of(GAIJI_RECORD_SIZE),
        "MEGDOS.SYS resident GAIJI records failed readback"
    );
    for (index, record) in compiled
        .glyph_records
        .as_chunks::<GAIJI_RECORD_SIZE>()
        .0
        .iter()
        .enumerate()
    {
        GaijiRecord::parse(record)
            .with_context(|| format!("MEGDOS.SYS GAIJI record {index} failed parsing"))?
            .require_target_format()?;
    }

    for entry in &compiled.entries {
        let offset = usize::from(entry.resident_offset);
        ensure!(
            output.get(offset..offset + entry.bytes.len()) == Some(entry.bytes.as_slice())
                && entry.bytes.last() == Some(&0)
                && !entry.bytes[..entry.bytes.len() - 1].contains(&0),
            "MEGDOS.SYS record {} failed null-boundary readback",
            entry.id
        );
    }
    for reference in &compiled.runtime.references {
        let entry = compiled
            .entries
            .iter()
            .find(|entry| entry.id == reference.entry_id)
            .with_context(|| format!("missing translated record {}", reference.entry_id))?;
        let source_id = reference_source_id(reference.instruction_offset);
        ensure_typed_source(output, reference.instruction_offset, &source_id, compiled)?;
        let decoded = decode_bytes(&output[reference.instruction_offset..])?;
        ensure!(
            matches!(
                decoded.instruction,
                Instruction::Mov {
                    dest: Operand::Reg16(Register16::DX),
                    src: Operand::Imm16(address),
                } if address == entry.resident_offset
            ),
            "MEGDOS.SYS reference for {} failed typed readback",
            entry.id
        );
    }

    ensure_typed_source(
        output,
        compiled.trampoline_offset,
        TRAMPOLINE_SOURCE_ID,
        compiled,
    )?;
    let original_lds = &compiled.source_bytes[compiled.runtime.post_stack_hook_offset
        ..compiled.runtime.post_stack_hook_offset + compiled.runtime.post_stack_hook_byte_count];
    ensure!(
        compiled.trampoline.bytes().starts_with(original_lds),
        "MEGDOS.SYS trampoline did not preserve the displaced LDS bytes"
    );
    let trampoline_handoff = compiled
        .trampoline
        .instruction_spans()
        .last()
        .context("MEGDOS.SYS trampoline has no typed instructions")?;
    ensure!(
        relative_jump_target(
            usize::from(trampoline_handoff.location.off),
            &trampoline_handoff.instruction,
            trampoline_handoff.bytes.len().get(),
        )? == i32::from(compiled.runtime.post_stack_resume_address),
        "MEGDOS.SYS trampoline misses the original resume address"
    );
    let padding = &output[compiled.trampoline_offset + compiled.trampoline.bytes().len()
        ..usize::from(compiled.output_resident_byte_count)];
    ensure!(
        padding.iter().all(|byte| *byte == 0),
        "MEGDOS.SYS resident paragraph padding is not zero"
    );
    Ok(())
}

fn ensure_typed_source(
    output: &[u8],
    offset: usize,
    source_id: &str,
    compiled: &CompiledSystemLoaderText,
) -> Result<()> {
    let source = compiled
        .typed_sources
        .get(source_id)
        .with_context(|| format!("missing typed V30 MEGDOS.SYS source {source_id}"))?;
    ensure!(
        output.get(offset..offset + source.bytes().len()) == Some(source.bytes()),
        "MEGDOS.SYS typed source {source_id} failed readback"
    );
    Ok(())
}

fn relative_jump_target(
    instruction_offset: usize,
    instruction: &Instruction,
    byte_len: usize,
) -> Result<i32> {
    match instruction {
        Instruction::Jmp {
            target: JmpTarget::Rel16(displacement),
        } => Ok(i32::try_from(instruction_offset + byte_len)? + i32::from(*displacement)),
        _ => anyhow::bail!("MEGDOS.SYS typed source is not a near jump"),
    }
}
