use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use v30::{CallTarget, Instruction, Operand, Register8, Register16, decode_bytes};

use super::{
    MenuTextNonReferenceOccurrence, MenuTextRuntimeReference, MenuTextRuntimeReferenceKind,
};
use crate::game_data::binary::read_u16;
use crate::game_data::menu::{MenuTextEntry, MenuTextReference};

const DISK_ERROR_TABLE_OFFSET: usize = 0x0571;
const FLOPPY_ERROR_TABLE_OFFSET: usize = 0x0eae;
const PROCESS_ERROR_SWITCH_RANGE: std::ops::Range<usize> = 0x07da..0x0805;
const FIXED_RENDERER_INSTRUCTIONS: [(&str, usize, Register16); 4] = [
    ("wrong_disk_script", 0x0158, Register16::SI),
    ("disk_retry_prompt", 0x0519, Register16::SI),
    ("hard_disk_guard", 0x0e2d, Register16::SI),
    ("floppy_drive_label", 0x0e74, Register16::SI),
];
const TARGET_LIKE_MOV_AH_09_OFFSETS: [usize; 12] = [
    0x0055, 0x01e8, 0x0281, 0x0305, 0x0347, 0x03a5, 0x06d5, 0x0805, 0x080c, 0x092a, 0x0b2e, 0x0c01,
];
const TARGET_LIKE_RELATIVE_CALL_OFFSET: usize = 0x040c;

pub(super) struct CatalogedReferences {
    pub references: Vec<MenuTextRuntimeReference>,
    pub non_reference_occurrences: Vec<MenuTextNonReferenceOccurrence>,
    pub referenced_entry_count: usize,
    pub machine_code_reference_count: usize,
    pub metadata_reference_count: usize,
}

