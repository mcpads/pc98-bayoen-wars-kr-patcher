use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use v30::{Instruction, JmpTarget, Operand, Register16, decode_bytes};

use super::model::CompiledMouseDriverText;
use super::typed_sources::{ENTRY_SOURCE_ID, INSTALLER_SOURCE_ID, reference_source_id};
use crate::korean_patch::gaiji_record::{GAIJI_RECORD_SIZE, GaijiRecord};

const NMOUSE_FILE: &str = "NMOUSE.COM";
const COM_ORIGIN: usize = 0x100;

pub(super) fn verify_mouse_driver_text_files(
    files: &BTreeMap<String, Vec<u8>>,
    compiled: &CompiledMouseDriverText,
) -> Result<()> {
    let nmouse = files
        .get(NMOUSE_FILE)
        .context("mouse-driver text files are missing NMOUSE.COM")?;
    ensure!(
        nmouse.len() == compiled.output_file_size,
        "resized NMOUSE.COM length failed readback"
    );
    ensure!(
        decode_bytes(nmouse)?.instruction == Instruction::Cli,
        "mouse-driver entry lost its original CLI"
    );
    ensure_typed_source(
        nmouse,
        compiled.runtime.entry_jump_offset,
        ENTRY_SOURCE_ID,
        compiled,
    )?;
    let entry_jump = decode_bytes(&nmouse[compiled.runtime.entry_jump_offset..])?;
    let entry_target = relative_jump_target(
        compiled.runtime.entry_jump_offset + COM_ORIGIN,
        &entry_jump.instruction,
        entry_jump.byte_len,
    )?;
    ensure!(
        entry_target == i32::try_from(compiled.installer_offset + COM_ORIGIN)?,
        "mouse-driver entry hook does not target the embedded installer"
    );

    ensure!(
        nmouse.get(compiled.installer_offset..compiled.glyph_records_offset)
            == Some(compiled.installer.bytes()),
        "embedded mouse-driver GAIJI installer failed readback"
    );
    ensure_typed_source(
        nmouse,
        compiled.installer_offset,
        INSTALLER_SOURCE_ID,
        compiled,
    )?;
    let final_span = compiled
        .installer
        .instruction_spans()
        .last()
        .context("embedded mouse-driver installer has no typed instructions")?;
    let handoff = relative_jump_target(
        usize::from(final_span.location.off),
        &final_span.instruction,
        final_span.bytes.len().get(),
    )?;
    ensure!(
        handoff == i32::from(compiled.runtime.transient_entry_com_address),
        "embedded mouse-driver installer does not return to transient initialization"
    );

    ensure!(
        nmouse.get(compiled.glyph_records_offset..compiled.text_offset)
            == Some(compiled.glyph_records.as_slice()),
        "embedded mouse-driver GAIJI records failed readback"
    );
    for (index, record) in compiled
        .glyph_records
        .as_chunks::<GAIJI_RECORD_SIZE>()
        .0
        .iter()
        .enumerate()
    {
        let parsed = GaijiRecord::parse(record)
            .with_context(|| format!("mouse-driver GAIJI record {index} failed parsing"))?;
        parsed.require_target_format()?;
    }
    ensure!(
        nmouse.get(compiled.text_offset..compiled.output_file_size)
            == Some(compiled.packed_text.as_slice()),
        "relocated mouse-driver text block failed readback"
    );

    let resident_paragraphs =
        decode_bytes(&nmouse[compiled.runtime.resident_paragraph_load_offset..])?;
    ensure!(
        matches!(
            resident_paragraphs.instruction,
            Instruction::Mov {
                dest: Operand::Reg16(Register16::DX),
                src: Operand::Imm16(count),
            } if count == compiled.runtime.resident_paragraph_count
        ),
        "mouse-driver resident paragraph count changed after patching"
    );
    ensure!(
        compiled.installer_offset + COM_ORIGIN
            > usize::from(compiled.runtime.resident_paragraph_count) * 16,
        "embedded mouse-driver data entered the resident image"
    );

    for table in &compiled.pointer_tables {
        ensure!(
            nmouse.get(table.offset..table.offset + table.replacement.len())
                == Some(table.replacement.as_slice()),
            "{} failed metadata readback",
            table.id
        );
        let addresses = table
            .replacement
            .as_chunks::<2>()
            .0
            .iter()
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>();
        ensure!(
            addresses == table.addresses,
            "{} address population failed readback",
            table.id
        );
    }

    for entry in &compiled.entries {
        ensure!(
            nmouse.get(entry.file_offset..entry.file_offset + entry.bytes.len())
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
        if entry.table_index.is_none() {
            let source_id = reference_source_id(entry.consumer_offset);
            ensure_typed_source(nmouse, entry.consumer_offset, &source_id, compiled)?;
            let decoded = decode_bytes(&nmouse[entry.consumer_offset..])?;
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
        _ => anyhow::bail!("mouse-driver typed source is not a near jump"),
    }
}

fn ensure_typed_source(
    nmouse: &[u8],
    offset: usize,
    source_id: &str,
    compiled: &CompiledMouseDriverText,
) -> Result<()> {
    let source = compiled
        .typed_sources
        .get(source_id)
        .with_context(|| format!("missing mouse-driver typed source {source_id}"))?;
    ensure!(
        nmouse.get(offset..offset + source.bytes().len()) == Some(source.bytes()),
        "mouse-driver typed source {source_id} failed readback"
    );
    Ok(())
}
