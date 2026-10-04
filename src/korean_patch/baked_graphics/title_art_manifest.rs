use std::collections::BTreeSet;

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

use crate::source_disk::{SOURCE_DISK_SHA256, sha256_hex};
use crate::title_runtime::{SOURCE_TITLE_PALETTE_RGB4, TitlePaletteRgb4};
use crate::title_screen::{TITLE_CONTENT_TRANSFERS, TITLE_SCREEN_HEIGHT, TITLE_SCREEN_WIDTH};

const INPUT_KIND: &str = "source_extracted_active_frame";
const SOURCE_ASSET: &str = "TITLE.DAT + TITLE2.DAT + MAD.COM";
const BUILD_STATUS: &str = "development_only";
const ARTWORK_KIND: &str = "project_authored_consumer_frame";
const CONVERSION_SOURCE_KIND: &str = "untracked_full_generation_master";
const CONVERSION_METHOD: &str = "pixel_exact_rgb_crop_without_background_separation";
const MASTER_COLOR_MODE: &str = "srgb_rgb24_unrestricted";
const PALETTE: &str = "pc98_source_title_runtime_palette_rgb4";
const QUANTIZATION: &str = "nearest_runtime_palette_after_crop_and_frame_placement";
const BACKGROUND_POLICY: &str = "restore_source_background_texture_by_edge_connectivity";
const OUTSIDE_TRANSFER_POLICY: &str = "restore_source_for_neutral_backing_only";
const OUTSIDE_TRANSFER_TRIMMABLE_PALETTE_INDICES: [u8; 5] = [2, 3, 4, 5, 6];
const PROTECTED_COMPONENTS: [&str; 1] = ["source_copyright_band"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TitleArtworkManifest {
    pub(super) id: String,
    pub(super) build_status: String,
    pub(super) input: SourceInput,
    pub(super) artwork: ArtworkInput,
    pub(super) consumer: ConsumerInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceInput {
    pub(super) kind: String,
    pub(super) supported_source_sha256: String,
    pub(super) source_asset: String,
    pub(super) title_asset_packed_sha256: String,
    pub(super) title_asset_decoded_sha256: String,
    pub(super) title_animation_source_offset: usize,
    pub(super) source_preview_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArtworkInput {
    pub(super) kind: String,
    pub(super) sha256: String,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) color_mode: String,
    pub(super) conversion: ArtworkConversionInput,
    pub(super) generation_supporting_references: Vec<GenerationSupportingReferenceInput>,
    pub(super) crop: CropInput,
    pub(super) preserved_component_regions: Vec<serde_json::Value>,
    pub(super) translation_draft_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArtworkConversionInput {
    pub(super) source_kind: String,
    pub(super) source_sha256: String,
    pub(super) source_width: usize,
    pub(super) source_height: usize,
    pub(super) source_region: CropInput,
    pub(super) output_rgb_sha256: String,
    pub(super) method: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GenerationSupportingReferenceInput {
    role: String,
    page_url: String,
    asset_url: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CropInput {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConsumerInput {
    pub(super) screen_width: usize,
    pub(super) screen_height: usize,
    pub(super) sampling_width: usize,
    pub(super) sampling_height: usize,
    pub(super) sampling_placement: SamplingPlacementInput,
    pub(super) palette: String,
    pub(super) runtime_palette_rgb4: TitlePaletteRgb4,
    pub(super) quantization: String,
    pub(super) background_palette_index: u8,
    pub(super) background_palette_indices: Vec<u8>,
    pub(super) background_policy: String,
    pub(super) outside_transfer_policy: String,
    pub(super) outside_transfer_trimmable_palette_indices: Vec<u8>,
    pub(super) content_transfer_count: usize,
    pub(super) protected_components: Vec<String>,
    pub(super) title2_animation_enabled: bool,
    pub(super) maximum_packed_size: usize,
    pub(super) maximum_decode_command_count: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SamplingPlacementInput {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

pub(super) fn validate_title_artwork_manifest(
    manifest: &TitleArtworkManifest,
    source_packed: &[u8],
    source_decoded: &[u8],
    translation_draft_sha256: &str,
) -> Result<()> {
    ensure!(
        manifest.build_status == BUILD_STATUS,
        "title artwork is not marked development_only"
    );
    ensure!(
        manifest.input.kind == INPUT_KIND,
        "title artwork input must be a source-extracted active frame"
    );
    ensure!(
        manifest.input.supported_source_sha256 == SOURCE_DISK_SHA256,
        "title artwork targets a different source disk"
    );
    ensure!(
        manifest.input.source_asset == SOURCE_ASSET,
        "title artwork must originate from the complete source title composition"
    );
    ensure!(
        sha256_hex(source_packed) == manifest.input.title_asset_packed_sha256,
        "title artwork manifest targets a different packed TITLE.DAT"
    );
    ensure!(
        sha256_hex(source_decoded) == manifest.input.title_asset_decoded_sha256,
        "title artwork manifest targets a different decoded TITLE.DAT"
    );
    ensure!(
        translation_draft_sha256 == manifest.artwork.translation_draft_sha256,
        "title translation draft changed after the artwork was generated"
    );
    validate_project_authored_artwork(&manifest.artwork)?;
    validate_consumer(&manifest.consumer)?;
    Ok(())
}

fn validate_project_authored_artwork(artwork: &ArtworkInput) -> Result<()> {
    ensure!(
        artwork.kind == ARTWORK_KIND,
        "title artwork must be a project-authored consumer frame"
    );
    ensure!(
        artwork.color_mode == MASTER_COLOR_MODE,
        "title artwork master must remain unrestricted 24-bit sRGB"
    );
    ensure!(
        artwork.conversion.source_kind == CONVERSION_SOURCE_KIND,
        "title artwork conversion must identify its untracked full generation master"
    );
    ensure!(
        is_sha256(&artwork.conversion.source_sha256),
        "title artwork conversion source SHA-256 is invalid"
    );
    validate_artwork_crop(
        &artwork.conversion.source_region,
        artwork.conversion.source_width,
        artwork.conversion.source_height,
    )?;
    ensure!(
        artwork.conversion.source_region.width == artwork.width
            && artwork.conversion.source_region.height == artwork.height,
        "title artwork dimensions must equal the selected conversion source region"
    );
    ensure!(
        is_sha256(&artwork.conversion.output_rgb_sha256),
        "title artwork conversion output RGB SHA-256 is invalid"
    );
    ensure!(
        artwork.conversion.method == CONVERSION_METHOD,
        "title artwork conversion must be a pixel-exact crop without background separation"
    );
    validate_artwork_crop(&artwork.crop, artwork.width, artwork.height)?;
    ensure!(
        artwork.crop.x == 0
            && artwork.crop.y == 0
            && artwork.crop.width == artwork.width
            && artwork.crop.height == artwork.height,
        "tracked title artwork must expose its complete consumer frame"
    );
    ensure!(
        artwork.preserved_component_regions.is_empty(),
        "tracked title artwork must exclude source-owned component regions"
    );
    ensure!(
        artwork.generation_supporting_references.len() == 1,
        "title artwork must declare its one supporting character reference"
    );
    let supporting_reference = &artwork.generation_supporting_references[0];
    ensure!(
        supporting_reference.role == "character_identity_anatomy_only_not_edit_target"
            && supporting_reference.page_url.starts_with("https://")
            && supporting_reference.asset_url.starts_with("https://")
            && is_sha256(&supporting_reference.sha256),
        "title artwork supporting character reference is incomplete"
    );
    Ok(())
}

fn validate_consumer(consumer: &ConsumerInput) -> Result<()> {
    ensure!(
        consumer.screen_width == TITLE_SCREEN_WIDTH
            && consumer.screen_height == TITLE_SCREEN_HEIGHT,
        "title artwork consumer must be the 640x400 title screen"
    );
    ensure!(
        consumer.sampling_width > 0
            && consumer.sampling_height > 0
            && TITLE_SCREEN_WIDTH.is_multiple_of(consumer.sampling_width)
            && TITLE_SCREEN_HEIGHT.is_multiple_of(consumer.sampling_height),
        "title artwork sampling grid must divide the 640x400 title screen"
    );
    ensure!(
        consumer.sampling_width * TITLE_SCREEN_HEIGHT
            == consumer.sampling_height * TITLE_SCREEN_WIDTH,
        "title artwork sampling grid must have the 8:5 title-screen aspect ratio"
    );
    validate_sampling_placement(
        &consumer.sampling_placement,
        consumer.sampling_width,
        consumer.sampling_height,
    )?;
    ensure!(
        consumer.palette == PALETTE,
        "title artwork must target its declared PC-98 RGB4 runtime palette"
    );
    ensure!(
        consumer
            .runtime_palette_rgb4
            .iter()
            .flatten()
            .all(|component| *component <= 0x0f),
        "title artwork runtime palette components must fit PC-98 RGB4"
    );
    ensure!(
        consumer
            .runtime_palette_rgb4
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            == 16,
        "title artwork runtime palette must contain 16 distinct colors"
    );
    ensure!(
        consumer.runtime_palette_rgb4 == SOURCE_TITLE_PALETTE_RGB4,
        "title artwork must preserve the complete source title runtime palette"
    );
    ensure!(
        consumer.quantization == QUANTIZATION,
        "title artwork must use the declared runtime-palette quantization"
    );
    ensure!(
        consumer.background_palette_index == 0,
        "title artwork background must use preserved black palette index 0"
    );
    ensure!(
        consumer.background_palette_indices == [0, 1],
        "title artwork must recognize the source black and dark-red background indices"
    );
    ensure!(
        consumer.background_policy == BACKGROUND_POLICY,
        "title artwork must preserve the source title background producer"
    );
    ensure!(
        consumer.outside_transfer_policy == OUTSIDE_TRANSFER_POLICY,
        "title artwork outside-transfer policy changed"
    );
    ensure!(
        consumer.outside_transfer_trimmable_palette_indices
            == OUTSIDE_TRANSFER_TRIMMABLE_PALETTE_INDICES,
        "title artwork may trim only the neutral silver backing palette indices"
    );
    ensure!(
        consumer.content_transfer_count == TITLE_CONTENT_TRANSFERS.len(),
        "title artwork content-transfer population changed"
    );
    ensure!(
        consumer.protected_components == PROTECTED_COMPONENTS.map(str::to_owned).to_vec(),
        "title artwork protected-component declaration changed"
    );
    ensure!(
        !consumer.title2_animation_enabled,
        "TITLE2 animation must be disabled when the base title artwork is replaced"
    );
    Ok(())
}

fn validate_sampling_placement(
    placement: &SamplingPlacementInput,
    sampling_width: usize,
    sampling_height: usize,
) -> Result<()> {
    ensure!(
        placement.width > 0 && placement.height > 0,
        "title artwork sampling placement is empty"
    );
    ensure!(
        placement
            .x
            .checked_add(placement.width)
            .is_some_and(|end| end <= sampling_width)
            && placement
                .y
                .checked_add(placement.height)
                .is_some_and(|end| end <= sampling_height),
        "title artwork sampling placement exceeds its canvas"
    );
    ensure!(
        placement.width * TITLE_SCREEN_HEIGHT == placement.height * TITLE_SCREEN_WIDTH,
        "title artwork sampling placement must preserve the 8:5 frame aspect ratio"
    );
    Ok(())
}

fn validate_artwork_crop(
    crop: &CropInput,
    artwork_width: usize,
    artwork_height: usize,
) -> Result<()> {
    ensure!(
        crop.width > 0 && crop.height > 0,
        "title artwork crop is empty"
    );
    ensure!(
        crop.x
            .checked_add(crop.width)
            .is_some_and(|end| end <= artwork_width)
            && crop
                .y
                .checked_add(crop.height)
                .is_some_and(|end| end <= artwork_height),
        "title artwork crop exceeds its RGB frame"
    );
    let scaled_width = crop
        .width
        .checked_mul(TITLE_SCREEN_HEIGHT)
        .context("title artwork crop width overflow")?;
    let scaled_height = crop
        .height
        .checked_mul(TITLE_SCREEN_WIDTH)
        .context("title artwork crop height overflow")?;
    let difference = scaled_width.abs_diff(scaled_height);
    ensure!(
        difference.saturating_mul(1000) <= scaled_height,
        "title artwork crop differs from the 8:5 title screen by more than 0.1%"
    );
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
#[path = "title_art_manifest_tests.rs"]
mod title_art_manifest_tests;
