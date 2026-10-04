use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use encoding_rs::SHIFT_JIS;
use serde::Serialize;

use super::binary::{com_address_to_file_offset, ensure_prefix, read_u16};
use crate::byte_string::encode_lower_hex;
use crate::source_disk::sha256_hex;

const DISK_ERROR_CONSUMER_OFFSET: usize = 0x2ca1;
const DISK_ERROR_TABLE_ADDRESS: usize = 0x2dc8;
const DISK_ERROR_COUNT: usize = 14;
const INITIALIZATION_ERROR_SWITCH_OFFSET: usize = 0x3058;

const INITIALIZATION_ERROR_REFERENCES: [(usize, usize, &str); 9] = [
    (0x2e14, 0x3068, "drive_not_ready"),
    (0x2e14, 0x307a, "drive_not_ready"),
    (0x2e83, 0x3085, "insufficient_memory"),
    (0x2ed3, 0x309c, "nmouse_not_resident"),
    (0x2efb, 0x30a7, "sound_driver_not_resident"),
    (0x2f25, 0x30b2, "sampling_driver_not_resident"),
    (0x2f52, 0x30bd, "gaiji_not_installed"),
    (0x2e4c, 0x30c3, "generic_error"),
    (0x2e59, 0x30e8, "data_file_not_found"),
];

