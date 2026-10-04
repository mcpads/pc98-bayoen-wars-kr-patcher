use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use v30::{
    CallTarget, EffectiveAddressBase, EffectiveAddressDisplacement, Instruction, JmpTarget,
    Operand, Register16, decode_bytes,
};

use super::model::{CompiledMenuText, CompiledMenuTextEntry};
use super::typed_sources::{
    ENTRY_SOURCE_ID, INSTALLER_SOURCE_ID, TRAMPOLINE_SOURCE_ID, reference_source_id,
    runtime_insert_source_id,
};
use crate::game_data::{MenuRuntimeCatalog, MenuTextRuntimeReferenceKind};
use crate::korean_patch::gaiji_record::{GAIJI_RECORD_SIZE, GaijiRecord};

const MENU_FILE: &str = "MENU.COM";
const COM_ORIGIN: usize = 0x100;

pub(super) fn verify_menu_text_files(
    files: &BTreeMap<String, Vec<u8>>,
    runtime: &MenuRuntimeCatalog,
    compiled: &CompiledMenuText,
) -> Result<()> {
    let menu = files
        .get(MENU_FILE)
        .context("MENU text files are missing MENU.COM")?;
    ensure!(
        menu.len() == compiled.output_file_size,
        "resized MENU.COM length failed readback"
    );
    ensure_typed_source(
        menu,
        runtime.entry.hook_instruction_offset,
        ENTRY_SOURCE_ID,
        compiled,
    )?;
    let entry = decode_bytes(&menu[runtime.entry.hook_instruction_offset..])?;
    ensure!(
        relative_jump_target(
            runtime.entry.hook_com_address,
            &entry.instruction,
            entry.byte_len,
        )? == i32::try_from(compiled.installer_offset + COM_ORIGIN)?,
        "MENU entry hook misses its embedded installer"
    );

    ensure!(
        menu.get(compiled.installer_offset..compiled.glyph_records_offset)
            == Some(compiled.installer.bytes()),
        "MENU embedded GAIJI installer failed readback"
    );
    ensure_typed_source(
        menu,
        compiled.installer_offset,
        INSTALLER_SOURCE_ID,
        compiled,
    )?;
    let installer_handoff = compiled
        .installer
        .instruction_spans()
        .last()
        .context("MENU installer has no typed instructions")?;
    ensure!(
        relative_jump_target(
            installer_handoff.location.off,
            &installer_handoff.instruction,
            installer_handoff.bytes.len().get(),
        )? == i32::try_from(compiled.trampoline_offset + COM_ORIGIN)?,
        "MENU installer misses its original-entry trampoline"
    );

    ensure!(
        menu.get(compiled.glyph_records_offset..compiled.trampoline_offset)
            == Some(compiled.glyph_records.as_slice()),
        "MENU embedded GAIJI records failed readback"
    );
    ensure!(
        compiled
            .glyph_records
            .len()
            .is_multiple_of(GAIJI_RECORD_SIZE),
        "MENU embedded GAIJI record size changed"
    );
    for (index, record) in compiled
        .glyph_records
        .as_chunks::<GAIJI_RECORD_SIZE>()
        .0
        .iter()
        .enumerate()
    {
        GaijiRecord::parse(record)
            .with_context(|| format!("MENU GAIJI record {index} failed parsing"))?
            .require_target_format()?;
    }

    ensure_typed_source(
        menu,
        compiled.trampoline_offset,
        TRAMPOLINE_SOURCE_ID,
        compiled,
    )?;
    verify_trampoline(runtime, compiled)?;
    ensure!(
        menu.get(compiled.text_offset..compiled.output_file_size)
            == Some(compiled.packed_text.as_slice()),
        "MENU relocated text block failed readback"
    );
    for entry in &compiled.entries {
        verify_record(menu, entry)?;
    }

    for reference in &runtime.references {
        let entry = compiled_entry(compiled, &reference.target_entry_id)?;
        match reference.storage_kind {
            MenuTextRuntimeReferenceKind::MachineCodeImmediate => {
                let instruction_offset = reference
                    .instruction_offset
                    .context("MENU machine-code reference lost its instruction offset")?;
                let source_id = reference_source_id(instruction_offset);
                ensure_typed_source(menu, instruction_offset, &source_id, compiled)?;
                let decoded = decode_bytes(&menu[instruction_offset..])?;
                ensure!(
                    matches!(
                        decoded.instruction,
                        Instruction::Mov {
                            dest: Operand::Reg16(Register16::DX | Register16::SI),
                            src: Operand::Imm16(address),
                        } if address == entry.com_address
                    ),
                    "MENU machine reference for {} failed typed readback",
                    entry.id
                );
            }
            MenuTextRuntimeReferenceKind::MetadataTableEntry => ensure!(
                read_u16(menu, reference.storage_offset)? == entry.com_address,
                "MENU metadata reference for {} failed readback",
                entry.id
            ),
        }
    }
    for reference in &runtime.runtime_insert_references {
        let entry = compiled_entry(compiled, &reference.target_entry_id)?;
        let field_offset = *entry
            .runtime_field_offsets
            .get(&reference.id)
            .with_context(|| format!("{} runtime field failed record readback", reference.id))?;
        let target = u16::try_from(usize::from(entry.com_address) + field_offset)
            .context("MENU runtime field readback address exceeds 16 bits")?;
        let source_id = runtime_insert_source_id(&reference.id);
        ensure_typed_source(menu, reference.instruction_offset, &source_id, compiled)?;
        let decoded = decode_bytes(&menu[reference.instruction_offset..])?;
        let memory = match decoded.instruction {
            Instruction::Mov {
                dest: Operand::Mem(memory),
                src: Operand::Reg16(Register16::AX),
            } => memory,
            _ => anyhow::bail!("{} failed typed direct-store readback", reference.id),
        };
        ensure!(
            memory.base() == EffectiveAddressBase::Direct
                && memory.displacement() == EffectiveAddressDisplacement::Absolute(target),
            "{} misses its relocated runtime field",
            reference.id
        );
    }
    Ok(())
}

