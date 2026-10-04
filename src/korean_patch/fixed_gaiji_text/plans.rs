use anyhow::{Result, ensure};
use expected_write::{ExpectedWrite, ImageRegion, RegionKind, WriteIntent, WritePlan};

use super::model::CompiledFixedGaijiText;
use crate::korean_patch::payload_writes::PayloadFileWritePlan;

const MAD_FILE: &str = "MAD.COM";

pub(in crate::korean_patch) fn fixed_slots_plan(
    mad_com: &[u8],
    compiled: &CompiledFixedGaijiText,
) -> Result<PayloadFileWritePlan> {
    ensure!(!compiled.slots.is_empty(), "fixed-text plan has no slots");
    for pair in compiled.slots.windows(2) {
        ensure!(
            pair[1].file_offset == pair[0].file_offset + pair[0].bytes.len(),
            "fixed GAIJI text slots are no longer contiguous"
        );
    }
    let offset = compiled.slots[0].file_offset;
    let replacement = compiled
        .slots
        .iter()
        .flat_map(|slot| slot.bytes.iter().copied())
        .collect::<Vec<_>>();
    let end = offset + replacement.len();
    let expected_original = mad_com
        .get(offset..end)
        .ok_or_else(|| anyhow::anyhow!("fixed GAIJI text region lies outside MAD.COM"))?
        .to_vec();
    Ok(PayloadFileWritePlan {
        file_name: MAD_FILE,
        plan: WritePlan::new()
            .region(ImageRegion {
                id: "fixed-gaiji-text-slots".to_owned(),
                range: offset..end,
                kind: RegionKind::Data,
                reason: "ten consumer-bound 6x7 GAIJI text slots".to_owned(),
            })
            .write(ExpectedWrite {
                id: "fixed-gaiji-text-slots".to_owned(),
                owner: "fixed-gaiji-text-encoder".to_owned(),
                purpose: "replace all ten fixed text slots without changing their pointers"
                    .to_owned(),
                offset,
                expected_original,
                replacement,
                intent: WriteIntent::Data,
            }),
    })
}
