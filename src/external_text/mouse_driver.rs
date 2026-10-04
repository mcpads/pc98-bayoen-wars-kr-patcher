use anyhow::{Context, Result, ensure};
use v30::{Instruction, JmpTarget, Operand, Register16, decode_bytes};

use super::catalog::{ExternalProgramTextCatalog, ExternalTextStorage};
use super::program_text::ProgramTextBuilder;

#[path = "mouse_driver/command_routes.rs"]
mod command_routes;

use command_routes::verify_mouse_driver_command_routes;

const FREQUENCY_POINTER_TABLE_OFFSET: usize = 0x0a6e;
const FREQUENCY_POINTER_COUNT: usize = 4;
const COMMAND_MODE_POINTER_TABLE_OFFSET: usize = 0x0a76;
const COMMAND_MODE_POINTER_COUNT: usize = 2;

const ENTRY_JUMP_OFFSET: usize = 0x0001;
const TRANSIENT_ENTRY_COM_ADDRESS: u16 = 0x094e;
const RESIDENT_PARAGRAPH_LOAD_OFFSET: usize = 0x096c;
const RESIDENT_TERMINATION_SUFFIX: &[u8] = &[0xb8, 0x00, 0x31, 0xcd, 0x21];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct MouseDriverRuntimeCatalog {
    pub entry_jump_offset: usize,
    pub transient_entry_com_address: u16,
    pub resident_paragraph_load_offset: usize,
    pub resident_paragraph_count: u16,
    pub frequency_pointer_table_offset: usize,
    pub frequency_pointer_count: usize,
    pub command_mode_pointer_table_offset: usize,
    pub command_mode_pointer_count: usize,
}

pub(crate) fn catalog_mouse_driver(bytes: &[u8]) -> Result<ExternalProgramTextCatalog> {
    catalog_mouse_driver_runtime(bytes)?;
    verify_mouse_driver_command_routes(bytes)?;
    ensure!(
        bytes.get(0x090f..0x0943)
            == Some(
                &[
                    0xba, 0x7a, 0x0b, 0xb4, 0x09, 0xcd, 0x21, 0x8a, 0x1e, 0x30, 0x01, 0xb7, 0x00,
                    0xd1, 0xe3, 0x81, 0xc3, 0x6e, 0x0b, 0x2e, 0x8b, 0x17, 0xb4, 0x09, 0xcd, 0x21,
                    0xba, 0x6e, 0x0c, 0xb4, 0x09, 0xcd, 0x21, 0x8a, 0x1e, 0x31, 0x01, 0xb7, 0x00,
                    0xd1, 0xe3, 0x81, 0xc3, 0x76, 0x0b, 0x2e, 0x8b, 0x17, 0xb4, 0x09, 0xcd, 0x21,
                ][..]
            ),
        "NMOUSE.COM startup text consumer does not match"
    );

    let mut builder = ProgramTextBuilder::new(
        "NMOUSE.COM",
        ExternalTextStorage::OriginalFile,
        bytes.len(),
        bytes,
    );
    for (address, consumer_offset, role) in [
        (0x0c2f, 0x088e, "other_mouse_driver_status"),
        (0x0c0e, 0x08db, "unload_status"),
        (0x0ee0, 0x0903, "invalid_parameter"),
        (0x0b7a, 0x090f, "startup_banner"),
        (0x0c6e, 0x0929, "command_mode_label"),
        (0x0cc7, 0x0974, "help_message"),
        (0x0e95, 0x09a2, "not_resident"),
        (0x0eae, 0x09aa, "other_driver_resident"),
        (0x0ed3, 0x09b2, "resident_status"),
    ] {
        builder.add_com_dos_reference(address, consumer_offset, role, None)?;
    }
    add_pointer_table(
        bytes,
        &mut builder,
        FREQUENCY_POINTER_TABLE_OFFSET,
        FREQUENCY_POINTER_COUNT,
        0x0925,
        "frequency_label_table",
    )?;
    add_pointer_table(
        bytes,
        &mut builder,
        COMMAND_MODE_POINTER_TABLE_OFFSET,
        COMMAND_MODE_POINTER_COUNT,
        0x093f,
        "command_mode_label_table",
    )?;
    builder.finish()
}

