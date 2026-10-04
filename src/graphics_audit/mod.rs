mod battle_backgrounds;
mod battle_graphics;
mod battle_maps;
mod character_asset_comparison;
mod character_sprites;
mod comparison;
mod contact_sheet;
mod decoded;
mod defeat;
mod diagnostic_assets;
mod dialogue_text;
mod fixed_gaiji_text;
mod gaiji;
mod interface_text;
mod masked_sheet;
mod monochrome_pages;
mod monochrome_sheet;
mod monochrome_sprites;
mod output;
mod pc98_font;
mod pc98_text;
mod planar;
mod scene_sheet;
mod selection;
mod text_comparison;
mod tile_graphics;
mod tile_map;
mod tile_sheet;
mod title;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::GraphicsAuditReport;

pub(super) use comparison::LocalizationComparisonInputs;

pub(super) fn write_graphics_audit(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let mut previews = scene_sheet::render_scene_sheet_previews(installer_payload)?;
    let mad_com = installer_payload
        .get("MAD.COM")
        .ok_or_else(|| anyhow::anyhow!("verified installer payload is missing MAD.COM"))?;
    previews.extend(character_sprites::render_character_sprite_previews(
        mad_com,
        installer_payload,
    )?);
    previews.extend(battle_maps::render_battle_map_previews(
        mad_com,
        installer_payload,
    )?);
    previews.extend(battle_backgrounds::render_battle_background_previews(
        mad_com,
        installer_payload,
    )?);
    previews.extend(battle_graphics::render_battle_graphics_previews(
        mad_com,
        installer_payload,
    )?);
    previews.extend(tile_graphics::render_tile_graphics_previews(
        mad_com,
        installer_payload,
    )?);
    previews.extend(monochrome_sprites::render_monochrome_sprite_previews(
        mad_com,
        installer_payload,
    )?);
    previews.extend(monochrome_pages::render_monochrome_page_previews(
        mad_com,
        installer_payload,
    )?);
    previews.extend(selection::render_selection_previews(installer_payload)?);
    previews.extend(title::render_title_previews(mad_com, installer_payload)?);
    previews.extend(title::render_title_source_frame_previews(
        mad_com,
        installer_payload,
    )?);
    previews.push(gaiji::render_gaiji_preview(installer_payload)?);
    previews.push(defeat::render_defeat_preview(installer_payload)?);
    previews.extend(diagnostic_assets::render_diagnostic_previews(
        installer_payload,
    )?);
    output::publish_previews(output_directory, previews)
}

pub(super) fn write_monochrome_text_audit(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let mad_com = installer_payload
        .get("MAD.COM")
        .ok_or_else(|| anyhow::anyhow!("patched installer payload is missing MAD.COM"))?;
    let previews = monochrome_pages::render_monochrome_page_previews(mad_com, installer_payload)?;
    output::publish_previews(output_directory, previews)
}

pub(super) fn write_narrative_graphics_audit(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let mad_com = installer_payload
        .get("MAD.COM")
        .ok_or_else(|| anyhow::anyhow!("patched installer payload is missing MAD.COM"))?;
    let mut previews =
        monochrome_pages::render_monochrome_page_previews(mad_com, installer_payload)?;
    previews.extend(selection::render_selection_previews(installer_payload)?);
    previews.extend(title::render_title_previews(mad_com, installer_payload)?);
    output::publish_previews(output_directory, previews)
}

pub(super) fn write_title_artwork_audit(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let mad_com = installer_payload
        .get("MAD.COM")
        .ok_or_else(|| anyhow::anyhow!("patched installer payload is missing MAD.COM"))?;
    let mut previews =
        monochrome_pages::render_monochrome_page_previews(mad_com, installer_payload)?;
    previews.extend(selection::render_selection_previews(installer_payload)?);
    previews.extend(title::render_title_previews(mad_com, installer_payload)?);
    previews.push(title::render_title_runtime_palette_preview(
        mad_com,
        installer_payload,
    )?);
    output::publish_previews(output_directory, previews)
}

pub(super) fn write_localization_comparison_audit(
    inputs: LocalizationComparisonInputs<'_>,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    comparison::write_localization_comparison_audit(inputs, output_directory)
}

pub(super) fn write_arle_character_asset_audit(
    source: &BTreeMap<String, Vec<u8>>,
    replacement: &BTreeMap<String, Vec<u8>>,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let previews =
        character_asset_comparison::render_arle_character_comparisons(source, replacement)?;
    output::publish_previews(output_directory, previews)
}

pub(super) fn write_fixed_gaiji_text_audit(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let previews = fixed_gaiji_text::render_fixed_gaiji_text_previews(installer_payload)?;
    output::publish_previews(output_directory, previews)
}

pub(super) fn write_interface_text_audit(
    report: &crate::InterfaceTextPatchReport,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let previews = interface_text::render_interface_text_previews(report)?;
    output::publish_previews(output_directory, previews)
}

pub(super) fn write_dialogue_text_audit(
    report: &crate::DialogueTextPatchReport,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let previews = dialogue_text::render_dialogue_text_previews(report)?;
    output::publish_previews(output_directory, previews)
}

pub(super) struct Preview {
    source_asset: String,
    output_file: String,
    evidence: String,
    image: planar::RgbImage,
}
