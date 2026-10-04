use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use v30::Instruction;

use super::typed_v30::decode_complete_block;
use crate::byte_string::encode_lower_hex;
use crate::localization_assets::decode_all_streams;
use crate::source_disk::sha256_hex;

const TITLE_CONSUMER_RANGE: Range<usize> = 0xab6e..0xac09;
const SELECTION_TILE_CONSUMER_RANGE: Range<usize> = 0xa0f5..0xa17a;
const SELECTION_HEADING_FIRST_RANGE: Range<usize> = 0xa1d8..0xa1ff;
const SELECTION_HEADING_SECOND_RANGE: Range<usize> = 0xa1ff..0xa22d;
const SELECTION_NAMES_CONSUMER_RANGE: Range<usize> = 0xa22e..0xa2cc;

const SEL1_TILE_SIZE: usize = 16 * 16 / 8 * 4;
const SEL1_TILE_COUNT: usize = 57;
const SEL1_TILE_COLUMNS: usize = 40;
const SEL1_TILE_ROWS: usize = 25;
const SEL1_TILE_ATLAS_SIZE: usize = SEL1_TILE_SIZE * SEL1_TILE_COUNT;
const SEL1_PANEL_WIDTH_TILES: usize = 12;
const SEL1_PANEL_HEIGHT_TILES: usize = 2;

