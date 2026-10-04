use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::localization_assets::decode_all_streams;

const LOADER_FILE_OFFSET: usize = 0xadb0;
const LOADER_SIGNATURE: &[u8] = &[
    0xba, 0x2f, 0xcb, 0x2e, 0x8b, 0x1e, 0xc1, 0xd5, 0xe8, 0x15, 0x81,
];
const CONSUMER_FILE_OFFSET: usize = 0x2329;
const CONSUMER_SIGNATURE: &[u8] = &[
    0x2e, 0x8e, 0x1e, 0xc1, 0xd5, 0x33, 0xdb, 0xf6, 0x46, 0x13, 0x01, 0x75, 0x03, 0x83, 0xc3, 0x0c,
    0x03, 0xd8, 0x8b, 0x1f,
];
const POINTER_COUNT: usize = 12;
const POINTER_TABLE_SIZE: usize = POINTER_COUNT * 2;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MovementTablePointer {
    pub index: usize,
    pub section_offset: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MovementTableSection {
    pub start: usize,
    pub end: usize,
    pub word_count: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MovementTableCatalog {
    pub name: String,
    pub decoded_size: usize,
    pub loader_file_offset: usize,
    pub consumer_file_offset: usize,
    pub pointer_table_size: usize,
    pub pointers: Vec<MovementTablePointer>,
    pub sections: Vec<MovementTableSection>,
}

pub(crate) fn catalog_movement_tables(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<MovementTableCatalog> {
    require_signature(
        mad_com,
        LOADER_FILE_OFFSET,
        LOADER_SIGNATURE,
        "movement-table loader",
    )?;
    require_signature(
        mad_com,
        CONSUMER_FILE_OFFSET,
        CONSUMER_SIGNATURE,
        "movement-table word consumer",
    )?;
    let decoded = decode_single(installer_payload, "ST")?;
    ensure!(
        decoded.len() >= POINTER_TABLE_SIZE && decoded.len().is_multiple_of(2),
        "ST is not a complete word-aligned movement table"
    );

    let pointers = (0..POINTER_COUNT)
        .map(|index| {
            let section_offset = usize::from(read_u16(&decoded, index * 2)?);
            ensure!(
                section_offset >= POINTER_TABLE_SIZE
                    && section_offset < decoded.len()
                    && section_offset.is_multiple_of(2),
                "ST pointer {index} does not address a word-aligned payload section"
            );
            Ok(MovementTablePointer {
                index,
                section_offset,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let unique_offsets = pointers
        .iter()
        .map(|pointer| pointer.section_offset)
        .collect::<BTreeSet<_>>();
    ensure!(
        unique_offsets.first().copied() == Some(POINTER_TABLE_SIZE),
        "ST leaves bytes between its pointer table and first payload section"
    );
    let starts = unique_offsets.into_iter().collect::<Vec<_>>();
    let sections = starts
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let end = starts.get(index + 1).copied().unwrap_or(decoded.len());
            ensure!(
                end > *start && (end - start).is_multiple_of(2),
                "invalid ST section boundary"
            );
            Ok(MovementTableSection {
                start: *start,
                end,
                word_count: (end - start) / 2,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(MovementTableCatalog {
        name: "ST".to_owned(),
        decoded_size: decoded.len(),
        loader_file_offset: LOADER_FILE_OFFSET,
        consumer_file_offset: CONSUMER_FILE_OFFSET,
        pointer_table_size: POINTER_TABLE_SIZE,
        pointers,
        sections,
    })
}

fn decode_single(installer_payload: &BTreeMap<String, Vec<u8>>, name: &str) -> Result<Vec<u8>> {
    let packed = installer_payload
        .get(name)
        .with_context(|| format!("verified installer payload is missing {name}"))?;
    let streams = decode_all_streams(packed)?;
    ensure!(
        streams.len() == 1,
        "{name} does not have exactly one packed stream"
    );
    Ok(streams.into_iter().next().unwrap().output)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let raw: [u8; 2] = bytes
        .get(offset..offset + 2)
        .context("truncated movement-table word")?
        .try_into()
        .expect("a two-byte range converts to an array");
    Ok(u16::from_le_bytes(raw))
}

fn require_signature(program: &[u8], offset: usize, signature: &[u8], role: &str) -> Result<()> {
    ensure!(
        program.get(offset..offset + signature.len()) == Some(signature),
        "MAD.COM {role} signature does not match at file offset {offset:#x}"
    );
    Ok(())
}

#[cfg(test)]
#[path = "movement_tables_tests.rs"]
mod movement_tables_tests;
