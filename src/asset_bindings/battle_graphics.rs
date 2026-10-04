use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::localization_assets::decode_all_streams;

const GROUP_LOADER_FILE_OFFSET: usize = 0x28ab;
const GROUP_LOADER_SIGNATURE: &[u8] = &[
    0xe8, 0x6d, 0x01, 0xba, 0x06, 0xcb, 0x2e, 0x8b, 0x1e, 0x97, 0xd5, 0xe8, 0x17, 0x06, 0xba, 0x0d,
    0xcb, 0x2e, 0x8b, 0x1e, 0x9b, 0xd5,
];
const OBJECT_CONSUMER_FILE_OFFSET: usize = 0x855;
const OBJECT_CONSUMER_SIGNATURE: &[u8] = &[
    0x2e, 0x8e, 0x1e, 0x97, 0xd5, 0x8b, 0xf8, 0xba, 0x03, 0x00, 0xb9, 0x30, 0x00,
];
const UNIT_CONSUMER_FILE_OFFSET: usize = 0xaab;
const UNIT_CONSUMER_SIGNATURE: &[u8] = &[
    0x2e, 0x8e, 0x1e, 0x9b, 0xd5, 0x8b, 0xf8, 0xba, 0x03, 0x00, 0xb9, 0x30, 0x00,
];
const CAR_CONSUMER_FILE_OFFSET: usize = 0xb8eb;
const CAR_CONSUMER_SIGNATURE: &[u8] = &[
    0x1e, 0x8b, 0x36, 0xe3, 0xb8, 0x2e, 0x8e, 0x1e, 0x91, 0xd5, 0x8b, 0xf8, 0xba, 0x03, 0x00, 0xb9,
    0x30, 0x00,
];
const WINDOW_LOADER_FILE_OFFSET: usize = 0x7caa;
const WINDOW_LOADER_SIGNATURE: &[u8] =
    &[0xba, 0x1c, 0xcb, 0x8b, 0x1e, 0x97, 0xd5, 0xe8, 0x1c, 0xb2];
