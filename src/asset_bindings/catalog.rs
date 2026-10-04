use std::collections::BTreeMap;

use anyhow::Result;
use serde::Serialize;

use super::baked_text::{BakedTextCatalog, catalog_baked_text};
use super::battle_backgrounds::{BattleBackgroundCatalog, catalog_battle_backgrounds};
use super::battle_graphics::{BattleGraphicsCatalog, catalog_battle_graphics};
use super::battle_maps::{BattleMapCatalog, catalog_battle_maps};
use super::character_sprites::{CharacterSpriteCatalog, catalog_character_sprites};
use super::monochrome_sprites::{MonochromeSpriteCatalog, catalog_monochrome_sprites};
use super::monochrome_text::{MonochromeTextCatalog, catalog_monochrome_text};
use super::movement_tables::{MovementTableCatalog, catalog_movement_tables};
use super::sample_language_review::{SampleLanguageReviewCatalog, catalog_sample_language_review};
use super::sample_playback::{SamplePlaybackCatalog, catalog_sample_playback};
use super::sound_data::{SoundDataCatalog, catalog_sound_data};
use super::support_files::{SupportFileCatalog, catalog_support_files};
use super::system_io::{SystemIoCatalog, catalog_system_io};
use super::tile_graphics::{TileGraphicsCatalog, catalog_tile_graphics};
use super::voice_samples::{VoiceSampleCatalog, catalog_voice_samples};

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AssetBindingCatalog {
    pub baked_text: BakedTextCatalog,
    pub battle_backgrounds: BattleBackgroundCatalog,
    pub battle_graphics: BattleGraphicsCatalog,
    pub battle_maps: BattleMapCatalog,
    pub character_sprites: CharacterSpriteCatalog,
    pub monochrome_sprites: MonochromeSpriteCatalog,
    pub monochrome_text: MonochromeTextCatalog,
    pub movement_tables: MovementTableCatalog,
    pub sample_language_review: SampleLanguageReviewCatalog,
    pub sample_playback: SamplePlaybackCatalog,
    pub sound_data: SoundDataCatalog,
    pub support_files: SupportFileCatalog,
    pub system_io: SystemIoCatalog,
    pub tile_graphics: TileGraphicsCatalog,
    pub voice_samples: VoiceSampleCatalog,
}

pub(crate) fn catalog_asset_bindings(
    mad_com: &[u8],
    installer_payload: &BTreeMap<String, Vec<u8>>,
    runtime_files: &BTreeMap<String, Vec<u8>>,
    boot_files: &BTreeMap<String, Vec<u8>>,
) -> Result<AssetBindingCatalog> {
    let sample_playback = catalog_sample_playback(installer_payload, runtime_files)?;
    let sample_language_review = catalog_sample_language_review(&sample_playback)?;
    let voice_samples =
        catalog_voice_samples(installer_payload, &sample_playback, &sample_language_review)?;
    let monochrome_sprites = catalog_monochrome_sprites(mad_com, installer_payload)?;
    let monochrome_text = catalog_monochrome_text(mad_com, installer_payload, &monochrome_sprites)?;
    Ok(AssetBindingCatalog {
        baked_text: catalog_baked_text(mad_com, installer_payload)?,
        battle_backgrounds: catalog_battle_backgrounds(mad_com, installer_payload)?,
        battle_graphics: catalog_battle_graphics(mad_com, installer_payload)?,
        battle_maps: catalog_battle_maps(mad_com, installer_payload)?,
        character_sprites: catalog_character_sprites(mad_com, installer_payload)?,
        monochrome_sprites,
        monochrome_text,
        movement_tables: catalog_movement_tables(mad_com, installer_payload)?,
        sample_language_review,
        sample_playback,
        sound_data: catalog_sound_data(installer_payload)?,
        support_files: catalog_support_files(installer_payload, boot_files)?,
        system_io: catalog_system_io(boot_files)?,
        tile_graphics: catalog_tile_graphics(mad_com, installer_payload)?,
        voice_samples,
    })
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod catalog_tests;
