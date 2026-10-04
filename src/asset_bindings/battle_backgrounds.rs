use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::localization_assets::decode_all_streams;

const INITIAL_LOADER_FILE_OFFSET: usize = 0xadbb;
const INITIAL_LOADER_SIGNATURE: &[u8] = &[
    0xba, 0x23, 0xcb, 0x2e, 0x8b, 0x1e, 0x97, 0xd5, 0xe8, 0x0a, 0x81, 0xe8, 0x0a, 0x00,
];
const ALTERNATE_LOADER_FILE_OFFSET: usize = 0xae09;
const ALTERNATE_LOADER_SIGNATURE: &[u8] = &[
    0xba, 0x23, 0xcb, 0x2e, 0x8b, 0x1e, 0x97, 0xd5, 0xe8, 0xbc, 0x80, 0xeb, 0x0b, 0xba, 0x27, 0xcb,
];
const INITIAL_RENDERER_FILE_OFFSET: usize = 0xadd3;
const INITIAL_RENDERER_SIGNATURE: &[u8] = &[
    0x1e, 0xbe, 0x00, 0x00, 0x2e, 0x8e, 0x1e, 0x97, 0xd5, 0xbf, 0x00, 0x00, 0xba, 0x08, 0x00, 0xb9,
    0x80, 0x00,
];
const SELECTION_TABLE_FILE_OFFSET: usize = 0xae64;
const SELECTION_OFFSETS: [u16; 10] = [
    0x2000, 0x4000, 0x6000, 0x8000, 0xa000, 0xc000, 0x0000, 0x2000, 0x4000, 0x6000,
];
const FRAME_WIDTH: usize = 128;
const FRAME_HEIGHT: usize = 128;
const FRAME_SIZE: usize = FRAME_WIDTH / 8 * FRAME_HEIGHT * 4;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BattleBackgroundAsset {
    pub name: String,
    pub decoded_size: usize,
    pub frame_width: usize,
    pub frame_height: usize,
    pub frame_count: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BattleBackgroundCatalog {
    pub initial_loader_file_offset: usize,
    pub alternate_loader_file_offset: usize,
    pub renderer_file_offset: usize,
    pub selection_table_file_offset: usize,
    pub assets: Vec<BattleBackgroundAsset>,
}

pub(crate) fn catalog_battle_backgrounds(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<BattleBackgroundCatalog> {
    require_signature(
        mad_com,
        INITIAL_LOADER_FILE_OFFSET,
        INITIAL_LOADER_SIGNATURE,
        "initial battle-background loader",
    )?;
    require_signature(
        mad_com,
        ALTERNATE_LOADER_FILE_OFFSET,
        ALTERNATE_LOADER_SIGNATURE,
        "alternate battle-background loader",
    )?;
    require_signature(
        mad_com,
        INITIAL_RENDERER_FILE_OFFSET,
        INITIAL_RENDERER_SIGNATURE,
        "battle-background renderer",
    )?;
    let observed_offsets = (0..SELECTION_OFFSETS.len())
        .map(|index| read_u16(mad_com, SELECTION_TABLE_FILE_OFFSET + index * 2))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        observed_offsets == SELECTION_OFFSETS,
        "MAD.COM battle-background selection table changed"
    );

    let assets = [("B04", 7usize), ("B05", 4usize)]
        .into_iter()
        .map(|(name, frame_count)| {
            let decoded = decode_single(installer_payload, name)?;
            ensure!(
                decoded.len() == frame_count * FRAME_SIZE,
                "{name} is not the expected complete 128x128 B/R/G/I frame set"
            );
            Ok(BattleBackgroundAsset {
                name: name.to_owned(),
                decoded_size: decoded.len(),
                frame_width: FRAME_WIDTH,
                frame_height: FRAME_HEIGHT,
                frame_count,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(BattleBackgroundCatalog {
        initial_loader_file_offset: INITIAL_LOADER_FILE_OFFSET,
        alternate_loader_file_offset: ALTERNATE_LOADER_FILE_OFFSET,
        renderer_file_offset: INITIAL_RENDERER_FILE_OFFSET,
        selection_table_file_offset: SELECTION_TABLE_FILE_OFFSET,
        assets,
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
        .with_context(|| format!("truncated battle-background offset at {offset:#x}"))?
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
#[path = "battle_backgrounds_tests.rs"]
mod battle_backgrounds_tests;
