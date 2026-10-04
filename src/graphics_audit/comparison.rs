use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, ensure};

use super::Preview;
use super::contact_sheet::compose_contact_sheet;
use super::pc98_font::{Pc98Font, SUPPORTED_PC98_FONT_BMP_SHA256};
use super::planar::RgbImage;
use super::text_comparison::{
    render_fixed_gaiji_slot, render_text_lines, replacement_gaiji_overrides, source_gaiji_overrides,
};
use crate::asset_bindings::catalog_baked_text;
use crate::game_data::catalog_game_data;
use crate::{
    DialogueTextPatchReport, GraphicsAuditReport, InterfaceTextPatchReport, MenuTextPatchReport,
};

const MENU_ID: &str = "menu-text-002";
const INTERFACE_ID: &str = "interface-text-001";
const DIALOGUE_ID: &str = "dialogue-g01-r01";
const FIXED_ID: &str = "fixed-gaiji-text-01";
const OPENING_ID: &str = "opening-page-01.png";
const DIFFICULTY_ID: &str = "difficulty-easy";
const SELECTION_ID: &str = "character-name-03";
const SOURCE_TITLE_FRAME: &str = "title-source-frame-0c00.png";

pub(crate) struct LocalizationComparisonInputs<'a> {
    pub(crate) source: &'a BTreeMap<String, Vec<u8>>,
    pub(crate) pc98_font_bmp: &'a Path,
    pub(crate) menu_files: &'a BTreeMap<String, Vec<u8>>,
    pub(crate) menu_report: &'a MenuTextPatchReport,
    pub(crate) interface_files: &'a BTreeMap<String, Vec<u8>>,
    pub(crate) interface_report: &'a InterfaceTextPatchReport,
    pub(crate) dialogue_files: &'a BTreeMap<String, Vec<u8>>,
    pub(crate) dialogue_report: &'a DialogueTextPatchReport,
    pub(crate) fixed_files: &'a BTreeMap<String, Vec<u8>>,
    pub(crate) monochrome_files: &'a BTreeMap<String, Vec<u8>>,
    pub(crate) baked_files: &'a BTreeMap<String, Vec<u8>>,
}

