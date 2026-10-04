use expected_write::{ExpectedWrite, ImageRegion, RegionKind, WriteIntent, WritePlan};

use super::CompiledGaijiBank;
use crate::korean_patch::payload_writes::PayloadFileWritePlan;

const GAIJI_FILE: &str = "GAIJI.COM";

pub(crate) fn gaiji_bank_plan(
    compiled: &CompiledGaijiBank,
    id_prefix: &str,
    owner: &str,
    purpose: &str,
) -> PayloadFileWritePlan {
    let mut plan = WritePlan::new();
    for (index, span) in compiled.spans.iter().enumerate() {
        let id = format!("{id_prefix}-gaiji-bank-span-{}", index + 1);
        let range = span.offset..span.offset + span.replacement.len();
        plan = plan
            .region(ImageRegion {
                id: id.clone(),
                range,
                kind: RegionKind::Data,
                reason: "one contiguous run of target-format BIOS GAIJI records".to_owned(),
            })
            .write(ExpectedWrite {
                id,
                owner: owner.to_owned(),
                purpose: format!(
                    "{purpose} beginning at GAIJI slot {}",
                    span.first_slot_index
                ),
                offset: span.offset,
                expected_original: span.expected_original.clone(),
                replacement: span.replacement.clone(),
                intent: WriteIntent::Data,
            });
    }
    PayloadFileWritePlan {
        file_name: GAIJI_FILE,
        plan,
    }
}
