use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use v30::{Instruction, JmpTarget, Operand, Register16, decode_bytes};

use super::model::CompiledShellText;
use super::typed_sources::{
    ENTRY_SOURCE_ID, INSTALLER_SOURCE_ID, MEMORY_END_SOURCE_ID, ORIGINAL_ENTRY_COM_ADDRESS,
    reference_source_id,
};
use crate::korean_patch::gaiji_record::{GAIJI_RECORD_SIZE, GaijiRecord};

const DSH_FILE: &str = "DSH.COM";
const COM_ORIGIN: usize = 0x100;

pub(super) fn verify_shell_text_files(
    files: &BTreeMap<String, Vec<u8>>,
    compiled: &CompiledShellText,
) -> Result<()> {
    let dsh = files
        .get(DSH_FILE)
        .context("shell-text files are missing DSH.COM")?;
    ensure!(
        dsh.len() == compiled.output_file_size,
        "resized DSH.COM length failed readback"
    );
    ensure_typed_source(dsh, 0, ENTRY_SOURCE_ID, compiled)?;
    ensure_typed_source(
        dsh,
        super::typed_sources::MEMORY_END_INSTRUCTION_OFFSET,
        MEMORY_END_SOURCE_ID,
        compiled,
    )?;
    ensure!(
        dsh.get(compiled.installer_offset..compiled.glyph_records_offset)
            == Some(compiled.installer.bytes()),
        "embedded DSH GAIJI installer failed readback"
    );
    ensure_typed_source(
        dsh,
        compiled.installer_offset,
        INSTALLER_SOURCE_ID,
        compiled,
    )?;
    ensure!(
        dsh.get(compiled.glyph_records_offset..compiled.text_offset)
            == Some(compiled.glyph_records.as_slice()),
        "embedded DSH GAIJI records failed readback"
    );
    for (index, record) in compiled
        .glyph_records
        .as_chunks::<GAIJI_RECORD_SIZE>()
        .0
        .iter()
        .enumerate()
    {
        let parsed = GaijiRecord::parse(record)
            .with_context(|| format!("DSH GAIJI record {index} failed parsing"))?;
        parsed.require_target_format()?;
    }
    ensure!(
        dsh.get(compiled.text_offset..compiled.output_file_size)
            == Some(compiled.packed_text.as_slice()),
        "relocated DSH text block failed readback"
    );

    let entry_jump = decode_bytes(dsh)?;
    let entry_target = match entry_jump.instruction {
        Instruction::Jmp {
            target: JmpTarget::Rel16(displacement),
        } => 0x0103_i32 + i32::from(displacement),
        _ => anyhow::bail!("DSH entry hook failed typed V30 readback"),
    };
    ensure!(
        entry_target == i32::try_from(compiled.installer_offset + COM_ORIGIN)?,
        "DSH entry hook does not target the embedded installer"
    );
    let final_span = compiled
        .installer
        .instruction_spans()
        .last()
        .context("embedded DSH installer has no typed instructions")?;
    let handoff = match final_span.instruction {
        Instruction::Jmp {
            target: JmpTarget::Rel16(displacement),
        } => {
            i32::from(final_span.location.off)
                + i32::try_from(final_span.bytes.len().get())?
                + i32::from(displacement)
        }
        _ => anyhow::bail!("embedded DSH installer has no final near jump"),
    };
    ensure!(
        handoff == i32::from(ORIGINAL_ENTRY_COM_ADDRESS),
        "embedded DSH installer does not return to the original entry"
    );

    for entry in &compiled.entries {
        ensure!(
            dsh.get(entry.file_offset..entry.file_offset + entry.bytes.len())
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
            ensure_typed_source(dsh, consumer_offset, &source_id, compiled)?;
            let decoded = decode_bytes(&dsh[consumer_offset..])?;
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

fn ensure_typed_source(
    dsh: &[u8],
    offset: usize,
    source_id: &str,
    compiled: &CompiledShellText,
) -> Result<()> {
    let source = compiled
        .typed_sources
        .get(source_id)
        .with_context(|| format!("missing DSH typed source {source_id}"))?;
    ensure!(
        dsh.get(offset..offset + source.bytes().len()) == Some(source.bytes()),
        "DSH typed source {source_id} failed readback"
    );
    Ok(())
}