pub(super) fn write_localization_comparison_audit(
    inputs: LocalizationComparisonInputs<'_>,
    output_directory: &Path,
) -> Result<GraphicsAuditReport> {
    let font = Pc98Font::load(inputs.pc98_font_bmp)?;
    let source_mad = required(inputs.source, "MAD.COM")?;
    let source_gaiji_program = required(inputs.source, "GAIJI.COM")?;
    let source_catalog = catalog_game_data(
        source_mad,
        source_gaiji_program,
        required(inputs.source, "MENU.COM")?,
    )?;
    let source_overrides = source_gaiji_overrides(source_gaiji_program, &source_catalog.gaiji)?;

    let mut previews = Vec::new();
    previews.push(text_pair(
        TextPairInput {
            category: "menu",
            id: MENU_ID,
            source_lines: lines(
                &source_catalog
                    .menu
                    .text
                    .entries
                    .iter()
                    .find(|entry| entry.id == MENU_ID)
                    .context("representative source menu entry is absent")?
                    .text,
            ),
            replacement_lines: inputs
                .menu_report
                .entries
                .iter()
                .find(|entry| entry.id == MENU_ID)
                .context("representative replacement menu entry is absent")?
                .lines
                .clone(),
            source_overrides: &source_overrides,
            replacement_overrides: &replacement_gaiji_overrides(
                required(inputs.menu_files, "MENU.COM")?,
                &inputs.menu_report.glyphs,
            )?,
            replacement_font_profile: &inputs.menu_report.font_profile,
        },
        &font,
    )?);
    previews.push(text_pair(
        TextPairInput {
            category: "interface",
            id: INTERFACE_ID,
            source_lines: lines(
                &source_catalog
                    .mad_com
                    .interface_text
                    .entries
                    .iter()
                    .find(|entry| entry.id == INTERFACE_ID)
                    .context("representative source interface entry is absent")?
                    .text,
            ),
            replacement_lines: inputs
                .interface_report
                .entries
                .iter()
                .find(|entry| entry.id == INTERFACE_ID)
                .context("representative replacement interface entry is absent")?
                .lines
                .clone(),
            source_overrides: &source_overrides,
            replacement_overrides: &replacement_gaiji_overrides(
                required(inputs.interface_files, "GAIJI.COM")?,
                &inputs.interface_report.glyphs,
            )?,
            replacement_font_profile: &inputs.interface_report.font_profile,
        },
        &font,
    )?);
    let source_dialogue = source_catalog
        .mad_com
        .dialogue
        .groups
        .iter()
        .flat_map(|group| &group.entries)
        .find(|entry| entry.id == DIALOGUE_ID)
        .context("representative source dialogue entry is absent")?;
    previews.push(text_pair(
        TextPairInput {
            category: "dialogue",
            id: DIALOGUE_ID,
            source_lines: source_dialogue.lines.clone(),
            replacement_lines: inputs
                .dialogue_report
                .entries
                .iter()
                .find(|entry| entry.id == DIALOGUE_ID)
                .context("representative replacement dialogue entry is absent")?
                .lines
                .clone(),
            source_overrides: &source_overrides,
            replacement_overrides: &replacement_gaiji_overrides(
                required(inputs.dialogue_files, "GAIJI.COM")?,
                &inputs.dialogue_report.glyphs,
            )?,
            replacement_font_profile: &inputs.dialogue_report.font_profile,
        },
        &font,
    )?);

    let source_fixed_catalog = &source_catalog.mad_com.fixed_gaiji_text;
    let source_fixed_slot = source_fixed_catalog
        .slots
        .iter()
        .find(|slot| slot.id == FIXED_ID)
        .context("representative source fixed GAIJI slot is absent")?;
    let replacement_fixed_catalog = catalog_game_data(
        required(inputs.fixed_files, "MAD.COM")?,
        required(inputs.fixed_files, "GAIJI.COM")?,
        required(inputs.fixed_files, "MENU.COM")?,
    )?;
    let replacement_fixed_slot = replacement_fixed_catalog
        .mad_com
        .fixed_gaiji_text
        .slots
        .iter()
        .find(|slot| slot.id == FIXED_ID)
        .context("representative replacement fixed GAIJI slot is absent")?;
    previews.push(graphics_pair(
        "fixed-gaiji",
        FIXED_ID,
        render_fixed_gaiji_slot(
            source_fixed_slot,
            source_gaiji_program,
            &source_catalog.gaiji,
            &font,
        )?,
        render_fixed_gaiji_slot(
            replacement_fixed_slot,
            required(inputs.fixed_files, "GAIJI.COM")?,
            &replacement_fixed_catalog.gaiji,
            &font,
        )?,
        &font,
        "MAD.COM 6x7 fixed cells read back through source and replacement GAIJI.COM",
    )?);

    let source_monochrome =
        super::monochrome_pages::render_monochrome_page_previews(source_mad, inputs.source)?;
    let replacement_monochrome = super::monochrome_pages::render_monochrome_page_previews(
        required(inputs.monochrome_files, "MAD.COM")?,
        inputs.monochrome_files,
    )?;
    previews.push(preview_pair(
        "opening",
        "opening-page-01",
        take_preview(source_monochrome, OPENING_ID)?,
        take_preview(replacement_monochrome, OPENING_ID)?,
        &font,
        "OPM.DAT glyph-index page rendered through the source 32x32 consumer",
    )?);

    let baked_catalog = catalog_baked_text(source_mad, inputs.source)?;
    let source_selection = super::selection::render_selection_previews(inputs.source)?;
    let replacement_selection = super::selection::render_selection_previews(inputs.baked_files)?;
    let source_difficulty = take_preview_named(&source_selection, "selection-tile-map.png")?;
    let replacement_difficulty =
        take_preview_named(&replacement_selection, "selection-tile-map.png")?;
    let difficulty_region = unit_region(&baked_catalog, DIFFICULTY_ID)?;
    previews.push(graphics_pair(
        "difficulty",
        DIFFICULTY_ID,
        crop_image(source_difficulty, difficulty_region)?,
        crop_image(replacement_difficulty, difficulty_region)?,
        &font,
        "SEL1.DAT source-proven tile-map region rendered through the 16x16 baked consumer",
    )?);
    let source_names = take_preview_named(&source_selection, "sel3-sprite-overlays.png")?;
    let replacement_names = take_preview_named(&replacement_selection, "sel3-sprite-overlays.png")?;
    let selection_region = unit_region(&baked_catalog, SELECTION_ID)?;
    previews.push(graphics_pair(
        "stage-selection",
        SELECTION_ID,
        crop_image(source_names, selection_region)?,
        crop_image(replacement_names, selection_region)?,
        &font,
        "SEL3.DAT source-proven masked name region rendered through the 16x16 baked consumer",
    )?);

    let source_title = take_preview(
        super::title::render_title_source_frame_previews(source_mad, inputs.source)?,
        SOURCE_TITLE_FRAME,
    )?;
    let replacement_title = super::title::render_title_runtime_palette_preview(
        required(inputs.baked_files, "MAD.COM")?,
        inputs.baked_files,
    )?;
    previews.push(preview_pair(
        "title",
        "title-logo",
        source_title,
        replacement_title,
        &font,
        "source active TITLE/TITLE2 frame versus generated master after crop, palette quantization, packing, and readback",
    )?);

    let contact_sheet = compose_contact_sheet(
        previews
            .iter()
            .map(|preview| preview.image.clone())
            .collect(),
        1400,
        12,
    )?;
    previews.push(Preview {
        source_asset: "representative localization consumers".to_owned(),
        output_file: "localization-glyph-comparison-contact-sheet.png".to_owned(),
        evidence: format!(
            "left SOURCE JP, right PATCHED KR; PC-98 font BMP SHA-256 {SUPPORTED_PC98_FONT_BMP_SHA256}; development diagnostics only, not human approval"
        ),
        image: contact_sheet,
    });
    super::output::publish_previews(output_directory, previews)
}

