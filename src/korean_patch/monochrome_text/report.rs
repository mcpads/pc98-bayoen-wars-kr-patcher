use std::collections::BTreeMap;

use super::atlas::GLYPH_BYTE_SIZE;
use super::model::{MonochromeSequencePatchReport, SequencePatch};
use crate::source_disk::sha256_hex;

pub(super) fn sequence_report(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    patch: &SequencePatch,
) -> MonochromeSequencePatchReport {
    let original = &installer_payload[patch.file_name];
    MonochromeSequencePatchReport {
        id: patch.id.clone(),
        source_asset: patch.file_name.to_owned(),
        glyph_capacity: patch.atlas.decoded.len() / GLYPH_BYTE_SIZE,
        used_glyph_count: patch.atlas.glyph_slots.len(),
        unused_glyph_count: patch.atlas.decoded.len() / GLYPH_BYTE_SIZE
            - patch.atlas.glyph_slots.len(),
        glyph_slots: patch.atlas.glyph_slots.clone(),
        decoded_atlas_sha256: sha256_hex(&patch.atlas.decoded),
        original_packed_size: original.len(),
        replacement_packed_size: patch.packed_atlas.len(),
        replacement_packed_sha256: sha256_hex(&patch.packed_atlas),
        page_count: patch.pages.page_byte_sizes.len(),
        page_data_size: patch.pages.page_data.len(),
        page_storage_capacity: patch.pages.storage_capacity,
        page_byte_sizes: patch.pages.page_byte_sizes.clone(),
    }
}
