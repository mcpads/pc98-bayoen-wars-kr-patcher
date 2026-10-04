use std::collections::BTreeMap;

use anyhow::{Context, Result};
use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, RegionKind, ResizePlan, WriteIntent,
    WritePlan,
};

use super::model::CompiledSystemLoaderText;
use super::typed_sources::{
    INSTALLER_SOURCE_ID, POST_STACK_HOOK_SOURCE_ID, RESIDENT_COPY_SOURCE_ID,
    RESIDENT_SIZE_SOURCE_ID, TRAMPOLINE_SOURCE_ID, reference_source_id,
};
use crate::korean_patch::payload_writes::{
    PayloadFileWritePlan, UnpackedWriteReport, unpacked_write_report,
};

const SYSTEM_LOADER_FILE: &str = "MEGDOS.SYS";

struct ResidentWrite<'a> {
    id: &'a str,
    owner: &'a str,
    purpose: &'a str,
    offset: usize,
    replacement: Vec<u8>,
    kind: RegionKind,
    intent: WriteIntent,
}

pub(super) fn apply_resident_layout(
    compiled: &CompiledSystemLoaderText,
) -> Result<(Vec<u8>, Vec<UnpackedWriteReport>)> {
    let source = &compiled.reconstructed_source;
    let mut plan = WritePlan::new().region(ImageRegion {
        id: "megdos-shifted-tail".to_owned(),
        range: usize::from(compiled.output_resident_byte_count)..source.len(),
        kind: RegionKind::Protected,
        reason: "the original post-resident loader modules move as one unchanged byte sequence"
            .to_owned(),
    });
    for (offset, id, purpose) in [
        (
            compiled.runtime.resident_size_load_offset,
            RESIDENT_SIZE_SOURCE_ID,
            "reserve the complete paragraph-aligned Korean resident block",
        ),
        (
            compiled.runtime.resident_copy_count_load_offset,
            RESIDENT_COPY_SOURCE_ID,
            "copy the complete paragraph-aligned Korean resident block",
        ),
        (
            compiled.runtime.post_stack_hook_offset,
            POST_STACK_HOOK_SOURCE_ID,
            "route post-stack initialization through the resident GAIJI installer",
        ),
    ] {
        plan = add_machine_write(plan, source, offset, id, purpose, &compiled.typed_sources)?;
    }
    for reference in &compiled.runtime.references {
        let source_id = reference_source_id(reference.instruction_offset);
        plan = add_machine_write(
            plan,
            source,
            reference.instruction_offset,
            &source_id,
            &format!(
                "relocate the resident null-text reference for {}",
                reference.entry_id
            ),
            &compiled.typed_sources,
        )?;
    }
    plan = add_machine_write(
        plan,
        source,
        compiled.installer_offset,
        INSTALLER_SOURCE_ID,
        "install only the Korean glyphs used by the system loader",
        &compiled.typed_sources,
    )?;
    plan = add_data_write(
        plan,
        source,
        compiled.glyph_records_offset,
        compiled.glyph_records.clone(),
        "megdos-resident-gaiji-records",
        "store the system loader's 34-byte BIOS GAIJI records in resident memory",
    )?;
    plan = add_data_write(
        plan,
        source,
        compiled.text_offset,
        compiled.packed_text.clone(),
        "megdos-resident-korean-text",
        "pack all seven reviewed null-terminated system messages in resident memory",
    )?;
    plan = add_machine_write(
        plan,
        source,
        compiled.trampoline_offset,
        TRAMPOLINE_SOURCE_ID,
        "replay the displaced LDS and resume original initialization",
        &compiled.typed_sources,
    )?;

    let typed_sources = compiled.typed_sources.clone();
    let verifier = v30::ExpectedWriteVerifier::new(move |source_id: &str| {
        typed_sources.get(source_id).cloned().ok_or_else(|| {
            expected_write::MachineCodeVerifierError::new(format!(
                "unknown typed V30 MEGDOS.SYS source {source_id}"
            ))
        })
    });
    let output = plan.apply(source, Some(&verifier))?;
    plan.audit(source, &output, Some(&verifier))?;
    Ok((output, unpacked_write_report(&plan)))
}

pub(super) fn system_loader_file_plan(source: &[u8], replacement: &[u8]) -> PayloadFileWritePlan {
    let plan = WritePlan::new()
        .resize(ResizePlan {
            owner: "system-loader-resident-layout".to_owned(),
            purpose: "insert the audited resident extension before the unchanged loader tail"
                .to_owned(),
            expected_input_len: source.len(),
            output_len: replacement.len(),
        })
        .region(ImageRegion {
            id: "megdos-relocated-container".to_owned(),
            range: 0..replacement.len(),
            kind: RegionKind::Data,
            reason:
                "relocated container bytes; executable changes are verified in the resident layout"
                    .to_owned(),
        })
        .write(ExpectedWrite {
            id: "megdos-resident-layout-serialization".to_owned(),
            owner: "system-loader-resident-layout".to_owned(),
            purpose: "serialize the audited resident Expected Writes and exact shifted source tail"
                .to_owned(),
            offset: 0,
            expected_original: source.to_vec(),
            replacement: replacement.to_vec(),
            intent: WriteIntent::Data,
        });
    PayloadFileWritePlan {
        file_name: SYSTEM_LOADER_FILE,
        plan,
    }
}

fn add_machine_write(
    plan: WritePlan,
    source: &[u8],
    offset: usize,
    source_id: &str,
    purpose: &str,
    typed_sources: &BTreeMap<String, v30::AssembledProgram>,
) -> Result<WritePlan> {
    let replacement = typed_sources
        .get(source_id)
        .with_context(|| format!("missing typed V30 MEGDOS.SYS source {source_id}"))?
        .bytes()
        .to_vec();
    add_resident_write(
        plan,
        source,
        ResidentWrite {
            id: source_id,
            owner: "system-loader-typed-relocator",
            purpose,
            offset,
            replacement,
            kind: RegionKind::MachineCode,
            intent: WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: source_id.to_owned(),
                isa_profile_id: v30::PROFILE_ID.to_owned(),
            }),
        },
    )
}

fn add_data_write(
    plan: WritePlan,
    source: &[u8],
    offset: usize,
    replacement: Vec<u8>,
    id: &str,
    purpose: &str,
) -> Result<WritePlan> {
    add_resident_write(
        plan,
        source,
        ResidentWrite {
            id,
            owner: "system-loader-resident-layout",
            purpose,
            offset,
            replacement,
            kind: RegionKind::Data,
            intent: WriteIntent::Data,
        },
    )
}

fn add_resident_write(
    plan: WritePlan,
    source: &[u8],
    write: ResidentWrite<'_>,
) -> Result<WritePlan> {
    let range = write.offset..write.offset + write.replacement.len();
    let expected_original = source
        .get(range.clone())
        .with_context(|| format!("{} lies outside reconstructed MEGDOS.SYS", write.id))?
        .to_vec();
    Ok(plan
        .region(ImageRegion {
            id: write.id.to_owned(),
            range,
            kind: write.kind,
            reason: write.purpose.to_owned(),
        })
        .write(ExpectedWrite {
            id: write.id.to_owned(),
            owner: write.owner.to_owned(),
            purpose: write.purpose.to_owned(),
            offset: write.offset,
            expected_original,
            replacement: write.replacement,
            intent: write.intent,
        }))
}
