use anyhow::{Context, Result, ensure};
use v30::{Instruction, JmpTarget, Operand, Register16, decode_bytes};

use super::catalog::{ExternalProgramTextCatalog, ExternalTextStorage};
use super::program_text::ProgramTextBuilder;
use super::sound_driver_runtime::{
    SamplingDriverLifetimeCatalog, catalog_packed_sound_driver_runtime_from_image,
};
use crate::dos_program::unpack_self_expanding_com;

const BPLAY_DIRECT_SCAN: std::ops::Range<usize> = 0x2700..0x2d79;
const BPLAY_ALTERNATE_CONSUMER: usize = 0x27bf;
const BPLAY_ALTERNATE_PATTERN: &[u8] = &[
    0xb4, 0x09, 0xba, 0xf9, 0x2a, 0x80, 0x3e, 0xbb, 0x01, 0x00, 0x74, 0x03, 0xba, 0x54, 0x2b, 0xcd,
    0x21,
];
const BPLAY_TIME_CONSUMER: usize = 0x2d2c;
const BPLAY_TIME_TABLE_ADDRESS: usize = 0x2e79;
const BPLAY_TIME_TABLE_PATTERN: &[u8] = &[
    0x00, 0x01, 0xa1, 0x2e, 0x00, 0x02, 0xcb, 0x2e, 0x01, 0x02, 0xb3, 0x2f, 0x00, 0x03, 0xf9, 0x2e,
    0x00, 0x04, 0x1d, 0x2f, 0x00, 0x06, 0x4d, 0x2f, 0x00, 0x08, 0x81, 0x2f, 0x00, 0x16, 0xb3, 0x2f,
    0x00, 0x17, 0xba, 0x2f, 0x00, 0x18, 0xd8, 0x2f,
];

const FPLAY_DIRECT_SCAN: std::ops::Range<usize> = 0x2700..0x2cae;
const FPLAY_TIME_CONSUMER: usize = 0x2c61;
const FPLAY_TIME_TABLE_ADDRESS: usize = 0x2dae;
const FPLAY_TIME_TABLE_PATTERN: &[u8] = &[
    0x00, 0x01, 0xd6, 0x2d, 0x00, 0x02, 0xec, 0x2d, 0x01, 0x02, 0xb6, 0x2e, 0x00, 0x03, 0x08, 0x2e,
    0x00, 0x04, 0x36, 0x2e, 0x00, 0x06, 0x58, 0x2e, 0x00, 0x08, 0x78, 0x2e, 0x00, 0x16, 0xb6, 0x2e,
    0x00, 0x17, 0xbd, 0x2e, 0x00, 0x18, 0xe7, 0x2e,
];

const TIME_TABLE_RECORDS: usize = 10;
const TIME_TABLE_RECORD_SIZE: usize = 4;
const BSAMP_ENTRY_JUMP_OFFSET: usize = 0x0001;
const BSAMP_TRANSIENT_ENTRY_COM_ADDRESS: u16 = 0x15ec;
const BSAMP_TRANSIENT_ENTRY_OFFSET: usize = 0x14ec;
const BSAMP_RESIDENT_END_LOAD_OFFSET: usize = 0x1562;
const BSAMP_RESIDENT_TERMINATION_SUFFIX: &[u8] = &[
    0x83, 0xc2, 0x0f, 0xc1, 0xea, 0x04, 0xb8, 0x00, 0x31, 0xcd, 0x21,
];

pub(super) fn catalog_sound_drivers(
    bplay: &[u8],
    fplay: &[u8],
    bsamp: &[u8],
) -> Result<Vec<ExternalProgramTextCatalog>> {
    Ok(vec![
        catalog_playback_driver("BPLAY6.COM", bplay)?,
        catalog_playback_driver("FPLAY6.COM", fplay)?,
        catalog_sampling_driver(bsamp)?,
    ])
}

