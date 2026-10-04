use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use super::Preview;
use super::decoded::decode_single_asset;
use super::planar::render_strided_brgi;
use crate::asset_bindings::catalog_battle_maps;

const OVERVIEW_ROW_STRIDE: usize = 20;

pub(super) fn render_battle_map_previews(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    let catalog = catalog_battle_maps(mad_com, installer_payload)?;
    catalog
        .maps
        .into_iter()
        .map(|asset| {
            let decoded = decode_single_asset(installer_payload, &asset.name)?;
            let overview = decoded
                .get(..asset.overview_byte_size)
                .with_context(|| format!("{} has no complete battle-map overview", asset.name))?;
            let plane_size = asset
                .overview_byte_size
                .checked_div(4)
                .context("battle-map overview size is not divisible by four")?;
            ensure!(
                plane_size == OVERVIEW_ROW_STRIDE * asset.overview_height,
                "{} overview geometry disagrees with its renderer binding",
                asset.name
            );

            Ok(Preview {
                source_asset: asset.name.clone(),
                output_file: format!("{}-consumer-overview.png", asset.name.to_ascii_lowercase()),
                evidence: format!(
                    "MAD.COM file {:#X} selects this map and file {:#X} renders its first {} decoded bytes as a {}x{} B/R/G/I overview; the remaining bytes feed the map-state consumer at file {:#X}",
                    catalog.map_loader_file_offset,
                    catalog.overview_renderer_file_offset,
                    asset.overview_byte_size,
                    asset.overview_width_pixels,
                    asset.overview_height,
                    catalog.map_data_consumer_file_offset,
                ),
                image: render_strided_brgi(
                    overview,
                    asset.overview_width_pixels,
                    asset.overview_height,
                    OVERVIEW_ROW_STRIDE,
                    plane_size,
                )?,
            })
        })
        .collect()
}