const DYNAMIC_FILE_NAME_ADDRESS: usize = 0x2e59;
const DYNAMIC_FILE_NAME_INSERT_OFFSET: usize = 11;
const DYNAMIC_FILE_NAME_CAPACITY: usize = 8;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadSystemTextCatalog {
    pub disk_error_consumer_offset: usize,
    pub disk_error_pointer_table_offset: usize,
    pub disk_error_pointer_count: usize,
    pub initialization_error_switch_offset: usize,
    pub unique_text_count: usize,
    pub reference_count: usize,
    pub entries: Vec<MadSystemTextEntry>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadSystemTextEntry {
    pub id: String,
    pub file_offset: usize,
    pub com_address: usize,
    pub byte_size: usize,
    pub raw_sha256: String,
    pub raw_hex: String,
    pub text: String,
    pub runtime_insert: Option<RuntimeTextInsert>,
    pub references: Vec<MadSystemTextReference>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeTextInsert {
    pub role: String,
    pub byte_offset: usize,
    pub byte_capacity: usize,
    pub consumer_file_offset: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadSystemTextReference {
    pub role: String,
    pub consumer_file_offset: usize,
    pub table_index: Option<usize>,
}

#[derive(Debug)]
struct TextBuilder {
    file_offset: usize,
    com_address: usize,
    bytes: Vec<u8>,
    references: Vec<MadSystemTextReference>,
}

pub(super) fn parse_mad_system_text(bytes: &[u8]) -> Result<MadSystemTextCatalog> {
    require_consumers(bytes)?;

    let table_offset = com_address_to_file_offset(DISK_ERROR_TABLE_ADDRESS)?;
    let mut entries = BTreeMap::new();
    for index in 0..DISK_ERROR_COUNT {
        let pointer_offset = table_offset
            .checked_add(index * 2)
            .context("MAD disk-error pointer-table offset overflow")?;
        let address = read_u16(bytes, pointer_offset)? as usize;
        add_reference(
            bytes,
            &mut entries,
            address,
            MadSystemTextReference {
                role: "dos_critical_error_pointer_table".to_owned(),
                consumer_file_offset: DISK_ERROR_CONSUMER_OFFSET,
                table_index: Some(index),
            },
        )?;
    }

    for (address, consumer_file_offset, role) in INITIALIZATION_ERROR_REFERENCES {
        add_reference(
            bytes,
            &mut entries,
            address,
            MadSystemTextReference {
                role: role.to_owned(),
                consumer_file_offset,
                table_index: None,
            },
        )?;
    }

    let entries: Vec<_> = entries
        .into_values()
        .enumerate()
        .map(|(index, entry)| {
            let text = decode_system_text(&entry.bytes, entry.file_offset)?;
            let runtime_insert = (entry.com_address == DYNAMIC_FILE_NAME_ADDRESS)
                .then(|| validate_dynamic_file_name_insert(&entry.bytes))
                .transpose()?;
            Ok(MadSystemTextEntry {
                id: format!("mad-system-text-{:03}", index + 1),
                file_offset: entry.file_offset,
                com_address: entry.com_address,
                byte_size: entry.bytes.len(),
                raw_sha256: sha256_hex(&entry.bytes),
                raw_hex: encode_lower_hex(&entry.bytes),
                text,
                runtime_insert,
                references: entry.references,
            })
        })
        .collect::<Result<_>>()?;
    let reference_count = entries.iter().map(|entry| entry.references.len()).sum();

    Ok(MadSystemTextCatalog {
        disk_error_consumer_offset: DISK_ERROR_CONSUMER_OFFSET,
        disk_error_pointer_table_offset: table_offset,
        disk_error_pointer_count: DISK_ERROR_COUNT,
        initialization_error_switch_offset: INITIALIZATION_ERROR_SWITCH_OFFSET,
        unique_text_count: entries.len(),
        reference_count,
        entries,
    })
}

fn require_consumers(bytes: &[u8]) -> Result<()> {
    ensure_prefix(
        bytes,
        DISK_ERROR_CONSUMER_OFFSET,
        &[
            0x51, 0x52, 0x56, 0x57, 0x55, 0x1e, 0x06, 0x2e, 0x8b, 0x1e, 0xcc, 0xd5, 0xd1, 0xe3,
            0x8b, 0x97, 0xc8, 0x2d, 0xb4, 0x09, 0xcd, 0x21,
        ],
        "MAD DOS critical-error consumer",
    )?;
    ensure_prefix(
        bytes,
        INITIALIZATION_ERROR_SWITCH_OFFSET,
        &[
            0xfc, 0x03, 0x74, 0x24, 0x80, 0xfc, 0x01, 0x75, 0x0d, 0x3c, 0x02, 0x75, 0x03, 0xeb,
            0x62, 0x90, 0xba, 0x14, 0x2e,
        ],
        "MAD initialization-error switch",
    )?;
    ensure_prefix(
        bytes,
        0x30c9,
        &[
            0xb9, 0x0b, 0x00, 0x33, 0xd2, 0x33, 0xc0, 0x2e, 0x8b, 0x1e, 0xc7, 0xd5, 0x03, 0xda,
            0x8a, 0x07, 0x3c, 0x2e, 0x74, 0x0b, 0xbb, 0x59, 0x2e,
        ],
        "MAD missing-data-file name inserter",
    )?;
    Ok(())
}

fn add_reference(
    bytes: &[u8],
    entries: &mut BTreeMap<usize, TextBuilder>,
    address: usize,
    reference: MadSystemTextReference,
) -> Result<()> {
    let file_offset = com_address_to_file_offset(address)?;
    let raw = read_dos_text(bytes, file_offset)?;
    if let Some(existing) = entries.get_mut(&file_offset) {
        ensure!(
            existing.bytes == raw,
            "MAD system text at file offset {file_offset:#x} has conflicting boundaries"
        );
        existing.references.push(reference);
        return Ok(());
    }

    entries.insert(
        file_offset,
        TextBuilder {
            file_offset,
            com_address: address,
            bytes: raw.to_vec(),
            references: vec![reference],
        },
    );
    Ok(())
}

fn read_dos_text(bytes: &[u8], file_offset: usize) -> Result<&[u8]> {
    let tail = bytes
        .get(file_offset..)
        .with_context(|| format!("MAD system text starts outside the file at {file_offset:#x}"))?;
    let end = tail
        .iter()
        .position(|byte| *byte == b'$')
        .with_context(|| format!("unterminated MAD system text at file offset {file_offset:#x}"))?;
    Ok(&tail[..end])
}

fn decode_system_text(bytes: &[u8], file_offset: usize) -> Result<String> {
    let decoded = SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(bytes)
        .with_context(|| {
            format!("invalid Shift_JIS system text at file offset {file_offset:#x}")
        })?;
    Ok(decoded.into_owned())
}

fn validate_dynamic_file_name_insert(bytes: &[u8]) -> Result<RuntimeTextInsert> {
    let insert_end = DYNAMIC_FILE_NAME_INSERT_OFFSET + DYNAMIC_FILE_NAME_CAPACITY;
    ensure!(
        bytes.get(DYNAMIC_FILE_NAME_INSERT_OFFSET..insert_end) == Some(&[b' '; 8]),
        "MAD missing-data-file text does not preserve its 8-byte runtime insertion field"
    );
    ensure!(
        bytes.get(insert_end..insert_end + 4) == Some(b".DAT"),
        "MAD missing-data-file text has no .DAT suffix after its runtime insertion field"
    );
    Ok(RuntimeTextInsert {
        role: "data_file_stem".to_owned(),
        byte_offset: DYNAMIC_FILE_NAME_INSERT_OFFSET,
        byte_capacity: DYNAMIC_FILE_NAME_CAPACITY,
        consumer_file_offset: 0x30c9,
    })
}

#[cfg(test)]
#[path = "mad_system_text_tests.rs"]
mod mad_system_text_tests;