pub(crate) fn catalog_mouse_driver_runtime(bytes: &[u8]) -> Result<MouseDriverRuntimeCatalog> {
    let entry_cli = decode_bytes(bytes)?;
    ensure!(
        entry_cli.instruction == Instruction::Cli && entry_cli.byte_len == 1,
        "NMOUSE.COM entry no longer disables interrupts before its near jump"
    );
    let entry_jump = decode_bytes(
        bytes
            .get(ENTRY_JUMP_OFFSET..)
            .context("NMOUSE.COM entry jump lies outside the file")?,
    )?;
    let displacement = match entry_jump.instruction {
        Instruction::Jmp {
            target: JmpTarget::Rel16(displacement),
        } if entry_jump.byte_len == 3 => displacement,
        _ => anyhow::bail!("NMOUSE.COM entry no longer uses the typed near jump"),
    };
    let jump_next = 0x100_i32
        + i32::try_from(ENTRY_JUMP_OFFSET + entry_jump.byte_len)
            .context("NMOUSE.COM entry jump address overflow")?;
    ensure!(
        jump_next + i32::from(displacement) == i32::from(TRANSIENT_ENTRY_COM_ADDRESS),
        "NMOUSE.COM transient entry moved"
    );

    let resident_paragraphs = decode_bytes(
        bytes
            .get(RESIDENT_PARAGRAPH_LOAD_OFFSET..)
            .context("NMOUSE.COM resident paragraph load lies outside the file")?,
    )?;
    let resident_paragraph_count = match resident_paragraphs.instruction {
        Instruction::Mov {
            dest: Operand::Reg16(Register16::DX),
            src: Operand::Imm16(count),
        } if resident_paragraphs.byte_len == 3 => count,
        _ => anyhow::bail!("NMOUSE.COM resident size no longer uses typed MOV DX, imm16"),
    };
    ensure!(
        resident_paragraph_count == 0x0096,
        "NMOUSE.COM resident paragraph count changed"
    );
    ensure!(
        bytes.get(
            RESIDENT_PARAGRAPH_LOAD_OFFSET + resident_paragraphs.byte_len
                ..RESIDENT_PARAGRAPH_LOAD_OFFSET
                    + resident_paragraphs.byte_len
                    + RESIDENT_TERMINATION_SUFFIX.len()
        ) == Some(RESIDENT_TERMINATION_SUFFIX),
        "NMOUSE.COM terminate-and-stay-resident sequence changed"
    );

    Ok(MouseDriverRuntimeCatalog {
        entry_jump_offset: ENTRY_JUMP_OFFSET,
        transient_entry_com_address: TRANSIENT_ENTRY_COM_ADDRESS,
        resident_paragraph_load_offset: RESIDENT_PARAGRAPH_LOAD_OFFSET,
        resident_paragraph_count,
        frequency_pointer_table_offset: FREQUENCY_POINTER_TABLE_OFFSET,
        frequency_pointer_count: FREQUENCY_POINTER_COUNT,
        command_mode_pointer_table_offset: COMMAND_MODE_POINTER_TABLE_OFFSET,
        command_mode_pointer_count: COMMAND_MODE_POINTER_COUNT,
    })
}

fn add_pointer_table(
    bytes: &[u8],
    builder: &mut ProgramTextBuilder<'_>,
    table_offset: usize,
    count: usize,
    consumer_offset: usize,
    role: &str,
) -> Result<()> {
    for index in 0..count {
        let offset = table_offset + index * 2;
        let raw: [u8; 2] = bytes
            .get(offset..offset + 2)
            .with_context(|| format!("truncated NMOUSE.COM pointer at {offset:#x}"))?
            .try_into()
            .expect("a two-byte range converts to an array");
        builder.add_com_dos_reference(
            u16::from_le_bytes(raw) as usize,
            consumer_offset,
            role,
            Some(index),
        )?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "mouse_driver_tests.rs"]
mod mouse_driver_tests;
