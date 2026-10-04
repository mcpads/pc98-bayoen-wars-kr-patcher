use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use v30::{Instruction, JmpTarget, Operand, Register16, decode_bytes};

use super::model::CompiledSamplingDriverText;
use super::typed_sources::{ENTRY_SOURCE_ID, INSTALLER_SOURCE_ID, reference_source_id};
use crate::korean_patch::gaiji_record::{GAIJI_RECORD_SIZE, GaijiRecord};

const BSAMP_FILE: &str = "BSAMP.COM";
const COM_ORIGIN: usize = 0x100;

pub(super) fn verify_sampling_driver_text_files(
    files: &BTreeMap<String, Vec<u8>>,
    compiled: &CompiledSamplingDriverText,
) -> Result<()> {
    let bsamp = files
        .get(BSAMP_FILE)
        .context("sampling-driver text files are missing BSAMP.COM")?;
    ensure!(
        bsamp.len() == compiled.output_file_size,
        "resized BSAMP.COM length failed readback"
    );
    ensure!(
        decode_bytes(bsamp)?.instruction == Instruction::Cli,
        "sampling-driver entry lost its original CLI"
    );
    ensure_typed_source(
        bsamp,
        compiled.lifetime.entry_jump_offset,
        ENTRY_SOURCE_ID,
        compiled,
    )?;
    let entry_jump = decode_bytes(&bsamp[compiled.lifetime.entry_jump_offset..])?;
    let entry_target = relative_jump_target(
        compiled.lifetime.entry_jump_offset + COM_ORIGIN,
        &entry_jump.instruction,
        entry_jump.byte_len,
    )?;
    ensure!(
        entry_target == i32::try_from(compiled.installer_offset + COM_ORIGIN)?,
        "sampling-driver entry hook does not target the embedded installer"
    );

    ensure!(
        bsamp.get(compiled.installer_offset..compiled.glyph_records_offset)
            == Some(compiled.installer.bytes()),
        "embedded sampling-driver GAIJI installer failed readback"
    );
    ensure_typed_source(
        bsamp,
        compiled.installer_offset,
        INSTALLER_SOURCE_ID,
        compiled,
    )?;
    let final_span = compiled
        .installer
        .instruction_spans()
        .last()
        .context("embedded sampling-driver installer has no typed instructions")?;
    let handoff = relative_jump_target(
        usize::from(final_span.location.off),
        &final_span.instruction,
        final_span.bytes.len().get(),
    )?;
    ensure!(
        handoff == i32::from(compiled.lifetime.transient_entry_com_address),
        "embedded sampling-driver installer does not return to transient initialization"
    );

    ensure!(
        bsamp.get(compiled.glyph_records_offset..compiled.text_offset)
            == Some(compiled.glyph_records.as_slice()),
        "embedded sampling-driver GAIJI records failed readback"
    );
    for (index, record) in compiled
        .glyph_records
        .as_chunks::<GAIJI_RECORD_SIZE>()
        .0
        .iter()
        .enumerate()
    {
        let parsed = GaijiRecord::parse(record)
            .with_context(|| format!("sampling-driver GAIJI record {index} failed parsing"))?;
        parsed.require_target_format()?;
    }
    ensure!(
        bsamp.get(compiled.text_offset..compiled.output_file_size)
            == Some(compiled.packed_text.as_slice()),
        "relocated sampling-driver text block failed readback"
    );

    let resident_end = decode_bytes(&bsamp[compiled.lifetime.resident_end_load_offset..])?;
    ensure!(
        matches!(
            resident_end.instruction,
            Instruction::Mov {
                dest: Operand::Reg16(Register16::DX),
                src: Operand::Imm16(address),
            } if address == compiled.lifetime.resident_end_com_address
        ),
        "sampling-driver resident boundary changed after patching"
    );
    ensure!(
        usize::from(compiled.lifetime.resident_end_com_address)
            < compiled.installer_offset + COM_ORIGIN,
        "embedded sampling-driver data entered the resident image"
    );

    for entry in &compiled.entries {
        ensure!(
            bsamp.get(entry.file_offset..entry.file_offset + entry.bytes.len())
                == Some(entry.bytes.as_slice()),
            "{} relocated bytes failed readback",
            entry.id
        );
        ensure!(
            entry.bytes.last() == Some(&b'$')
                && !entry.bytes[..entry.bytes.len() - 1].contains(&b'$'),
            "{} changed its DOS string boundary",
            entry.id
        );
        for &consumer_offset in &entry.consumer_offsets {
            let source_id = reference_source_id(consumer_offset);
            ensure_typed_source(bsamp, consumer_offset, &source_id, compiled)?;
            let decoded = decode_bytes(&bsamp[consumer_offset..])?;
            ensure!(
                matches!(
                    decoded.instruction,
                    Instruction::Mov {
                        dest: Operand::Reg16(Register16::DX),
                        src: Operand::Imm16(address),
                    } if address == entry.com_address
                ),
                "{} DOS output reference failed typed readback",
                entry.id
            );
        }
    }
    Ok(())
}

fn relative_jump_target(
    instruction_com_address: usize,
    instruction: &Instruction,
    byte_len: usize,
) -> Result<i32> {
    match instruction {
        Instruction::Jmp {
            target: JmpTarget::Rel16(displacement),
        } => Ok(i32::try_from(instruction_com_address + byte_len)? + i32::from(*displacement)),
        _ => anyhow::bail!("sampling-driver typed source is not a near jump"),
    }
}

fn ensure_typed_source(
    bsamp: &[u8],
    offset: usize,
    source_id: &str,
    compiled: &CompiledSamplingDriverText,
) -> Result<()> {
    let source = compiled
        .typed_sources
        .get(source_id)
        .with_context(|| format!("missing sampling-driver typed source {source_id}"))?;
    ensure!(
        bsamp.get(offset..offset + source.bytes().len()) == Some(source.bytes()),
        "sampling-driver typed source {source_id} failed readback"
    );
    Ok(())
}
