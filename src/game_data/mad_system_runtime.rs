use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::{Instruction, Operand, Register16, decode_bytes};

use super::MadSystemTextCatalog;

const INITIALIZATION_CONSUMER_END: usize = 0x3109;
const DYNAMIC_INSERT_INSTRUCTION_OFFSET: usize = 0x30dd;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MadSystemRuntimeReferenceKind {
    MachineCodeImmediate,
    MetadataTableEntry,
    RuntimeInsertMachineCode,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct MadSystemRuntimeReference {
    pub id: String,
    pub target_entry_id: String,
    pub consumer_role: String,
    pub kind: MadSystemRuntimeReferenceKind,
    pub instruction_offset: Option<usize>,
    pub storage_offset: usize,
    pub original_com_address: u16,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadSystemRuntimeCatalog {
    pub semantic_reference_count: usize,
    pub storage_reference_count: usize,
    pub machine_code_reference_count: usize,
    pub metadata_reference_count: usize,
    pub runtime_insert_reference_count: usize,
    pub reference_population_complete: bool,
    pub references: Vec<MadSystemRuntimeReference>,
}

pub(super) fn catalog_mad_system_runtime(
    program: &[u8],
    text: &MadSystemTextCatalog,
) -> Result<MadSystemRuntimeCatalog> {
    let mut references = Vec::new();
    for entry in &text.entries {
        let target = u16::try_from(entry.com_address)
            .context("MAD system text COM address exceeds 16 bits")?;
        for reference in &entry.references {
            let (kind, instruction_offset, storage_offset) = match reference.table_index {
                Some(index) => {
                    let storage_offset = text
                        .disk_error_pointer_table_offset
                        .checked_add(index * 2)
                        .context("MAD system metadata offset overflow")?;
                    ensure!(
                        read_word(program, storage_offset)? == target,
                        "MAD system metadata target changed"
                    );
                    (
                        MadSystemRuntimeReferenceKind::MetadataTableEntry,
                        None,
                        storage_offset,
                    )
                }
                None => {
                    require_mov_immediate(
                        program,
                        reference.consumer_file_offset,
                        Register16::DX,
                        target,
                        &reference.role,
                    )?;
                    (
                        MadSystemRuntimeReferenceKind::MachineCodeImmediate,
                        Some(reference.consumer_file_offset),
                        reference.consumer_file_offset + 1,
                    )
                }
            };
            references.push(MadSystemRuntimeReference {
                id: format!("mad-system-reference-{storage_offset:04x}"),
                target_entry_id: entry.id.clone(),
                consumer_role: reference.role.clone(),
                kind,
                instruction_offset,
                storage_offset,
                original_com_address: target,
            });
        }
    }

    let insert_entry = text
        .entries
        .iter()
        .find(|entry| entry.runtime_insert.is_some())
        .context("MAD system text has no runtime file-name insertion field")?;
    let insert_target = u16::try_from(insert_entry.com_address)
        .context("MAD system runtime insert address exceeds 16 bits")?;
    require_mov_immediate(
        program,
        DYNAMIC_INSERT_INSTRUCTION_OFFSET,
        Register16::BX,
        insert_target,
        "data-file name destination",
    )?;
    references.push(MadSystemRuntimeReference {
        id: format!(
            "mad-system-runtime-insert-{:04x}",
            DYNAMIC_INSERT_INSTRUCTION_OFFSET + 1
        ),
        target_entry_id: insert_entry.id.clone(),
        consumer_role: "data_file_stem_destination".to_owned(),
        kind: MadSystemRuntimeReferenceKind::RuntimeInsertMachineCode,
        instruction_offset: Some(DYNAMIC_INSERT_INSTRUCTION_OFFSET),
        storage_offset: DYNAMIC_INSERT_INSTRUCTION_OFFSET + 1,
        original_com_address: insert_target,
    });
    references.sort_by_key(|reference| reference.storage_offset);

    let targets = text
        .entries
        .iter()
        .map(|entry| Ok((u16::try_from(entry.com_address)?, entry.id.as_str())))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut occurrences = BTreeSet::new();
    for range in [
        text.disk_error_pointer_table_offset..text.disk_error_pointer_table_offset + 28,
        text.initialization_error_switch_offset..INITIALIZATION_CONSUMER_END,
    ] {
        let bytes = program
            .get(range.clone())
            .context("MAD system consumer range lies outside the program")?;
        occurrences.extend(
            scan_target_occurrences(bytes, range.start, &targets)
                .into_iter()
                .map(|(offset, _, _)| offset),
        );
    }
    let reference_offsets = references
        .iter()
        .map(|reference| reference.storage_offset)
        .collect::<BTreeSet<_>>();
    ensure!(
        occurrences == reference_offsets,
        "MAD system text has an unclassified target-like address"
    );

    let semantic_reference_count = text.reference_count;
    let machine_code_reference_count = references
        .iter()
        .filter(|reference| {
            matches!(
                reference.kind,
                MadSystemRuntimeReferenceKind::MachineCodeImmediate
                    | MadSystemRuntimeReferenceKind::RuntimeInsertMachineCode
            )
        })
        .count();
    let metadata_reference_count = references.len() - machine_code_reference_count;
    let runtime_insert_reference_count = references
        .iter()
        .filter(|reference| {
            reference.kind == MadSystemRuntimeReferenceKind::RuntimeInsertMachineCode
        })
        .count();
    ensure!(
        semantic_reference_count == 23
            && references.len() == 24
            && machine_code_reference_count == 10
            && metadata_reference_count == 14
            && runtime_insert_reference_count == 1,
        "MAD system runtime reference population changed"
    );

    Ok(MadSystemRuntimeCatalog {
        semantic_reference_count,
        storage_reference_count: references.len(),
        machine_code_reference_count,
        metadata_reference_count,
        runtime_insert_reference_count,
        reference_population_complete: true,
        references,
    })
}

fn require_mov_immediate(
    program: &[u8],
    offset: usize,
    register: Register16,
    target: u16,
    role: &str,
) -> Result<()> {
    let decoded = decode_bytes(
        program
            .get(offset..)
            .with_context(|| format!("MAD system {role} lies outside the program"))?,
    )?;
    ensure!(
        matches!(
            decoded.instruction,
            Instruction::Mov {
                dest: Operand::Reg16(actual),
                src: Operand::Imm16(address),
            } if actual == register && address == target
        ) && decoded.byte_len == 3
            && decoded.prefixes.is_empty(),
        "MAD system {role} is not the verified typed MOV {}, imm16",
        register.name()
    );
    Ok(())
}

fn read_word(program: &[u8], offset: usize) -> Result<u16> {
    let raw: [u8; 2] = program
        .get(offset..offset + 2)
        .context("MAD system reference lies outside the program")?
        .try_into()
        .expect("two bytes convert to a word");
    Ok(u16::from_le_bytes(raw))
}

fn scan_target_occurrences<'a>(
    bytes: &[u8],
    base_offset: usize,
    targets: &BTreeMap<u16, &'a str>,
) -> Vec<(usize, u16, &'a str)> {
    bytes
        .windows(2)
        .enumerate()
        .filter_map(|(relative, window)| {
            let value = u16::from_le_bytes([window[0], window[1]]);
            targets
                .get(&value)
                .map(|id| (base_offset + relative, value, *id))
        })
        .collect()
}

#[cfg(test)]
#[path = "mad_system_runtime_tests.rs"]
mod mad_system_runtime_tests;
