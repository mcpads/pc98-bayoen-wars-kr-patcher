use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::localization_assets::decode_all_streams;

const COM_LOAD_ADDRESS: usize = 0x100;
const MAP_LOADER_FILE_OFFSET: usize = 0xa499;
const MAP_LOADER_SIGNATURE: &[u8] = &[
    0x2e, 0x8b, 0x97, 0xad, 0xa5, 0x2e, 0x8b, 0x1e, 0xc5, 0xd5, 0xe8, 0x2a, 0x8a,
];
const MAP_POINTER_TABLE_FILE_OFFSET: usize = 0xa4ad;
const MAP_COUNT: usize = 10;
const MAP_DATA_CONSUMER_FILE_OFFSET: usize = 0x29cf;
const MAP_DATA_CONSUMER_SIGNATURE: &[u8] = &[
    0x1e, 0x2e, 0x8e, 0x1e, 0xc5, 0xd5, 0xbe, 0x5a, 0x00, 0x83, 0xc6, 0x02, 0xbb, 0x36, 0xe9,
];
const OVERVIEW_RENDERER_FILE_OFFSET: usize = 0xa53f;
const OVERVIEW_RENDERER_SIGNATURE: &[u8] = &[
    0x1e, 0x2e, 0x8e, 0x1e, 0xc5, 0xd5, 0x33, 0xf6, 0x8b, 0xd7, 0xb8, 0x00, 0xa8, 0xe8, 0x1a, 0x00,
];
const DECODED_MAP_SIZE: usize = 4_502;
const OVERVIEW_WIDTH_PIXELS: usize = 160;
const OVERVIEW_HEIGHT: usize = 40;
const OVERVIEW_PLANE_SIZE: usize = 20 * OVERVIEW_HEIGHT;
const OVERVIEW_BYTE_SIZE: usize = OVERVIEW_PLANE_SIZE * 4;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BattleMapAsset {
    pub name: String,
    pub decoded_size: usize,
    pub overview_width_pixels: usize,
    pub overview_height: usize,
    pub overview_byte_size: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BattleMapDuplicate {
    pub name: String,
    pub identical_to: String,
    pub byte_size: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BattleMapCatalog {
    pub map_loader_file_offset: usize,
    pub map_data_consumer_file_offset: usize,
    pub overview_renderer_file_offset: usize,
    pub map_count: usize,
    pub decoded_map_size: usize,
    pub maps: Vec<BattleMapAsset>,
    pub archive_duplicate: BattleMapDuplicate,
}

pub(crate) fn catalog_battle_maps(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<BattleMapCatalog> {
    require_signature(
        mad_com,
        MAP_LOADER_FILE_OFFSET,
        MAP_LOADER_SIGNATURE,
        "battle-map loader",
    )?;
    require_signature(
        mad_com,
        MAP_DATA_CONSUMER_FILE_OFFSET,
        MAP_DATA_CONSUMER_SIGNATURE,
        "battle-map data consumer",
    )?;
    require_signature(
        mad_com,
        OVERVIEW_RENDERER_FILE_OFFSET,
        OVERVIEW_RENDERER_SIGNATURE,
        "battle-map overview renderer",
    )?;

    let mut names = Vec::with_capacity(MAP_COUNT);
    for index in 0..MAP_COUNT {
        let address = read_u16(mad_com, MAP_POINTER_TABLE_FILE_OFFSET + index * 2)?;
        names.push(read_com_string(mad_com, address)?);
    }
    let expected: BTreeSet<_> = (1..=MAP_COUNT)
        .map(|index| format!("BWM{index}.DAT"))
        .collect();
    ensure!(
        names.iter().cloned().collect::<BTreeSet<_>>() == expected,
        "MAD.COM battle-map pointer table does not cover BWM1.DAT through BWM10.DAT"
    );

    let mut maps = Vec::with_capacity(MAP_COUNT);
    for name in names {
        let packed = installer_payload
            .get(&name)
            .with_context(|| format!("verified installer payload is missing {name}"))?;
        let streams = decode_all_streams(packed)?;
        ensure!(
            streams.len() == 1 && streams[0].output.len() == DECODED_MAP_SIZE,
            "{name} does not decode to one {DECODED_MAP_SIZE}-byte battle-map stream"
        );
        maps.push(BattleMapAsset {
            name,
            decoded_size: DECODED_MAP_SIZE,
            overview_width_pixels: OVERVIEW_WIDTH_PIXELS,
            overview_height: OVERVIEW_HEIGHT,
            overview_byte_size: OVERVIEW_BYTE_SIZE,
        });
    }

    let mapb = installer_payload
        .get("MAPB.DAT")
        .context("verified installer payload is missing MAPB.DAT")?;
    let bwm10 = installer_payload
        .get("BWM10.DAT")
        .context("verified installer payload is missing BWM10.DAT")?;
    ensure!(
        mapb == bwm10,
        "MAPB.DAT is not the expected byte-identical archive duplicate of BWM10.DAT"
    );

    Ok(BattleMapCatalog {
        map_loader_file_offset: MAP_LOADER_FILE_OFFSET,
        map_data_consumer_file_offset: MAP_DATA_CONSUMER_FILE_OFFSET,
        overview_renderer_file_offset: OVERVIEW_RENDERER_FILE_OFFSET,
        map_count: maps.len(),
        decoded_map_size: DECODED_MAP_SIZE,
        maps,
        archive_duplicate: BattleMapDuplicate {
            name: "MAPB.DAT".to_owned(),
            identical_to: "BWM10.DAT".to_owned(),
            byte_size: mapb.len(),
        },
    })
}

fn read_com_string(program: &[u8], address: u16) -> Result<String> {
    let offset = usize::from(address)
        .checked_sub(COM_LOAD_ADDRESS)
        .context("battle-map name address is below the COM load address")?;
    let tail = program
        .get(offset..)
        .with_context(|| format!("battle-map name starts outside MAD.COM at {offset:#x}"))?;
    let length = tail
        .iter()
        .position(|byte| *byte == 0)
        .context("battle-map name has no null terminator")?;
    Ok(std::str::from_utf8(&tail[..length])
        .context("battle-map name is not ASCII")?
        .to_owned())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let raw: [u8; 2] = bytes
        .get(offset..offset + 2)
        .with_context(|| format!("truncated 16-bit field at file offset {offset:#x}"))?
        .try_into()
        .expect("a two-byte range converts to a two-byte array");
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
#[path = "battle_maps_tests.rs"]
mod battle_maps_tests;