pub(crate) fn catalog_playback_driver(
    file_name: &'static str,
    packed: &[u8],
) -> Result<ExternalProgramTextCatalog> {
    match file_name {
        "BPLAY6.COM" => catalog_packed_driver(
            file_name,
            packed,
            BPLAY_DIRECT_SCAN,
            Some((BPLAY_ALTERNATE_CONSUMER, BPLAY_ALTERNATE_PATTERN, 0x2b54)),
            BPLAY_TIME_CONSUMER,
            BPLAY_TIME_TABLE_ADDRESS,
            BPLAY_TIME_TABLE_PATTERN,
        ),
        "FPLAY6.COM" => catalog_packed_driver(
            file_name,
            packed,
            FPLAY_DIRECT_SCAN,
            None,
            FPLAY_TIME_CONSUMER,
            FPLAY_TIME_TABLE_ADDRESS,
            FPLAY_TIME_TABLE_PATTERN,
        ),
        _ => anyhow::bail!("unsupported packed sound driver {file_name}"),
    }
}

fn catalog_packed_driver(
    file_name: &'static str,
    packed: &[u8],
    direct_scan: std::ops::Range<usize>,
    alternate: Option<(usize, &[u8], usize)>,
    time_consumer: usize,
    time_table_address: usize,
    time_table_pattern: &[u8],
) -> Result<ExternalProgramTextCatalog> {
    let image = unpack_self_expanding_com(packed)?;
    let mut builder = ProgramTextBuilder::new(
        file_name,
        ExternalTextStorage::UnpackedComImage,
        image.packed_size,
        &image.unpacked,
    );

    collect_direct_dos_outputs(&image.unpacked, direct_scan, &mut builder)?;
    if let Some((consumer_offset, pattern, alternate_address)) = alternate {
        ensure_bytes(&image.unpacked, consumer_offset, pattern, file_name)?;
        builder.add_com_dos_reference_with_id(
            "external-bplay6-text-020",
            0x2af9,
            consumer_offset,
            "conditional_dos_output",
            None,
        )?;
        builder.add_com_dos_reference(
            alternate_address,
            consumer_offset,
            "conditional_dos_output",
            None,
        )?;
    }

    let table_offset = time_table_address
        .checked_sub(0x100)
        .context("sound-driver time table lies below the COM origin")?;
    ensure_bytes(&image.unpacked, table_offset, time_table_pattern, file_name)?;
    for index in 0..TIME_TABLE_RECORDS {
        let record_offset = table_offset + index * TIME_TABLE_RECORD_SIZE;
        let message_address = read_u16(&image.unpacked, record_offset + 2)? as usize;
        builder.add_com_dos_reference(
            message_address,
            time_consumer,
            "time_message_table",
            Some(index),
        )?;
    }

    let catalog = builder.finish()?;
    catalog_packed_sound_driver_runtime_from_image(&image.unpacked, &catalog)?;
    Ok(catalog)
}

fn collect_direct_dos_outputs(
    bytes: &[u8],
    scan: std::ops::Range<usize>,
    builder: &mut ProgramTextBuilder<'_>,
) -> Result<()> {
    ensure!(
        scan.end <= bytes.len(),
        "sound-driver code scan exceeds the unpacked image"
    );
    for offset in scan {
        let tail = &bytes[offset..];
        if tail.len() >= 7 && tail[0..3] == [0xb4, 0x09, 0xba] && tail[5..7] == [0xcd, 0x21] {
            let address = u16::from_le_bytes([tail[3], tail[4]]) as usize;
            builder.add_com_dos_reference(address, offset, "direct_dos_output", None)?;
        }
        if tail.len() >= 7
            && tail[0] == 0xba
            && tail[3..5] == [0xb4, 0x09]
            && tail[5..7] == [0xcd, 0x21]
        {
            let address = u16::from_le_bytes([tail[1], tail[2]]) as usize;
            builder.add_com_dos_reference(address, offset, "direct_dos_output", None)?;
        }
    }
    Ok(())
}

pub(crate) fn catalog_sampling_driver(bytes: &[u8]) -> Result<ExternalProgramTextCatalog> {
    catalog_sampling_driver_lifetime(bytes)?;
    ensure_bytes(
        bytes,
        0x14f7,
        &[
            0x8c, 0x0e, 0xba, 0x15, 0xba, 0x94, 0x17, 0xb4, 0x09, 0xcd, 0x21,
        ],
        "BSAMP.COM",
    )?;
    let mut builder = ProgramTextBuilder::new(
        "BSAMP.COM",
        ExternalTextStorage::OriginalFile,
        bytes.len(),
        bytes,
    );
    for (address, consumer_offset, role) in [
        (0x1794, 0x14fb, "startup_banner"),
        (0x1806, 0x153a, "unload_status"),
        (0x17e9, 0x1554, "resident_status"),
    ] {
        builder.add_com_dos_reference(address, consumer_offset, role, None)?;
    }
    builder.finish()
}

