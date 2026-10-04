mod consumer;
mod frame_placement;
mod layout;
mod manifest;
mod model;
mod payload;
mod planar;
mod plans;
mod raster;
mod selection;
mod source_coordinates;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, ensure};

use self::consumer::{find_asset, validate_arle_consumer, validate_frame_population};
use self::frame_placement::SubjectScale;
use self::layout::{
    AuthoringPanel, ConsumerFrame, SheetLayout, fit_authoring_panel, indexed_visible_bounds,
};
use self::manifest::load_manifest;
use self::model::{PatchedArleCharacterAssetPayload, PreparedArleAssetFile};
use self::payload::{decode_single_asset, file_report, required, verify_replacement_readback};
use self::planar::{decode_compact_brgi_indices, encode_compact_brgi_indices};
use self::plans::character_asset_plan;
use self::raster::{load_rgb_sheet, validate_declared_palette};
use self::source_coordinates::SourceCoordinateTransform;
use super::monochrome_text::DevelopmentBuildStatus;
use super::payload_writes::apply_payload_write_plans;
use crate::asset_bindings::catalog_character_sprites;
use crate::character_runtime::{expand_character_palette, read_character_palette_rgb4};
use crate::localization_assets::encode_single_stream;
use crate::source_disk::SOURCE_DISK_SHA256;

pub(super) const TARGET_FILES: [&str; 2] = ["C07", "C08"];

pub use model::{ArleCharacterAssetPatchReport, ArleCharacterFileReport, ArleCharacterFrameReport};
pub(crate) use selection::ResolvedArleCharacterAssetSelection;
pub use selection::{
    ARLE_CHARACTER_ASSET_SET_FILENAMES, ArleCharacterAssetSelection, ArleCharacterAssetSet,
};

