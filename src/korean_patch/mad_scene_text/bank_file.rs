use anyhow::{Context, Result, ensure};

use super::super::gaiji_record::GAIJI_RECORD_SIZE;
use super::super::shared_text::CompiledGaijiBank;
use crate::game_data::GaijiCatalog;

pub(super) struct CompiledGaijiBankFile {
    pub file_offset: usize,
    pub records: Vec<u8>,
    pub record_count: usize,
    pub output_file_size: usize,
}

pub(super) fn compile_gaiji_bank_file(
    source_file: &[u8],
    source: &GaijiCatalog,
    bank: &CompiledGaijiBank,
    file_offset: usize,
    minimum_record_count: usize,
) -> Result<CompiledGaijiBankFile> {
    ensure!(
        source.glyphs.len() == source.installer.glyph_count,
        "GAIJI source record population changed"
    );
    ensure!(
        source
            .glyphs
            .windows(2)
            .all(|pair| { pair[0].file_offset + GAIJI_RECORD_SIZE == pair[1].file_offset }),
        "GAIJI source records are not contiguous"
    );
    let first = source
        .glyphs
        .first()
        .context("GAIJI source record table is empty")?
        .file_offset;
    let end = source
        .glyphs
        .last()
        .context("GAIJI source record table is empty")?
        .file_offset
        + GAIJI_RECORD_SIZE;
    ensure!(
        end == source_file.len() && bank.patched_program.len() == source_file.len(),
        "GAIJI source record table no longer owns the original file tail"
    );
    let mut records = bank
        .patched_program
        .get(first..end)
        .context("compiled appended GAIJI records lie outside the bank")?
        .to_vec();
    records.extend_from_slice(&bank.appended_records);
    let natural_record_count = records.len() / GAIJI_RECORD_SIZE;
    ensure!(
        records.len() % GAIJI_RECORD_SIZE == 0
            && natural_record_count >= source.glyphs.len()
            && bank
                .appended_records
                .len()
                .is_multiple_of(GAIJI_RECORD_SIZE),
        "compiled appended GAIJI byte population changed"
    );
    ensure!(
        natural_record_count <= minimum_record_count,
        "compiled GAIJI bank has {natural_record_count} records but its shared installer count is {minimum_record_count}"
    );
    let blank_record = crate::korean_patch::gaiji_record::GaijiRecord::from_bitmap(
        [0; crate::korean_patch::font_rasterizer::GLYPH_BYTES],
    )
    .to_bytes();
    while records.len() / GAIJI_RECORD_SIZE < minimum_record_count {
        records.extend_from_slice(&blank_record);
    }
    ensure!(
        file_offset >= source_file.len(),
        "appended GAIJI bank begins inside the immutable source file"
    );
    Ok(CompiledGaijiBankFile {
        file_offset,
        output_file_size: file_offset + records.len(),
        records,
        record_count: minimum_record_count,
    })
}
