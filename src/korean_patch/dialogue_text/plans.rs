use anyhow::{Context, Result};
use expected_write::{ExpectedWrite, ImageRegion, RegionKind, WriteIntent, WritePlan};

use super::model::CompiledDialogueText;
use crate::korean_patch::payload_writes::PayloadFileWritePlan;

const MAD_FILE: &str = "MAD.COM";

pub(super) fn dialogue_text_plan(
    mad_com: &[u8],
    compiled: &CompiledDialogueText,
) -> Result<PayloadFileWritePlan> {
    let record_range = compiled.record_table_start..compiled.record_table_end;
    let record_original = mad_com
        .get(record_range.clone())
        .context("dialogue record tables lie outside MAD.COM")?
        .to_vec();
    let text_range = compiled.text_region_start..compiled.text_region_end;
    let text_original = mad_com
        .get(text_range.clone())
        .context("dialogue text storage lies outside MAD.COM")?
        .to_vec();
    let plan = WritePlan::new()
        .region(ImageRegion {
            id: "dialogue-record-tables".to_owned(),
            range: record_range,
            kind: RegionKind::Metadata,
            reason: "all 55 text pointers with presentation variants and terminators preserved"
                .to_owned(),
        })
        .region(ImageRegion {
            id: "dialogue-text-storage".to_owned(),
            range: text_range,
            kind: RegionKind::Data,
            reason: "all 55 relocated dialogue records and zeroed storage tail".to_owned(),
        })
        .write(ExpectedWrite {
            id: "dialogue-record-tables".to_owned(),
            owner: "dialogue-reference-relocator".to_owned(),
            purpose: "relocate every dialogue text pointer without changing executable code"
                .to_owned(),
            offset: compiled.record_table_start,
            expected_original: record_original,
            replacement: compiled.record_table_replacement.clone(),
            intent: WriteIntent::Metadata,
        })
        .write(ExpectedWrite {
            id: "dialogue-text-storage".to_owned(),
            owner: "dialogue-text-packer".to_owned(),
            purpose: "pack reviewed dialogue text inside the verified source storage region"
                .to_owned(),
            offset: compiled.text_region_start,
            expected_original: text_original,
            replacement: compiled.text_region_replacement.clone(),
            intent: WriteIntent::Data,
        });
    Ok(PayloadFileWritePlan {
        file_name: MAD_FILE,
        plan,
    })
}
