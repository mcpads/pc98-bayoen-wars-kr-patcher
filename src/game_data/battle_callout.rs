use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::{CallTarget, Instruction, Operand, Register16, decode_bytes};

use super::gaiji::GaijiCatalog;
use super::interface_text::{InterfaceTextEntry, parse_interface_text_record};
use crate::byte_string::encode_lower_hex;
use crate::source_disk::sha256_hex;

const COM_ORIGIN: usize = 0x100;
const POINTER_TABLE_OFFSET: usize = 0x91f0;
const UNIT_COUNT: usize = 18;
const CALLOUTS_PER_UNIT: usize = 4;
const POINTER_COUNT: usize = UNIT_COUNT * CALLOUTS_PER_UNIT;
const TEXT_START_OFFSET: usize = 0x9280;
const TEXT_END_OFFSET: usize = 0x96a2;
const RENDERER_FILE_OFFSET: usize = 0x91a4;
const RENDERER_COM_ADDRESS: u16 = 0x92a4;
const POINTER_TABLE_LOAD_OFFSET: usize = 0x91ab;
const CALL_SITE_OFFSETS: [usize; 6] = [0x8058, 0x8081, 0x80d9, 0x8119, 0x816c, 0x81c1];

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BattleCalloutCatalog {
    pub pointer_table_offset: usize,
    pub unit_count: usize,
    pub callouts_per_unit: usize,
    pub pointer_count: usize,
    pub text_start_offset: usize,
    pub text_end_offset: usize,
    pub strings_are_contiguous: bool,
    pub entry_count: usize,
    pub referenced_entry_count: usize,
    pub renderer_file_offset: usize,
    pub renderer_com_address: u16,
    pub call_site_count: usize,
    pub calls: Vec<BattleCalloutCall>,
    pub pointers: Vec<BattleCalloutPointer>,
    pub entries: Vec<InterfaceTextEntry>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BattleCalloutPointer {
    pub id: String,
    pub storage_offset: usize,
    pub unit_index: usize,
    pub callout_index: usize,
    pub target_com_address: u16,
    pub target_entry_id: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BattleCalloutCall {
    pub id: String,
    pub file_offset: usize,
    pub com_address: u16,
    pub byte_size: usize,
    pub target_com_address: u16,
}

pub(super) fn parse_battle_callouts(
    program: &[u8],
    gaiji: &GaijiCatalog,
) -> Result<BattleCalloutCatalog> {
    verify_renderer(program)?;
    let mut entries = Vec::new();
    let mut cursor = TEXT_START_OFFSET;
    while cursor < TEXT_END_OFFSET {
        let parsed = parse_interface_text_record(program, cursor, gaiji)
            .with_context(|| format!("invalid MAD battle callout record {}", entries.len() + 1))?;
        ensure!(
            parsed.end_offset <= TEXT_END_OFFSET,
            "MAD battle callout record {} crosses the verified text boundary",
            entries.len() + 1
        );
        let raw = &program[cursor..parsed.end_offset];
        entries.push(InterfaceTextEntry {
            id: format!("battle-callout-{:03}", entries.len() + 1),
            file_offset: cursor,
            com_address: cursor + COM_ORIGIN,
            byte_size: raw.len(),
            sha256: sha256_hex(raw),
            raw_hex: encode_lower_hex(raw),
            text: parsed.text,
            gaiji_glyph_indices: parsed.gaiji_glyph_indices,
            tokens: parsed.tokens,
        });
        cursor = parsed.end_offset;
    }
    ensure!(
        cursor == TEXT_END_OFFSET && entries.len() == 57,
        "MAD battle callout population changed"
    );

    let entries_by_address = entries
        .iter()
        .map(|entry| (entry.com_address as u16, entry))
        .collect::<BTreeMap<_, _>>();
    ensure!(
        entries_by_address.len() == entries.len(),
        "MAD battle callout addresses are not unique"
    );
    let pointers = (0..POINTER_COUNT)
        .map(|index| {
            let storage_offset = POINTER_TABLE_OFFSET + index * 2;
            let bytes = program
                .get(storage_offset..storage_offset + 2)
                .context("MAD battle callout pointer lies outside MAD.COM")?;
            let target_com_address = u16::from_le_bytes([bytes[0], bytes[1]]);
            let target = entries_by_address.get(&target_com_address).with_context(|| {
                format!(
                    "MAD battle callout pointer {index} targets unknown address {target_com_address:#06x}"
                )
            })?;
            Ok(BattleCalloutPointer {
                id: format!("battle-callout-pointer-{:02}-{}", index / 4, index % 4),
                storage_offset,
                unit_index: index / CALLOUTS_PER_UNIT,
                callout_index: index % CALLOUTS_PER_UNIT,
                target_com_address,
                target_entry_id: target.id.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let referenced = pointers
        .iter()
        .map(|pointer| pointer.target_entry_id.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        referenced.len() == entries.len(),
        "MAD battle callout table no longer references every text record"
    );

    let scanned_calls = scan_calls_to(program, RENDERER_COM_ADDRESS);
    ensure!(
        scanned_calls == CALL_SITE_OFFSETS.into_iter().collect(),
        "MAD battle callout renderer-call population changed"
    );
    let calls = CALL_SITE_OFFSETS
        .into_iter()
        .enumerate()
        .map(|(index, file_offset)| catalog_call(program, index, file_offset))
        .collect::<Result<Vec<_>>>()?;

    Ok(BattleCalloutCatalog {
        pointer_table_offset: POINTER_TABLE_OFFSET,
        unit_count: UNIT_COUNT,
        callouts_per_unit: CALLOUTS_PER_UNIT,
        pointer_count: pointers.len(),
        text_start_offset: TEXT_START_OFFSET,
        text_end_offset: TEXT_END_OFFSET,
        strings_are_contiguous: true,
        entry_count: entries.len(),
        referenced_entry_count: referenced.len(),
        renderer_file_offset: RENDERER_FILE_OFFSET,
        renderer_com_address: RENDERER_COM_ADDRESS,
        call_site_count: calls.len(),
        calls,
        pointers,
        entries,
    })
}

fn verify_renderer(program: &[u8]) -> Result<()> {
    let mut cursor = RENDERER_FILE_OFFSET;
    for expected in [
        Instruction::Shl {
            dest: Operand::Reg16(Register16::AX),
            count: v30::ShiftCount::One,
        },
        Instruction::Shl {
            dest: Operand::Reg16(Register16::AX),
            count: v30::ShiftCount::One,
        },
        Instruction::Shl {
            dest: Operand::Reg16(Register16::AX),
            count: v30::ShiftCount::One,
        },
        Instruction::Mov {
            dest: Operand::Reg16(Register16::BX),
            src: Operand::Imm16((POINTER_TABLE_OFFSET + COM_ORIGIN) as u16),
        },
    ] {
        let decoded = decode_bytes(
            program
                .get(cursor..)
                .context("MAD battle callout renderer lies outside MAD.COM")?,
        )?;
        ensure!(
            decoded.instruction == expected && decoded.prefixes.is_empty(),
            "MAD battle callout renderer changed at {cursor:#x}"
        );
        cursor += decoded.byte_len;
    }
    ensure!(
        cursor - 2 == POINTER_TABLE_LOAD_OFFSET,
        "MAD battle callout pointer-table load moved"
    );
    Ok(())
}

fn scan_calls_to(program: &[u8], target: u16) -> BTreeSet<usize> {
    program
        .windows(3)
        .enumerate()
        .filter_map(|(offset, bytes)| {
            (bytes[0] == 0xe8)
                .then(|| {
                    let displacement = i16::from_le_bytes([bytes[1], bytes[2]]);
                    let next = u16::try_from(offset + COM_ORIGIN + 3).ok()?;
                    (next.wrapping_add_signed(displacement) == target).then_some(offset)
                })
                .flatten()
        })
        .collect()
}

fn catalog_call(program: &[u8], index: usize, file_offset: usize) -> Result<BattleCalloutCall> {
    let decoded = decode_bytes(
        program
            .get(file_offset..)
            .context("MAD battle callout call lies outside MAD.COM")?,
    )?;
    let target_com_address = match decoded.instruction {
        Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } if decoded.byte_len == 3 && decoded.prefixes.is_empty() => {
            let next = u16::try_from(file_offset + COM_ORIGIN + 3)?;
            next.wrapping_add_signed(displacement)
        }
        _ => anyhow::bail!("MAD battle callout site {file_offset:#x} is not a near CALL"),
    };
    ensure!(
        target_com_address == RENDERER_COM_ADDRESS,
        "MAD battle callout site {file_offset:#x} targets {target_com_address:#06x}"
    );
    Ok(BattleCalloutCall {
        id: format!("battle-callout-call-{:02}", index + 1),
        file_offset,
        com_address: u16::try_from(file_offset + COM_ORIGIN)?,
        byte_size: decoded.byte_len,
        target_com_address,
    })
}

#[cfg(test)]
#[path = "battle_callout_tests.rs"]
mod battle_callout_tests;
