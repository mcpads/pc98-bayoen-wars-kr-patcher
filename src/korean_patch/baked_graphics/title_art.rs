use std::collections::BTreeSet;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result, ensure};

use super::model::{AssetPatch, TitleArtworkPatchReport};
use super::title_art_bounds::{
    VisibleTitleBounds, validate_replacement_title_bounds, visible_title_bounds,
};
use super::title_art_layout::{
    PixelRect, RgbFrame, SamplingPlacement, TitleFrameLayout, place_and_quantize_title_frame,
};
use super::title_art_manifest::{TitleArtworkManifest, validate_title_artwork_manifest};
use super::title_art_transfer_fit::{
    TitleTransferFitStats, preserve_source_outside_title_transfers,
};
use super::title_background_texture::{
    TitleBackgroundTextureStats, restore_edge_connected_background_texture,
};
use crate::localization_assets::decode_all_streams;
use crate::source_disk::sha256_hex;
use crate::title_runtime::{
    TitlePaletteRgb4, expand_title_palette, read_title_animation_frame_sources,
    read_title_palette_rgb4,
};
use crate::title_screen::{
    TITLE_SCREEN_HEIGHT, TITLE_SCREEN_WIDTH, render_title_color_indices_with_animation,
    render_title_screen_with_animation, replace_title_artwork_preserving_source_background,
};

pub(crate) struct TitleArtworkInput<'a> {
    pub(crate) manifest_path: &'a Path,
    pub(crate) source_preview_path: &'a Path,
    pub(crate) artwork_path: &'a Path,
}

pub(super) struct PreparedTitleArtwork {
    decoded: Vec<u8>,
    manifest: TitleArtworkManifest,
    used_palette_color_count: usize,
    background_texture_stats: TitleBackgroundTextureStats,
    transfer_fit_stats: TitleTransferFitStats,
    source_visible_bounds: VisibleTitleBounds,
    replacement_visible_bounds: VisibleTitleBounds,
}

impl PreparedTitleArtwork {
    pub(super) fn into_parts(self) -> (Vec<u8>, TitleArtworkEvidence, TitlePaletteRgb4) {
        let runtime_palette_rgb4 = self.manifest.consumer.runtime_palette_rgb4;
        let evidence = TitleArtworkEvidence {
            manifest: self.manifest,
            used_palette_color_count: self.used_palette_color_count,
            background_texture_stats: self.background_texture_stats,
            transfer_fit_stats: self.transfer_fit_stats,
            source_visible_bounds: self.source_visible_bounds,
            replacement_visible_bounds: self.replacement_visible_bounds,
        };
        (self.decoded, evidence, runtime_palette_rgb4)
    }
}

pub(super) struct TitleArtworkEvidence {
    manifest: TitleArtworkManifest,
    used_palette_color_count: usize,
    background_texture_stats: TitleBackgroundTextureStats,
    transfer_fit_stats: TitleTransferFitStats,
    source_visible_bounds: VisibleTitleBounds,
    replacement_visible_bounds: VisibleTitleBounds,
}

