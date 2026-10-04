use std::collections::BTreeMap;

use anyhow::Result;

use super::Preview;
use super::decoded::decode_single_asset;
use super::monochrome_sheet::render_monochrome_contact_sheet;
use crate::asset_bindings::catalog_monochrome_sprites;

const SHEET_COLUMNS: usize = 16;
const CELL_GAP: usize = 2;

pub(super) fn render_monochrome_sprite_previews(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    let catalog = catalog_monochrome_sprites(mad_com, installer_payload)?;
    catalog
        .assets
        .into_iter()
        .map(|asset| {
            let decoded = decode_single_asset(installer_payload, &asset.name)?;
            Ok(Preview {
                source_asset: asset.name.clone(),
                output_file: format!(
                    "{}-consumer-monochrome-sprites.png",
                    asset.name.to_ascii_lowercase().replace('.', "-")
                ),
                evidence: format!(
                    "MAD.COM file {:#X} indexes {} records and file {:#X} renders each as a 32x32 bitmap copied to every graphics plane",
                    catalog.consumer_file_offset, asset.sprite_count, catalog.blitter_file_offset
                ),
                image: render_monochrome_contact_sheet(
                    &decoded,
                    asset.sprite_width,
                    asset.sprite_height,
                    SHEET_COLUMNS,
                    CELL_GAP,
                )?,
            })
        })
        .collect()
}
