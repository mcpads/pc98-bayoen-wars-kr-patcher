use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use v30::{Instruction, JmpTarget, Operand, Register16, decode_bytes};

use super::model::CompiledPlaybackDriver;
use super::typed_sources::{entry_source_id, installer_source_id, reference_source_id};
use crate::dos_program::{pack_self_expanding_com, unpack_self_expanding_com};
use crate::external_text::PackedSoundDriverReferenceKind;
use crate::korean_patch::gaiji_record::{GAIJI_RECORD_SIZE, GaijiRecord};

const COM_ORIGIN: usize = 0x100;

pub(super) fn verify_playback_driver_files(
    files: &BTreeMap<String, Vec<u8>>,
    compiled: &[CompiledPlaybackDriver],
) -> Result<()> {
    for driver in compiled {
        verify_driver(
            files
                .get(driver.file_name)
                .with_context(|| format!("patched files are missing {}", driver.file_name))?,
            driver,
        )?;
    }
    Ok(())
}

fn verify_driver(packed: &[u8], compiled: &CompiledPlaybackDriver) -> Result<()> {
    ensure!(
        packed == compiled.output_packed,
        "{} packed container failed final readback",
        compiled.file_name
    );
    let unpacked = unpack_self_expanding_com(packed)?.unpacked;
    ensure!(
        unpacked == compiled.output_unpacked,
        "{} unpacked image differs from the audited Expected Write result",
        compiled.file_name
    );
    ensure!(
        pack_self_expanding_com(&unpacked)? == packed,
        "{} final packed container is not the deterministic serialization",
        compiled.file_name
    );
    ensure!(
        compiled.installer_offset + COM_ORIGIN
            == usize::from(compiled.runtime.resident_end_com_address)
            && compiled.installer_offset == compiled.runtime.resident_end_file_offset,
        "{} installer does not begin at the first non-resident byte",
        compiled.file_name
    );

    let entry_id = entry_source_id(compiled.file_name);
    ensure_typed_source(
        &unpacked,
        compiled.runtime.entry_jump_offset,
        &entry_id,
        compiled,
    )?;
    let entry = decode_bytes(&unpacked[compiled.runtime.entry_jump_offset..])?;
    ensure!(
        relative_jump_target(
            compiled.runtime.entry_jump_offset + COM_ORIGIN,
            &entry.instruction,
            entry.byte_len,
        )? == i32::try_from(compiled.installer_offset + COM_ORIGIN)?,
        "{} entry hook misses its embedded installer",
        compiled.file_name
    );

    ensure!(
        unpacked.get(compiled.installer_offset..compiled.glyph_records_offset)
            == Some(compiled.installer.bytes()),
        "{} embedded installer failed readback",
        compiled.file_name
    );
    let installer_id = installer_source_id(compiled.file_name);
    ensure_typed_source(
        &unpacked,
        compiled.installer_offset,
        &installer_id,
        compiled,
    )?;
    let handoff_span = compiled
        .installer
        .instruction_spans()
        .last()
        .context("playback-driver installer has no typed instructions")?;
    ensure!(
        relative_jump_target(
            usize::from(handoff_span.location.off),
            &handoff_span.instruction,
            handoff_span.bytes.len().get(),
        )? == i32::from(compiled.runtime.transient_entry_com_address),
        "{} installer does not return to the original transient entry",
        compiled.file_name
    );

    let glyph_end = compiled.glyph_records_offset + compiled.glyph_records.len();
    ensure!(
        unpacked.get(compiled.glyph_records_offset..glyph_end)
            == Some(compiled.glyph_records.as_slice())
            && glyph_end == unpacked.len(),
        "{} embedded GAIJI record tail failed readback",
        compiled.file_name
    );
    ensure!(
        compiled
            .glyph_records
            .len()
            .is_multiple_of(GAIJI_RECORD_SIZE),
        "{} embedded GAIJI records changed size",
        compiled.file_name
    );
    for (index, record) in compiled
        .glyph_records
        .as_chunks::<GAIJI_RECORD_SIZE>()
        .0
        .iter()
        .enumerate()
    {
        GaijiRecord::parse(record)
            .with_context(|| format!("{} GAIJI record {index} failed parsing", compiled.file_name))?
            .require_target_format()?;
    }

    for storage in &compiled.text_storage_writes {
        ensure!(
            unpacked.get(storage.offset..storage.offset + storage.replacement.len())
                == Some(storage.replacement.as_slice()),
            "{} source text slot {} failed readback",
            compiled.file_name,
            storage.id
        );
    }
    for entry in &compiled.entries {
        ensure!(
            unpacked.get(entry.file_offset..entry.file_offset + entry.bytes.len())
                == Some(entry.bytes.as_slice())
                && entry.bytes.last() == Some(&b'$')
                && !entry.bytes[..entry.bytes.len() - 1].contains(&b'$'),
            "{} record {} failed DOS-boundary readback",
            compiled.file_name,
            entry.id
        );
    }
    for reference in &compiled.runtime.references {
        let entry = compiled
            .entries
            .iter()
            .find(|entry| entry.id == reference.entry_id)
            .with_context(|| format!("missing translated record {}", reference.entry_id))?;
        match reference.kind {
            PackedSoundDriverReferenceKind::MachineCode => {
                let offset = reference
                    .instruction_offset
                    .context("machine-code playback reference lost its instruction offset")?;
                let source_id = reference_source_id(compiled.file_name, offset);
                ensure_typed_source(&unpacked, offset, &source_id, compiled)?;
                let decoded = decode_bytes(&unpacked[offset..])?;
                ensure!(
                    matches!(
                        decoded.instruction,
                        Instruction::Mov {
                            dest: Operand::Reg16(Register16::DX),
                            src: Operand::Imm16(address),
                        } if address == entry.com_address
                    ),
                    "{} machine reference for {} failed typed readback",
                    compiled.file_name,
                    entry.id
                );
            }
            PackedSoundDriverReferenceKind::Metadata => ensure!(
                read_u16(&unpacked, reference.storage_offset)? == entry.com_address,
                "{} metadata reference for {} failed readback",
                compiled.file_name,
                entry.id
            ),
        }
    }
    Ok(())
}

fn ensure_typed_source(
    unpacked: &[u8],
    offset: usize,
    source_id: &str,
    compiled: &CompiledPlaybackDriver,
) -> Result<()> {
    let source = compiled
        .typed_sources
        .get(source_id)
        .with_context(|| format!("missing playback-driver typed source {source_id}"))?;
    ensure!(
        unpacked.get(offset..offset + source.bytes().len()) == Some(source.bytes()),
        "{} typed source {source_id} failed readback",
        compiled.file_name
    );
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
        _ => anyhow::bail!("playback-driver typed source is not a near jump"),
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        bytes
            .get(offset..offset + 2)
            .context("playback-driver metadata read lies outside the image")?
            .try_into()
            .expect("a two-byte range converts to an array"),
    ))
}
