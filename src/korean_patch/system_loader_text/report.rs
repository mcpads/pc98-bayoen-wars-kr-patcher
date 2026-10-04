use super::model::{
    CompiledSystemLoaderText, CompiledSystemLoaderTextEntry, SystemLoaderTextEntryPatchReport,
};
use crate::korean_patch::gaiji_record::GAIJI_RECORD_SIZE;
use crate::korean_patch::shared_text::SharedGaijiGlyph;
use crate::source_disk::sha256_hex;

pub(super) fn entry_report(
    entry: &CompiledSystemLoaderTextEntry,
    compiled: &CompiledSystemLoaderText,
) -> SystemLoaderTextEntryPatchReport {
    SystemLoaderTextEntryPatchReport {
        id: entry.id.clone(),
        original_file_offset: entry.original_file_offset,
        resident_offset: entry.resident_offset,
        byte_size: entry.bytes.len(),
        reference_count: compiled
            .runtime
            .references
            .iter()
            .filter(|reference| reference.entry_id == entry.id)
            .count(),
        content_sha256: sha256_hex(&entry.bytes),
        lines: entry.lines.clone(),
    }
}

pub(super) fn glyph_reports(compiled: &CompiledSystemLoaderText) -> Vec<SharedGaijiGlyph> {
    compiled
        .bank
        .glyphs
        .iter()
        .enumerate()
        .map(|(index, glyph)| SharedGaijiGlyph {
            character: glyph.character.clone(),
            slot_index: glyph.slot_index,
            shift_jis_code: glyph.shift_jis_code.clone(),
            record_offset: compiled.glyph_records_offset + index * GAIJI_RECORD_SIZE,
        })
        .collect()
}
