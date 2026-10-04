use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use v30::{Instruction, Operand, Register16, decode_bytes};

use super::model::CompiledMadSystemText;
use crate::game_data::{
    GaijiCatalog, MadSystemRuntimeCatalog, MadSystemRuntimeReferenceKind, parse_gaiji_program,
};
use crate::korean_patch::shared_text::verify_gaiji_bank_readback;

pub(in crate::korean_patch) fn verify_mad_system_text_payload(
    files: &BTreeMap<String, Vec<u8>>,
    source_gaiji: &GaijiCatalog,
    runtime: &MadSystemRuntimeCatalog,
    compiled: &CompiledMadSystemText,
) -> Result<()> {
    let gaiji_com = files
        .get("GAIJI.COM")
        .context("MAD system payload is missing GAIJI.COM")?;
    verify_gaiji_bank_readback(gaiji_com, source_gaiji, &compiled.bank)?;
    parse_gaiji_program(gaiji_com)?;
    let mad_com = files
        .get("MAD.COM")
        .context("MAD system payload is missing MAD.COM")?;
    verify_mad_system_text_records_and_references(mad_com, runtime, compiled)
}

pub(in crate::korean_patch) fn verify_mad_system_text_records_and_references(
    mad_com: &[u8],
    runtime: &MadSystemRuntimeCatalog,
    compiled: &CompiledMadSystemText,
) -> Result<()> {
    ensure!(
        mad_com.get(compiled.text_region_start..compiled.text_region_end)
            == Some(compiled.text_region_replacement.as_slice()),
        "MAD system text storage failed readback"
    );
    ensure!(
        compiled.text_region_replacement[compiled.populated_storage_bytes..]
            .iter()
            .all(|byte| *byte == 0),
        "MAD system text storage tail is not zero-filled"
    );
    for entry in &compiled.entries {
        let bytes = mad_com
            .get(entry.file_offset..entry.file_offset + entry.bytes.len())
            .with_context(|| format!("{} lies outside MAD.COM", entry.id))?;
        ensure!(bytes == entry.bytes, "{} bytes failed readback", entry.id);
        ensure!(
            bytes.last() == Some(&b'$') && !bytes[..bytes.len() - 1].contains(&b'$'),
            "{} changed its DOS terminator boundary",
            entry.id
        );
        if let (Some(offset), Some(capacity)) = (
            entry.runtime_insert_byte_offset,
            entry.runtime_insert_byte_capacity,
        ) {
            let end = offset + capacity;
            ensure!(
                bytes
                    .get(offset..end)
                    .is_some_and(|field| field.iter().all(|byte| *byte == b' '))
                    && bytes.get(end..end + 4) == Some(b".DAT"),
                "{} runtime insert field failed readback",
                entry.id
            );
        }
    }

    let addresses = compiled
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry.com_address))
        .collect::<BTreeMap<_, _>>();
    let mut verified_offsets = BTreeSet::new();
    for reference in &runtime.references {
        let expected_address = addresses[reference.target_entry_id.as_str()];
        let actual_address = match reference.kind {
            MadSystemRuntimeReferenceKind::MetadataTableEntry => {
                read_word(mad_com, reference.storage_offset)?
            }
            MadSystemRuntimeReferenceKind::MachineCodeImmediate
            | MadSystemRuntimeReferenceKind::RuntimeInsertMachineCode => {
                let instruction_offset = reference
                    .instruction_offset
                    .context("MAD system machine reference has no instruction offset")?;
                let decoded = decode_bytes(
                    mad_com
                        .get(instruction_offset..)
                        .context("MAD system machine reference lies outside MAD.COM")?,
                )?;
                let expected_register =
                    if reference.kind == MadSystemRuntimeReferenceKind::RuntimeInsertMachineCode {
                        Register16::BX
                    } else {
                        Register16::DX
                    };
                match decoded.instruction {
                    Instruction::Mov {
                        dest: Operand::Reg16(actual_register),
                        src: Operand::Imm16(address),
                    } if actual_register == expected_register
                        && decoded.byte_len == 3
                        && decoded.prefixes.is_empty() =>
                    {
                        address
                    }
                    _ => anyhow::bail!(
                        "MAD system reference at {instruction_offset:#x} failed typed V30 readback"
                    ),
                }
            }
        };
        ensure!(
            actual_address == expected_address,
            "{} targets {actual_address:#x}, expected {expected_address:#x}",
            reference.id
        );
        ensure!(
            verified_offsets.insert(reference.storage_offset),
            "MAD system reference storage was verified twice"
        );
    }
    ensure!(
        verified_offsets.len() == runtime.storage_reference_count,
        "MAD system reference readback population changed"
    );
    Ok(())
}

fn read_word(bytes: &[u8], offset: usize) -> Result<u16> {
    let raw: [u8; 2] = bytes
        .get(offset..offset + 2)
        .context("MAD system metadata reference lies outside MAD.COM")?
        .try_into()
        .expect("two bytes convert to a word");
    Ok(u16::from_le_bytes(raw))
}
