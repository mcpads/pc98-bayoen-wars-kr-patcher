use super::model::{CompiledMouseDriverTextEntry, MouseDriverTextEntryPatchReport};
use crate::source_disk::sha256_hex;

pub(super) fn entry_report(
    entry: &CompiledMouseDriverTextEntry,
) -> MouseDriverTextEntryPatchReport {
    MouseDriverTextEntryPatchReport {
        id: entry.id.clone(),
        original_file_offset: entry.original_file_offset,
        file_offset: entry.file_offset,
        com_address: entry.com_address,
        byte_size: entry.bytes.len(),
        line_count: entry.lines.len(),
        reference_kind: if entry.table_index.is_some() {
            "metadata".to_owned()
        } else {
            "machine_code".to_owned()
        },
        content_sha256: sha256_hex(&entry.bytes),
        lines: entry.lines.clone(),
    }
}