struct TextPairInput<'a> {
    category: &'a str,
    id: &'a str,
    source_lines: Vec<String>,
    replacement_lines: Vec<String>,
    source_overrides: &'a super::text_comparison::GlyphOverrides,
    replacement_overrides: &'a super::text_comparison::GlyphOverrides,
    replacement_font_profile: &'a str,
}

fn text_pair(input: TextPairInput<'_>, font: &Pc98Font) -> Result<Preview> {
    graphics_pair(
        input.category,
        input.id,
        render_text_lines(&input.source_lines, font, input.source_overrides)?,
        render_text_lines(&input.replacement_lines, font, input.replacement_overrides)?,
        font,
        &format!(
            "source text uses source GAIJI plus the verified PC-98 BMP; current text uses read-back GAIJI records from {}",
            input.replacement_font_profile
        ),
    )
}

fn preview_pair(
    category: &str,
    id: &str,
    source: Preview,
    replacement: Preview,
    font: &Pc98Font,
    evidence: &str,
) -> Result<Preview> {
    graphics_pair(
        category,
        id,
        source.image,
        replacement.image,
        font,
        evidence,
    )
}

fn graphics_pair(
    category: &str,
    id: &str,
    source: RgbImage,
    replacement: RgbImage,
    font: &Pc98Font,
    evidence: &str,
) -> Result<Preview> {
    Ok(Preview {
        source_asset: format!("{category}:{id}"),
        output_file: format!("{category}-{id}-original-vs-current.png"),
        evidence: format!("{category} heading; left SOURCE JP, right PATCHED KR; {evidence}"),
        image: compose_pair(category, source, replacement, font)?,
    })
}

fn compose_pair(
    category: &str,
    source: RgbImage,
    replacement: RgbImage,
    font: &Pc98Font,
) -> Result<RgbImage> {
    const HEADER_HEIGHT: usize = 20;
    const PADDING: usize = 4;
    const GAP: usize = 12;
    let no_overrides = BTreeMap::new();
    let category_label = render_text_lines(&[pair_heading(category)], font, &no_overrides)?;
    let original_label = render_text_lines(&["SOURCE JP".to_owned()], font, &no_overrides)?;
    let current_label = render_text_lines(&["PATCHED KR".to_owned()], font, &no_overrides)?;
    let mut panel_width = source
        .width
        .max(replacement.width)
        .max(original_label.width)
        .max(current_label.width)
        + PADDING * 2;
    panel_width = panel_width.max(
        category_label
            .width
            .saturating_add(PADDING * 2)
            .saturating_sub(GAP)
            .div_ceil(2),
    );
    let panel_height = source.height.max(replacement.height) + PADDING * 2;
    let width = panel_width * 2 + GAP;
    let height = HEADER_HEIGHT * 2 + panel_height;
    let mut image = RgbImage {
        width,
        height,
        pixels: [24, 24, 24].repeat(width * height),
    };
    fill_rect(&mut image, 0, 0, width, HEADER_HEIGHT, [64, 64, 64]);
    copy_image(&mut image, &category_label, PADDING, 2)?;
    fill_rect(
        &mut image,
        0,
        HEADER_HEIGHT,
        panel_width,
        HEADER_HEIGHT,
        [48, 48, 48],
    );
    fill_rect(
        &mut image,
        panel_width + GAP,
        HEADER_HEIGHT,
        panel_width,
        HEADER_HEIGHT,
        [48, 48, 48],
    );
    copy_image(&mut image, &original_label, PADDING, HEADER_HEIGHT + 2)?;
    copy_image(
        &mut image,
        &current_label,
        panel_width + GAP + PADDING,
        HEADER_HEIGHT + 2,
    )?;
    copy_image(&mut image, &source, PADDING, HEADER_HEIGHT * 2 + PADDING)?;
    copy_image(
        &mut image,
        &replacement,
        panel_width + GAP + PADDING,
        HEADER_HEIGHT * 2 + PADDING,
    )?;
    Ok(image)
}

