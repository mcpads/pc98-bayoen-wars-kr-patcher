use std::collections::BTreeMap;

use super::model::{AssetPatch, BakedGraphicsAssetPatchReport};
use crate::source_disk::sha256_hex;

pub(super) fn asset_report(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    patch: &AssetPatch,
) -> BakedGraphicsAssetPatchReport {
    BakedGraphicsAssetPatchReport {
        source_asset: patch.file_name.to_owned(),
        translation_unit_ids: patch.unit_ids.clone(),
        decoded_sha256: sha256_hex(&patch.decoded),
        original_packed_size: installer_payload[patch.file_name].len(),
        replacement_packed_size: patch.packed.len(),
        replacement_packed_sha256: sha256_hex(&patch.packed),
    }
}
