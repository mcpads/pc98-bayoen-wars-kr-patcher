use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use encoding_rs::SHIFT_JIS;
use serde::Serialize;

use super::binary::{com_address_to_file_offset, ensure_prefix, read_u16};
use crate::byte_string::encode_lower_hex;
use crate::source_disk::sha256_hex;

use super::menu_runtime::{MenuRuntimeCatalog, catalog_menu_runtime};

const TEXT_RENDERER_OFFSET: usize = 0x0c90;
const DISK_ERROR_TABLE_CONSUMER_OFFSET: usize = 0x050d;
const DISK_ERROR_TABLE_ADDRESS: usize = 0x0671;
const DISK_ERROR_COUNT: usize = 13;
const PROCESS_ERROR_SWITCH_OFFSET: usize = 0x07da;
const FLOPPY_ERROR_TABLE_CONSUMER_OFFSET: usize = 0x0e81;
const FLOPPY_ERROR_TABLE_ADDRESS: usize = 0x0fae;
const FLOPPY_ERROR_COUNT: usize = 18;

const PROCESS_ERROR_ADDRESSES: [usize; 6] = [0x0964, 0x0973, 0x098c, 0x09a1, 0x09b4, 0x09c3];
const FIXED_RENDERER_STRINGS: [(usize, u8, &str, usize); 4] = [
    (0x0282, b'@', "wrong_disk_script", 0x0158),
    (0x079d, b'$', "disk_retry_prompt", 0x0619),
    (0x0f35, b'$', "hard_disk_guard", 0x0e2d),
    (0x0f8f, b'$', "floppy_drive_label", 0x0e73),
];

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MenuProgramCatalog {
    pub sha256: String,
    pub file_size: usize,
    pub text_renderer_offset: usize,
    pub text: MenuTextCatalog,
    pub runtime: MenuRuntimeCatalog,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MenuTextCatalog {
    pub unique_text_count: usize,
    pub reference_count: usize,
    pub entries: Vec<MenuTextEntry>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MenuTextEntry {
    pub id: String,
    pub file_offset: usize,
    pub com_address: usize,
    pub byte_size: usize,
    pub terminator_hex: String,
    pub raw_sha256: String,
    pub raw_hex: String,
    pub text: String,
    pub references: Vec<MenuTextReference>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MenuTextReference {
    pub role: String,
    pub consumer_file_offset: usize,
    pub table_index: Option<usize>,
}

#[derive(Debug)]
struct TextBuilder {
    file_offset: usize,
    com_address: usize,
    bytes: Vec<u8>,
    terminator: u8,
    references: Vec<MenuTextReference>,
}

pub(super) fn parse_menu_program(bytes: &[u8]) -> Result<MenuProgramCatalog> {
    require_consumers(bytes)?;

    let mut entries = BTreeMap::new();
    add_direct_dos_prints(bytes, &mut entries)?;
    add_pointer_table(
        bytes,
        &mut entries,
        DISK_ERROR_TABLE_ADDRESS,
        DISK_ERROR_COUNT,
        "disk_error_pointer_table",
        DISK_ERROR_TABLE_CONSUMER_OFFSET,
    )?;
    for (index, address) in PROCESS_ERROR_ADDRESSES.into_iter().enumerate() {
        add_text_reference(
            bytes,
            &mut entries,
            address,
            b'$',
            MenuTextReference {
                role: "process_error_switch".to_owned(),
                consumer_file_offset: PROCESS_ERROR_SWITCH_OFFSET,
                table_index: Some(index),
            },
        )?;
    }
    add_pointer_table(
        bytes,
        &mut entries,
        FLOPPY_ERROR_TABLE_ADDRESS,
        FLOPPY_ERROR_COUNT,
        "floppy_bios_error_pointer_table",
        FLOPPY_ERROR_TABLE_CONSUMER_OFFSET,
    )?;
    for (address, terminator, role, consumer_file_offset) in FIXED_RENDERER_STRINGS {
        add_text_reference(
            bytes,
            &mut entries,
            address,
            terminator,
            MenuTextReference {
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
            let decoded = decode_menu_text(&entry.bytes)?;
            Ok(MenuTextEntry {
                id: format!("menu-text-{:03}", index + 1),
                file_offset: entry.file_offset,
                com_address: entry.com_address,
                byte_size: entry.bytes.len(),
                terminator_hex: format!("{:02X}", entry.terminator),
                raw_sha256: sha256_hex(&entry.bytes),
                raw_hex: encode_lower_hex(&entry.bytes),
                text: decoded.replace('@', "\n"),
                references: entry.references,
            })
        })
        .collect::<Result<_>>()?;
    let reference_count = entries.iter().map(|entry| entry.references.len()).sum();

    let runtime = catalog_menu_runtime(bytes, &entries, reference_count)?;
    Ok(MenuProgramCatalog {
        sha256: sha256_hex(bytes),
        file_size: bytes.len(),
        text_renderer_offset: TEXT_RENDERER_OFFSET,
        text: MenuTextCatalog {
            unique_text_count: entries.len(),
            reference_count,
            entries,
        },
        runtime,
    })
}

fn require_consumers(bytes: &[u8]) -> Result<()> {
    ensure_prefix(
        bytes,
        TEXT_RENDERER_OFFSET,
        &[0x8a, 0x3e, 0x87, 0x12, 0x8a, 0x1e, 0x88, 0x12],
        "MENU Shift_JIS renderer",
    )?;
    ensure_prefix(
        bytes,
        DISK_ERROR_TABLE_CONSUMER_OFFSET,
        &[0xbb, 0x71, 0x06, 0x03, 0xc0, 0x03, 0xd8, 0x8b, 0x37],
        "MENU disk-error pointer-table consumer",
    )?;
    ensure_prefix(
        bytes,
        PROCESS_ERROR_SWITCH_OFFSET,
        &[0xba, 0x73, 0x09, 0x3d, 0x01, 0x00],
        "MENU process-error switch",
    )?;
    ensure_prefix(
        bytes,
        FLOPPY_ERROR_TABLE_CONSUMER_OFFSET,
        &[0xbb, 0xae, 0x0f, 0x03, 0xc0, 0x03, 0xd8, 0x2e, 0x8b, 0x37],
        "MENU floppy-error pointer-table consumer",
    )?;
    Ok(())
}

fn add_direct_dos_prints(bytes: &[u8], entries: &mut BTreeMap<usize, TextBuilder>) -> Result<()> {
    for offset in 0..bytes.len().saturating_sub(6) {
        let address = if bytes[offset] == 0xba
            && bytes.get(offset + 3..offset + 7) == Some(&[0xb4, 0x09, 0xcd, 0x21])
        {
            Some(read_u16(bytes, offset + 1)? as usize)
        } else if bytes.get(offset..offset + 3) == Some(&[0xb4, 0x09, 0xba])
            && bytes.get(offset + 5..offset + 7) == Some(&[0xcd, 0x21])
        {
            Some(read_u16(bytes, offset + 3)? as usize)
        } else {
            None
        };
        let Some(address) = address else {
            continue;
        };
        let raw = read_terminated(bytes, address, b'$')?;
        let decoded = decode_menu_text(raw)?;
        if !contains_japanese(&decoded) {
            continue;
        }
        add_text_reference(
            bytes,
            entries,
            address,
            b'$',
            MenuTextReference {
                role: "dos_ah09_direct".to_owned(),
                consumer_file_offset: offset,
                table_index: None,
            },
        )?;
    }
    Ok(())
}

fn add_pointer_table(
    bytes: &[u8],
    entries: &mut BTreeMap<usize, TextBuilder>,
    table_address: usize,
    count: usize,
    role: &str,
    consumer_file_offset: usize,
) -> Result<()> {
    let table_offset = com_address_to_file_offset(table_address)?;
    for index in 0..count {
        let pointer_offset = table_offset
            .checked_add(index * 2)
            .context("MENU pointer-table offset overflow")?;
        let address = read_u16(bytes, pointer_offset)? as usize;
        add_text_reference(
            bytes,
            entries,
            address,
            b'$',
            MenuTextReference {
                role: role.to_owned(),
                consumer_file_offset,
                table_index: Some(index),
            },
        )?;
    }
    Ok(())
}

fn add_text_reference(
    bytes: &[u8],
    entries: &mut BTreeMap<usize, TextBuilder>,
    address: usize,
    terminator: u8,
    reference: MenuTextReference,
) -> Result<()> {
    let file_offset = com_address_to_file_offset(address)?;
    let raw = read_terminated(bytes, address, terminator)?;
    if let Some(existing) = entries.get_mut(&file_offset) {
        ensure!(
            existing.terminator == terminator && existing.bytes == raw,
            "MENU text at file offset 0x{file_offset:X} has conflicting boundaries"
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
            terminator,
            references: vec![reference],
        },
    );
    Ok(())
}

fn read_terminated(bytes: &[u8], address: usize, terminator: u8) -> Result<&[u8]> {
    let file_offset = com_address_to_file_offset(address)?;
    let tail = bytes
        .get(file_offset..)
        .with_context(|| format!("MENU text address 0x{address:04X} is outside the program"))?;
    let end = tail
        .iter()
        .position(|byte| *byte == terminator)
        .with_context(|| {
            format!(
                "MENU text at file offset 0x{file_offset:X} has no 0x{terminator:02X} terminator"
            )
        })?;
    Ok(&tail[..end])
}

fn decode_menu_text(bytes: &[u8]) -> Result<String> {
    let decoded = SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(bytes)
        .context("MENU text is not valid Shift_JIS")?;
    Ok(decoded.into_owned())
}

fn contains_japanese(text: &str) -> bool {
    text.chars().any(|character| {
        matches!(
            character as u32,
            0x3040..=0x30ff | 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xff66..=0xff9f
        )
    })
}

#[cfg(test)]
#[path = "menu_tests.rs"]
mod menu_tests;