pub(super) fn catalog_references(
    program: &[u8],
    entries: &[MenuTextEntry],
    semantic_reference_count: usize,
) -> Result<CatalogedReferences> {
    let entries_by_address = entries
        .iter()
        .map(|entry| {
            Ok((
                u16::try_from(entry.com_address).context("MENU text address exceeds 16 bits")?,
                entry,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    ensure!(
        entries_by_address.len() == entries.len(),
        "MENU text has duplicate COM addresses"
    );

    let mut references_by_storage = BTreeMap::new();
    for text_entry in entries {
        for source_reference in &text_entry.references {
            let reference = catalog_reference(program, text_entry, source_reference)?;
            merge_reference(&mut references_by_storage, reference)?;
        }
    }
    let references = references_by_storage.into_values().collect::<Vec<_>>();
    ensure!(
        references.len() == 49 && semantic_reference_count == 50,
        "MENU semantic or physical reference population changed"
    );

    let referenced_entry_ids = references
        .iter()
        .map(|reference| reference.target_entry_id.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        referenced_entry_ids.len() == entries.len()
            && entries
                .iter()
                .all(|entry| referenced_entry_ids.contains(entry.id.as_str())),
        "MENU referenced text population changed"
    );
    let reference_offsets = references
        .iter()
        .map(|reference| reference.storage_offset)
        .collect::<BTreeSet<_>>();
    ensure!(
        reference_offsets.len() == references.len(),
        "MENU physical reference storage overlaps after merging"
    );

    let targets = entries_by_address
        .iter()
        .map(|(address, entry)| (*address, entry.id.as_str()))
        .collect::<BTreeMap<_, _>>();
    let non_reference_occurrences = scan_target_occurrences(program, &targets)
        .into_iter()
        .filter(|(offset, _, _)| !reference_offsets.contains(offset))
        .map(|(storage_offset, word, matched_target_id)| {
            let reason = classify_non_reference(program, storage_offset)?;
            Ok(MenuTextNonReferenceOccurrence {
                storage_offset,
                word,
                matched_target_id: matched_target_id.to_owned(),
                reason: reason.to_owned(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let expected_non_references = TARGET_LIKE_MOV_AH_09_OFFSETS
        .into_iter()
        .chain([TARGET_LIKE_RELATIVE_CALL_OFFSET + 1])
        .collect::<BTreeSet<_>>();
    ensure!(
        non_reference_occurrences
            .iter()
            .map(|occurrence| occurrence.storage_offset)
            .collect::<BTreeSet<_>>()
            == expected_non_references,
        "MENU target-like non-reference population changed"
    );

    let machine_code_reference_count = references
        .iter()
        .filter(|reference| {
            reference.storage_kind == MenuTextRuntimeReferenceKind::MachineCodeImmediate
        })
        .count();
    let metadata_reference_count = references.len() - machine_code_reference_count;
    ensure!(
        machine_code_reference_count == 18 && metadata_reference_count == 31,
        "MENU machine-code or metadata reference population changed"
    );
    let referenced_entry_count = referenced_entry_ids.len();

    Ok(CatalogedReferences {
        references,
        non_reference_occurrences,
        referenced_entry_count,
        machine_code_reference_count,
        metadata_reference_count,
    })
}

fn catalog_reference(
    program: &[u8],
    entry: &MenuTextEntry,
    source_reference: &MenuTextReference,
) -> Result<MenuTextRuntimeReference> {
    let target_com_address = u16::try_from(entry.com_address)
        .with_context(|| format!("{} COM address exceeds 16 bits", entry.id))?;
    match source_reference.role.as_str() {
        "disk_error_pointer_table" => metadata_reference(
            program,
            entry,
            source_reference,
            DISK_ERROR_TABLE_OFFSET,
            target_com_address,
        ),
        "floppy_bios_error_pointer_table" => metadata_reference(
            program,
            entry,
            source_reference,
            FLOPPY_ERROR_TABLE_OFFSET,
            target_com_address,
        ),
        "process_error_switch" => {
            let instruction_offset = locate_mov_immediate(
                program,
                PROCESS_ERROR_SWITCH_RANGE,
                target_com_address,
                Register16::DX,
                "process-error switch",
            )?;
            machine_reference(
                program,
                entry,
                source_reference,
                instruction_offset,
                target_com_address,
                Register16::DX,
            )
        }
        "dos_ah09_direct" => {
            let start = source_reference.consumer_file_offset;
            let end = start
                .checked_add(7)
                .context("MENU direct DOS-output reference window overflow")?;
            let instruction_offset = locate_mov_immediate(
                program,
                start..end,
                target_com_address,
                Register16::DX,
                "direct DOS output",
            )?;
            machine_reference(
                program,
                entry,
                source_reference,
                instruction_offset,
                target_com_address,
                Register16::DX,
            )
        }
        role => {
            let (_, instruction_offset, register) = FIXED_RENDERER_INSTRUCTIONS
                .iter()
                .find(|(candidate, _, _)| *candidate == role)
                .with_context(|| {
                    format!("{} has an unknown MENU consumer role {role}", entry.id)
                })?;
            machine_reference(
                program,
                entry,
                source_reference,
                *instruction_offset,
                target_com_address,
                *register,
            )
        }
    }
}

fn metadata_reference(
    program: &[u8],
    entry: &MenuTextEntry,
    source_reference: &MenuTextReference,
    table_offset: usize,
    target_com_address: u16,
) -> Result<MenuTextRuntimeReference> {
    let index = source_reference
        .table_index
        .with_context(|| format!("{} metadata reference lost its table index", entry.id))?;
    let storage_offset = table_offset
        .checked_add(index * 2)
        .context("MENU metadata reference offset overflow")?;
    ensure!(
        read_u16(program, storage_offset)? == target_com_address,
        "{} metadata reference target changed",
        entry.id
    );
    Ok(MenuTextRuntimeReference {
        id: format!("menu-metadata-reference-{storage_offset:04x}"),
        storage_kind: MenuTextRuntimeReferenceKind::MetadataTableEntry,
        storage_offset,
        instruction_offset: None,
        target_com_address,
        target_entry_id: entry.id.clone(),
        consumer_roles: vec![source_reference.role.clone()],
    })
}

fn machine_reference(
    program: &[u8],
    entry: &MenuTextEntry,
    source_reference: &MenuTextReference,
    instruction_offset: usize,
    target_com_address: u16,
    register: Register16,
) -> Result<MenuTextRuntimeReference> {
    require_mov_immediate(
        program,
        instruction_offset,
        target_com_address,
        register,
        &source_reference.role,
    )?;
    Ok(MenuTextRuntimeReference {
        id: format!("menu-code-reference-{instruction_offset:04x}"),
        storage_kind: MenuTextRuntimeReferenceKind::MachineCodeImmediate,
        storage_offset: instruction_offset + 1,
        instruction_offset: Some(instruction_offset),
        target_com_address,
        target_entry_id: entry.id.clone(),
        consumer_roles: vec![source_reference.role.clone()],
    })
}

fn locate_mov_immediate(
    program: &[u8],
    range: std::ops::Range<usize>,
    target: u16,
    register: Register16,
    role: &str,
) -> Result<usize> {
    ensure!(
        range.end <= program.len(),
        "MENU {role} window exceeds the file"
    );
    let matches = range
        .filter(|offset| {
            decode_bytes(&program[*offset..]).is_ok_and(|decoded| {
                decoded.byte_len == 3
                    && decoded.prefixes.is_empty()
                    && decoded.instruction
                        == (Instruction::Mov {
                            dest: Operand::Reg16(register),
                            src: Operand::Imm16(target),
                        })
            })
        })
        .collect::<Vec<_>>();
    ensure!(
        matches.len() == 1,
        "MENU {role} has {} typed V30 references to {target:#06x}",
        matches.len()
    );
    Ok(matches[0])
}

fn require_mov_immediate(
    program: &[u8],
    instruction_offset: usize,
    target: u16,
    register: Register16,
    role: &str,
) -> Result<()> {
    let decoded = decode_bytes(
        program
            .get(instruction_offset..)
            .with_context(|| format!("MENU {role} instruction lies outside the file"))?,
    )?;
    ensure!(
        decoded.byte_len == 3
            && decoded.prefixes.is_empty()
            && decoded.instruction
                == (Instruction::Mov {
                    dest: Operand::Reg16(register),
                    src: Operand::Imm16(target),
                }),
        "MENU {role} changed its typed V30 MOV {}, imm16 reference",
        register.name()
    );
    Ok(())
}

pub(super) fn merge_reference(
    references: &mut BTreeMap<usize, MenuTextRuntimeReference>,
    reference: MenuTextRuntimeReference,
) -> Result<()> {
    if let Some(existing) = references.get_mut(&reference.storage_offset) {
        ensure!(
            existing.storage_kind == reference.storage_kind
                && existing.instruction_offset == reference.instruction_offset
                && existing.target_com_address == reference.target_com_address
                && existing.target_entry_id == reference.target_entry_id,
            "MENU semantic references disagree at physical storage offset {:#x}",
            reference.storage_offset
        );
        for role in reference.consumer_roles {
            if !existing.consumer_roles.contains(&role) {
                existing.consumer_roles.push(role);
            }
        }
    } else {
        references.insert(reference.storage_offset, reference);
    }
    Ok(())
}

pub(super) fn scan_target_occurrences<'a>(
    program: &[u8],
    targets: &BTreeMap<u16, &'a str>,
) -> Vec<(usize, u16, &'a str)> {
    program
        .windows(2)
        .enumerate()
        .filter_map(|(offset, pair)| {
            let word = u16::from_le_bytes([pair[0], pair[1]]);
            targets
                .get(&word)
                .map(|target_id| (offset, word, *target_id))
        })
        .collect()
}

fn classify_non_reference(program: &[u8], storage_offset: usize) -> Result<&'static str> {
    if TARGET_LIKE_MOV_AH_09_OFFSETS.contains(&storage_offset) {
        let decoded = decode_bytes(
            program
                .get(storage_offset..)
                .context("MENU target-like MOV AH instruction lies outside the file")?,
        )?;
        ensure!(
            decoded.byte_len == 2
                && decoded.prefixes.is_empty()
                && decoded.instruction
                    == (Instruction::Mov {
                        dest: Operand::Reg8(Register8::AH),
                        src: Operand::Imm8(0x09),
                    }),
            "MENU target-like bytes no longer form the verified MOV AH, 09h"
        );
        return Ok("target-like bytes are the opcode and immediate of typed V30 MOV AH, 09h");
    }
    if storage_offset == TARGET_LIKE_RELATIVE_CALL_OFFSET + 1 {
        let decoded = decode_bytes(
            program
                .get(TARGET_LIKE_RELATIVE_CALL_OFFSET..)
                .context("MENU target-like relative CALL lies outside the file")?,
        )?;
        ensure!(
            decoded.byte_len == 3
                && decoded.prefixes.is_empty()
                && matches!(
                    decoded.instruction,
                    Instruction::Call {
                        target: CallTarget::Rel16(_),
                    }
                ),
            "MENU target-like bytes no longer lie in the verified relative CALL"
        );
        return Ok("target-like bytes are a typed V30 relative CALL displacement");
    }
    anyhow::bail!("unexpected MENU target-like occurrence at {storage_offset:#x}")
}
