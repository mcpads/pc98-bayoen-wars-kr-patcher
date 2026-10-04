use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use super::MAD_FILE;
use super::atlas::GLYPH_BYTE_SIZE;
use super::model::SequencePatch;
use crate::asset_bindings::{
    MonochromeTextSequence, catalog_monochrome_sprites, catalog_monochrome_text,
};
use crate::localization_assets::decode_all_streams;

pub(super) fn verify_patched_payload(
    files: &BTreeMap<String, Vec<u8>>,
    opening: &SequencePatch,
    ending: &SequencePatch,
) -> Result<()> {
    let mad_com = files
        .get(MAD_FILE)
        .context("patched payload is missing MAD.COM")?;
    let sprites = catalog_monochrome_sprites(mad_com, files)?;
    let catalog = catalog_monochrome_text(mad_com, files, &sprites)?;
    verify_sequence(files, opening, &catalog.opening)?;
    verify_sequence(files, ending, &catalog.ending)?;
    Ok(())
}

fn verify_sequence(
    files: &BTreeMap<String, Vec<u8>>,
    patch: &SequencePatch,
    catalog: &MonochromeTextSequence,
) -> Result<()> {
    let streams = decode_all_streams(&files[patch.file_name])?;
    ensure!(
        streams.len() == 1 && streams[0].output == patch.atlas.decoded,
        "{} packed atlas failed decoded readback",
        patch.file_name
    );
    ensure!(
        catalog.glyphs.len() * GLYPH_BYTE_SIZE == patch.atlas.decoded.len(),
        "{} glyph capacity changed after readback",
        catalog.id
    );
    ensure!(
        catalog.pages.len() == patch.pages.lines.len(),
        "{} page count changed after readback",
        catalog.id
    );
    for ((page, expected_lines), expected_size) in catalog
        .pages
        .iter()
        .zip(&patch.pages.lines)
        .zip(&patch.pages.page_byte_sizes)
    {
        ensure!(
            page.lines == *expected_lines && page.byte_size == *expected_size,
            "{} page stream differs after readback",
            page.id
        );
    }
    Ok(())
}
