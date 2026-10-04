use std::collections::BTreeMap;

use anyhow::Result;

use super::Preview;
use super::decoded::decode_single_asset;
use super::tile_sheet::render_compact_brgi_tiles;
use crate::asset_bindings::catalog_battle_backgrounds;

const SHEET_COLUMNS: usize = 4;

pub(super) fn render_battle_background_previews(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    let catalog = catalog_battle_backgrounds(mad_com, installer_payload)?;
    catalog
        .assets
        .into_iter()
        .map(|asset| {
            let decoded = decode_single_asset(installer_payload, &asset.name)?;
            Ok(Preview {
                source_asset: asset.name.clone(),
                output_file: format!("{}-consumer-backgrounds.png", asset.name.to_ascii_lowercase()),
                evidence: format!(
                    "MAD.COM file {:#X} renders all {} compact {}x{} B/R/G/I battle-background frames selected by file {:#X}",
                    catalog.renderer_file_offset,
                    asset.frame_count,
                    asset.frame_width,
                    asset.frame_height,
                    catalog.selection_table_file_offset,
                ),
                image: render_compact_brgi_tiles(
                    &decoded,
                    asset.frame_width / 8,
                    asset.frame_height,
                    SHEET_COLUMNS,
                )?,
            })
        })
        .collect()
}