pub(crate) fn catalog_sampling_driver_lifetime(
    bytes: &[u8],
) -> Result<SamplingDriverLifetimeCatalog> {
    let entry_cli = decode_bytes(bytes)?;
    ensure!(
        entry_cli.instruction == Instruction::Cli && entry_cli.byte_len == 1,
        "BSAMP.COM entry no longer disables interrupts before its near jump"
    );
    let entry_jump = decode_bytes(
        bytes
            .get(BSAMP_ENTRY_JUMP_OFFSET..)
            .context("BSAMP.COM entry jump lies outside the file")?,
    )?;
    let displacement = match entry_jump.instruction {
        Instruction::Jmp {
            target: JmpTarget::Rel16(displacement),
        } if entry_jump.byte_len == 3 => displacement,
        _ => anyhow::bail!("BSAMP.COM entry no longer uses the typed near jump"),
    };
    let jump_next = 0x100_i32
        + i32::try_from(BSAMP_ENTRY_JUMP_OFFSET + entry_jump.byte_len)
            .context("BSAMP.COM entry jump address overflow")?;
    let transient_entry = jump_next + i32::from(displacement);
    ensure!(
        transient_entry == i32::from(BSAMP_TRANSIENT_ENTRY_COM_ADDRESS),
        "BSAMP.COM transient entry moved"
    );
    let transient_start = decode_bytes(
        bytes
            .get(BSAMP_TRANSIENT_ENTRY_OFFSET..)
            .context("BSAMP.COM transient entry lies outside the file")?,
    )?;
    ensure!(
        transient_start.instruction == Instruction::Sti && transient_start.byte_len == 1,
        "BSAMP.COM transient entry no longer restores interrupts"
    );

    let resident_end = decode_bytes(
        bytes
            .get(BSAMP_RESIDENT_END_LOAD_OFFSET..)
            .context("BSAMP.COM resident-end load lies outside the file")?,
    )?;
    let resident_end_com_address = match resident_end.instruction {
        Instruction::Mov {
            dest: Operand::Reg16(Register16::DX),
            src: Operand::Imm16(address),
        } if resident_end.byte_len == 3 => address,
        _ => anyhow::bail!("BSAMP.COM resident end no longer uses typed MOV DX, imm16"),
    };
    ensure!(
        resident_end_com_address == BSAMP_TRANSIENT_ENTRY_COM_ADDRESS,
        "BSAMP.COM resident boundary no longer ends before transient initialization"
    );
    ensure_bytes(
        bytes,
        BSAMP_RESIDENT_END_LOAD_OFFSET + resident_end.byte_len,
        BSAMP_RESIDENT_TERMINATION_SUFFIX,
        "BSAMP.COM",
    )?;

    Ok(SamplingDriverLifetimeCatalog {
        entry_jump_offset: BSAMP_ENTRY_JUMP_OFFSET,
        transient_entry_com_address: BSAMP_TRANSIENT_ENTRY_COM_ADDRESS,
        resident_end_load_offset: BSAMP_RESIDENT_END_LOAD_OFFSET,
        resident_end_com_address,
    })
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let raw: [u8; 2] = bytes
        .get(offset..offset + 2)
        .with_context(|| format!("truncated sound-driver field at {offset:#x}"))?
        .try_into()
        .expect("a two-byte range converts to an array");
    Ok(u16::from_le_bytes(raw))
}

fn ensure_bytes(bytes: &[u8], offset: usize, expected: &[u8], file_name: &str) -> Result<()> {
    ensure!(
        bytes.get(offset..offset + expected.len()) == Some(expected),
        "{file_name} consumer signature does not match at {offset:#x}"
    );
    Ok(())
}

#[cfg(test)]
#[path = "sound_drivers_tests.rs"]
mod sound_drivers_tests;
