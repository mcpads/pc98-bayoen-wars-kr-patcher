use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, RegionKind, ResizePlan, WriteIntent,
    WritePlan,
};

use super::model::{CompiledPlaybackDriverTextEntry, TextStorageWrite};
use super::typed_sources::{entry_source_id, installer_source_id, reference_source_id};
use crate::external_text::{PackedSoundDriverReferenceKind, PackedSoundDriverRuntimeCatalog};
use crate::korean_patch::payload_writes::{
    PayloadFileWritePlan, UnpackedWriteReport, unpacked_write_report,
};

pub(super) struct UnpackedLayout<'a> {
    pub file_name: &'static str,
    pub runtime: &'a PackedSoundDriverRuntimeCatalog,
    pub typed_sources: &'a BTreeMap<String, v30::AssembledProgram>,
    pub text_storage_writes: &'a [TextStorageWrite],
    pub entries: &'a [CompiledPlaybackDriverTextEntry],
    pub installer_offset: usize,
    pub installer: &'a v30::AssembledProgram,
    pub glyph_records_offset: usize,
    pub glyph_records: &'a [u8],
}

struct UnpackedWrite<'a> {
    id: &'a str,
    owner: &'a str,
    purpose: &'a str,
    offset: usize,
    expected_original: Option<Vec<u8>>,
    replacement: Vec<u8>,
    kind: RegionKind,
    intent: WriteIntent,
}

pub(super) fn apply_unpacked_layout(
    source: &[u8],
    layout: UnpackedLayout<'_>,
) -> Result<(Vec<u8>, Vec<UnpackedWriteReport>)> {
    let output_len = layout
        .glyph_records_offset
        .checked_add(layout.glyph_records.len())
        .context("playback-driver unpacked output size overflow")?;
    let mut plan = WritePlan::new().resize(ResizePlan {
        owner: "playback-driver-unpacked-layout".to_owned(),
        purpose: "add a first-nonresident GAIJI installer and relocate Korean DOS text".to_owned(),
        expected_input_len: source.len(),
        output_len,
    });

    let entry_id = entry_source_id(layout.file_name);
    plan = add_machine_write(
        plan,
        source,
        layout.runtime.entry_jump_offset,
        &entry_id,
        "route this invocation through its own GAIJI installer",
        layout.typed_sources,
    )?;
    for reference in &layout.runtime.references {
        match reference.kind {
            PackedSoundDriverReferenceKind::MachineCode => {
                let instruction_offset = reference
                    .instruction_offset
                    .context("machine-code playback reference lost its instruction offset")?;
                let source_id = reference_source_id(layout.file_name, instruction_offset);
                plan = add_machine_write(
                    plan,
                    source,
                    instruction_offset,
                    &source_id,
                    &format!(
                        "relocate the DOS output reference for {}",
                        reference.entry_id
                    ),
                    layout.typed_sources,
                )?;
            }
            PackedSoundDriverReferenceKind::Metadata => {
                let address = layout
                    .entries
                    .iter()
                    .find(|entry| entry.id == reference.entry_id)
                    .with_context(|| format!("missing translated record {}", reference.entry_id))?
                    .com_address;
                plan = add_exact_write(
                    plan,
                    source,
                    UnpackedWrite {
                        id: &format!(
                            "{}-time-reference-{:04x}",
                            source_stem(layout.file_name),
                            reference.storage_offset
                        ),
                        owner: "playback-driver-time-table",
                        purpose: &format!(
                            "relocate the time-message table reference for {}",
                            reference.entry_id
                        ),
                        offset: reference.storage_offset,
                        expected_original: None,
                        replacement: address.to_le_bytes().to_vec(),
                        kind: RegionKind::Metadata,
                        intent: WriteIntent::Metadata,
                    },
                )?;
            }
        }
    }
    for storage in layout.text_storage_writes {
        plan = add_unpacked_write(
            plan,
            UnpackedWrite {
                id: &format!(
                    "{}-source-slot-{}",
                    source_stem(layout.file_name),
                    storage.id
                ),
                owner: "playback-driver-text-layout",
                purpose: "replace one fully catalogued source DOS-string slot",
                offset: storage.offset,
                expected_original: Some(storage.expected_original.clone()),
                replacement: storage.replacement.clone(),
                kind: RegionKind::Data,
                intent: WriteIntent::Data,
            },
        );
    }
    if source.len() < layout.installer_offset {
        plan = add_unpacked_write(
            plan,
            UnpackedWrite {
                id: &format!("{}-nonresident-padding", source_stem(layout.file_name)),
                owner: "playback-driver-unpacked-layout",
                purpose: "pad only to the first file byte released by the verified TSR paragraph count",
                offset: source.len(),
                expected_original: Some(Vec::new()),
                replacement: vec![0; layout.installer_offset - source.len()],
                kind: RegionKind::Data,
                intent: WriteIntent::Data,
            },
        );
    }
    let installer_id = installer_source_id(layout.file_name);
    plan = add_unpacked_write(
        plan,
        UnpackedWrite {
            id: &installer_id,
            owner: "playback-driver-unpacked-layout",
            purpose: "install only this playback driver's active Korean GAIJI bank before its transient path",
            offset: layout.installer_offset,
            expected_original: Some(Vec::new()),
            replacement: layout.installer.bytes().to_vec(),
            kind: RegionKind::MachineCode,
            intent: WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: installer_id.clone(),
                isa_profile_id: v30::PROFILE_ID.to_owned(),
            }),
        },
    );
    plan = add_unpacked_write(
        plan,
        UnpackedWrite {
            id: &format!("{}-gaiji-records", source_stem(layout.file_name)),
            owner: "playback-driver-unpacked-layout",
            purpose: "store BIOS GAIJI records entirely outside the resident allocation",
            offset: layout.glyph_records_offset,
            expected_original: Some(Vec::new()),
            replacement: layout.glyph_records.to_vec(),
            kind: RegionKind::Data,
            intent: WriteIntent::Data,
        },
    );

    let typed_sources = layout.typed_sources.clone();
    let verifier = v30::ExpectedWriteVerifier::new(move |source_id: &str| {
        typed_sources.get(source_id).cloned().ok_or_else(|| {
            expected_write::MachineCodeVerifierError::new(format!(
                "unknown typed V30 playback-driver source {source_id}"
            ))
        })
    });
    let output = plan.apply(source, Some(&verifier))?;
    plan.audit(source, &output, Some(&verifier))?;
    Ok((output, unpacked_write_report(&plan)))
}

