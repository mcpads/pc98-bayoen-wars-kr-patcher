use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use super::{MonochromeRuntimeBinding, catalog_monochrome_runtime};
use crate::localization_assets::decode_all_streams;

const OPENING_LOADER_FILE_OFFSET: usize = 0x971a;
const OPENING_LOADER_SIGNATURE: &[u8] = &[
    0xba, 0xb3, 0xca, 0x2e, 0x8b, 0x1e, 0x93, 0xd5, 0xe8, 0xab, 0x97,
];
const ENDING_LOADER_FILE_OFFSET: usize = 0x979a;
const ENDING_LOADER_SIGNATURE: &[u8] = &[
    0xba, 0xd3, 0xca, 0x2e, 0x8b, 0x1e, 0x93, 0xd5, 0xe8, 0x2b, 0x97,
];
const SPRITE_CONSUMER_FILE_OFFSET: usize = 0x9cb4;
const SPRITE_CONSUMER_SIGNATURE: &[u8] = &[
    0x32, 0xc0, 0xd1, 0xe8, 0x1e, 0x2e, 0x8e, 0x1e, 0x93, 0xd5, 0x8b, 0xf0, 0x2e, 0x8b, 0x7e, 0x0b,
    0xe8, 0x69, 0x31, 0x1f, 0xc3,
];
const MONOCHROME_BLITTER_FILE_OFFSET: usize = 0xce30;
const MONOCHROME_BLITTER_SIGNATURE: &[u8] = &[
    0xfc, 0xb8, 0x00, 0xa8, 0xe8, 0x0f, 0x00, 0xb8, 0x00, 0xb0, 0xe8, 0x09, 0x00, 0xb8, 0x00, 0xb8,
    0xe8, 0x03, 0x00, 0xb8, 0x00, 0xe0,
];
const SPRITE_WIDTH: usize = 32;
const SPRITE_HEIGHT: usize = 32;
const SPRITE_SIZE: usize = SPRITE_WIDTH / 8 * SPRITE_HEIGHT;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MonochromeSpriteAsset {
    pub name: String,
    pub decoded_size: usize,
    pub sprite_width: usize,
    pub sprite_height: usize,
    pub sprite_count: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MonochromeSpriteCatalog {
    pub consumer_file_offset: usize,
    pub blitter_file_offset: usize,
    pub runtime: MonochromeRuntimeBinding,
    pub assets: Vec<MonochromeSpriteAsset>,
}

pub(crate) fn catalog_monochrome_sprites(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<MonochromeSpriteCatalog> {
    let runtime = catalog_monochrome_runtime(mad_com)?;
    require_signature(
        mad_com,
        OPENING_LOADER_FILE_OFFSET,
        OPENING_LOADER_SIGNATURE,
        "opening monochrome-sprite loader",
    )?;
    require_signature(
        mad_com,
        ENDING_LOADER_FILE_OFFSET,
        ENDING_LOADER_SIGNATURE,
        "ending monochrome-sprite loader",
    )?;
    require_signature(
        mad_com,
        SPRITE_CONSUMER_FILE_OFFSET,
        SPRITE_CONSUMER_SIGNATURE,
        "monochrome-sprite consumer",
    )?;
    require_signature(
        mad_com,
        MONOCHROME_BLITTER_FILE_OFFSET,
        MONOCHROME_BLITTER_SIGNATURE,
        "monochrome-sprite blitter",
    )?;

    let assets = ["OPM.DAT", "EDM.DAT"]
        .into_iter()
        .map(|name| {
            let decoded = decode_single(installer_payload, name)?;
            ensure!(
                decoded.len().is_multiple_of(SPRITE_SIZE),
                "{name} ends inside a 32x32 monochrome sprite"
            );
            Ok(MonochromeSpriteAsset {
                name: name.to_owned(),
                decoded_size: decoded.len(),
                sprite_width: SPRITE_WIDTH,
                sprite_height: SPRITE_HEIGHT,
                sprite_count: decoded.len() / SPRITE_SIZE,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(MonochromeSpriteCatalog {
        consumer_file_offset: SPRITE_CONSUMER_FILE_OFFSET,
        blitter_file_offset: MONOCHROME_BLITTER_FILE_OFFSET,
        runtime,
        assets,
    })
}

pub(super) fn decode_single(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    name: &str,
) -> Result<Vec<u8>> {
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

fn require_signature(program: &[u8], offset: usize, signature: &[u8], role: &str) -> Result<()> {
    ensure!(
        program.get(offset..offset + signature.len()) == Some(signature),
        "MAD.COM {role} signature does not match at file offset {offset:#x}"
    );
    Ok(())
}

#[cfg(test)]
#[path = "monochrome_sprites_tests.rs"]
mod monochrome_sprites_tests;
