use anyhow::{Context, Result};
use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, RegionKind, ResizePlan, WriteIntent,
    WritePlan,
};

use super::model::CompiledShellText;
use super::typed_sources::{
    ENTRY_SOURCE_ID, INSTALLER_SOURCE_ID, MEMORY_END_INSTRUCTION_OFFSET, MEMORY_END_SOURCE_ID,
    reference_source_id,
};
use crate::korean_patch::payload_writes::PayloadFileWritePlan;

const DSH_FILE: &str = "DSH.COM";

pub(super) fn shell_text_plan(
    source: &[u8],
    compiled: &CompiledShellText,
) -> Result<PayloadFileWritePlan> {
    let mut plan = WritePlan::new().resize(ResizePlan {
        owner: "dsh-hangul-layout".to_owned(),
        purpose: "append an owned boot-shell GAIJI installer and relocated Korean strings"
            .to_owned(),
        expected_input_len: compiled.source_file_size,
        output_len: compiled.output_file_size,
    });
    plan = add_existing_machine_write(
        plan,
        source,
        0,
        ENTRY_SOURCE_ID,
        "route shell entry through the embedded GAIJI installer",
        compiled,
    )?;
    plan = add_existing_machine_write(
        plan,
        source,
        MEMORY_END_INSTRUCTION_OFFSET,
        MEMORY_END_SOURCE_ID,
        "retain the complete resized shell image after DOS memory shrink",
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
        "dsh-embedded-gaiji-installer",
        "install only the Korean glyph records needed by the boot shell",
    );
    plan = add_appended_write(
        plan,
        compiled.glyph_records_offset,
        compiled.glyph_records.clone(),
        RegionKind::Data,
        WriteIntent::Data,
        "dsh-gaiji-records",
        "store owned 34-byte BIOS GAIJI records inside the resized shell",
    );
    plan = add_appended_write(
        plan,
        compiled.text_offset,
        compiled.packed_text.clone(),
        RegionKind::Data,
        WriteIntent::Data,
        "dsh-korean-text",
        "pack all eight reviewed shell strings with preserved CRLF and DOS terminators",
    );
    Ok(PayloadFileWritePlan {
        file_name: DSH_FILE,
        plan,
    })
}

fn add_existing_machine_write(
    plan: WritePlan,
    source: &[u8],
    offset: usize,
    source_id: &str,
    purpose: &str,
    compiled: &CompiledShellText,
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
        .with_context(|| format!("DSH machine source {source_id} lies outside the file"))?
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
            owner: "dsh-typed-relocator".to_owned(),
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
        owner: "dsh-hangul-layout".to_owned(),
        purpose: purpose.to_owned(),
        offset,
        expected_original: Vec::new(),
        replacement,
        intent,
    })
}
