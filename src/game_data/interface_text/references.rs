use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::{Instruction, Operand, Register16, decode_bytes};

use super::InterfaceTextEntry;
use crate::game_data::binary::read_u16;

const MACHINE_CODE_REFERENCE_OFFSETS: [usize; 44] = [
    0x42b0, 0x42bb, 0x4453, 0x445c, 0x4466, 0x446f, 0x4478, 0x4482, 0x448b, 0x5723, 0x573f, 0x575b,
    0x5771, 0x5787, 0x579d, 0x57b0, 0x57c3, 0x57de, 0x57fe, 0x5816, 0x5821, 0x5834, 0x5847, 0x5863,
    0x586e, 0x5881, 0x588c, 0x589f, 0x58b2, 0x58c5, 0x58d8, 0x58eb, 0x58fe, 0x5911, 0x5935, 0x5948,
    0x5977, 0x5991, 0x599d, 0x59b7, 0x59c2, 0x59ce, 0x59d9, 0x66cf,
];

const METADATA_TABLES: [MetadataTableSpec; 5] = [
    MetadataTableSpec {
        id: "selected-unit-name-pointers",
        offset: 0x42d6,
        entry_count: 18,
        stride: 2,
    },
    MetadataTableSpec {
        id: "resource-level-label-pointers",
        offset: 0x42fa,
        entry_count: 10,
        stride: 2,
    },
    MetadataTableSpec {
        id: "terrain-name-pointers",
        offset: 0x4322,
        entry_count: 13,
        stride: 2,
    },
    MetadataTableSpec {
        id: "interface-label-layout-records",
        offset: 0x5644,
        entry_count: 37,
        stride: 4,
    },
    MetadataTableSpec {
        id: "map-unit-name-pointers",
        offset: 0x6712,
        entry_count: 17,
        stride: 2,
    },
];

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct InterfaceTextReferenceCatalog {
    pub reference_population_complete: bool,
    pub reference_count: usize,
    pub machine_code_reference_count: usize,
    pub metadata_reference_count: usize,
    pub referenced_entry_count: usize,
    pub unreferenced_entry_ids: Vec<String>,
    pub metadata_tables: Vec<InterfaceTextReferenceTable>,
    pub references: Vec<InterfaceTextReference>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct InterfaceTextReferenceTable {
    pub id: String,
    pub offset: usize,
    pub entry_count: usize,
    pub stride: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct InterfaceTextReference {
    pub id: String,
    pub storage_kind: InterfaceTextReferenceKind,
    pub storage_offset: usize,
    pub instruction_offset: Option<usize>,
    pub target_com_address: u16,
    pub target_entry_id: String,
    pub consumer_role: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceTextReferenceKind {
    MachineCodeImmediate,
    MetadataTableEntry,
}

#[derive(Clone, Copy)]
struct MetadataTableSpec {
    id: &'static str,
    offset: usize,
    entry_count: usize,
    stride: usize,
}

pub(super) fn catalog_interface_text_references(
    program: &[u8],
    entries: &[InterfaceTextEntry],
) -> Result<InterfaceTextReferenceCatalog> {
    let entries_by_address = entries
        .iter()
        .map(|entry| (entry.com_address as u16, entry))
        .collect::<BTreeMap<_, _>>();
    ensure!(
        entries_by_address.len() == entries.len(),
        "MAD interface text has duplicate COM addresses"
    );

    let expected_machine_offsets = MACHINE_CODE_REFERENCE_OFFSETS
        .into_iter()
        .collect::<BTreeSet<_>>();
    let scanned_machine_offsets = scan_machine_code_immediates(program, &entries_by_address);
    ensure!(
        scanned_machine_offsets == expected_machine_offsets,
        "MAD interface machine-code reference population changed"
    );

    let mut references = Vec::new();
    for instruction_offset in MACHINE_CODE_REFERENCE_OFFSETS {
        references.push(machine_code_reference(
            program,
            instruction_offset,
            &entries_by_address,
        )?);
    }
    for table in METADATA_TABLES {
        references.extend(metadata_table_references(
            program,
            table,
            &entries_by_address,
        )?);
    }
    references.sort_by_key(|reference| reference.storage_offset);
    ensure!(
        references
            .windows(2)
            .all(|pair| pair[0].storage_offset != pair[1].storage_offset),
        "MAD interface reference storage offsets overlap"
    );

    let referenced_ids = references
        .iter()
        .map(|reference| reference.target_entry_id.as_str())
        .collect::<BTreeSet<_>>();
    let unreferenced_entry_ids = entries
        .iter()
        .filter(|entry| !referenced_ids.contains(entry.id.as_str()))
        .map(|entry| entry.id.clone())
        .collect::<Vec<_>>();
    ensure!(
        unreferenced_entry_ids == ["interface-text-006"],
        "MAD interface unreferenced text population changed"
    );
    let machine_code_reference_count = references
        .iter()
        .filter(|reference| {
            reference.storage_kind == InterfaceTextReferenceKind::MachineCodeImmediate
        })
        .count();
    let metadata_reference_count = references.len() - machine_code_reference_count;

    Ok(InterfaceTextReferenceCatalog {
        reference_population_complete: true,
        reference_count: references.len(),
        machine_code_reference_count,
        metadata_reference_count,
        referenced_entry_count: referenced_ids.len(),
        unreferenced_entry_ids,
        metadata_tables: METADATA_TABLES
            .iter()
            .map(|table| InterfaceTextReferenceTable {
                id: table.id.to_owned(),
                offset: table.offset,
                entry_count: table.entry_count,
                stride: table.stride,
            })
            .collect(),
        references,
    })
}

fn scan_machine_code_immediates(
    program: &[u8],
    entries_by_address: &BTreeMap<u16, &InterfaceTextEntry>,
) -> BTreeSet<usize> {
    program
        .windows(3)
        .enumerate()
        .filter_map(|(offset, bytes)| {
            let opcode = bytes[0];
            let address = u16::from_le_bytes([bytes[1], bytes[2]]);
            ((0xb8..=0xbf).contains(&opcode) && entries_by_address.contains_key(&address))
                .then_some(offset)
        })
        .collect()
}

fn machine_code_reference(
    program: &[u8],
    instruction_offset: usize,
    entries_by_address: &BTreeMap<u16, &InterfaceTextEntry>,
) -> Result<InterfaceTextReference> {
    let decoded = decode_bytes(
        program
            .get(instruction_offset..)
            .context("MAD interface reference instruction lies outside the file")?,
    )
    .with_context(|| {
        format!("MAD interface reference at {instruction_offset:#x} is not typed V30 code")
    })?;
    let (register, target_com_address) = match decoded.instruction {
        Instruction::Mov {
            dest: Operand::Reg16(register),
            src: Operand::Imm16(address),
        } => (register, address),
        _ => anyhow::bail!(
            "MAD interface reference at {instruction_offset:#x} is not MOV reg16, imm16"
        ),
    };
    ensure!(
        decoded.byte_len == 3 && decoded.prefixes.is_empty(),
        "MAD interface reference at {instruction_offset:#x} changed instruction encoding"
    );
    ensure!(
        matches!(register, Register16::BX | Register16::DX),
        "MAD interface reference at {instruction_offset:#x} uses an unexpected register"
    );
    let target = entries_by_address
        .get(&target_com_address)
        .with_context(|| {
            format!(
                "MAD interface reference at {instruction_offset:#x} targets unknown address {target_com_address:#x}"
            )
        })?;
    Ok(InterfaceTextReference {
        id: format!("interface-code-reference-{instruction_offset:04x}"),
        storage_kind: InterfaceTextReferenceKind::MachineCodeImmediate,
        storage_offset: instruction_offset + 1,
        instruction_offset: Some(instruction_offset),
        target_com_address,
        target_entry_id: target.id.clone(),
        consumer_role: format!(
            "typed V30 MOV {} immediate feeding an interface text renderer",
            register.name()
        ),
    })
}

fn metadata_table_references(
    program: &[u8],
    table: MetadataTableSpec,
    entries_by_address: &BTreeMap<u16, &InterfaceTextEntry>,
) -> Result<Vec<InterfaceTextReference>> {
    (0..table.entry_count)
        .map(|index| {
            let storage_offset = table.offset + index * table.stride;
            let target_com_address = read_u16(program, storage_offset)?;
            let target = entries_by_address
                .get(&target_com_address)
                .with_context(|| {
                    format!(
                        "MAD interface table {} entry {} targets unknown address {target_com_address:#x}",
                        table.id,
                        index + 1
                    )
                })?;
            Ok(InterfaceTextReference {
                id: format!("{}-{:02}", table.id, index + 1),
                storage_kind: InterfaceTextReferenceKind::MetadataTableEntry,
                storage_offset,
                instruction_offset: None,
                target_com_address,
                target_entry_id: target.id.clone(),
                consumer_role: table.id.to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "references_tests.rs"]
mod references_tests;
