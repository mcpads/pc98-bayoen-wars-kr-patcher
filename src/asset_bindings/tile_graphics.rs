use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::localization_assets::decode_all_streams;

const MOUSE_LOADER_FILE_OFFSET: usize = 0x2a1b;
const MOUSE_LOADER_SIGNATURE: &[u8] = &[
    0xba, 0x7e, 0xca, 0x2e, 0x8b, 0x1e, 0xb1, 0xd5, 0xe8, 0xaa, 0x04, 0xc3,
];
const MOUSE_TILE_CONSUMER_FILE_OFFSET: usize = 0x3894;
const MOUSE_TILE_CONSUMER_SIGNATURE: &[u8] = &[
    0x1e, 0x8b, 0xf1, 0x2e, 0x8e, 0x1e, 0xb1, 0xd5, 0x8b, 0xfa, 0xba, 0x02, 0x00, 0xb9, 0x10, 0x00,
];
const MOUSE_CURSOR_CONSUMER_FILE_OFFSET: usize = 0x4161;
const MOUSE_CURSOR_CONSUMER_SIGNATURE: &[u8] = &[
    0x1e, 0xbe, 0x00, 0x46, 0x2e, 0x8e, 0x1e, 0xb1, 0xd5, 0x8b, 0xf8, 0xba, 0x03, 0x00, 0xb9, 0x30,
    0x00,
];
const FRAME_LOADER_FILE_OFFSET: usize = 0x2a27;
const FRAME_LOADER_SIGNATURE: &[u8] = &[
    0xba, 0x91, 0xca, 0x2e, 0x8b, 0x1e, 0x93, 0xd5, 0xe8, 0x9e, 0x04, 0xba, 0x9c, 0xca, 0x2e, 0x8b,
    0x1e, 0x95, 0xd5, 0xe8, 0x93, 0x04, 0xc3,
];
const FRAME_MAP_CONSUMER_FILE_OFFSET: usize = 0x44a1;
const FRAME_MAP_CONSUMER_SIGNATURE: &[u8] = &[
    0x8e, 0x06, 0x95, 0xd5, 0x26, 0x8b, 0x1e, 0x00, 0x00, 0x33, 0xc9, 0x26, 0x8a, 0x27, 0x32, 0xc0,
];
const BACKGROUND_LOADER_FILE_OFFSET: usize = 0xad9a;
const BACKGROUND_LOADER_SIGNATURE: &[u8] = &[
    0xba, 0x88, 0xca, 0x2e, 0x8b, 0x1e, 0xad, 0xd5, 0xe8, 0x2b, 0x81,
];
const BACKGROUND_TILE_CONSUMER_FILE_OFFSET: usize = 0xbd19;
const BACKGROUND_TILE_CONSUMER_SIGNATURE: &[u8] = &[
    0x1e, 0xbe, 0x80, 0x53, 0x2e, 0x8e, 0x1e, 0xad, 0xd5, 0xe8, 0xb9, 0x13, 0x1f, 0xc3,
];
const PORTRAIT_LOADER_FILE_OFFSET: usize = 0x28cd;
const PORTRAIT_LOADER_SIGNATURE: &[u8] = &[
    0xba, 0x2b, 0xcb, 0x2e, 0x8b, 0x1e, 0xa1, 0xd5, 0xe8, 0xf8, 0x05,
];
const PORTRAIT_CONSUMER_FILE_OFFSET: usize = 0x52b6;
const PORTRAIT_CONSUMER_SIGNATURE: &[u8] = &[
    0xb9, 0x40, 0x00, 0x1e, 0x2e, 0x8e, 0x1e, 0xa1, 0xd5, 0xe8, 0x40, 0x7b, 0x1f, 0xc3,
];

