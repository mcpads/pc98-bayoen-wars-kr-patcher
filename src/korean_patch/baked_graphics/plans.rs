use std::collections::BTreeMap;

use expected_write::{ExpectedWrite, ImageRegion, RegionKind, ResizePlan, WriteIntent, WritePlan};

use super::model::AssetPatch;
use crate::korean_patch::payload_writes::PayloadFileWritePlan;

pub(super) fn baked_asset_plan(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    patch: &AssetPatch,
) -> PayloadFileWritePlan {
    let baseline = &installer_payload[patch.file_name];
    let mut plan = WritePlan::new();
    if baseline.len() != patch.packed.len() {
        plan = plan.resize(ResizePlan {
            owner: patch.producer_id.to_owned(),
            purpose: "store one fixed-size decoded baked-text asset after recompression".to_owned(),
            expected_input_len: baseline.len(),
            output_len: patch.packed.len(),
        });
    }
    plan = plan
        .region(ImageRegion {
            id: format!("{}-packed-output", patch.producer_id),
            range: 0..patch.packed.len(),
            kind: RegionKind::Data,
            reason: "one consumer-linked Compile LZ baked-graphics stream".to_owned(),
        })
        .write(ExpectedWrite {
            id: format!("{}-packed-replacement", patch.producer_id),
            owner: patch.producer_id.to_owned(),
            purpose: "replace reviewed development text inside the decoded graphics layout"
                .to_owned(),
            offset: 0,
            expected_original: baseline[..baseline.len().min(patch.packed.len())].to_vec(),
            replacement: patch.packed.clone(),
            intent: WriteIntent::Data,
        });
    PayloadFileWritePlan {
        file_name: patch.file_name,
        plan,
    }
}
