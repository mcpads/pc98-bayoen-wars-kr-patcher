use anyhow::{Context, Result, ensure};
use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, RegionKind, ResizePlan, WriteIntent,
    WritePlan,
};

use super::model::CompiledMenuText;
use super::typed_sources::{
    ENTRY_SOURCE_ID, INSTALLER_SOURCE_ID, TRAMPOLINE_SOURCE_ID, reference_source_id,
    runtime_insert_source_id,
};
use crate::game_data::{MenuRuntimeCatalog, MenuTextRuntimeReferenceKind};
use crate::korean_patch::payload_writes::PayloadFileWritePlan;

const MENU_FILE: &str = "MENU.COM";

pub(super) fn menu_text_plan(
    source: &[u8],
    runtime: &MenuRuntimeCatalog,
    compiled: &CompiledMenuText,
) -> Result<PayloadFileWritePlan> {
    let mut plan = WritePlan::new().resize(ResizePlan {
        owner: "menu-hangul-layout".to_owned(),
        purpose:
            "append an owned MENU GAIJI installer, entry trampoline, and relocated Korean strings"
                .to_owned(),
        expected_input_len: compiled.source_file_size,
        output_len: compiled.output_file_size,
    });
    plan = add_machine_write(
        plan,
        source,
        runtime.entry.hook_instruction_offset,
        ENTRY_SOURCE_ID,
        "route MENU startup through the embedded GAIJI installer",
        compiled,
    )?;
    for reference in &runtime.references {
        let target = compiled_entry(compiled, &reference.target_entry_id)?.com_address;
        match reference.storage_kind {
            MenuTextRuntimeReferenceKind::MachineCodeImmediate => {
                let instruction_offset = reference
                    .instruction_offset
                    .context("MENU machine-code reference lost its instruction offset")?;
                let source_id = reference_source_id(instruction_offset);
                plan = add_machine_write(
                    plan,
                    source,
                    instruction_offset,
                    &source_id,
                    &format!(
                        "relocate the MENU reference for {}",
                        reference.target_entry_id
                    ),
                    compiled,
                )?;
            }
            MenuTextRuntimeReferenceKind::MetadataTableEntry => {
                plan = add_metadata_write(
                    plan,
                    source,
                    reference.storage_offset,
                    reference.target_com_address,
                    target,
                    &reference.id,
                    &reference.target_entry_id,
                )?;
            }
        }
    }
    for reference in &runtime.runtime_insert_references {
        let source_id = runtime_insert_source_id(&reference.id);
        plan = add_machine_write(
            plan,
            source,
            reference.instruction_offset,
            &source_id,
            &format!("relocate the runtime insertion field {}", reference.id),
            compiled,
        )?;
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
        INSTALLER_SOURCE_ID,
        "install the MENU-local Korean GAIJI bank before original initialization",
    );
    plan = add_appended_write(
        plan,
        compiled.glyph_records_offset,
        compiled.glyph_records.clone(),
        RegionKind::Data,
        WriteIntent::Data,
        "menu-gaiji-records",
        "store owned 34-byte BIOS GAIJI records in the resized MENU image",
    );
    plan = add_appended_write(
        plan,
        compiled.trampoline_offset,
        compiled.trampoline.bytes().to_vec(),
        RegionKind::MachineCode,
        WriteIntent::MachineCode(MachineCodeProvenance {
            assembly_source_id: TRAMPOLINE_SOURCE_ID.to_owned(),
            isa_profile_id: v30::PROFILE_ID.to_owned(),
        }),
        TRAMPOLINE_SOURCE_ID,
        "execute the overwritten original CALL and resume at the exact next instruction",
    );
    plan = add_appended_write(
        plan,
        compiled.text_offset,
        compiled.packed_text.clone(),
        RegionKind::Data,
        WriteIntent::Data,
        "menu-korean-text",
        "pack all MENU records with source control and terminator structure preserved",
    );
    ensure!(
        compiled.typed_sources.len() == runtime.total_machine_code_relocation_count + 3,
        "typed V30 MENU source population changed"
    );
    Ok(PayloadFileWritePlan {
        file_name: MENU_FILE,
        plan,
    })
}

fn add_machine_write(
    plan: WritePlan,
    source: &[u8],
    offset: usize,
    source_id: &str,
    purpose: &str,
    compiled: &CompiledMenuText,
) -> Result<WritePlan> {
    let replacement = compiled
        .typed_sources
        .get(source_id)
        .with_context(|| format!("missing typed V30 MENU source {source_id}"))?
        .bytes()
        .to_vec();
    let range = offset..offset + replacement.len();
    let expected_original = source
        .get(range.clone())
        .with_context(|| format!("MENU machine source {source_id} lies outside the file"))?
        .to_vec();
    Ok(plan
        .region(ImageRegion {
            id: source_id.to_owned(),
            range,
            kind: RegionKind::MachineCode,
            reason: "one complete typed V30 instruction or admitted instruction block".to_owned(),
        })
        .write(ExpectedWrite {
            id: source_id.to_owned(),
            owner: "menu-typed-relocator".to_owned(),
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

fn add_metadata_write(
    plan: WritePlan,
    source: &[u8],
    offset: usize,
    expected_target: u16,
    replacement_target: u16,
    id: &str,
    entry_id: &str,
) -> Result<WritePlan> {
    let expected_original = source
        .get(offset..offset + 2)
        .context("MENU metadata reference lies outside the file")?
        .to_vec();
    ensure!(
        expected_original == expected_target.to_le_bytes(),
        "MENU metadata reference {id} source target changed"
    );
    Ok(plan
        .region(ImageRegion {
            id: id.to_owned(),
            range: offset..offset + 2,
            kind: RegionKind::Metadata,
            reason: "one complete MENU pointer-table entry".to_owned(),
        })
        .write(ExpectedWrite {
            id: id.to_owned(),
            owner: "menu-pointer-table-relocator".to_owned(),
            purpose: format!("relocate the MENU table reference for {entry_id}"),
            offset,
            expected_original,
            replacement: replacement_target.to_le_bytes().to_vec(),
            intent: WriteIntent::Metadata,
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
        owner: "menu-hangul-layout".to_owned(),
        purpose: purpose.to_owned(),
        offset,
        expected_original: Vec::new(),
        replacement,
        intent,
    })
}

fn compiled_entry<'a>(
    compiled: &'a CompiledMenuText,
    id: &str,
) -> Result<&'a super::model::CompiledMenuTextEntry> {
    compiled
        .entries
        .iter()
        .find(|entry| entry.id == id)
        .with_context(|| format!("missing compiled MENU entry {id}"))
}