pub(super) fn prepare_title_artwork(
    source_packed: &[u8],
    source_decoded: &[u8],
    mad_com: &[u8],
    title_animation_decoded: &[u8],
    translation_draft_sha256: &str,
    input: TitleArtworkInput<'_>,
) -> Result<PreparedTitleArtwork> {
    let manifest_bytes = fs::read(input.manifest_path).with_context(|| {
        format!(
            "failed to read title artwork manifest {}",
            input.manifest_path.display()
        )
    })?;
    let manifest: TitleArtworkManifest = serde_json::from_slice(&manifest_bytes)
        .context("failed to parse title artwork manifest")?;
    validate_title_artwork_manifest(
        &manifest,
        source_packed,
        source_decoded,
        translation_draft_sha256,
    )?;

    let source_preview_bytes = read_matching_file(
        input.source_preview_path,
        &manifest.input.source_preview_sha256,
        "source-extracted title preview",
    )?;
    let source_preview = decode_rgb_png(&source_preview_bytes, "source-extracted title preview")?;
    ensure!(
        source_preview.width == TITLE_SCREEN_WIDTH && source_preview.height == TITLE_SCREEN_HEIGHT,
        "source-extracted title preview must be 640x400"
    );
    let source_palette = expand_title_palette(&read_title_palette_rgb4(mad_com)?);
    ensure!(
        read_title_animation_frame_sources(mad_com)?.contains(&u16::try_from(
            manifest.input.title_animation_source_offset
        )?),
        "source-extracted title preview selects an animation frame absent from MAD.COM"
    );
    let rendered_source = render_title_screen_with_animation(
        source_decoded,
        title_animation_decoded,
        manifest.input.title_animation_source_offset,
        &source_palette,
    )?;
    ensure!(
        source_preview.pixels == rendered_source.pixels,
        "source-extracted title preview pixels do not match the verified title composition"
    );
    let source_indices = render_title_color_indices_with_animation(
        source_decoded,
        title_animation_decoded,
        manifest.input.title_animation_source_offset,
    )?;

    let artwork_bytes = read_matching_file(
        input.artwork_path,
        &manifest.artwork.sha256,
        "generated title artwork",
    )?;
    let artwork = decode_rgb_png(&artwork_bytes, "generated title artwork")?;
    ensure!(
        artwork.width == manifest.artwork.width && artwork.height == manifest.artwork.height,
        "generated title artwork dimensions differ from its manifest"
    );
    ensure!(
        sha256_hex(&artwork.pixels) == manifest.artwork.conversion.output_rgb_sha256,
        "project-authored title frame RGB pixels differ from its conversion lineage"
    );
    let runtime_palette = expand_title_palette(&manifest.consumer.runtime_palette_rgb4);
    let mut indices = place_and_quantize_title_frame(
        &artwork,
        PixelRect {
            x: manifest.artwork.crop.x,
            y: manifest.artwork.crop.y,
            width: manifest.artwork.crop.width,
            height: manifest.artwork.crop.height,
        },
        &[],
        TitleFrameLayout {
            sampling_width: manifest.consumer.sampling_width,
            sampling_height: manifest.consumer.sampling_height,
            placement: SamplingPlacement {
                x: manifest.consumer.sampling_placement.x,
                y: manifest.consumer.sampling_placement.y,
                width: manifest.consumer.sampling_placement.width,
                height: manifest.consumer.sampling_placement.height,
            },
            output_width: TITLE_SCREEN_WIDTH,
            output_height: TITLE_SCREEN_HEIGHT,
            background_palette_index: manifest.consumer.background_palette_index,
        },
        &runtime_palette,
    )?;
    let transfer_fit_stats = preserve_source_outside_title_transfers(
        &source_indices,
        &mut indices,
        &manifest.consumer.background_palette_indices,
        &manifest.consumer.outside_transfer_trimmable_palette_indices,
    )?;
    let background_texture_stats = restore_edge_connected_background_texture(
        &source_indices,
        &mut indices,
        &manifest.consumer.background_palette_indices,
    )?;
    let source_visible_bounds = visible_title_bounds(
        &source_indices,
        &manifest.consumer.background_palette_indices,
    )?;
    let replacement_visible_bounds =
        visible_title_bounds(&indices, &manifest.consumer.background_palette_indices)?;
    validate_replacement_title_bounds(source_visible_bounds, replacement_visible_bounds)?;
    let used_palette_color_count = indices.iter().copied().collect::<BTreeSet<_>>().len();
    let mut decoded = source_decoded.to_vec();
    replace_title_artwork_preserving_source_background(
        &mut decoded,
        &indices,
        &manifest.consumer.background_palette_indices,
    )?;

    Ok(PreparedTitleArtwork {
        decoded,
        manifest,
        used_palette_color_count,
        background_texture_stats,
        transfer_fit_stats,
        source_visible_bounds,
        replacement_visible_bounds,
    })
}

