use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use v30::{Instruction, Operand, decode_bytes};

use super::model::CompiledInterfaceText;
use crate::game_data::{
    GaijiCatalog, InterfaceTextReferenceCatalog, InterfaceTextReferenceKind, InterfaceTextToken,
    parse_gaiji_program, parse_interface_text_record,
};
use crate::korean_patch::shared_text::verify_gaiji_bank_readback;

pub(in crate::korean_patch) fn verify_interface_text_payload(
    files: &BTreeMap<String, Vec<u8>>,
    source_gaiji: &GaijiCatalog,
    references: &InterfaceTextReferenceCatalog,
    compiled: &CompiledInterfaceText,
) -> Result<()> {
    let gaiji_com = files
        .get("GAIJI.COM")
        .context("interface payload is missing GAIJI.COM")?;
    verify_gaiji_bank_readback(gaiji_com, source_gaiji, &compiled.bank)?;
    let gaiji = parse_gaiji_program(gaiji_com)?;
    let mad_com = files
        .get("MAD.COM")
        .context("interface payload is missing MAD.COM")?;
    verify_interface_text_records_and_references(mad_com, &gaiji, references, compiled)
}

pub(in crate::korean_patch) fn verify_interface_text_records_and_references(
    mad_com: &[u8],
    gaiji: &GaijiCatalog,
    references: &InterfaceTextReferenceCatalog,
    compiled: &CompiledInterfaceText,
) -> Result<()> {
    ensure!(
        mad_com.get(compiled.text_region_start..compiled.text_region_end)
            == Some(compiled.text_region_replacement.as_slice()),
        "interface text storage failed readback"
    );
    ensure!(
        compiled.text_region_replacement[compiled.populated_storage_bytes..]
            .iter()
            .all(|byte| *byte == 0),
        "interface text storage tail is not zero-filled"
    );

    for entry in &compiled.entries {
        ensure!(
            mad_com.get(entry.file_offset..entry.file_offset + entry.bytes.len())
                == Some(entry.bytes.as_slice()),
            "{} interface bytes failed readback",
            entry.id
        );
        let parsed = parse_interface_text_record(mad_com, entry.file_offset, gaiji)
            .with_context(|| format!("{} interface record failed consumer parsing", entry.id))?;
        ensure!(
            parsed.end_offset == entry.file_offset + entry.bytes.len(),
            "{} interface record changed consumer boundary",
            entry.id
        );
        let line_break_count = parsed
            .tokens
            .iter()
            .filter(|token| matches!(token, InterfaceTextToken::LineBreak))
            .count();
        let attributes = parsed
            .tokens
            .iter()
            .filter_map(|token| match token {
                InterfaceTextToken::DisplayAttribute { code } => Some(*code),
                _ => None,
            })
            .collect::<Vec<_>>();
        let expected_attributes = entry
            .leading_attributes
            .iter()
            .chain(&entry.trailing_attributes)
            .copied()
            .collect::<Vec<_>>();
        ensure!(
            line_break_count + 1 == entry.lines.len() && attributes == expected_attributes,
            "{} interface controls changed after parsing",
            entry.id
        );
    }

    let addresses = compiled
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry.com_address))
        .collect::<BTreeMap<_, _>>();
    let mut verified_storage_offsets = BTreeSet::new();
    for reference in &references.references {
        let expected_address = addresses[reference.target_entry_id.as_str()];
        let actual_address = match reference.storage_kind {
            InterfaceTextReferenceKind::MetadataTableEntry => {
                let raw: [u8; 2] = mad_com
                    .get(reference.storage_offset..reference.storage_offset + 2)
                    .context("interface metadata reference lies outside MAD.COM")?
                    .try_into()
                    .expect("two bytes convert to a word");
                u16::from_le_bytes(raw)
            }
            InterfaceTextReferenceKind::MachineCodeImmediate => {
                let instruction_offset = reference
                    .instruction_offset
                    .context("interface machine reference has no instruction offset")?;
                let decoded = decode_bytes(
                    mad_com
                        .get(instruction_offset..)
                        .context("interface machine reference lies outside MAD.COM")?,
                )?;
                match decoded.instruction {
                    Instruction::Mov {
                        dest: Operand::Reg16(_),
                        src: Operand::Imm16(address),
                    } if decoded.byte_len == 3 && decoded.prefixes.is_empty() => address,
                    _ => anyhow::bail!(
                        "interface reference at {instruction_offset:#x} failed typed V30 readback"
                    ),
                }
            }
        };
        ensure!(
            actual_address == expected_address,
            "{} interface reference targets {actual_address:#x}, expected {expected_address:#x}",
            reference.id
        );
        ensure!(
            verified_storage_offsets.insert(reference.storage_offset),
            "interface reference storage was verified twice"
        );
    }
    ensure!(
        verified_storage_offsets.len() == references.reference_count,
        "interface reference readback population changed"
    );
    super::super::shared_alert_window::verify_shared_alert_window(
        mad_com,
        &compiled.stage_result_window,
    )?;
    super::super::shared_alert_window::verify_shared_alert_window(
        mad_com,
        &compiled.shared_alert_window,
    )?;
    super::super::spring_capture_window::verify_spring_capture_window(
        mad_com,
        &compiled.spring_capture_window,
    )?;
    super::super::spring_recovery_window::verify_spring_recovery_window(
        mad_com,
        &compiled.spring_recovery_window,
    )?;
    Ok(())
}
