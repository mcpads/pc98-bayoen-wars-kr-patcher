use std::collections::BTreeMap;

use expected_write::{ExpectedWrite, ImageRegion, RegionKind, ResizePlan, WriteIntent, WritePlan};

use super::super::payload_writes::PayloadFileWritePlan;
use super::model::PreparedArleAssetFile;

pub(super) fn character_asset_plan(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    replacement: &PreparedArleAssetFile,
) -> PayloadFileWritePlan {
    let source = &installer_payload[replacement.file_name];
    let owner = "arle-character-asset-producer";
    let mut plan = WritePlan::new();
    if source.len() != replacement.packed.len() {
        plan = plan.resize(ResizePlan {
            owner: owner.to_owned(),
            purpose: format!(
                "store the fixed-size decoded {} character asset after recompression",
                replacement.file_name
            ),
            expected_input_len: source.len(),
            output_len: replacement.packed.len(),
        });
    }
    plan = plan
        .region(ImageRegion {
            id: format!(
                "arle-character-{}-packed",
                replacement.file_name.to_ascii_lowercase()
            ),
            range: 0..replacement.packed.len(),
            kind: RegionKind::Data,
            reason: "one consumer-linked Compile LZ character stream".to_owned(),
        })
        .write(ExpectedWrite {
            id: format!(
                "replace-arle-character-{}",
                replacement.file_name.to_ascii_lowercase()
            ),
            owner: owner.to_owned(),
            purpose: "replace every consumer-bound Arle frame in this asset as one producer unit"
                .to_owned(),
            offset: 0,
            expected_original: source[..source.len().min(replacement.packed.len())].to_vec(),
            replacement: replacement.packed.clone(),
            intent: WriteIntent::Data,
        });
    PayloadFileWritePlan {
        file_name: replacement.file_name,
        plan,
    }
}
