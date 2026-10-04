use std::collections::BTreeMap;

use anyhow::{Result, ensure};

use super::Preview;
use super::contact_sheet::compose_contact_sheet;
use super::decoded::decode_single_asset;
use super::masked_sheet::render_masked_brgi_sheet;
use super::planar::render_strided_brgi;
use crate::asset_bindings::catalog_battle_graphics;

const SHEET_WIDTH: usize = 636;
const SHEET_GAP: usize = 4;

pub(super) fn render_battle_graphics_previews(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    let catalog = catalog_battle_graphics(mad_com, installer_payload)?;
    let mut previews = catalog
        .masked_sprites
        .into_iter()
        .map(|asset| {
            let decoded = decode_single_asset(installer_payload, &asset.name)?;
            Ok(Preview {
                source_asset: asset.name.clone(),
                output_file: format!(
                    "{}-consumer-masked-sprites.png",
                    asset.name.to_ascii_lowercase().replace('.', "-")
                ),
                evidence: format!(
                    "MAD.COM file {:#X} consumes {} mask-plus-B/R/G/I {}x{} records; {} trailing bytes are unreachable",
                    asset.consumer_file_offset,
                    asset.sprite_count,
                    asset.sprite_width,
                    asset.sprite_height,
                    asset.unbound_trailing_bytes,
                ),
                image: render_masked_brgi_sheet(
                    &decoded,
                    0,
                    asset.sprite_count,
                    asset.sprite_width,
                    asset.sprite_height,
                    SHEET_WIDTH,
                    SHEET_GAP,
                )?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let window = catalog.window;
    let decoded = decode_single_asset(installer_payload, &window.name)?;
    let images = window
        .transfers
        .iter()
        .map(|transfer| {
            let source = &decoded[transfer.source_start..transfer.source_end];
            let plane_size = source.len() / 4;
            ensure!(
                plane_size * 4 == source.len(),
                "battle-window transfer does not contain four complete planes"
            );
            render_strided_brgi(
                source,
                transfer.width_pixels,
                transfer.height,
                transfer.width_pixels / 8,
                plane_size,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    previews.push(Preview {
        source_asset: window.name.clone(),
        output_file: "bw-consumer-transfers.png".to_owned(),
        evidence: format!(
            "MAD.COM file {:#X} binds {} contiguous B/R/G/I transfers that consume all {} decoded bytes",
            window.transfer_table_file_offset,
            window.transfers.len(),
            window.decoded_size,
        ),
        image: compose_contact_sheet(images, SHEET_WIDTH, SHEET_GAP)?,
    });
    Ok(previews)
}