fn pair_heading(category: &str) -> String {
    category.to_ascii_uppercase()
}

fn crop_image(image: &RgbImage, region: (usize, usize, usize, usize)) -> Result<RgbImage> {
    let (x, y, width, height) = region;
    ensure!(
        width > 0
            && height > 0
            && x.checked_add(width).is_some_and(|end| end <= image.width)
            && y.checked_add(height).is_some_and(|end| end <= image.height),
        "comparison crop exceeds its rendered consumer frame"
    );
    let mut pixels = Vec::with_capacity(width * height * 3);
    for row in y..y + height {
        let start = (row * image.width + x) * 3;
        pixels.extend_from_slice(&image.pixels[start..start + width * 3]);
    }
    Ok(RgbImage {
        width,
        height,
        pixels,
    })
}

fn unit_region(
    catalog: &crate::BakedTextCatalog,
    id: &str,
) -> Result<(usize, usize, usize, usize)> {
    let unit = catalog
        .units
        .iter()
        .find(|unit| unit.id == id)
        .with_context(|| format!("comparison unit {id} is absent"))?;
    ensure!(
        unit.screen_regions.len() == 1,
        "comparison unit {id} does not have exactly one screen region"
    );
    let region = &unit.screen_regions[0];
    Ok((region.x, region.y, region.width, region.height))
}

fn take_preview(mut previews: Vec<Preview>, output_file: &str) -> Result<Preview> {
    let index = previews
        .iter()
        .position(|preview| preview.output_file == output_file)
        .with_context(|| format!("comparison preview {output_file} is absent"))?;
    Ok(previews.swap_remove(index))
}

fn take_preview_named<'a>(previews: &'a [Preview], output_file: &str) -> Result<&'a RgbImage> {
    Ok(&previews
        .iter()
        .find(|preview| preview.output_file == output_file)
        .with_context(|| format!("comparison preview {output_file} is absent"))?
        .image)
}

fn lines(text: &str) -> Vec<String> {
    let mut lines = text
        .split('\n')
        .map(|line| {
            line.chars()
                .filter(|character| !character.is_control())
                .collect()
        })
        .collect::<Vec<String>>();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

fn required<'a>(files: &'a BTreeMap<String, Vec<u8>>, name: &str) -> Result<&'a [u8]> {
    files
        .get(name)
        .map(Vec::as_slice)
        .with_context(|| format!("comparison payload is missing {name}"))
}

fn copy_image(destination: &mut RgbImage, source: &RgbImage, x: usize, y: usize) -> Result<()> {
    ensure!(
        x + source.width <= destination.width && y + source.height <= destination.height,
        "comparison image placement exceeds its canvas"
    );
    for row in 0..source.height {
        let source_start = row * source.width * 3;
        let destination_start = ((y + row) * destination.width + x) * 3;
        destination.pixels[destination_start..destination_start + source.width * 3]
            .copy_from_slice(&source.pixels[source_start..source_start + source.width * 3]);
    }
    Ok(())
}

fn fill_rect(
    image: &mut RgbImage,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: [u8; 3],
) {
    for row in y..y + height {
        for column in x..x + width {
            let pixel = (row * image.width + column) * 3;
            image.pixels[pixel..pixel + 3].copy_from_slice(&color);
        }
    }
}

#[cfg(test)]
#[path = "comparison_tests.rs"]
mod comparison_tests;
