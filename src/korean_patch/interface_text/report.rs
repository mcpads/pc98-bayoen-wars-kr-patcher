use crate::game_data::{InterfaceTextReference, InterfaceTextReferenceKind};
use crate::korean_patch::interface_text::model::{
    CompiledInterfaceTextEntry, InterfaceTextEntryPatchReport,
};
use crate::source_disk::sha256_hex;

pub(in crate::korean_patch) fn entry_report(
    entry: &CompiledInterfaceTextEntry,
    references: &[InterfaceTextReference],
) -> InterfaceTextEntryPatchReport {
    let entry_references = references
        .iter()
        .filter(|reference| reference.target_entry_id == entry.id)
        .collect::<Vec<_>>();
    let machine_code_reference_count = entry_references
        .iter()
        .filter(|reference| {
            reference.storage_kind == InterfaceTextReferenceKind::MachineCodeImmediate
        })
        .count();
    InterfaceTextEntryPatchReport {
        id: entry.id.clone(),
        original_file_offset: entry.original_file_offset,
        file_offset: entry.file_offset,
        com_address: entry.com_address,
        byte_size: entry.bytes.len(),
        line_count: entry.lines.len(),
        reference_count: entry_references.len(),
        machine_code_reference_count,
        metadata_reference_count: entry_references.len() - machine_code_reference_count,
        content_sha256: sha256_hex(&entry.bytes),
        lines: entry.lines.clone(),
    }
}
