use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::{CallTarget, Instruction, Operand, Register16, decode_bytes};

use super::DialogueGroup;
use crate::game_data::binary::{ensure_prefix, read_u16};

const COM_ORIGIN: usize = 0x100;
const TABLE_INSTRUCTION_OFFSET: usize = 0xaf64;
const EXPECTED_GROUP_COUNT: usize = 11;
const EXPECTED_ENTRY_COUNT: usize = 55;
const EXPECTED_NON_REFERENCE_OCCURRENCES: [(usize, NonReferenceContext); 6] = [
    (0x0112, NonReferenceContext::MovMemory(0x0111)),
    (0x1e94, NonReferenceContext::RelativeCall(0x1e93)),
    (0x3b39, NonReferenceContext::MovMemory(0x3b39)),
    (0x786b, NonReferenceContext::RelativeCall(0x786a)),
    (0x7b5d, NonReferenceContext::RelativeCall(0x7b5c)),
    (0x926b, NonReferenceContext::WordTable(0x9268)),
];

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct DialogueReferenceCatalog {
    pub reference_population_complete: bool,
    pub reference_count: usize,
    pub machine_code_reference_count: usize,
    pub group_pointer_reference_count: usize,
    pub text_pointer_reference_count: usize,
    pub non_reference_occurrence_count: usize,
    pub references: Vec<DialogueReference>,
    pub non_reference_occurrences: Vec<DialogueNonReferenceOccurrence>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct DialogueReference {
    pub id: String,
    pub storage_kind: DialogueReferenceKind,
    pub storage_offset: usize,
    pub instruction_offset: Option<usize>,
    pub target_com_address: u16,
    pub target_id: String,
    pub consumer_role: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DialogueReferenceKind {
    MachineCodeImmediate,
    GroupPointerTableEntry,
    TextPointerRecordField,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct DialogueNonReferenceOccurrence {
    pub storage_offset: usize,
    pub word: u16,
    pub matched_target_id: String,
    pub reason: String,
}

#[derive(Clone, Copy)]
enum NonReferenceContext {
    MovMemory(usize),
    RelativeCall(usize),
    WordTable(usize),
}

pub(super) fn catalog_dialogue_references(
    bytes: &[u8],
    group_pointer_table_offset: usize,
    groups: &[DialogueGroup],
) -> Result<DialogueReferenceCatalog> {
    ensure!(
        groups.len() == EXPECTED_GROUP_COUNT
            && groups
                .iter()
                .map(|group| group.entries.len())
                .sum::<usize>()
                == EXPECTED_ENTRY_COUNT,
        "MAD dialogue reference source population changed"
    );

    let group_table_address = u16::try_from(group_pointer_table_offset + COM_ORIGIN)
        .context("MAD dialogue group table address exceeds 16 bits")?;
    let decoded = decode_bytes(
        bytes
            .get(TABLE_INSTRUCTION_OFFSET..)
            .context("MAD dialogue table instruction lies outside the file")?,
    )?;
    ensure!(
        matches!(
            decoded.instruction,
            Instruction::Mov {
                dest: Operand::Reg16(Register16::BX),
                src: Operand::Imm16(address),
            } if address == group_table_address
        ) && decoded.byte_len == 3
            && decoded.prefixes.is_empty(),
        "MAD dialogue group table base is not the verified typed V30 MOV BX, imm16"
    );

    let mut references = vec![DialogueReference {
        id: "dialogue-group-table-base".to_owned(),
        storage_kind: DialogueReferenceKind::MachineCodeImmediate,
        storage_offset: TABLE_INSTRUCTION_OFFSET + 1,
        instruction_offset: Some(TABLE_INSTRUCTION_OFFSET),
        target_com_address: group_table_address,
        target_id: "dialogue-group-pointer-table".to_owned(),
        consumer_role: "typed V30 group table base feeding the dialogue record consumer".to_owned(),
    }];
    for group in groups {
        let target_com_address = u16::try_from(group.record_table_offset + COM_ORIGIN)
            .context("MAD dialogue record table address exceeds 16 bits")?;
        ensure!(
            read_u16(bytes, group.pointer_table_offset)? == target_com_address,
            "{} group pointer changed before reference cataloging",
            group.id
        );
        references.push(DialogueReference {
            id: format!("{}-record-table", group.id),
            storage_kind: DialogueReferenceKind::GroupPointerTableEntry,
            storage_offset: group.pointer_table_offset,
            instruction_offset: None,
            target_com_address,
            target_id: group.id.clone(),
            consumer_role: "group-indexed dialogue record table pointer".to_owned(),
        });
        for (entry_index, entry) in group.entries.iter().enumerate() {
            let storage_offset = group.record_table_offset + entry_index * 4 + 2;
            let target_com_address = u16::try_from(entry.file_offset + COM_ORIGIN)
                .context("MAD dialogue text address exceeds 16 bits")?;
            ensure!(
                read_u16(bytes, storage_offset)? == target_com_address,
                "{} text pointer changed before reference cataloging",
                entry.id
            );
            references.push(DialogueReference {
                id: format!("{}-text", entry.id),
                storage_kind: DialogueReferenceKind::TextPointerRecordField,
                storage_offset,
                instruction_offset: None,
                target_com_address,
                target_id: entry.id.clone(),
                consumer_role: format!("{} presentation record text pointer", group.id),
            });
        }
    }
    references.sort_by_key(|reference| reference.storage_offset);
    ensure!(
        references
            .windows(2)
            .all(|pair| pair[0].storage_offset != pair[1].storage_offset),
        "MAD dialogue reference storage offsets overlap"
    );

    let targets = references
        .iter()
        .map(|reference| (reference.target_com_address, reference.target_id.as_str()))
        .collect::<BTreeMap<_, _>>();
    let reference_offsets = references
        .iter()
        .map(|reference| reference.storage_offset)
        .collect::<BTreeSet<_>>();
    let non_reference_occurrences = scan_target_occurrences(bytes, &targets)
        .into_iter()
        .filter(|(offset, _, _)| !reference_offsets.contains(offset))
        .map(|(storage_offset, word, matched_target_id)| {
            classify_non_reference(bytes, storage_offset)?;
            Ok(DialogueNonReferenceOccurrence {
                storage_offset,
                word,
                matched_target_id: matched_target_id.to_owned(),
                reason: non_reference_reason(storage_offset).to_owned(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        non_reference_occurrences
            .iter()
            .map(|occurrence| occurrence.storage_offset)
            .eq(EXPECTED_NON_REFERENCE_OCCURRENCES
                .iter()
                .map(|(offset, _)| *offset)),
        "MAD dialogue target-like non-reference population changed"
    );

    let machine_code_reference_count = references
        .iter()
        .filter(|reference| reference.storage_kind == DialogueReferenceKind::MachineCodeImmediate)
        .count();
    let group_pointer_reference_count = references
        .iter()
        .filter(|reference| reference.storage_kind == DialogueReferenceKind::GroupPointerTableEntry)
        .count();
    let text_pointer_reference_count =
        references.len() - machine_code_reference_count - group_pointer_reference_count;
    ensure!(
        machine_code_reference_count == 1
            && group_pointer_reference_count == EXPECTED_GROUP_COUNT
            && text_pointer_reference_count == EXPECTED_ENTRY_COUNT,
        "MAD dialogue structured reference population changed"
    );

    Ok(DialogueReferenceCatalog {
        reference_population_complete: true,
        reference_count: references.len(),
        machine_code_reference_count,
        group_pointer_reference_count,
        text_pointer_reference_count,
        non_reference_occurrence_count: non_reference_occurrences.len(),
        references,
        non_reference_occurrences,
    })
}

fn scan_target_occurrences<'a>(
    bytes: &[u8],
    targets: &BTreeMap<u16, &'a str>,
) -> Vec<(usize, u16, &'a str)> {
    bytes
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

fn classify_non_reference(bytes: &[u8], storage_offset: usize) -> Result<()> {
    let context = EXPECTED_NON_REFERENCE_OCCURRENCES
        .iter()
        .find_map(|(offset, context)| (*offset == storage_offset).then_some(*context))
        .context("unexpected dialogue target-like occurrence")?;
    match context {
        NonReferenceContext::MovMemory(instruction_offset) => {
            let decoded = decode_bytes(
                bytes
                    .get(instruction_offset..)
                    .context("dialogue non-reference MOV lies outside MAD.COM")?,
            )?;
            ensure!(
                matches!(
                    decoded.instruction,
                    Instruction::Mov {
                        dest: Operand::Mem(_),
                        src: Operand::Reg16(Register16::AX),
                    }
                ) && (instruction_offset..instruction_offset + decoded.byte_len)
                    .contains(&storage_offset),
                "dialogue target-like word is no longer inside the verified typed V30 memory MOV"
            );
        }
        NonReferenceContext::RelativeCall(instruction_offset) => {
            let decoded = decode_bytes(
                bytes
                    .get(instruction_offset..)
                    .context("dialogue non-reference CALL lies outside MAD.COM")?,
            )?;
            ensure!(
                matches!(
                    decoded.instruction,
                    Instruction::Call {
                        target: CallTarget::Rel16(_),
                    }
                ) && decoded.byte_len == 3
                    && (instruction_offset..instruction_offset + decoded.byte_len)
                        .contains(&storage_offset),
                "dialogue target-like word is no longer inside the verified typed V30 relative CALL"
            );
        }
        NonReferenceContext::WordTable(table_offset) => ensure_prefix(
            bytes,
            table_offset,
            &[0xa4, 0x96, 0x80, 0x93, 0xb4, 0x96, 0xc4, 0x96],
            "MAD target-like unaligned occurrence inside a word table",
        )?,
    }
    Ok(())
}

fn non_reference_reason(storage_offset: usize) -> &'static str {
    match EXPECTED_NON_REFERENCE_OCCURRENCES
        .iter()
        .find_map(|(offset, context)| (*offset == storage_offset).then_some(*context))
        .expect("classified dialogue non-reference has a reason")
    {
        NonReferenceContext::MovMemory(_) => {
            "target-like bytes overlap a typed V30 memory MOV opcode or address operand"
        }
        NonReferenceContext::RelativeCall(_) => {
            "target-like bytes are a typed V30 relative CALL displacement"
        }
        NonReferenceContext::WordTable(_) => {
            "target-like bytes cross two adjacent values in a verified word table"
        }
    }
}

#[cfg(test)]
#[path = "references_tests.rs"]
mod references_tests;
