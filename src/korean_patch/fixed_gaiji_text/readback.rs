use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use super::model::CompiledFixedGaijiText;
use crate::game_data::{GaijiCatalog, catalog_game_data, parse_gaiji_program};
use crate::korean_patch::gaiji_record::{GAIJI_RECORD_SIZE, GaijiRecord};
use crate::korean_patch::shared_text::verify_gaiji_bank_readback;

pub(super) fn verify_fixed_gaiji_text_payload(
    files: &BTreeMap<String, Vec<u8>>,
    source_gaiji: &GaijiCatalog,
    compiled: &CompiledFixedGaijiText,
) -> Result<()> {
    let gaiji_com = files
        .get("GAIJI.COM")
        .context("fixed-text payload is missing GAIJI.COM")?;
    verify_gaiji_bank_readback(gaiji_com, source_gaiji, &compiled.bank)?;
    let gaiji = parse_gaiji_program(gaiji_com)?;
    for glyph in &compiled.bank.glyphs {
        let slot = &gaiji.glyphs[glyph.slot_index];
        let expected =
            &compiled.bank.patched_program[slot.file_offset..slot.file_offset + slot.byte_size];
        ensure!(
            &gaiji_com[slot.file_offset..slot.file_offset + slot.byte_size] == expected,
            "fixed-text GAIJI slot {} failed readback",
            glyph.slot_index
        );
    }

    let mad_com = files
        .get("MAD.COM")
        .context("fixed-text payload is missing MAD.COM")?;
    let first = source_gaiji
        .glyphs
        .first()
        .context("fixed-text source GAIJI table is empty")?
        .file_offset;
    verify_fixed_gaiji_text_component(mad_com, &gaiji_com[first..], source_gaiji, compiled)?;
    let menu_com = files
        .get("MENU.COM")
        .context("fixed-text payload is missing MENU.COM")?;
    let catalog = catalog_game_data(mad_com, gaiji_com, menu_com)?;
    ensure!(
        catalog.mad_com.fixed_gaiji_text.slots.len() == compiled.slots.len()
            && catalog
                .mad_com
                .fixed_gaiji_text
                .slots
                .iter()
                .zip(&compiled.slots)
                .all(|(read, expected)| {
                    read.id == expected.id
                        && read.file_offset == expected.file_offset
                        && read.byte_size == expected.bytes.len()
                }),
        "fixed-text consumer catalog changed after readback"
    );
    Ok(())
}

pub(in crate::korean_patch) fn verify_fixed_gaiji_text_component(
    mad_com: &[u8],
    bank_records: &[u8],
    source_gaiji: &GaijiCatalog,
    compiled: &CompiledFixedGaijiText,
) -> Result<()> {
    let first = source_gaiji
        .glyphs
        .first()
        .context("fixed-text source GAIJI table is empty")?
        .file_offset;
    let end = source_gaiji
        .glyphs
        .last()
        .context("fixed-text source GAIJI table is empty")?
        .file_offset
        + GAIJI_RECORD_SIZE;
    let expected = compiled
        .bank
        .patched_program
        .get(first..end)
        .context("compiled fixed-text GAIJI records lie outside the bank")?;
    ensure!(
        bank_records.get(..expected.len()) == Some(expected),
        "fixed-text appended GAIJI bank failed readback"
    );
    ensure!(
        bank_records.len().is_multiple_of(GAIJI_RECORD_SIZE)
            && bank_records
                .as_chunks::<GAIJI_RECORD_SIZE>()
                .0
                .iter()
                .all(|record| GaijiRecord::parse(record).is_ok()),
        "fixed-text appended GAIJI bank contains an invalid record"
    );
    for slot in &compiled.slots {
        ensure!(
            mad_com.get(slot.file_offset..slot.file_offset + slot.bytes.len())
                == Some(slot.bytes.as_slice()),
            "{} fixed-text slot failed byte readback",
            slot.id
        );
    }
    Ok(())
}