pub(super) fn packed_driver_plan(
    file_name: &'static str,
    source: &[u8],
    replacement: &[u8],
) -> PayloadFileWritePlan {
    let plan = WritePlan::new()
        .resize(ResizePlan {
            owner: "playback-driver-self-expanding-container".to_owned(),
            purpose: "replace the verified packed stream with the deterministic repack".to_owned(),
            expected_input_len: source.len(),
            output_len: replacement.len(),
        })
        .region(ImageRegion {
            id: format!("{}-packed-container", source_stem(file_name)),
            range: 0..replacement.len(),
            kind: RegionKind::Data,
            reason: "compressed container bytes; executable semantics are verified after unpacking"
                .to_owned(),
        })
        .write(ExpectedWrite {
            id: format!("{}-deterministic-repack", source_stem(file_name)),
            owner: "playback-driver-self-expanding-container".to_owned(),
            purpose: "serialize the audited unpacked Expected Writes through the verified codec"
                .to_owned(),
            offset: 0,
            expected_original: source.to_vec(),
            replacement: replacement.to_vec(),
            intent: WriteIntent::Data,
        });
    PayloadFileWritePlan { file_name, plan }
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
        .with_context(|| format!("missing typed V30 source {source_id}"))?
        .bytes()
        .to_vec();
    add_exact_write(
        plan,
        source,
        UnpackedWrite {
            id: source_id,
            owner: "playback-driver-typed-relocator",
            purpose,
            offset,
            expected_original: None,
            replacement,
            kind: RegionKind::MachineCode,
            intent: WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: source_id.to_owned(),
                isa_profile_id: v30::PROFILE_ID.to_owned(),
            }),
        },
    )
}

fn add_exact_write(
    plan: WritePlan,
    source: &[u8],
    mut write: UnpackedWrite<'_>,
) -> Result<WritePlan> {
    ensure!(
        write.expected_original.is_none(),
        "exact playback-driver write must derive its preimage from the source"
    );
    let expected_original = source
        .get(write.offset..write.offset + write.replacement.len())
        .with_context(|| format!("{} source range lies outside the unpacked image", write.id))?
        .to_vec();
    write.expected_original = Some(expected_original);
    Ok(add_unpacked_write(plan, write))
}

fn add_unpacked_write(plan: WritePlan, write: UnpackedWrite<'_>) -> WritePlan {
    let expected_original = write
        .expected_original
        .expect("unpacked playback-driver write must have an exact preimage");
    plan.region(ImageRegion {
        id: write.id.to_owned(),
        range: write.offset..write.offset + write.replacement.len(),
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
    })
}

fn source_stem(file_name: &str) -> String {
    file_name
        .split_once('.')
        .map_or(file_name, |(stem, _)| stem)
        .to_ascii_lowercase()
}
