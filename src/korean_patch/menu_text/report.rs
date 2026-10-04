use super::model::{CompiledMenuTextEntry, MenuTextEntryPatchReport};
use crate::game_data::MenuTextCatalog;
use crate::source_disk::sha256_hex;

pub(super) fn entry_report(
    entry: &CompiledMenuTextEntry,
    source: &MenuTextCatalog,
) -> MenuTextEntryPatchReport {
    let semantic_reference_count = source
        .entries
        .iter()
        .find(|source| source.id == entry.id)
        .expect("compiled MENU entry came from the source catalog")
        .references
        .len();
    MenuTextEntryPatchReport {
        id: entry.id.clone(),
        original_file_offset: entry.original_file_offset,
        file_offset: entry.file_offset,
        com_address: entry.com_address,
        byte_size: entry.bytes.len(),
        line_count: entry.lines.len(),
        semantic_reference_count,
        runtime_field_count: entry.runtime_field_offsets.len(),
        content_sha256: sha256_hex(&entry.bytes),
        lines: entry.lines.clone(),
    }
}