pub(crate) fn build_arle_character_asset_payload(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    manifest_path: &Path,
    artwork_path: &Path,
) -> Result<PatchedArleCharacterAssetPayload> {
    let manifest = load_manifest(manifest_path)?;
    validate_arle_consumer(installer_payload, &manifest)?;
    let sheet = load_rgb_sheet(
        artwork_path,
        &manifest.source.artwork_sha256,
        manifest.source.width,
        manifest.source.height,
    )?;
    validate_declared_palette(&sheet, &manifest.source.palette_rgb)?;
    let mad_com = required(installer_payload, "MAD.COM")?;
    let runtime_palette_rgb4 = read_character_palette_rgb4(mad_com)?;
    let palette = expand_character_palette(&runtime_palette_rgb4);
    let catalog = catalog_character_sprites(mad_com, installer_payload)?;
    validate_frame_population(&catalog.assets, &manifest.frame_bindings)?;

    let mut decoded_assets = TARGET_FILES
        .iter()
        .map(|file_name| {
            Ok((
                (*file_name).to_owned(),
                decode_single_asset(installer_payload, file_name)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut frame_reports = Vec::with_capacity(manifest.frame_bindings.len());
    let mut source_coordinate_transforms = BTreeMap::<String, SourceCoordinateTransform>::new();
    for binding in &manifest.frame_bindings {
        let asset = find_asset(&catalog.assets, &binding.target_asset)?;
        let transfer = asset
            .transfers
            .iter()
            .find(|transfer| {
                transfer.source_start == binding.target_source_start
                    && transfer.width_pixels == binding.target_width
                    && transfer.height == binding.target_height
            })
            .with_context(|| {
                format!(
                    "Arle frame {} has no matching Bayoen consumer transfer",
                    binding.role
                )
            })?;
        let decoded = decoded_assets
            .get_mut(&binding.target_asset)
            .context("Arle frame names an unprepared target asset")?;
        let source_indices = decode_compact_brgi_indices(
            &decoded[transfer.source_start..transfer.source_end],
            transfer.width_pixels,
            transfer.height,
        )?;
        let source_visible = indexed_visible_bounds(
            &source_indices,
            transfer.width_pixels,
            transfer.height,
            manifest.consumer.background_palette_index,
        )?;
        let source_coordinate_transform = binding
            .source_coordinate_reference
            .as_ref()
            .map(|role| {
                source_coordinate_transforms
                    .get(role)
                    .copied()
                    .with_context(|| {
                        format!(
                            "Arle frame {} references unavailable source coordinates from {role}",
                            binding.role
                        )
                    })
            })
            .transpose()?;
        let converted = fit_authoring_panel(
            &sheet,
            AuthoringPanel {
                sheet: SheetLayout {
                    columns: manifest.source.columns,
                    rows: manifest.source.rows,
                    panel_width: manifest.source.panel_width,
                    panel_height: manifest.source.panel_height,
                },
                frame_index: binding.authoring_frame_index,
                background: manifest.source.background_rgb,
            },
            ConsumerFrame {
                width: transfer.width_pixels,
                height: transfer.height,
                visible_bounds: source_visible,
                background_index: manifest.consumer.background_palette_index,
                placement: binding.placement,
                source_coordinate_transform,
                subject_scale: binding
                    .subject_scale
                    .as_ref()
                    .map(|reference| SubjectScale {
                        source_y: reference.source_bounds.y,
                        source_height: reference.source_bounds.height,
                        target_height: reference.target_height,
                        target_baseline_y: reference.target_baseline_y,
                    }),
            },
            &palette,
        )?;
        source_coordinate_transforms.insert(binding.role.clone(), converted.coordinate_transform);
        let encoded = encode_compact_brgi_indices(
            &converted.indices,
            transfer.width_pixels,
            transfer.height,
        )?;
        ensure!(
            encoded.len() == transfer.source_end - transfer.source_start,
            "converted Arle frame changed its decoded storage extent"
        );
        decoded[transfer.source_start..transfer.source_end].copy_from_slice(&encoded);
        frame_reports.push(ArleCharacterFrameReport {
            target_asset: binding.target_asset.clone(),
            target_source_start: binding.target_source_start,
            target_width: binding.target_width,
            target_height: binding.target_height,
            authoring_frame_index: binding.authoring_frame_index,
            role: binding.role.clone(),
            placement: binding.placement.as_str().to_owned(),
            source_coordinate_reference: binding.source_coordinate_reference.clone(),
            subject_source_height: binding
                .subject_scale
                .as_ref()
                .map(|reference| reference.source_bounds.height),
            subject_target_height: binding
                .subject_scale
                .as_ref()
                .map(|reference| reference.target_height),
            subject_target_baseline_y: binding
                .subject_scale
                .as_ref()
                .map(|reference| reference.target_baseline_y),
            source_visible_x: source_visible.x,
            source_visible_y: source_visible.y,
            source_visible_width: source_visible.width,
            source_visible_height: source_visible.height,
            replacement_visible_x: converted.visible_bounds.x,
            replacement_visible_y: converted.visible_bounds.y,
            replacement_visible_width: converted.visible_bounds.width,
            replacement_visible_height: converted.visible_bounds.height,
            used_palette_color_count: converted.used_palette_color_count,
        });
    }

    let mut replacements = Vec::with_capacity(TARGET_FILES.len());
    for file_name in TARGET_FILES {
        let decoded = decoded_assets
            .remove(file_name)
            .context("prepared Arle target asset disappeared")?;
        let original = decode_single_asset(installer_payload, file_name)?;
        ensure!(
            decoded.len() == original.len(),
            "Arle replacement changed decoded size"
        );
        ensure!(
            decoded != original,
            "Arle replacement makes no pixel changes to {file_name}"
        );
        let packed = encode_single_stream(&decoded)?;
        let source_packed = required(installer_payload, file_name)?;
        ensure!(
            packed.len() <= source_packed.len(),
            "Arle {file_name} replacement packed size {} exceeds the source-proven input extent {}",
            packed.len(),
            source_packed.len()
        );
        replacements.push(PreparedArleAssetFile {
            file_name,
            decoded,
            packed,
        });
    }
    let plans = replacements
        .iter()
        .map(|replacement| character_asset_plan(installer_payload, replacement))
        .collect();
    let applied = apply_payload_write_plans(installer_payload, plans)?;
    verify_replacement_readback(mad_com, &applied.files, &replacements)?;
    let mut file_reports = replacements
        .iter()
        .map(|replacement| file_report(installer_payload, replacement))
        .collect::<Result<Vec<_>>>()?;
    file_reports.sort_by(|left, right| left.file_name.cmp(&right.file_name));

    Ok(PatchedArleCharacterAssetPayload {
        files: applied.files,
        report: ArleCharacterAssetPatchReport {
            supported_source_sha256: SOURCE_DISK_SHA256.to_owned(),
            build_status: DevelopmentBuildStatus::DevelopmentOnly,
            approval_status: manifest.approval_status,
            manifest_id: manifest.id,
            manifest_sha256: manifest.manifest_sha256,
            origin_repository: manifest.source.origin_repository,
            origin_commit: manifest.source.origin_commit,
            origin_manifest_sha256: manifest.source.origin_manifest_sha256,
            artwork_sha256: manifest.source.artwork_sha256,
            authoring_sha256: manifest.source.authoring_sha256,
            source_palette: manifest.source.palette,
            source_actor_entry_id: manifest.consumer.unit_name_entry_id,
            character_slot_index: manifest.consumer.slot_index,
            palette: manifest.consumer.palette,
            palette_table_file_offset: manifest.consumer.palette_table_file_offset,
            runtime_palette_rgb4,
            runtime_palette_rgb: palette,
            quantization: manifest.consumer.quantization,
            placement: manifest.consumer.placement,
            frame_count: frame_reports.len(),
            frames: frame_reports,
            files: file_reports,
            writes: applied.report,
        },
    })
}
