use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;

use crate::localization_assets::decode_all_streams;

const COM_LOAD_ADDRESS: usize = 0x100;
const SLOT_TABLE_FILE_OFFSET: usize = 0x7bdd;
const SLOT_COUNT: usize = 18;
const SLOT_RECORD_SIZE: usize = 6;
const TRANSFER_RECORD_SIZE: usize = 16;
const FINAL_METADATA_ADDRESS: usize = 0x92a4;

const LOADER_FILE_OFFSET: usize = 0x7b47;
const LOADER_SIGNATURE: &[u8] = &[
    0x8b, 0xd8, 0xd1, 0xe3, 0xd1, 0xe3, 0x03, 0xd8, 0x03, 0xd8, 0x53, 0x2e, 0x8b, 0x97, 0xdd, 0x7c,
    0x2e, 0x8b, 0x1e, 0xa9, 0xd5,
];
const RENDERER_FILE_OFFSET: usize = 0x82e9;
const RENDERER_SIGNATURE: &[u8] = &[
    0xe8, 0x39, 0x00, 0x83, 0x3f, 0x01, 0x74, 0x0c, 0x1e, 0x2e, 0x8e, 0x1e, 0xa9, 0xd5, 0xe8, 0x08,
    0x4b, 0x1f,
];
const TRANSFER_FIELDS_FILE_OFFSET: usize = 0x8325;
const TRANSFER_FIELDS_SIGNATURE: &[u8] = &[
    0x8b, 0x77, 0x02, 0x8b, 0x7f, 0x04, 0x8b, 0x57, 0x06, 0x8b, 0x4f, 0x08, 0x8b, 0x47, 0x0e,
];