const SEL3_NAME_TRANSFERS: [(usize, usize, usize, usize); 10] = [
    (0x0000, 0x2304, 7, 0x01c0),
    (0x08c0, 0x2316, 7, 0x01c0),
    (0x1180, 0x2328, 10, 0x0280),
    (0x1e00, 0x4b04, 5, 0x0140),
    (0x2440, 0x4b16, 7, 0x01c0),
    (0x2d00, 0x4b28, 6, 0x0180),
    (0x3480, 0x7304, 5, 0x0140),
    (0x3ac0, 0x7316, 6, 0x0180),
    (0x4240, 0x7328, 4, 0x0100),
    (0x4740, 0x733e, 4, 0x0100),
];
const SEL3_NAME_ROLES: [&str; 10] = [
    "character_skeleton_t",
    "character_nasu_grave",
    "character_draco_centauros",
    "character_harpy",
    "character_suketoudara",
    "character_sasoriman",
    "character_witch",
    "character_zoh_daimaou",
    "character_schezo",
    "character_rulue",
];
const SEL3_HEADING_TRANSFERS: [(usize, &str); 2] = [(0x5340, "first"), (0x6b40, "second")];
const SCREEN_ROW_BYTES: usize = 80;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BakedTextCatalog {
    pub source_asset_count: usize,
    pub translation_unit_count: usize,
    pub consumer_blocks: Vec<BakedTextConsumerBlock>,
    pub units: Vec<BakedTextUnit>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BakedTextConsumerBlock {
    pub id: String,
    pub file_offset: usize,
    pub byte_size: usize,
    pub instruction_count: usize,
    pub call_count: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BakedTextUnit {
    pub id: String,
    pub source_asset: String,
    pub semantic_role: String,
    pub producer_group_id: String,
    pub packed_asset_sha256: String,
    pub decoded_asset_sha256: String,
    pub source_fragments: Vec<BakedTextSourceFragment>,
    pub screen_regions: Vec<BakedTextScreenRegion>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BakedTextSourceFragment {
    pub role: String,
    pub decoded_offset: usize,
    pub byte_size: usize,
    pub content_sha256: String,
    pub raw_hex: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BakedTextScreenRegion {
    pub variant: String,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

pub(crate) fn catalog_baked_text(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<BakedTextCatalog> {
    let consumer_blocks = [
        ("title-base", TITLE_CONSUMER_RANGE),
        ("selection-tile-map", SELECTION_TILE_CONSUMER_RANGE),
        ("stage-heading-first", SELECTION_HEADING_FIRST_RANGE),
        ("stage-heading-second", SELECTION_HEADING_SECOND_RANGE),
        ("selection-character-names", SELECTION_NAMES_CONSUMER_RANGE),
    ]
    .into_iter()
    .map(|(id, range)| catalog_consumer_block(mad_com, id, range))
    .collect::<Result<Vec<_>>>()?;

    let (title_packed_hash, title) = decode_asset(installer_payload, "TITLE.DAT")?;
    let (sel1_packed_hash, sel1) = decode_asset(installer_payload, "SEL1.DAT")?;
    let (sel3_packed_hash, sel3) = decode_asset(installer_payload, "SEL3.DAT")?;
    ensure!(title.len() == 0xf200, "TITLE.DAT decoded size changed");
    ensure!(
        sel1.len() == SEL1_TILE_ATLAS_SIZE + SEL1_TILE_COLUMNS * SEL1_TILE_ROWS,
        "SEL1.DAT decoded size changed"
    );
    ensure!(sel3.len() == 0x9b40, "SEL3.DAT decoded size changed");

    let mut units = catalog_title_units(&title, &title_packed_hash)?;
    units.extend(catalog_difficulty_units(&sel1, &sel1_packed_hash)?);
    units.extend(catalog_sel3_units(&sel3, &sel3_packed_hash)?);
    let ids = units
        .iter()
        .map(|unit| unit.id.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        ids.len() == units.len(),
        "baked text unit IDs are not unique"
    );

    Ok(BakedTextCatalog {
        source_asset_count: 3,
        translation_unit_count: units.len(),
        consumer_blocks,
        units,
    })
}

fn catalog_consumer_block(
    mad_com: &[u8],
    id: &str,
    range: Range<usize>,
) -> Result<BakedTextConsumerBlock> {
    let instructions = decode_complete_block(mad_com, range.clone(), id)?;
    let call_count = instructions
        .iter()
        .filter(|instruction| matches!(instruction, Instruction::Call { .. }))
        .count();
    ensure!(call_count > 0, "{id} consumer contains no typed CALL");
    Ok(BakedTextConsumerBlock {
        id: id.to_owned(),
        file_offset: range.start,
        byte_size: range.len(),
        instruction_count: instructions.len(),
        call_count,
    })
}

fn decode_asset(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    name: &str,
) -> Result<(String, Vec<u8>)> {
    let packed = installer_payload
        .get(name)
        .with_context(|| format!("verified installer payload is missing {name}"))?;
    let streams = decode_all_streams(packed)?;
    ensure!(
        streams.len() == 1,
        "{name} does not have exactly one stream"
    );
    Ok((
        sha256_hex(packed),
        streams.into_iter().next().unwrap().output,
    ))
}

fn catalog_title_units(decoded: &[u8], packed_hash: &str) -> Result<Vec<BakedTextUnit>> {
    let decoded_hash = sha256_hex(decoded);
    [
        (
            "title-logo-primary",
            "game_title",
            BakedTextScreenRegion {
                variant: "base".to_owned(),
                x: 96,
                y: 96,
                width: 464,
                height: 48,
            },
        ),
        (
            "title-logo-subtitle",
            "game_subtitle",
            BakedTextScreenRegion {
                variant: "base".to_owned(),
                x: 64,
                y: 144,
                width: 512,
                height: 112,
            },
        ),
    ]
    .into_iter()
    .map(|(id, role, screen_region)| {
        Ok(BakedTextUnit {
            id: id.to_owned(),
            source_asset: "TITLE.DAT".to_owned(),
            semantic_role: role.to_owned(),
            producer_group_id: "title-logo".to_owned(),
            packed_asset_sha256: packed_hash.to_owned(),
            decoded_asset_sha256: decoded_hash.clone(),
            source_fragments: vec![source_fragment(
                decoded,
                0..decoded.len(),
                "shared_title_image_producer",
            )?],
            screen_regions: vec![screen_region],
        })
    })
    .collect()
}

fn catalog_difficulty_units(decoded: &[u8], packed_hash: &str) -> Result<Vec<BakedTextUnit>> {
    let decoded_hash = sha256_hex(decoded);
    [
        ("difficulty-easy", "easy", 0),
        ("difficulty-normal", "normal", 8),
        ("difficulty-hard", "hard", 16),
    ]
    .into_iter()
    .map(|(id, role, map_row)| {
        let fragments = catalog_sel1_panel_fragments(decoded, map_row)?;
        Ok(BakedTextUnit {
            id: id.to_owned(),
            source_asset: "SEL1.DAT".to_owned(),
            semantic_role: format!("difficulty_{role}"),
            producer_group_id: "selection-difficulty-labels".to_owned(),
            packed_asset_sha256: packed_hash.to_owned(),
            decoded_asset_sha256: decoded_hash.clone(),
            source_fragments: fragments,
            screen_regions: vec![BakedTextScreenRegion {
                variant: "base".to_owned(),
                x: 0,
                y: map_row * 16,
                width: SEL1_PANEL_WIDTH_TILES * 16,
                height: SEL1_PANEL_HEIGHT_TILES * 16,
            }],
        })
    })
    .collect()
}

fn catalog_sel1_panel_fragments(
    decoded: &[u8],
    map_row: usize,
) -> Result<Vec<BakedTextSourceFragment>> {
    ensure!(
        map_row + SEL1_PANEL_HEIGHT_TILES <= SEL1_TILE_ROWS,
        "SEL1 difficulty panel lies outside the tile map"
    );
    let mut fragments = Vec::new();
    let mut tile_indices = BTreeSet::new();
    for row in map_row..map_row + SEL1_PANEL_HEIGHT_TILES {
        let start = SEL1_TILE_ATLAS_SIZE + row * SEL1_TILE_COLUMNS;
        let range = start..start + SEL1_PANEL_WIDTH_TILES;
        for &tile_index in decoded
            .get(range.clone())
            .context("SEL1 panel map row lies outside the decoded asset")?
        {
            ensure!(
                usize::from(tile_index) < SEL1_TILE_COUNT,
                "SEL1 panel references a tile outside its atlas"
            );
            tile_indices.insert(usize::from(tile_index));
        }
        fragments.push(source_fragment(
            decoded,
            range,
            &format!("tile_map_row_{row:02}"),
        )?);
    }
    for tile_index in tile_indices {
        let start = tile_index * SEL1_TILE_SIZE;
        fragments.push(source_fragment(
            decoded,
            start..start + SEL1_TILE_SIZE,
            &format!("tile_{tile_index:02}"),
        )?);
    }
    Ok(fragments)
}

fn catalog_sel3_units(decoded: &[u8], packed_hash: &str) -> Result<Vec<BakedTextUnit>> {
    let decoded_hash = sha256_hex(decoded);
    let mut units = SEL3_NAME_TRANSFERS
        .into_iter()
        .zip(SEL3_NAME_ROLES)
        .enumerate()
        .map(
            |(index, ((source_offset, destination_offset, width_words, plane_stride), role))| {
                let width_bytes = width_words * 2;
                ensure!(
                    plane_stride == width_bytes * 32,
                    "SEL3 name plane stride differs from its dimensions"
                );
                let source_end = source_offset + plane_stride * 5;
                Ok(BakedTextUnit {
                    id: format!("character-name-{:02}", index + 1),
                    source_asset: "SEL3.DAT".to_owned(),
                    semantic_role: role.to_owned(),
                    producer_group_id: "selection-character-names".to_owned(),
                    packed_asset_sha256: packed_hash.to_owned(),
                    decoded_asset_sha256: decoded_hash.clone(),
                    source_fragments: vec![source_fragment(
                        decoded,
                        source_offset..source_end,
                        "masked_name_overlay",
                    )?],
                    screen_regions: vec![screen_region(
                        "base",
                        destination_offset,
                        width_bytes * 8,
                        32,
                    )],
                })
            },
        )
        .collect::<Result<Vec<_>>>()?;

    let heading_fragments = SEL3_HEADING_TRANSFERS
        .into_iter()
        .map(|(offset, variant)| source_fragment(decoded, offset..offset + 32 * 48 * 4, variant))
        .collect::<Result<Vec<_>>>()?;
    units.push(BakedTextUnit {
        id: "stage-select-heading".to_owned(),
        source_asset: "SEL3.DAT".to_owned(),
        semantic_role: "stage_selection_heading".to_owned(),
        producer_group_id: "selection-stage-heading".to_owned(),
        packed_asset_sha256: packed_hash.to_owned(),
        decoded_asset_sha256: decoded_hash,
        source_fragments: heading_fragments,
        screen_regions: SEL3_HEADING_TRANSFERS
            .into_iter()
            .map(|(_, variant)| screen_region(variant, 0x002e, 256, 48))
            .collect(),
    });
    Ok(units)
}

fn screen_region(
    variant: &str,
    destination_offset: usize,
    width: usize,
    height: usize,
) -> BakedTextScreenRegion {
    BakedTextScreenRegion {
        variant: variant.to_owned(),
        x: destination_offset % SCREEN_ROW_BYTES * 8,
        y: destination_offset / SCREEN_ROW_BYTES,
        width,
        height,
    }
}

fn source_fragment(
    decoded: &[u8],
    range: Range<usize>,
    role: &str,
) -> Result<BakedTextSourceFragment> {
    let bytes = decoded
        .get(range.clone())
        .with_context(|| format!("{role} source fragment lies outside its decoded asset"))?;
    Ok(BakedTextSourceFragment {
        role: role.to_owned(),
        decoded_offset: range.start,
        byte_size: bytes.len(),
        content_sha256: sha256_hex(bytes),
        raw_hex: encode_lower_hex(bytes),
    })
}

#[cfg(test)]
#[path = "baked_text_tests.rs"]
mod baked_text_tests;