const WINDOW_RENDERER_FILE_OFFSET: usize = 0x7cb4;
const WINDOW_RENDERER_SIGNATURE: &[u8] = &[
    0x1e, 0x8e, 0x1e, 0x97, 0xd5, 0xbb, 0xe3, 0x7d, 0x53, 0x2e, 0x8b, 0x37, 0x83, 0xfe, 0xff, 0x74,
    0x15,
];
const WINDOW_TRANSFER_TABLE_FILE_OFFSET: usize = 0x7ce3;
const MASKED_SPRITE_WIDTH: usize = 48;
const MASKED_SPRITE_HEIGHT: usize = 48;
const MASKED_SPRITE_SIZE: usize = MASKED_SPRITE_WIDTH / 8 * MASKED_SPRITE_HEIGHT * 5;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MaskedBattleSpriteAsset {
    pub name: String,
    pub decoded_size: usize,
    pub sprite_width: usize,
    pub sprite_height: usize,
    pub sprite_count: usize,
    pub unbound_trailing_bytes: usize,
    pub consumer_file_offset: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BattleWindowTransfer {
    pub source_start: usize,
    pub source_end: usize,
    pub destination_vram_offset: usize,
    pub width_pixels: usize,
    pub height: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BattleWindowBinding {
    pub name: String,
    pub decoded_size: usize,
    pub transfer_table_file_offset: usize,
    pub transfers: Vec<BattleWindowTransfer>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BattleGraphicsCatalog {
    pub masked_sprites: Vec<MaskedBattleSpriteAsset>,
    pub window: BattleWindowBinding,
}

pub(crate) fn catalog_battle_graphics(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<BattleGraphicsCatalog> {
    require_signature(
        mad_com,
        GROUP_LOADER_FILE_OFFSET,
        GROUP_LOADER_SIGNATURE,
        "battle-sprite group loader",
    )?;
    require_signature(
        mad_com,
        OBJECT_CONSUMER_FILE_OFFSET,
        OBJECT_CONSUMER_SIGNATURE,
        "battle-object consumer",
    )?;
    require_signature(
        mad_com,
        UNIT_CONSUMER_FILE_OFFSET,
        UNIT_CONSUMER_SIGNATURE,
        "battle-unit consumer",
    )?;
    require_signature(
        mad_com,
        CAR_CONSUMER_FILE_OFFSET,
        CAR_CONSUMER_SIGNATURE,
        "car-sprite consumer",
    )?;
    require_signature(
        mad_com,
        WINDOW_LOADER_FILE_OFFSET,
        WINDOW_LOADER_SIGNATURE,
        "battle-window loader",
    )?;
    require_signature(
        mad_com,
        WINDOW_RENDERER_FILE_OFFSET,
        WINDOW_RENDERER_SIGNATURE,
        "battle-window renderer",
    )?;

    let masked_sprites = [
        ("BO.DAT", 24usize, 128usize, OBJECT_CONSUMER_FILE_OFFSET),
        ("UN.DAT", 24usize, 0usize, UNIT_CONSUMER_FILE_OFFSET),
        ("CAR.DAT", 5usize, 0usize, CAR_CONSUMER_FILE_OFFSET),
    ]
    .into_iter()
    .map(
        |(name, sprite_count, unbound_trailing_bytes, consumer_file_offset)| {
            let decoded = decode_single(installer_payload, name)?;
            ensure!(
                decoded.len() == sprite_count * MASKED_SPRITE_SIZE + unbound_trailing_bytes,
                "{name} does not match its mask-plus-B/R/G/I sprite population"
            );
            Ok(MaskedBattleSpriteAsset {
                name: name.to_owned(),
                decoded_size: decoded.len(),
                sprite_width: MASKED_SPRITE_WIDTH,
                sprite_height: MASKED_SPRITE_HEIGHT,
                sprite_count,
                unbound_trailing_bytes,
                consumer_file_offset,
            })
        },
    )
    .collect::<Result<Vec<_>>>()?;

    let window = decode_single(installer_payload, "BW.DAT")?;
    let transfers = parse_window_transfers(mad_com)?;
    ensure!(
        transfers.first().map(|transfer| transfer.source_start) == Some(0)
            && transfers.last().map(|transfer| transfer.source_end) == Some(window.len())
            && transfers
                .windows(2)
                .all(|pair| pair[0].source_end == pair[1].source_start),
        "MAD.COM battle-window transfers do not consume BW.DAT exactly once"
    );

    Ok(BattleGraphicsCatalog {
        masked_sprites,
        window: BattleWindowBinding {
            name: "BW.DAT".to_owned(),
            decoded_size: window.len(),
            transfer_table_file_offset: WINDOW_TRANSFER_TABLE_FILE_OFFSET,
            transfers,
        },
    })
}

fn parse_window_transfers(mad_com: &[u8]) -> Result<Vec<BattleWindowTransfer>> {
    let mut offset = WINDOW_TRANSFER_TABLE_FILE_OFFSET;
    let mut transfers = Vec::new();
    loop {
        let source_start = usize::from(read_u16(mad_com, offset)?);
        if source_start == usize::from(u16::MAX) {
            break;
        }
        let destination_vram_offset = usize::from(read_u16(mad_com, offset + 2)?);
        let width_words = usize::from(read_u16(mad_com, offset + 4)?);
        let height = usize::from(read_u16(mad_com, offset + 6)?);
        ensure!(
            width_words > 0 && height > 0,
            "empty battle-window transfer at {offset:#x}"
        );
        let source_end = source_start
            .checked_add(width_words * 2 * height * 4)
            .context("battle-window transfer boundary overflow")?;
        transfers.push(BattleWindowTransfer {
            source_start,
            source_end,
            destination_vram_offset,
            width_pixels: width_words * 16,
            height,
        });
        offset += 8;
    }
    ensure!(
        !transfers.is_empty(),
        "MAD.COM battle-window transfer table is empty"
    );
    Ok(transfers)
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
        .with_context(|| format!("truncated battle-graphics field at {offset:#x}"))?
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
#[path = "battle_graphics_tests.rs"]
mod battle_graphics_tests;