const EXPECTED_ASSETS: [&str; 21] = [
    "C00", "C01", "C02", "C03", "C04", "C05", "C06", "C07", "C08", "C09", "C10", "C11", "C12",
    "C13", "C15", "C16", "C17", "C18", "C19", "C20", "C21",
];

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct BoundByteRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CharacterSpriteTransfer {
    pub metadata_file_offset: usize,
    pub source_start: usize,
    pub source_end: usize,
    pub destination_vram_offset: usize,
    pub width_pixels: usize,
    pub height: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CharacterSpriteAsset {
    pub name: String,
    pub decoded_size: usize,
    pub consumer_bound_bytes: usize,
    pub unbound_ranges: Vec<BoundByteRange>,
    pub transfers: Vec<CharacterSpriteTransfer>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CharacterSpriteSlot {
    pub slot_index: usize,
    pub primary_asset: String,
    pub secondary_asset: Option<String>,
    pub metadata_file_offset: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CharacterSpriteCatalog {
    pub slot_table_file_offset: usize,
    pub loader_file_offset: usize,
    pub renderer_file_offset: usize,
    pub slot_count: usize,
    pub asset_count: usize,
    pub decoded_byte_count: usize,
    pub consumer_bound_byte_count: usize,
    pub unbound_byte_count: usize,
    pub slots: Vec<CharacterSpriteSlot>,
    pub assets: Vec<CharacterSpriteAsset>,
}

pub(crate) fn catalog_character_sprites(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<CharacterSpriteCatalog> {
    require_signature(
        mad_com,
        LOADER_FILE_OFFSET,
        LOADER_SIGNATURE,
        "dynamic character-asset loader",
    )?;
    require_signature(
        mad_com,
        RENDERER_FILE_OFFSET,
        RENDERER_SIGNATURE,
        "character B/R/G/I renderer",
    )?;
    require_signature(
        mad_com,
        TRANSFER_FIELDS_FILE_OFFSET,
        TRANSFER_FIELDS_SIGNATURE,
        "character transfer-record reader",
    )?;

    let slots = read_slots(mad_com)?;
    let referenced_assets: BTreeSet<_> = slots
        .iter()
        .flat_map(|slot| {
            std::iter::once(slot.primary_asset.as_str()).chain(slot.secondary_asset.as_deref())
        })
        .collect();
    let expected_assets: BTreeSet<_> = EXPECTED_ASSETS.into_iter().collect();
    ensure!(
        referenced_assets == expected_assets,
        "MAD.COM character slot table does not reference the expected C00..C21 asset population"
    );

    let mut decoded_assets = BTreeMap::new();
    for name in referenced_assets {
        let packed = installer_payload
            .get(name)
            .with_context(|| format!("verified installer payload is missing {name}"))?;
        let streams = decode_all_streams(packed)?;
        ensure!(
            streams.len() == 1,
            "character sprite asset {name} does not contain exactly one Compile LZ stream"
        );
        decoded_assets.insert(name.to_owned(), streams.into_iter().next().unwrap().output);
    }

    let mut transfers: BTreeMap<String, Vec<CharacterSpriteTransfer>> = BTreeMap::new();
    let unique_metadata: Vec<_> = slots
        .iter()
        .map(|slot| slot.metadata_file_offset + COM_LOAD_ADDRESS)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    for metadata_index in 0..unique_metadata.len() {
        let metadata_address = unique_metadata[metadata_index];
        let metadata_end = unique_metadata
            .get(metadata_index + 1)
            .copied()
            .unwrap_or(FINAL_METADATA_ADDRESS);
        ensure!(
            metadata_address < metadata_end
                && (metadata_end - metadata_address).is_multiple_of(TRANSFER_RECORD_SIZE),
            "MAD.COM character metadata span is not a whole number of transfer records"
        );
        let slot = slots
            .iter()
            .find(|slot| slot.metadata_file_offset + COM_LOAD_ADDRESS == metadata_address)
            .context("character metadata has no owning slot")?;
        for record_address in (metadata_address..metadata_end).step_by(TRANSFER_RECORD_SIZE) {
            let record_offset = record_address - COM_LOAD_ADDRESS;
            let source_selector = read_u16(mad_com, record_offset)?;
            let asset_name = match source_selector {
                0 => slot.primary_asset.as_str(),
                1 => slot.secondary_asset.as_deref().with_context(|| {
                    format!(
                        "character slot {} selects a missing secondary asset",
                        slot.slot_index
                    )
                })?,
                value => bail!(
                    "character transfer at file offset {record_offset:#x} has unknown source selector {value}"
                ),
            };
            let source_start = usize::from(read_u16(mad_com, record_offset + 2)?);
            let destination_vram_offset = usize::from(read_u16(mad_com, record_offset + 4)?);
            let width_words = usize::from(read_u16(mad_com, record_offset + 6)?);
            let height = usize::from(read_u16(mad_com, record_offset + 8)?);
            ensure!(
                width_words > 0 && height > 0,
                "character transfer at file offset {record_offset:#x} has empty dimensions"
            );
            let transfer_size = width_words
                .checked_mul(2)
                .and_then(|width_bytes| width_bytes.checked_mul(height))
                .and_then(|plane_size| plane_size.checked_mul(4))
                .context("character transfer size overflow")?;
            let source_end = source_start
                .checked_add(transfer_size)
                .context("character transfer source range overflow")?;
            let decoded = decoded_assets
                .get(asset_name)
                .context("character transfer references an undecoded asset")?;
            ensure!(
                source_end <= decoded.len(),
                "character transfer at file offset {record_offset:#x} ends at {source_end:#x}, past {asset_name} decoded size {:#x}",
                decoded.len()
            );
            transfers
                .entry(asset_name.to_owned())
                .or_default()
                .push(CharacterSpriteTransfer {
                    metadata_file_offset: record_offset,
                    source_start,
                    source_end,
                    destination_vram_offset,
                    width_pixels: width_words * 16,
                    height,
                });
        }
    }

    let mut assets = Vec::with_capacity(decoded_assets.len());
    for (name, decoded) in decoded_assets {
        let asset_transfers = transfers.remove(&name).unwrap_or_default();
        ensure!(
            !asset_transfers.is_empty(),
            "character sprite asset {name} has no consumer transfer"
        );
        let bound_ranges = merge_ranges(
            asset_transfers
                .iter()
                .map(|transfer| transfer.source_start..transfer.source_end),
        );
        let consumer_bound_bytes = bound_ranges.iter().map(Range::len).sum();
        let unbound_ranges = complement_ranges(decoded.len(), &bound_ranges)
            .into_iter()
            .map(|range| BoundByteRange {
                start: range.start,
                end: range.end,
            })
            .collect();
        assets.push(CharacterSpriteAsset {
            name,
            decoded_size: decoded.len(),
            consumer_bound_bytes,
            unbound_ranges,
            transfers: asset_transfers,
        });
    }

    let decoded_byte_count = assets.iter().map(|asset| asset.decoded_size).sum();
    let consumer_bound_byte_count = assets.iter().map(|asset| asset.consumer_bound_bytes).sum();
    let unbound_byte_count = decoded_byte_count - consumer_bound_byte_count;

    Ok(CharacterSpriteCatalog {
        slot_table_file_offset: SLOT_TABLE_FILE_OFFSET,
        loader_file_offset: LOADER_FILE_OFFSET,
        renderer_file_offset: RENDERER_FILE_OFFSET,
        slot_count: slots.len(),
        asset_count: assets.len(),
        decoded_byte_count,
        consumer_bound_byte_count,
        unbound_byte_count,
        slots,
        assets,
    })
}

fn read_slots(mad_com: &[u8]) -> Result<Vec<CharacterSpriteSlot>> {
    let mut slots = Vec::with_capacity(SLOT_COUNT);
    for slot_index in 0..SLOT_COUNT {
        let record_offset = SLOT_TABLE_FILE_OFFSET + slot_index * SLOT_RECORD_SIZE;
        let primary_address = read_u16(mad_com, record_offset)?;
        let secondary_address = read_u16(mad_com, record_offset + 2)?;
        let metadata_address = usize::from(read_u16(mad_com, record_offset + 4)?);
        let primary_asset = read_com_string(mad_com, primary_address)?;
        let secondary_asset = if secondary_address == u16::MAX {
            None
        } else {
            Some(read_com_string(mad_com, secondary_address)?)
        };
        let metadata_file_offset = metadata_address
            .checked_sub(COM_LOAD_ADDRESS)
            .context("character metadata address is below the COM load address")?;
        slots.push(CharacterSpriteSlot {
            slot_index,
            primary_asset,
            secondary_asset,
            metadata_file_offset,
        });
    }
    Ok(slots)
}

fn read_com_string(program: &[u8], address: u16) -> Result<String> {
    let offset = usize::from(address)
        .checked_sub(COM_LOAD_ADDRESS)
        .context("character asset name address is below the COM load address")?;
    let tail = program
        .get(offset..)
        .with_context(|| format!("character asset name starts outside MAD.COM at {offset:#x}"))?;
    let length = tail
        .iter()
        .position(|byte| *byte == 0)
        .context("character asset name has no null terminator")?;
    let name = std::str::from_utf8(&tail[..length]).context("character asset name is not ASCII")?;
    ensure!(!name.is_empty(), "character asset name is empty");
    Ok(name.to_owned())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let raw: [u8; 2] = bytes
        .get(offset..offset + 2)
        .with_context(|| format!("truncated 16-bit field at file offset {offset:#x}"))?
        .try_into()
        .expect("a two-byte range converts to a two-byte array");
    Ok(u16::from_le_bytes(raw))
}

fn merge_ranges(ranges: impl IntoIterator<Item = Range<usize>>) -> Vec<Range<usize>> {
    let mut ranges: Vec<_> = ranges.into_iter().collect();
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in ranges {
        if let Some(previous) = merged.last_mut()
            && range.start <= previous.end
        {
            previous.end = previous.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    merged
}

fn complement_ranges(size: usize, bound: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut unbound = Vec::new();
    let mut cursor = 0;
    for range in bound {
        if cursor < range.start {
            unbound.push(cursor..range.start);
        }
        cursor = range.end;
    }
    if cursor < size {
        unbound.push(cursor..size);
    }
    unbound
}

fn require_signature(program: &[u8], offset: usize, signature: &[u8], role: &str) -> Result<()> {
    ensure!(
        program.get(offset..offset + signature.len()) == Some(signature),
        "MAD.COM {role} signature does not match at file offset {offset:#x}"
    );
    Ok(())
}

#[cfg(test)]
#[path = "character_sprites_tests.rs"]
mod character_sprites_tests;
