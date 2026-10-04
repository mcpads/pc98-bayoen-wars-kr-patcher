use std::collections::BTreeMap;

use anyhow::Result;

use super::Preview;
use super::decoded::decode_single_asset;
use super::masked_sheet::render_masked_brgi_sheet;
use super::tile_map::render_compact_brgi_tile_map;
use super::tile_sheet::render_compact_brgi_tiles;
use crate::asset_bindings::catalog_tile_graphics;

const SHEET_GAP: usize = 4;

pub(super) fn render_tile_graphics_previews(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    let catalog = catalog_tile_graphics(mad_com, installer_payload)?;
    let mouse = decode_single_asset(installer_payload, &catalog.mouse.name)?;
    let frame_tiles = decode_single_asset(installer_payload, &catalog.frame_tiles.name)?;
    let frame_map = decode_single_asset(installer_payload, &catalog.frame_map.name)?;
    let background = decode_single_asset(installer_payload, &catalog.background_tiles.name)?;
    let portraits = decode_single_asset(installer_payload, &catalog.portraits.name)?;
    let map_entries = &frame_map[catalog.frame_map.entry_offset..];

    Ok(vec![
        Preview {
            source_asset: catalog.mouse.name.clone(),
            output_file: "mouse-consumer-tiles.png".to_owned(),
            evidence: format!(
                "MAD.COM file {:#X} consumes 56 mask-plus-B/R/G/I 32x16 mouse and map-marker records",
                catalog.mouse.tile_consumer_file_offset
            ),
            image: render_masked_brgi_sheet(
                &mouse,
                0,
                catalog.mouse.tile_count,
                catalog.mouse.tile_width,
                catalog.mouse.tile_height,
                636,
                SHEET_GAP,
            )?,
        },
        Preview {
            source_asset: catalog.mouse.name.clone(),
            output_file: "mouse-consumer-cursor.png".to_owned(),
            evidence: format!(
                "MAD.COM file {:#X} consumes the final mask-plus-B/R/G/I 48x48 cursor record",
                catalog.mouse.cursor_consumer_file_offset
            ),
            image: render_masked_brgi_sheet(
                &mouse,
                catalog.mouse.cursor_offset,
                1,
                catalog.mouse.cursor_width,
                catalog.mouse.cursor_height,
                catalog.mouse.cursor_width,
                0,
            )?,
        },
        Preview {
            source_asset: catalog.frame_tiles.name.clone(),
            output_file: "waku-p-consumer-tiles.png".to_owned(),
            evidence: format!(
                "all {} compact 16x16 B/R/G/I frame tiles are exposed in record order",
                catalog.frame_tiles.tile_count
            ),
            image: render_compact_brgi_tiles(&frame_tiles, 2, 16, 20)?,
        },
        Preview {
            source_asset: catalog.frame_map.name.clone(),
            output_file: "waku-img-consumer-map.png".to_owned(),
            evidence: format!(
                "MAD.COM file {:#X} binds all {} map bytes to a 10x25 WAKU_P.DAT tile map",
                catalog.frame_map.consumer_file_offset, catalog.frame_map.entry_count
            ),
            image: render_compact_brgi_tile_map(
                &frame_tiles,
                catalog.frame_tiles.tile_width,
                catalog.frame_tiles.tile_height,
                map_entries,
                catalog.frame_map.width_tiles,
                catalog.frame_map.height_tiles,
            )?,
        },
        Preview {
            source_asset: catalog.background_tiles.name.clone(),
            output_file: "background-consumer-tiles.png".to_owned(),
            evidence: format!(
                "MAD.COM file {:#X} consumes all {} compact 16x16 B/R/G/I background tiles by byte index",
                catalog.background_tiles.consumer_file_offset, catalog.background_tiles.tile_count
            ),
            image: render_compact_brgi_tiles(&background, 2, 16, 20)?,
        },
        Preview {
            source_asset: catalog.portraits.name.clone(),
            output_file: "kao-consumer-portraits.png".to_owned(),
            evidence: format!(
                "MAD.COM file {:#X} selects all {} compact 64x64 B/R/G/I portrait records",
                catalog.portraits.consumer_file_offset, catalog.portraits.tile_count
            ),
            image: render_compact_brgi_tiles(&portraits, 8, 64, 9)?,
        },
    ])
}
