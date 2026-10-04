use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};

use super::Preview;
use super::contact_sheet::compose_contact_sheet;
use super::decoded::decode_single_asset;
use super::planar::render_strided_brgi_with_palette;
use crate::asset_bindings::catalog_character_sprites;
use crate::character_runtime::{expand_character_palette, read_character_palette_rgb4};

const CONTACT_SHEET_WIDTH: usize = 640;
const CONTACT_SHEET_GAP: usize = 8;

pub(super) fn render_character_sprite_previews(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    let catalog = catalog_character_sprites(mad_com, installer_payload)?;
    let palette = expand_character_palette(&read_character_palette_rgb4(mad_com)?);
    catalog
        .assets
        .iter()
        .map(|asset| render_character_sprite_preview(installer_payload, asset, &palette))
        .collect()
}

pub(super) fn render_named_character_sprite_preview(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
    asset_name: &str,
) -> Result<Preview> {
    let catalog = catalog_character_sprites(mad_com, installer_payload)?;
    let asset = catalog
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .with_context(|| format!("character sprite catalog is missing {asset_name}"))?;
    let palette = expand_character_palette(&read_character_palette_rgb4(mad_com)?);
    render_character_sprite_preview(installer_payload, asset, &palette)
}

fn render_character_sprite_preview(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    asset: &crate::asset_bindings::CharacterSpriteAsset,
    palette: &[[u8; 3]; 16],
) -> Result<Preview> {
    let decoded = decode_single_asset(installer_payload, &asset.name)?;
    let mut unique_transfers = BTreeSet::new();
    let mut images = Vec::new();
    for transfer in &asset.transfers {
        let identity = (
            transfer.source_start,
            transfer.source_end,
            transfer.width_pixels,
            transfer.height,
        );
        if !unique_transfers.insert(identity) {
            continue;
        }
        let width_bytes = transfer.width_pixels / 8;
        let plane_size = width_bytes
            .checked_mul(transfer.height)
            .context("character sprite plane size overflow")?;
        ensure!(
            transfer.source_end - transfer.source_start == plane_size * 4,
            "character sprite transfer does not contain four complete planes"
        );
        images.push(render_strided_brgi_with_palette(
            &decoded[transfer.source_start..transfer.source_end],
            transfer.width_pixels,
            transfer.height,
            width_bytes,
            plane_size,
            palette,
        )?);
    }
    let unbound_bytes: usize = asset
        .unbound_ranges
        .iter()
        .map(|range| range.end - range.start)
        .sum();
    let unique_transfer_count = images.len();
    Ok(Preview {
        source_asset: asset.name.clone(),
        output_file: format!("{}-consumer-sprites.png", asset.name.to_ascii_lowercase()),
        evidence: format!(
            "MAD.COM file 0x7B47 slot loader and 0x82E9 renderer bind {unique_transfer_count} unique B/R/G/I source rectangles; runtime color comes from the indexed RGB4 table at file 0xD777; {} of {} decoded bytes are consumer-bound and {unbound_bytes} are unreachable padding or gaps",
            asset.consumer_bound_bytes, asset.decoded_size
        ),
        image: compose_contact_sheet(images, CONTACT_SHEET_WIDTH, CONTACT_SHEET_GAP)?,
    })
}
