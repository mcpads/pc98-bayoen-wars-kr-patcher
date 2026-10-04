use anyhow::{Context, Result};
use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, RegionKind, ResizePlan, WriteIntent,
    WritePlan,
};

use super::model::CompiledSamplingDriverText;
use super::typed_sources::{ENTRY_SOURCE_ID, INSTALLER_SOURCE_ID, reference_source_id};
use crate::korean_patch::payload_writes::PayloadFileWritePlan;

const BSAMP_FILE: &str = "BSAMP.COM";

pub(super) fn sampling_driver_text_plan(
    source: &[u8],
    compiled: &CompiledSamplingDriverText,
) -> Result<PayloadFileWritePlan> {
    let mut plan = WritePlan::new().resize(ResizePlan {
        owner: "sampling-driver-hangul-layout".to_owned(),
        purpose: "append a non-resident GAIJI installer and relocated Korean strings".to_owned(),
        expected_input_len: compiled.source_file_size,
        output_len: compiled.output_file_size,
    });
    plan = add_existing_machine_write(
        plan,
        source,
        compiled.lifetime.entry_jump_offset,
        ENTRY_SOURCE_ID,
        "route transient initialization through the embedded GAIJI installer",
        compiled,
    )?;
    for entry in &compiled.entries {
        for &consumer_offset in &entry.consumer_offsets {
            let source_id = reference_source_id(consumer_offset);
            plan = add_existing_machine_write(
                plan,
                source,
                consumer_offset,
                &source_id,
                &format!("relocate the DOS output reference for {}", entry.id),
                compiled,
            )?;
        }
    }
    plan = add_appended_write(
        plan,
        compiled.installer_offset,
        compiled.installer.bytes().to_vec(),
        RegionKind::MachineCode,
        WriteIntent::MachineCode(MachineCodeProvenance {
            assembly_source_id: INSTALLER_SOURCE_ID.to_owned(),
            isa_profile_id: v30::PROFILE_ID.to_owned(),
        }),
        "sampling-driver-embedded-gaiji-installer",
        "install only the Korean glyph records needed by the sampling driver",
    );
    plan = add_appended_write(
        plan,
        compiled.glyph_records_offset,
        compiled.glyph_records.clone(),
        RegionKind::Data,
        WriteIntent::Data,
        "sampling-driver-gaiji-records",
        "store non-resident 34-byte BIOS GAIJI records in the transient file tail",
    );
    plan = add_appended_write(
        plan,
        compiled.text_offset,
        compiled.packed_text.clone(),
        RegionKind::Data,
        WriteIntent::Data,
        "sampling-driver-korean-text",
        "pack all three reviewed DOS strings with preserved framing and CRLF",
    );
    Ok(PayloadFileWritePlan {
        file_name: BSAMP_FILE,
        plan,
    })
}

fn add_existing_machine_write(
    plan: WritePlan,
    source: &[u8],
    offset: usize,
    source_id: &str,
    purpose: &str,
    compiled: &CompiledSamplingDriverText,
) -> Result<WritePlan> {
    let replacement = compiled
        .typed_sources
        .get(source_id)
        .with_context(|| format!("missing typed V30 source {source_id}"))?
        .bytes()
        .to_vec();
    let range = offset..offset + replacement.len();
    let expected_original = source
        .get(range.clone())
        .with_context(|| format!("sampling-driver source {source_id} lies outside the file"))?
        .to_vec();
    Ok(plan
        .region(ImageRegion {
            id: source_id.to_owned(),
            range,
            kind: RegionKind::MachineCode,
            reason: "one complete typed V30 instruction".to_owned(),
        })
        .write(ExpectedWrite {
            id: source_id.to_owned(),
            owner: "sampling-driver-typed-relocator".to_owned(),
            purpose: purpose.to_owned(),
            offset,
            expected_original,
            replacement,
            intent: WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: source_id.to_owned(),
                isa_profile_id: v30::PROFILE_ID.to_owned(),
            }),
        }))
}

fn add_appended_write(
    plan: WritePlan,
    offset: usize,
    replacement: Vec<u8>,
    kind: RegionKind,
    intent: WriteIntent,
    id: &str,
    purpose: &str,
) -> WritePlan {
    let range = offset..offset + replacement.len();
    plan.region(ImageRegion {
        id: id.to_owned(),
        range,
        kind,
        reason: purpose.to_owned(),
    })
    .write(ExpectedWrite {
        id: id.to_owned(),
        owner: "sampling-driver-hangul-layout".to_owned(),
        purpose: purpose.to_owned(),
        offset,
        expected_original: Vec::new(),
        replacement,
        intent,
    })
}
