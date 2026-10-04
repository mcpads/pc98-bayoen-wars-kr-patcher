use std::collections::BTreeMap;

use serde::Serialize;

use crate::korean_patch::font_catalog::LocalizationFontProfileReport;
use crate::korean_patch::monochrome_text::DevelopmentBuildStatus;
use crate::korean_patch::payload_writes::PayloadWriteReport;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BakedGraphicsAssetPatchReport {
    pub source_asset: String,
    pub translation_unit_ids: Vec<String>,
    pub decoded_sha256: String,
    pub original_packed_size: usize,
    pub replacement_packed_size: usize,
    pub replacement_packed_sha256: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BakedGraphicsTextPatchReport {
    pub supported_source_sha256: String,
    pub translation_status: String,
    pub build_status: DevelopmentBuildStatus,
    pub fonts: Vec<LocalizationFontProfileReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_artwork: Option<TitleArtworkPatchReport>,
    pub assets: Vec<BakedGraphicsAssetPatchReport>,
    pub writes: Vec<PayloadWriteReport>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TitleArtworkPatchReport {
    pub id: String,
    pub input_kind: String,
    pub title_animation_source_offset: usize,
    pub source_preview_sha256: String,
    pub artwork_kind: String,
    pub artwork_sha256: String,
    pub artwork_width: usize,
    pub artwork_height: usize,
    pub master_color_mode: String,
    pub conversion_source_sha256: String,
    pub conversion_source_width: usize,
    pub conversion_source_height: usize,
    pub conversion_source_region_x: usize,
    pub conversion_source_region_y: usize,
    pub conversion_source_region_width: usize,
    pub conversion_source_region_height: usize,
    pub conversion_output_rgb_sha256: String,
    pub conversion_method: String,
    pub generation_supporting_reference_count: usize,
    pub crop_x: usize,
    pub crop_y: usize,
    pub crop_width: usize,
    pub crop_height: usize,
    pub preserved_component_region_count: usize,
    pub sampling_width: usize,
    pub sampling_height: usize,
    pub sampling_placement_x: usize,
    pub sampling_placement_y: usize,
    pub sampling_placement_width: usize,
    pub sampling_placement_height: usize,
    pub output_width: usize,
    pub output_height: usize,
    pub palette: String,
    pub runtime_palette_rgb4: [[u8; 3]; 16],
    pub quantization: String,
    pub background_palette_index: u8,
    pub background_palette_indices: Vec<u8>,
    pub background_policy: String,
    pub outside_transfer_policy: String,
    pub outside_transfer_trimmable_palette_indices: Vec<u8>,
    pub source_pixels_restored_outside_transfers: usize,
    pub source_visible_x: usize,
    pub source_visible_y: usize,
    pub source_visible_width: usize,
    pub source_visible_height: usize,
    pub replacement_visible_x: usize,
    pub replacement_visible_y: usize,
    pub replacement_visible_width: usize,
    pub replacement_visible_height: usize,
    pub edge_connected_background_pixels_preserved: usize,
    pub edge_connected_background_pixel_indices_restored: usize,
    pub artwork_owned_background_pixels_retained: usize,
    pub source_logo_pixels_replaced_with_background: usize,
    pub used_palette_color_count: usize,
    pub content_transfer_count: usize,
    pub protected_components: Vec<String>,
    pub title2_animation_enabled: bool,
    pub title2_animation_end_fallback_disabled: bool,
    pub replacement_packed_size: usize,
    pub maximum_packed_size: usize,
    pub replacement_decode_command_count: usize,
    pub maximum_decode_command_count: usize,
    pub approval_status: String,
}

pub(crate) struct PatchedBakedGraphicsPayload {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: BakedGraphicsTextPatchReport,
}

pub(super) struct AssetPatch {
    pub file_name: &'static str,
    pub producer_id: &'static str,
    pub unit_ids: Vec<String>,
    pub decoded: Vec<u8>,
    pub packed: Vec<u8>,
}
