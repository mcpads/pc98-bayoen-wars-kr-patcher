use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use super::model::SourceSegmentReference;
use crate::SourceStructureReport;
use crate::source_disk::sha256_hex;

#[derive(Serialize)]
struct SourceSegmentDocument<'a, T> {
    id: &'a str,
    source_file: &'a str,
    surface: &'a str,
    entries: &'a [T],
}

pub(super) fn write_source_segments(
    source: &SourceStructureReport,
    staged: &Path,
) -> Result<Vec<SourceSegmentReference>> {
    let directory = staged.join("segments");
    fs::create_dir(&directory).context("failed to create translation segment directory")?;
    let mut segments = Vec::new();
    let mad = &source.game_data.mad_com;

    segments.push(write_segment(
        &directory,
        "mad-system",
        "MAD.COM",
        "dos_system_text",
        &mad.system_text.entries,
    )?);
    segments.push(write_segment(
        &directory,
        "mad-interface",
        "MAD.COM",
        "interface_text",
        &mad.interface_text.entries,
    )?);
    segments.push(write_segment(
        &directory,
        "mad-unit-list-status",
        "MAD.COM",
        "interface_text",
        &mad.unit_list_status.entries,
    )?);
    segments.push(write_segment(
        &directory,
        "mad-battle-callouts",
        "MAD.COM",
        "battle_callout",
        &mad.battle_callouts.entries,
    )?);
    segments.push(write_segment(
        &directory,
        "mad-fixed-gaiji",
        "MAD.COM",
        "fixed_gaiji_text",
        &mad.fixed_gaiji_text.slots,
    )?);
    for group in &mad.dialogue.groups {
        segments.push(write_segment(
            &directory,
            &group.id,
            "MAD.COM",
            "dialogue",
            &group.entries,
        )?);
    }
    let monochrome = &source.asset_bindings.monochrome_text;
    segments.push(write_segment(
        &directory,
        "opening-glyph-atlas",
        "OPM.DAT",
        "monochrome_glyph_atlas",
        &monochrome.opening.glyphs,
    )?);
    segments.push(write_segment(
        &directory,
        "opening-pages",
        "MAD.COM",
        "opening_glyph_index_pages",
        &monochrome.opening.pages,
    )?);
    segments.push(write_segment(
        &directory,
        "ending-glyph-atlas",
        "EDM.DAT",
        "monochrome_glyph_atlas",
        &monochrome.ending.glyphs,
    )?);
    segments.push(write_segment(
        &directory,
        "ending-pages",
        "MAD.COM",
        "ending_glyph_index_pages",
        &monochrome.ending.pages,
    )?);
    segments.push(write_segment(
        &directory,
        "menu",
        "MENU.COM",
        "menu_text",
        &source.game_data.menu.text.entries,
    )?);
    for program in &source.external_text.programs {
        let stem = program
            .file_name
            .split_once('.')
            .map_or(program.file_name.as_str(), |(stem, _)| stem)
            .to_ascii_lowercase();
        segments.push(write_segment(
            &directory,
            &format!("external-{stem}"),
            &program.file_name,
            "external_program_text",
            &program.entries,
        )?);
    }
    segments.push(write_segment(
        &directory,
        "voice-samples",
        "SAMPA",
        "japanese_voice_performance",
        &source.asset_bindings.voice_samples.streams,
    )?);
    for (id, source_file) in [
        ("title-baked-text", "TITLE.DAT"),
        ("difficulty-baked-text", "SEL1.DAT"),
        ("selection-baked-text", "SEL3.DAT"),
    ] {
        let entries = source
            .asset_bindings
            .baked_text
            .units
            .iter()
            .filter(|unit| unit.source_asset == source_file)
            .collect::<Vec<_>>();
        segments.push(write_segment(
            &directory,
            id,
            source_file,
            "baked_graphics_text",
            &entries,
        )?);
    }
    Ok(segments)
}

fn write_segment<T: Serialize>(
    directory: &Path,
    id: &str,
    source_file: &str,
    surface: &str,
    entries: &[T],
) -> Result<SourceSegmentReference> {
    let document = SourceSegmentDocument {
        id,
        source_file,
        surface,
        entries,
    };
    let mut bytes = serde_json::to_vec_pretty(&document)?;
    bytes.push(b'\n');
    let file_name = format!("{id}.json");
    let path = directory.join(&file_name);
    fs::write(&path, &bytes).with_context(|| {
        format!(
            "failed to write translation source segment {}",
            path.display()
        )
    })?;
    Ok(SourceSegmentReference {
        id: id.to_owned(),
        source_file: source_file.to_owned(),
        surface: surface.to_owned(),
        path: format!("segments/{file_name}"),
        entry_count: entries.len(),
        content_sha256: sha256_hex(&bytes),
    })
}

#[cfg(test)]
#[path = "segments_tests.rs"]
mod segments_tests;
