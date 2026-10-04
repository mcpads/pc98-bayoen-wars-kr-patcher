use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use v30::{Instruction, Operand, Register16, decode_bytes};

use super::model::CompiledDialogueText;
use crate::game_data::{
    DialogueReferenceCatalog, DialogueReferenceKind, GaijiCatalog, parse_dialogue_text_controls,
};
use crate::korean_patch::shared_text::verify_gaiji_bank_readback;

pub(super) fn verify_dialogue_text_payload(
    files: &BTreeMap<String, Vec<u8>>,
    source_gaiji: &GaijiCatalog,
    references: &DialogueReferenceCatalog,
    compiled: &CompiledDialogueText,
) -> Result<()> {
    let gaiji_com = files
        .get("GAIJI.COM")
        .context("dialogue payload is missing GAIJI.COM")?;
    verify_gaiji_bank_readback(gaiji_com, source_gaiji, &compiled.bank)?;

    let mad_com = files
        .get("MAD.COM")
        .context("dialogue payload is missing MAD.COM")?;
    ensure!(
        mad_com.get(compiled.text_region_start..compiled.text_region_end)
            == Some(compiled.text_region_replacement.as_slice()),
        "dialogue text storage failed readback"
    );
    ensure!(
        compiled.text_region_replacement[compiled.packed_storage_bytes..]
            .iter()
            .all(|byte| *byte == 0),
        "dialogue text storage tail is not zero-filled"
    );

    verify_dialogue_text_records_and_references(mad_com, references, compiled)
}

pub(in crate::korean_patch) fn verify_dialogue_text_records_and_references(
    mad_com: &[u8],
    references: &DialogueReferenceCatalog,
    compiled: &CompiledDialogueText,
) -> Result<()> {
    ensure!(
        mad_com.get(compiled.record_table_start..compiled.record_table_end)
            == Some(compiled.record_table_replacement.as_slice()),
        "dialogue record tables failed readback"
    );
    let packed_end = compiled.text_region_start + compiled.packed_storage_bytes;
    ensure!(
        mad_com.get(compiled.text_region_start..packed_end)
            == Some(&compiled.text_region_replacement[..compiled.packed_storage_bytes]),
        "packed dialogue text failed readback"
    );

    for entry in &compiled.entries {
        ensure!(
            mad_com.get(entry.file_offset..entry.file_offset + entry.bytes.len())
                == Some(entry.bytes.as_slice()),
            "{} dialogue bytes failed readback",
            entry.id
        );
        let parsed = parse_dialogue_text_controls(mad_com, entry.file_offset)
            .with_context(|| format!("{} dialogue record failed consumer parsing", entry.id))?;
        ensure!(
            parsed.end_offset == entry.file_offset + entry.bytes.len()
                && parsed.line_count == entry.lines.len(),
            "{} dialogue controls changed after parsing",
            entry.id
        );
    }

    verify_references(mad_com, references, compiled)
}

fn verify_references(
    mad_com: &[u8],
    references: &DialogueReferenceCatalog,
    compiled: &CompiledDialogueText,
) -> Result<()> {
    let addresses = compiled
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry.com_address))
        .collect::<BTreeMap<_, _>>();
    let mut verified_offsets = BTreeSet::new();
    for reference in &references.references {
        let expected_address = match reference.storage_kind {
            DialogueReferenceKind::TextPointerRecordField => addresses
                .get(reference.target_id.as_str())
                .copied()
                .with_context(|| {
                    format!("missing compiled dialogue target {}", reference.target_id)
                })?,
            DialogueReferenceKind::MachineCodeImmediate
            | DialogueReferenceKind::GroupPointerTableEntry => reference.target_com_address,
        };
        let actual_address = match reference.storage_kind {
            DialogueReferenceKind::MachineCodeImmediate => {
                let instruction_offset = reference
                    .instruction_offset
                    .context("dialogue machine reference has no instruction offset")?;
                let decoded = decode_bytes(
                    mad_com
                        .get(instruction_offset..)
                        .context("dialogue machine reference lies outside MAD.COM")?,
                )?;
                match decoded.instruction {
                    Instruction::Mov {
                        dest: Operand::Reg16(Register16::BX),
                        src: Operand::Imm16(address),
                    } if decoded.byte_len == 3 && decoded.prefixes.is_empty() => address,
                    _ => bail!(
                        "dialogue reference at {instruction_offset:#x} failed typed V30 readback"
                    ),
                }
            }
            DialogueReferenceKind::GroupPointerTableEntry
            | DialogueReferenceKind::TextPointerRecordField => {
                let raw: [u8; 2] = mad_com
                    .get(reference.storage_offset..reference.storage_offset + 2)
                    .context("dialogue metadata reference lies outside MAD.COM")?
                    .try_into()
                    .expect("two bytes convert to a word");
                u16::from_le_bytes(raw)
            }
        };
        ensure!(
            actual_address == expected_address,
            "{} dialogue reference targets {actual_address:#x}, expected {expected_address:#x}",
            reference.id
        );
        ensure!(
            verified_offsets.insert(reference.storage_offset),
            "dialogue reference storage was verified twice"
        );
    }
    ensure!(
        verified_offsets.len() == references.reference_count,
        "dialogue reference readback population changed"
    );
    Ok(())
}
