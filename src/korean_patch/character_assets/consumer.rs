use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};

use super::TARGET_FILES;
use super::manifest::{ArleCharacterAssetManifest, FrameBinding};
use super::payload::required;
use crate::asset_bindings::{CharacterSpriteAsset, catalog_character_sprites};
use crate::character_runtime::{CHARACTER_PALETTE_TABLE_FILE_OFFSET, read_character_palette_rgb4};
use crate::game_data::catalog_game_data;

pub(super) fn validate_arle_consumer(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    manifest: &ArleCharacterAssetManifest,
) -> Result<()> {
    let mad_com = required(installer_payload, "MAD.COM")?;
    ensure!(
        manifest.consumer.palette_table_file_offset == CHARACTER_PALETTE_TABLE_FILE_OFFSET
            && read_character_palette_rgb4(mad_com)? == manifest.consumer.runtime_palette_rgb4,
        "Arle character runtime palette differs from its manifest"
    );
    let character_catalog = catalog_character_sprites(mad_com, installer_payload)?;
    ensure!(
        character_catalog.slot_table_file_offset == manifest.consumer.slot_table_file_offset,
        "Arle slot table offset differs from its manifest"
    );
    let slot = character_catalog
        .slots
        .iter()
        .find(|slot| slot.slot_index == manifest.consumer.slot_index)
        .context("Arle character slot is absent")?;
    ensure!(
        slot.primary_asset == manifest.consumer.primary_asset
            && slot.secondary_asset.as_deref() == Some(&manifest.consumer.secondary_asset),
        "Arle character slot no longer selects C07 and C08"
    );

    let game = catalog_game_data(
        mad_com,
        required(installer_payload, "GAIJI.COM")?,
        required(installer_payload, "MENU.COM")?,
    )?;
    let name_table = game
        .mad_com
        .interface_text
        .reference_catalog
        .metadata_tables
        .iter()
        .find(|table| table.id == "selected-unit-name-pointers")
        .context("selected-unit name table is absent")?;
    ensure!(
        name_table.offset == manifest.consumer.unit_name_table_file_offset,
        "Arle name-table offset differs from its manifest"
    );
    let name_storage = name_table.offset + manifest.consumer.unit_name_index * name_table.stride;
    let name_reference = game
        .mad_com
        .interface_text
        .reference_catalog
        .references
        .iter()
        .find(|reference| reference.storage_offset == name_storage)
        .context("Arle selected-unit name reference is absent")?;
    ensure!(
        name_reference.target_entry_id == manifest.consumer.unit_name_entry_id,
        "Arle name-table entry targets a different interface record"
    );
    let name_entry = game
        .mad_com
        .interface_text
        .entries
        .iter()
        .find(|entry| entry.id == manifest.consumer.unit_name_entry_id)
        .context("Arle interface name record is absent")?;
    ensure!(
        name_entry.text == "アルル",
        "verified Arle interface name changed"
    );
    Ok(())
}

pub(super) fn validate_frame_population(
    assets: &[CharacterSpriteAsset],
    bindings: &[FrameBinding],
) -> Result<()> {
    let mut source = BTreeSet::new();
    for file_name in TARGET_FILES {
        let asset = find_asset(assets, file_name)?;
        source.extend(asset.transfers.iter().map(|transfer| {
            (
                file_name,
                transfer.source_start,
                transfer.width_pixels,
                transfer.height,
            )
        }));
    }
    let declared = bindings
        .iter()
        .map(|binding| {
            (
                binding.target_asset.as_str(),
                binding.target_source_start,
                binding.target_width,
                binding.target_height,
            )
        })
        .collect::<BTreeSet<_>>();
    ensure!(
        declared == source,
        "Arle manifest does not cover the complete unique C07/C08 frame population"
    );
    for file_name in TARGET_FILES {
        let asset = find_asset(assets, file_name)?;
        ensure!(
            asset.unbound_ranges.is_empty() && asset.consumer_bound_bytes == asset.decoded_size,
            "Arle asset {file_name} contains bytes outside the verified consumer population"
        );
    }
    Ok(())
}

pub(super) fn find_asset<'a>(
    assets: &'a [CharacterSpriteAsset],
    file_name: &str,
) -> Result<&'a CharacterSpriteAsset> {
    assets
        .iter()
        .find(|asset| asset.name == file_name)
        .with_context(|| format!("character catalog is missing {file_name}"))
}
