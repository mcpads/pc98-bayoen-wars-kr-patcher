use super::model::{CompiledDialogueTextEntry, DialogueTextEntryPatchReport};
use crate::source_disk::sha256_hex;

pub(in crate::korean_patch) fn entry_report(
    entry: &CompiledDialogueTextEntry,
) -> DialogueTextEntryPatchReport {
    DialogueTextEntryPatchReport {
        id: entry.id.clone(),
        group_id: entry.group_id.clone(),
        presentation_variant: entry.presentation_variant,
        text_pointer_offset: entry.text_pointer_offset,
        original_file_offset: entry.original_file_offset,
        file_offset: entry.file_offset,
        com_address: entry.com_address,
        byte_size: entry.bytes.len(),
        line_count: entry.lines.len(),
        content_sha256: sha256_hex(&entry.bytes),
        lines: entry.lines.clone(),
    }
}
