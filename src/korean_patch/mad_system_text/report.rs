use super::model::{CompiledMadSystemTextEntry, MadSystemTextEntryPatchReport};
use crate::game_data::{MadSystemRuntimeReference, MadSystemRuntimeReferenceKind};
use crate::source_disk::sha256_hex;

pub(in crate::korean_patch) fn entry_report(
    entry: &CompiledMadSystemTextEntry,
    references: &[MadSystemRuntimeReference],
) -> MadSystemTextEntryPatchReport {
    let entry_references = references
        .iter()
        .filter(|reference| reference.target_entry_id == entry.id)
        .collect::<Vec<_>>();
    let machine_code_reference_count = entry_references
        .iter()
        .filter(|reference| reference.kind != MadSystemRuntimeReferenceKind::MetadataTableEntry)
        .count();
    MadSystemTextEntryPatchReport {
        id: entry.id.clone(),
        original_file_offset: entry.original_file_offset,
        file_offset: entry.file_offset,
        com_address: entry.com_address,
        byte_size: entry.bytes.len(),
        reference_count: entry_references.len(),
        machine_code_reference_count,
        metadata_reference_count: entry_references.len() - machine_code_reference_count,
        runtime_insert_byte_offset: entry.runtime_insert_byte_offset,
        runtime_insert_byte_capacity: entry.runtime_insert_byte_capacity,
        content_sha256: sha256_hex(&entry.bytes),
        lines: entry.lines.clone(),
    }
}