const COMPACT_TILE_WIDTH: usize = 16;
const COMPACT_TILE_HEIGHT: usize = 16;
const COMPACT_TILE_SIZE: usize = COMPACT_TILE_WIDTH / 8 * COMPACT_TILE_HEIGHT * 4;
const MOUSE_TILE_WIDTH: usize = 32;
const MOUSE_TILE_HEIGHT: usize = 16;
const MASKED_TILE_SIZE: usize = MOUSE_TILE_WIDTH / 8 * MOUSE_TILE_HEIGHT * 5;
const MOUSE_TILE_COUNT: usize = 56;
const MOUSE_CURSOR_WIDTH: usize = 48;
const MOUSE_CURSOR_HEIGHT: usize = 48;
const MOUSE_CURSOR_OFFSET: usize = MOUSE_TILE_COUNT * MASKED_TILE_SIZE;
const MOUSE_CURSOR_SIZE: usize = MOUSE_CURSOR_WIDTH / 8 * MOUSE_CURSOR_HEIGHT * 5;
const FRAME_TILE_COUNT: usize = 182;
const FRAME_MAP_WIDTH_TILES: usize = 10;
const FRAME_MAP_HEIGHT_TILES: usize = 25;
const BACKGROUND_TILE_COUNT: usize = 256;
const PORTRAIT_WIDTH: usize = 64;
const PORTRAIT_HEIGHT: usize = 64;
const PORTRAIT_COUNT: usize = 18;
const PORTRAIT_SIZE: usize = PORTRAIT_WIDTH / 8 * PORTRAIT_HEIGHT * 4;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CompactTileSetBinding {
    pub name: String,
    pub decoded_size: usize,
    pub tile_width: usize,
    pub tile_height: usize,
    pub tile_count: usize,
    pub consumer_file_offset: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MaskedTileSetBinding {
    pub name: String,
    pub decoded_size: usize,
    pub tile_width: usize,
    pub tile_height: usize,
    pub tile_count: usize,
    pub cursor_offset: usize,
    pub cursor_width: usize,
    pub cursor_height: usize,
    pub tile_consumer_file_offset: usize,
    pub cursor_consumer_file_offset: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TileMapBinding {
    pub name: String,
    pub decoded_size: usize,
    pub entry_offset: usize,
    pub width_tiles: usize,
    pub height_tiles: usize,
    pub entry_count: usize,
    pub maximum_tile_index: usize,
    pub consumer_file_offset: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct TileGraphicsCatalog {
    pub mouse: MaskedTileSetBinding,
    pub frame_tiles: CompactTileSetBinding,
    pub frame_map: TileMapBinding,
    pub background_tiles: CompactTileSetBinding,
    pub portraits: CompactTileSetBinding,
}

pub(crate) fn catalog_tile_graphics(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<TileGraphicsCatalog> {
    require_signature(
        mad_com,
        MOUSE_LOADER_FILE_OFFSET,
        MOUSE_LOADER_SIGNATURE,
        "mouse-graphics loader",
    )?;
    require_signature(
        mad_com,
        MOUSE_TILE_CONSUMER_FILE_OFFSET,
        MOUSE_TILE_CONSUMER_SIGNATURE,
        "mouse-tile consumer",
    )?;
    require_signature(
        mad_com,
        MOUSE_CURSOR_CONSUMER_FILE_OFFSET,
        MOUSE_CURSOR_CONSUMER_SIGNATURE,
        "mouse-cursor consumer",
    )?;
    require_signature(
        mad_com,
        FRAME_LOADER_FILE_OFFSET,
        FRAME_LOADER_SIGNATURE,
        "frame-graphics loader",
    )?;
    require_signature(
        mad_com,
        FRAME_MAP_CONSUMER_FILE_OFFSET,
        FRAME_MAP_CONSUMER_SIGNATURE,
        "frame-map consumer",
    )?;
    require_signature(
        mad_com,
        BACKGROUND_LOADER_FILE_OFFSET,
        BACKGROUND_LOADER_SIGNATURE,
        "background-tile loader",
    )?;
    require_signature(
        mad_com,
        BACKGROUND_TILE_CONSUMER_FILE_OFFSET,
        BACKGROUND_TILE_CONSUMER_SIGNATURE,
        "background-tile consumer",
    )?;
    require_signature(
        mad_com,
        PORTRAIT_LOADER_FILE_OFFSET,
        PORTRAIT_LOADER_SIGNATURE,
        "portrait loader",
    )?;
    require_signature(
        mad_com,
        PORTRAIT_CONSUMER_FILE_OFFSET,
        PORTRAIT_CONSUMER_SIGNATURE,
        "portrait consumer",
    )?;

    let mouse = decode_single(installer_payload, "MOUSE.DAT")?;
    ensure!(
        mouse.len() == MOUSE_CURSOR_OFFSET + MOUSE_CURSOR_SIZE,
        "MOUSE.DAT does not contain the consumer-bound tile and cursor records"
    );

    let frame_tiles = decode_single(installer_payload, "WAKU_P.DAT")?;
    ensure!(
        frame_tiles.len() == FRAME_TILE_COUNT * COMPACT_TILE_SIZE,
        "WAKU_P.DAT is not the expected compact 16x16 tile set"
    );
    let frame_map = decode_single(installer_payload, "WAKU_IMG.DAT")?;
    let entry_offset = usize::from(read_u16(&frame_map, 0)?);
    let entry_count = FRAME_MAP_WIDTH_TILES * FRAME_MAP_HEIGHT_TILES;
    ensure!(
        entry_offset + entry_count == frame_map.len(),
        "WAKU_IMG.DAT does not consist of one pointer and a complete 10x25 tile map"
    );
    let maximum_tile_index = frame_map[entry_offset..]
        .iter()
        .copied()
        .max()
        .map(usize::from)
        .context("WAKU_IMG.DAT tile map is empty")?;
    ensure!(
        maximum_tile_index < FRAME_TILE_COUNT,
        "WAKU_IMG.DAT references a tile outside WAKU_P.DAT"
    );

    let background = decode_single(installer_payload, "BG__.DAT")?;
    ensure!(
        background.len() == BACKGROUND_TILE_COUNT * COMPACT_TILE_SIZE,
        "BG__.DAT is not the expected 256-tile compact set"
    );

    let portraits = decode_single(installer_payload, "KAO")?;
    ensure!(
        portraits.len() == PORTRAIT_COUNT * PORTRAIT_SIZE,
        "KAO is not the expected eighteen-record portrait set"
    );

    Ok(TileGraphicsCatalog {
        mouse: MaskedTileSetBinding {
            name: "MOUSE.DAT".to_owned(),
            decoded_size: mouse.len(),
            tile_width: MOUSE_TILE_WIDTH,
            tile_height: MOUSE_TILE_HEIGHT,
            tile_count: MOUSE_TILE_COUNT,
            cursor_offset: MOUSE_CURSOR_OFFSET,
            cursor_width: MOUSE_CURSOR_WIDTH,
            cursor_height: MOUSE_CURSOR_HEIGHT,
            tile_consumer_file_offset: MOUSE_TILE_CONSUMER_FILE_OFFSET,
            cursor_consumer_file_offset: MOUSE_CURSOR_CONSUMER_FILE_OFFSET,
        },
        frame_tiles: CompactTileSetBinding {
            name: "WAKU_P.DAT".to_owned(),
            decoded_size: frame_tiles.len(),
            tile_width: COMPACT_TILE_WIDTH,
            tile_height: COMPACT_TILE_HEIGHT,
            tile_count: FRAME_TILE_COUNT,
            consumer_file_offset: FRAME_MAP_CONSUMER_FILE_OFFSET,
        },
        frame_map: TileMapBinding {
            name: "WAKU_IMG.DAT".to_owned(),
            decoded_size: frame_map.len(),
            entry_offset,
            width_tiles: FRAME_MAP_WIDTH_TILES,
            height_tiles: FRAME_MAP_HEIGHT_TILES,
            entry_count,
            maximum_tile_index,
            consumer_file_offset: FRAME_MAP_CONSUMER_FILE_OFFSET,
        },
        background_tiles: CompactTileSetBinding {
            name: "BG__.DAT".to_owned(),
            decoded_size: background.len(),
            tile_width: COMPACT_TILE_WIDTH,
            tile_height: COMPACT_TILE_HEIGHT,
            tile_count: BACKGROUND_TILE_COUNT,
            consumer_file_offset: BACKGROUND_TILE_CONSUMER_FILE_OFFSET,
        },
        portraits: CompactTileSetBinding {
            name: "KAO".to_owned(),
            decoded_size: portraits.len(),
            tile_width: PORTRAIT_WIDTH,
            tile_height: PORTRAIT_HEIGHT,
            tile_count: PORTRAIT_COUNT,
            consumer_file_offset: PORTRAIT_CONSUMER_FILE_OFFSET,
        },
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
        .context("truncated tile-map pointer")?
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
#[path = "tile_graphics_tests.rs"]
mod tile_graphics_tests;
