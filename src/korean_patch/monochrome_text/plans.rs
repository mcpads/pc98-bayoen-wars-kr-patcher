use std::collections::BTreeMap;

use anyhow::{Context, Result};
use expected_write::{ExpectedWrite, ImageRegion, RegionKind, ResizePlan, WriteIntent, WritePlan};

use super::MAD_FILE;
use super::model::SequencePatch;
use super::pages::CompiledPages;
use crate::asset_bindings::MonochromeTextSequence;
use crate::korean_patch::payload_writes::PayloadFileWritePlan;

pub(super) fn packed_atlas_plan(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    patch: &SequencePatch,
) -> PayloadFileWritePlan {
    let baseline = &installer_payload[patch.file_name];
    let mut plan = WritePlan::new();
    if baseline.len() != patch.packed_atlas.len() {
        plan = plan.resize(ResizePlan {
            owner: format!("{}-atlas-compressor", patch.id),
            purpose: "store the fixed-capacity translated monochrome glyph atlas".to_owned(),
            expected_input_len: baseline.len(),
            output_len: patch.packed_atlas.len(),
        });
    }
    plan = plan
        .region(ImageRegion {
            id: format!("{}-packed-atlas", patch.id),
            range: 0..patch.packed_atlas.len(),
            kind: RegionKind::Data,
            reason: "one consumer-linked Compile LZ monochrome atlas stream".to_owned(),
        })
        .write(ExpectedWrite {
            id: format!("{}-translated-atlas", patch.id),
            owner: format!("{}-atlas-compressor", patch.id),
            purpose: "replace the source atlas with reviewed development glyphs".to_owned(),
            offset: 0,
            expected_original: baseline[..baseline.len().min(patch.packed_atlas.len())].to_vec(),
            replacement: patch.packed_atlas.clone(),
            intent: WriteIntent::Data,
        });
    PayloadFileWritePlan {
        file_name: patch.file_name,
        plan,
    }
}

pub(super) fn mad_page_plan(
    mad_com: &[u8],
    opening_source: &MonochromeTextSequence,
    opening: &CompiledPages,
    ending_source: &MonochromeTextSequence,
    ending: &CompiledPages,
) -> Result<PayloadFileWritePlan> {
    let mut plan = WritePlan::new();
    for (source, compiled) in [(opening_source, opening), (ending_source, ending)] {
        let pointer_start = source.pointer_table_file_offset;
        let pointer_end = pointer_start + compiled.pointer_table.len();
        let page_start = source.pages[0].file_offset;
        let page_end = page_start + compiled.page_data.len();
        let pointer_range = pointer_start..pointer_end;
        let page_range = page_start..page_end;
        let pointer_original = mad_com
            .get(pointer_range.clone())
            .with_context(|| format!("{} pointer table lies outside MAD.COM", source.id))?;
        let page_original = mad_com
            .get(page_range.clone())
            .with_context(|| format!("{} page storage lies outside MAD.COM", source.id))?;
        plan = plan
            .region(ImageRegion {
                id: format!("{}-page-pointer-table", source.id),
                range: pointer_range,
                kind: RegionKind::Metadata,
                reason: format!(
                    "{} page starts consumed by the typed V30 table reader",
                    source.id
                ),
            })
            .region(ImageRegion {
                id: format!("{}-page-data", source.id),
                range: page_range,
                kind: RegionKind::Data,
                reason: format!(
                    "{} glyph-index, line-break, and terminator stream",
                    source.id
                ),
            })
            .write(ExpectedWrite {
                id: format!("{}-page-pointers", source.id),
                owner: format!("{}-page-compiler", source.id),
                purpose: "point every page entry at its compiled development text".to_owned(),
                offset: pointer_start,
                expected_original: pointer_original.to_vec(),
                replacement: compiled.pointer_table.clone(),
                intent: WriteIntent::Metadata,
            })
            .write(ExpectedWrite {
                id: format!("{}-page-streams", source.id),
                owner: format!("{}-page-compiler", source.id),
                purpose: "replace source page streams without growing their storage region"
                    .to_owned(),
                offset: page_start,
                expected_original: page_original.to_vec(),
                replacement: compiled.page_data.clone(),
                intent: WriteIntent::Data,
            });
    }
    Ok(PayloadFileWritePlan {
        file_name: MAD_FILE,
        plan,
    })
}