pub(super) fn complete_title_artwork_report(
    patch: &AssetPatch,
    evidence: TitleArtworkEvidence,
) -> Result<TitleArtworkPatchReport> {
    let streams = decode_all_streams(&patch.packed)?;
    ensure!(
        streams.len() == 1,
        "generated TITLE.DAT must contain exactly one Compile LZ stream"
    );
    let command_count = streams[0].command_count;
    ensure!(
        patch.packed.len() <= evidence.manifest.consumer.maximum_packed_size,
        "generated TITLE.DAT packed size {} exceeds the source-proven maximum {}",
        patch.packed.len(),
        evidence.manifest.consumer.maximum_packed_size
    );
    ensure!(
        command_count <= evidence.manifest.consumer.maximum_decode_command_count,
        "generated TITLE.DAT decode command count {command_count} exceeds the source-proven maximum {}",
        evidence.manifest.consumer.maximum_decode_command_count
    );

    Ok(TitleArtworkPatchReport {
        id: evidence.manifest.id,
        input_kind: evidence.manifest.input.kind,
        title_animation_source_offset: evidence.manifest.input.title_animation_source_offset,
        source_preview_sha256: evidence.manifest.input.source_preview_sha256,
        artwork_kind: evidence.manifest.artwork.kind,
        artwork_sha256: evidence.manifest.artwork.sha256,
        artwork_width: evidence.manifest.artwork.width,
        artwork_height: evidence.manifest.artwork.height,
        master_color_mode: evidence.manifest.artwork.color_mode,
        conversion_source_sha256: evidence.manifest.artwork.conversion.source_sha256,
        conversion_source_width: evidence.manifest.artwork.conversion.source_width,
        conversion_source_height: evidence.manifest.artwork.conversion.source_height,
        conversion_source_region_x: evidence.manifest.artwork.conversion.source_region.x,
        conversion_source_region_y: evidence.manifest.artwork.conversion.source_region.y,
        conversion_source_region_width: evidence.manifest.artwork.conversion.source_region.width,
        conversion_source_region_height: evidence.manifest.artwork.conversion.source_region.height,
        conversion_output_rgb_sha256: evidence.manifest.artwork.conversion.output_rgb_sha256,
        conversion_method: evidence.manifest.artwork.conversion.method,
        generation_supporting_reference_count: evidence
            .manifest
            .artwork
            .generation_supporting_references
            .len(),
        crop_x: evidence.manifest.artwork.crop.x,
        crop_y: evidence.manifest.artwork.crop.y,
        crop_width: evidence.manifest.artwork.crop.width,
        crop_height: evidence.manifest.artwork.crop.height,
        preserved_component_region_count: evidence
            .manifest
            .artwork
            .preserved_component_regions
            .len(),
        sampling_width: evidence.manifest.consumer.sampling_width,
        sampling_height: evidence.manifest.consumer.sampling_height,
        sampling_placement_x: evidence.manifest.consumer.sampling_placement.x,
        sampling_placement_y: evidence.manifest.consumer.sampling_placement.y,
        sampling_placement_width: evidence.manifest.consumer.sampling_placement.width,
        sampling_placement_height: evidence.manifest.consumer.sampling_placement.height,
        output_width: evidence.manifest.consumer.screen_width,
        output_height: evidence.manifest.consumer.screen_height,
        palette: evidence.manifest.consumer.palette,
        runtime_palette_rgb4: evidence.manifest.consumer.runtime_palette_rgb4,
        quantization: evidence.manifest.consumer.quantization,
        background_palette_index: evidence.manifest.consumer.background_palette_index,
        background_palette_indices: evidence.manifest.consumer.background_palette_indices,
        background_policy: evidence.manifest.consumer.background_policy,
        outside_transfer_policy: evidence.manifest.consumer.outside_transfer_policy,
        outside_transfer_trimmable_palette_indices: evidence
            .manifest
            .consumer
            .outside_transfer_trimmable_palette_indices,
        source_pixels_restored_outside_transfers: evidence
            .transfer_fit_stats
            .source_pixels_restored_outside_transfers,
        source_visible_x: evidence.source_visible_bounds.x,
        source_visible_y: evidence.source_visible_bounds.y,
        source_visible_width: evidence.source_visible_bounds.width,
        source_visible_height: evidence.source_visible_bounds.height,
        replacement_visible_x: evidence.replacement_visible_bounds.x,
        replacement_visible_y: evidence.replacement_visible_bounds.y,
        replacement_visible_width: evidence.replacement_visible_bounds.width,
        replacement_visible_height: evidence.replacement_visible_bounds.height,
        edge_connected_background_pixels_preserved: evidence
            .background_texture_stats
            .edge_connected_background_pixels_preserved,
        edge_connected_background_pixel_indices_restored: evidence
            .background_texture_stats
            .edge_connected_background_pixel_indices_restored,
        artwork_owned_background_pixels_retained: evidence
            .background_texture_stats
            .artwork_owned_background_pixels_retained,
        source_logo_pixels_replaced_with_background: evidence
            .background_texture_stats
            .source_logo_pixels_replaced_with_background,
        used_palette_color_count: evidence.used_palette_color_count,
        content_transfer_count: evidence.manifest.consumer.content_transfer_count,
        protected_components: evidence.manifest.consumer.protected_components,
        title2_animation_enabled: evidence.manifest.consumer.title2_animation_enabled,
        title2_animation_end_fallback_disabled: true,
        replacement_packed_size: patch.packed.len(),
        maximum_packed_size: evidence.manifest.consumer.maximum_packed_size,
        replacement_decode_command_count: command_count,
        maximum_decode_command_count: evidence.manifest.consumer.maximum_decode_command_count,
        approval_status: "needs_human_review".to_owned(),
    })
}

fn read_matching_file(path: &Path, expected_sha256: &str, role: &str) -> Result<Vec<u8>> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read {role} {}", path.display()))?;
    ensure!(
        sha256_hex(&bytes) == expected_sha256,
        "{role} SHA-256 differs from its manifest"
    );
    Ok(bytes)
}

fn decode_rgb_png(bytes: &[u8], role: &str) -> Result<RgbFrame> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder
        .read_info()
        .with_context(|| format!("failed to read {role} PNG header"))?;
    let mut pixels = vec![
        0;
        reader
            .output_buffer_size()
            .context("PNG output buffer size overflow")?
    ];
    let info = reader
        .next_frame(&mut pixels)
        .with_context(|| format!("failed to decode {role} PNG"))?;
    ensure!(
        info.color_type == png::ColorType::Rgb && info.bit_depth == png::BitDepth::Eight,
        "{role} must be an opaque 8-bit RGB PNG"
    );
    pixels.truncate(info.buffer_size());
    Ok(RgbFrame {
        width: usize::try_from(info.width)?,
        height: usize::try_from(info.height)?,
        pixels,
    })
}

#[cfg(test)]
#[path = "title_art_tests.rs"]
mod title_art_tests;