fn verify_trampoline(runtime: &MenuRuntimeCatalog, compiled: &CompiledMenuText) -> Result<()> {
    let spans = compiled.trampoline.instruction_spans();
    ensure!(
        spans.len() == 2,
        "MENU entry trampoline instruction count changed"
    );
    let call_target = match spans[0].instruction {
        Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } => {
            i32::from(spans[0].location.off)
                + i32::try_from(spans[0].bytes.len().get())?
                + i32::from(displacement)
        }
        _ => anyhow::bail!("MENU entry trampoline lost its original CALL"),
    };
    ensure!(
        call_target == i32::from(runtime.entry.original_call_target_com_address),
        "MENU entry trampoline calls the wrong original target"
    );
    ensure!(
        relative_jump_target(
            spans[1].location.off,
            &spans[1].instruction,
            spans[1].bytes.len().get(),
        )? == i32::from(runtime.entry.resume_com_address),
        "MENU entry trampoline resumes at the wrong original instruction"
    );
    Ok(())
}

fn verify_record(menu: &[u8], entry: &CompiledMenuTextEntry) -> Result<()> {
    ensure!(
        menu.get(entry.file_offset..entry.file_offset + entry.bytes.len())
            == Some(entry.bytes.as_slice()),
        "{} relocated bytes failed readback",
        entry.id
    );
    ensure!(
        matches!(entry.bytes.last(), Some(b'$' | b'@')),
        "{} lost its source terminator family",
        entry.id
    );
    for (field_id, &offset) in &entry.runtime_field_offsets {
        let expected = match field_id.as_str() {
            "menu-floppy-position-field" => [0x00, 0x00],
            "menu-floppy-drive-label-field" => *b"??",
            _ => anyhow::bail!("{} has an unknown runtime field {field_id}", entry.id),
        };
        ensure!(
            entry.bytes.get(offset..offset + 2) == Some(expected.as_slice()),
            "{field_id} content failed readback"
        );
    }
    Ok(())
}

fn ensure_typed_source(
    menu: &[u8],
    offset: usize,
    source_id: &str,
    compiled: &CompiledMenuText,
) -> Result<()> {
    let source = compiled
        .typed_sources
        .get(source_id)
        .with_context(|| format!("missing typed V30 MENU source {source_id}"))?;
    ensure!(
        menu.get(offset..offset + source.bytes().len()) == Some(source.bytes()),
        "MENU typed source {source_id} failed readback"
    );
    Ok(())
}

fn relative_jump_target(
    instruction_com_address: u16,
    instruction: &Instruction,
    byte_len: usize,
) -> Result<i32> {
    match instruction {
        Instruction::Jmp {
            target: JmpTarget::Rel16(displacement),
        } => Ok(i32::from(instruction_com_address)
            + i32::try_from(byte_len)?
            + i32::from(*displacement)),
        _ => anyhow::bail!("MENU typed source is not a near jump"),
    }
}

fn compiled_entry<'a>(
    compiled: &'a CompiledMenuText,
    id: &str,
) -> Result<&'a CompiledMenuTextEntry> {
    compiled
        .entries
        .iter()
        .find(|entry| entry.id == id)
        .with_context(|| format!("missing compiled MENU entry {id}"))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        bytes
            .get(offset..offset + 2)
            .context("MENU metadata read lies outside the image")?
            .try_into()
            .expect("a two-byte range converts to an array"),
    ))
}
