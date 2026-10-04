use super::model::{
    CompiledPlaybackDriver, CompiledPlaybackDriverTextEntry, PlaybackDriverPatchReport,
    PlaybackDriverTextEntryPatchReport,
};
use crate::external_text::PackedSoundDriverReferenceKind;
use crate::korean_patch::gaiji_record::GAIJI_RECORD_SIZE;
use crate::korean_patch::shared_text::SharedGaijiGlyph;
use crate::source_disk::sha256_hex;

pub(super) fn driver_report(compiled: &CompiledPlaybackDriver) -> PlaybackDriverPatchReport {
    let glyphs = compiled
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
        .collect();
    PlaybackDriverPatchReport {
        file_name: compiled.file_name.to_owned(),
        source_packed_size: compiled.source_packed_size,
        output_packed_size: compiled.output_packed.len(),
        source_unpacked_size: compiled.source_unpacked.len(),
        output_unpacked_size: compiled.output_unpacked.len(),
        resident_end_com_address: compiled.runtime.resident_end_com_address,
        first_nonresident_file_offset: compiled.runtime.resident_end_file_offset,
        available_gaiji_slots: compiled.bank.available_slot_count,
        used_gaiji_slots: compiled.bank.glyphs.len(),
        installer_code_bytes: compiled.installer.bytes().len(),
        glyph_record_bytes: compiled.glyph_records.len(),
        text_storage_capacity: compiled.text_storage_capacity,
        text_storage_used: compiled.text_storage_used,
        reference_count: compiled.runtime.references.len(),
        machine_reference_count: compiled
            .runtime
            .references
            .iter()
            .filter(|reference| reference.kind == PackedSoundDriverReferenceKind::MachineCode)
            .count(),
        metadata_reference_count: compiled
            .runtime
            .references
            .iter()
            .filter(|reference| reference.kind == PackedSoundDriverReferenceKind::Metadata)
            .count(),
        glyphs,
        entries: compiled
            .entries
            .iter()
            .map(|entry| entry_report(entry, compiled))
            .collect(),
        unpacked_writes: compiled.unpacked_writes.clone(),
    }
}

fn entry_report(
    entry: &CompiledPlaybackDriverTextEntry,
    compiled: &CompiledPlaybackDriver,
) -> PlaybackDriverTextEntryPatchReport {
    PlaybackDriverTextEntryPatchReport {
        id: entry.id.clone(),
        original_file_offset: entry.original_file_offset,
        file_offset: entry.file_offset,
        com_address: entry.com_address,
        byte_size: entry.bytes.len(),
        line_count: entry.lines.len(),
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
