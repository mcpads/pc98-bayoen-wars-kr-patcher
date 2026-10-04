use anyhow::{Context, Result, ensure};
use v30::{Instruction, JmpTarget, Operand, Register16, decode_bytes};

use super::catalog::{ExternalProgramTextCatalog, ExternalTextStorage};
use crate::dos_program::unpack_self_expanding_com;

const COM_ORIGIN: usize = 0x100;
const DOS_PSP_PARAGRAPHS: usize = 0x10;
const DIRECT_REFERENCE_WINDOW: usize = 7;
const CONDITIONAL_REFERENCE_WINDOW: usize = 17;
const TIME_TABLE_RECORDS: usize = 10;
const TIME_TABLE_RECORD_SIZE: usize = 4;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct SamplingDriverLifetimeCatalog {
    pub entry_jump_offset: usize,
    pub transient_entry_com_address: u16,
    pub resident_end_load_offset: usize,
    pub resident_end_com_address: u16,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum PackedSoundDriverReferenceKind {
    MachineCode,
    Metadata,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct PackedSoundDriverTextReference {
    pub entry_id: String,
    pub storage_offset: usize,
    pub instruction_offset: Option<usize>,
    pub original_com_address: u16,
    pub consumer_offset: usize,
    pub table_index: Option<usize>,
    pub kind: PackedSoundDriverReferenceKind,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct PackedSoundDriverRuntimeCatalog {
    pub file_name: String,
    pub entry_jump_offset: usize,
    pub transient_entry_com_address: u16,
    pub resident_paragraphs: u16,
    pub resident_end_com_address: u16,
    pub resident_end_file_offset: usize,
    pub references: Vec<PackedSoundDriverTextReference>,
}

#[derive(Clone, Copy)]
struct PackedSoundDriverProfile {
    file_name: &'static str,
    transient_entry_com_address: u16,
    shrink_paragraph_load_offset: usize,
    terminate_resident_offset: usize,
    time_table_address: usize,
}

const BPLAY_PROFILE: PackedSoundDriverProfile = PackedSoundDriverProfile {
    file_name: "BPLAY6.COM",
    transient_entry_com_address: 0x2800,
    shrink_paragraph_load_offset: 0x275a,
    terminate_resident_offset: 0x27f5,
    time_table_address: 0x2e79,
};

const FPLAY_PROFILE: PackedSoundDriverProfile = PackedSoundDriverProfile {
    file_name: "FPLAY6.COM",
    transient_entry_com_address: 0x2807,
    shrink_paragraph_load_offset: 0x27d4,
    terminate_resident_offset: 0x289d,
    time_table_address: 0x2dae,
};

pub(crate) fn catalog_packed_sound_driver_runtime(
    packed: &[u8],
    source: &ExternalProgramTextCatalog,
) -> Result<PackedSoundDriverRuntimeCatalog> {
    let image = unpack_self_expanding_com(packed)?;
    ensure!(
        image.packed_size == source.packed_size && image.unpacked.len() == source.text_storage_size,
        "{} packed or unpacked source size changed",
        source.file_name
    );
    catalog_packed_sound_driver_runtime_from_image(&image.unpacked, source)
}

pub(super) fn catalog_packed_sound_driver_runtime_from_image(
    unpacked: &[u8],
    source: &ExternalProgramTextCatalog,
) -> Result<PackedSoundDriverRuntimeCatalog> {
    let profile = packed_sound_driver_profile(&source.file_name)?;
    ensure!(
        source.storage == ExternalTextStorage::UnpackedComImage,
        "{} no longer uses unpacked COM text storage",
        source.file_name
    );

    let entry_jump_offset = 0;
    let entry_jump = decode_bytes(unpacked)?;
    let displacement = match entry_jump.instruction {
        Instruction::Jmp {
            target: JmpTarget::Rel16(displacement),
        } if entry_jump.byte_len == 3 => displacement,
        _ => anyhow::bail!(
            "{} entry no longer uses a typed near jump",
            profile.file_name
        ),
    };
    let jump_next = i32::try_from(COM_ORIGIN + entry_jump.byte_len)?;
    ensure!(
        jump_next + i32::from(displacement) == i32::from(profile.transient_entry_com_address),
        "{} transient entry moved",
        profile.file_name
    );

    ensure_bytes(
        unpacked,
        profile.shrink_paragraph_load_offset - 2,
        &[0xb4, 0x4a],
        profile.file_name,
    )?;
    let shrink_load = decode_bytes(
        unpacked
            .get(profile.shrink_paragraph_load_offset..)
            .context("packed sound-driver shrink load lies outside the image")?,
    )?;
    let resident_paragraphs = match shrink_load.instruction {
        Instruction::Mov {
            dest: Operand::Reg16(Register16::BX),
            src: Operand::Imm16(paragraphs),
        } if shrink_load.byte_len == 3 => paragraphs,
        _ => anyhow::bail!(
            "{} memory shrink no longer uses typed MOV BX, imm16",
            profile.file_name
        ),
    };
    ensure_bytes(
        unpacked,
        profile.shrink_paragraph_load_offset + shrink_load.byte_len,
        &[0xcd, 0x21],
        profile.file_name,
    )?;

    let terminate_load = decode_bytes(
        unpacked
            .get(profile.terminate_resident_offset..)
            .context("packed sound-driver resident termination lies outside the image")?,
    )?;
    ensure!(
        terminate_load.instruction
            == (Instruction::Mov {
                dest: Operand::Reg16(Register16::AX),
                src: Operand::Imm16(0x3102),
            })
            && terminate_load.byte_len == 3,
        "{} terminate-and-stay-resident mode changed",
        profile.file_name
    );
    let resident_size_load_offset = profile.terminate_resident_offset + terminate_load.byte_len;
    let resident_size_load = decode_bytes(
        unpacked
            .get(resident_size_load_offset..)
            .context("packed sound-driver resident size load lies outside the image")?,
    )?;
    let termination_paragraphs = match resident_size_load.instruction {
        Instruction::Mov {
            dest: Operand::Reg16(Register16::DX),
            src: Operand::Imm16(paragraphs),
        } if resident_size_load.byte_len == 3 => paragraphs,
        _ => anyhow::bail!(
            "{} resident termination no longer uses typed MOV DX, imm16",
            profile.file_name
        ),
    };
    ensure!(
        termination_paragraphs == resident_paragraphs,
        "{} shrink and resident paragraph counts disagree",
        profile.file_name
    );
    ensure_bytes(
        unpacked,
        resident_size_load_offset + resident_size_load.byte_len,
        &[0xcd, 0x21],
        profile.file_name,
    )?;

    let resident_end_com_address = usize::from(resident_paragraphs)
        .checked_mul(16)
        .context("packed sound-driver resident size overflow")?;
    ensure!(
        usize::from(resident_paragraphs) > DOS_PSP_PARAGRAPHS
            && resident_end_com_address > COM_ORIGIN,
        "{} resident size does not retain the COM program",
        profile.file_name
    );
    let resident_end_file_offset = resident_end_com_address - COM_ORIGIN;
    ensure!(
        unpacked.len() < resident_end_file_offset,
        "{} source image already reaches its first non-resident file offset",
        profile.file_name
    );

    let references = catalog_references(unpacked, source, profile)?;
    Ok(PackedSoundDriverRuntimeCatalog {
        file_name: profile.file_name.to_owned(),
        entry_jump_offset,
        transient_entry_com_address: profile.transient_entry_com_address,
        resident_paragraphs,
        resident_end_com_address: u16::try_from(resident_end_com_address)
            .context("packed sound-driver resident end exceeds 16 bits")?,
        resident_end_file_offset,
        references,
    })
}

fn catalog_references(
    unpacked: &[u8],
    source: &ExternalProgramTextCatalog,
    profile: PackedSoundDriverProfile,
) -> Result<Vec<PackedSoundDriverTextReference>> {
    let mut references = Vec::new();
    let table_offset = profile
        .time_table_address
        .checked_sub(COM_ORIGIN)
        .context("packed sound-driver time table lies below the COM origin")?;
    for entry in &source.entries {
        let original_com_address = u16::try_from(entry.runtime_address)
            .context("packed sound-driver text address exceeds 16 bits")?;
        for reference in &entry.references {
            let (storage_offset, instruction_offset, kind) = match reference.role.as_str() {
                "direct_dos_output" | "conditional_dos_output" => {
                    let window = if reference.role == "direct_dos_output" {
                        DIRECT_REFERENCE_WINDOW
                    } else {
                        CONDITIONAL_REFERENCE_WINDOW
                    };
                    let instruction_offset = locate_mov_dx_reference(
                        unpacked,
                        reference.consumer_offset,
                        window,
                        original_com_address,
                        profile.file_name,
                    )?;
                    (
                        instruction_offset + 1,
                        Some(instruction_offset),
                        PackedSoundDriverReferenceKind::MachineCode,
                    )
                }
                "time_message_table" => {
                    let index = reference
                        .table_index
                        .context("packed sound-driver time reference lost its table index")?;
                    ensure!(
                        index < TIME_TABLE_RECORDS,
                        "packed sound-driver time reference index is out of range"
                    );
                    (
                        table_offset + index * TIME_TABLE_RECORD_SIZE + 2,
                        None,
                        PackedSoundDriverReferenceKind::Metadata,
                    )
                }
                role => anyhow::bail!(
                    "{} has unsupported packed sound-driver reference role {role}",
                    entry.id
                ),
            };
            ensure!(
                read_u16(unpacked, storage_offset)? == original_com_address,
                "{} reference at {storage_offset:#x} changed",
                entry.id
            );
            references.push(PackedSoundDriverTextReference {
                entry_id: entry.id.clone(),
                storage_offset,
                instruction_offset,
                original_com_address,
                consumer_offset: reference.consumer_offset,
                table_index: reference.table_index,
                kind,
            });
        }

        let mut expected_occurrences = references
            .iter()
            .filter(|reference| reference.entry_id == entry.id)
            .map(|reference| reference.storage_offset)
            .collect::<Vec<_>>();
        expected_occurrences.sort_unstable();
        let mut actual_occurrences = find_all_u16(unpacked, original_com_address);
        actual_occurrences.sort_unstable();
        ensure!(
            actual_occurrences == expected_occurrences,
            "{} direct 16-bit reference audit changed: expected {expected_occurrences:?}, found {actual_occurrences:?}",
            entry.id
        );
    }
    references.sort_by_key(|reference| reference.storage_offset);
    ensure!(
        references.len() == source.reference_count,
        "{} runtime reference population changed",
        profile.file_name
    );
    ensure!(
        references
            .windows(2)
            .all(|pair| pair[0].storage_offset + 2 <= pair[1].storage_offset),
        "{} reference fields overlap",
        profile.file_name
    );
    Ok(references)
}

fn packed_sound_driver_profile(file_name: &str) -> Result<PackedSoundDriverProfile> {
    match file_name {
        "BPLAY6.COM" => Ok(BPLAY_PROFILE),
        "FPLAY6.COM" => Ok(FPLAY_PROFILE),
        _ => anyhow::bail!("unsupported packed sound driver {file_name}"),
    }
}

fn locate_mov_dx_reference(
    bytes: &[u8],
    start: usize,
    length: usize,
    target: u16,
    file_name: &str,
) -> Result<usize> {
    let end = start
        .checked_add(length)
        .context("packed sound-driver reference window overflow")?;
    ensure!(
        end <= bytes.len(),
        "{file_name} reference window lies outside the unpacked image"
    );
    let mut cursor = start;
    let mut matches = Vec::new();
    while cursor < end {
        let decoded = decode_bytes(&bytes[cursor..end])?;
        if decoded.instruction
            == (Instruction::Mov {
                dest: Operand::Reg16(Register16::DX),
                src: Operand::Imm16(target),
            })
        {
            matches.push(cursor);
        }
        cursor += decoded.byte_len;
    }
    ensure!(
        cursor == end && matches.len() == 1,
        "{file_name} needs exactly one typed MOV DX, {target:#06x} in {start:#x}..{end:#x}"
    );
    Ok(matches[0])
}

fn find_all_u16(bytes: &[u8], value: u16) -> Vec<usize> {
    let needle = value.to_le_bytes();
    bytes
        .windows(needle.len())
        .enumerate()
        .filter_map(|(offset, window)| (window == needle).then_some(offset))
        .collect()
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
