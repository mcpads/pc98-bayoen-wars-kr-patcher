use anyhow::{Context, Result};

use super::CompiledGaijiBank;
use crate::game_data::GaijiCatalog;
use crate::korean_patch::gaiji_record::GAIJI_RECORD_SIZE;

pub(crate) struct EmbeddedGaijiRecords {
    pub bytes: Vec<u8>,
    pub character_codes: Vec<u16>,
}

impl EmbeddedGaijiRecords {
    pub fn segment_addresses(
        &self,
        records_file_offset: usize,
        address_bias: usize,
    ) -> Result<Vec<u16>> {
        (0..self.character_codes.len())
            .map(|index| {
                u16::try_from(records_file_offset + index * GAIJI_RECORD_SIZE + address_bias)
                    .context("embedded GAIJI record segment address exceeds 16 bits")
            })
            .collect()
    }
}

pub(crate) fn collect_embedded_gaiji_records(
    source_gaiji: &GaijiCatalog,
    bank: &CompiledGaijiBank,
) -> Result<EmbeddedGaijiRecords> {
    let mut bytes = Vec::with_capacity(bank.glyphs.len() * GAIJI_RECORD_SIZE);
    let mut character_codes = Vec::with_capacity(bank.glyphs.len());
    for glyph in &bank.glyphs {
        let source_slot = source_gaiji
            .glyphs
            .get(glyph.slot_index)
            .context("embedded GAIJI slot lies outside the source catalog")?;
        let range = source_slot.file_offset..source_slot.file_offset + GAIJI_RECORD_SIZE;
        bytes.extend_from_slice(
            bank.patched_program
                .get(range)
                .context("embedded GAIJI record lies outside the compiled bank")?,
        );
        character_codes.push(source_slot.character_code);
    }
    Ok(EmbeddedGaijiRecords {
        bytes,
        character_codes,
    })
}
