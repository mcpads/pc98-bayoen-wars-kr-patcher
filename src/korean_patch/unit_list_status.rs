use super::{
    battle_callout_text::assemble_banked_text_wrapper,
    interface_text::{CompiledInterfaceTextEntry, compile_record},
    interface_window_layout::assemble_window_mov,
    shared_text::CompiledGaijiBank,
};
use crate::{game_data::UnitListStatusCatalog, translation_drafts::TranslationDraftSegment};
use anyhow::{Result, ensure};
use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, RegionKind, WriteIntent, WritePlan,
};
use std::collections::BTreeMap;
use v30::{AssembledProgram, Assembler, CallTarget, CodeLocation, Instruction, Register16};

pub(in crate::korean_patch) struct CompiledUnitListStatus {
    pub entries: Vec<CompiledInterfaceTextEntry>,
    pub text_start: usize,
    pub text_end: usize,
    pub bytes: Vec<u8>,
    pub wrapper_offset: usize,
    pub sources: BTreeMap<String, (usize, AssembledProgram)>,
}
pub(in crate::korean_patch) fn compile_unit_list_status(
    source: &UnitListStatusCatalog,
    draft: &TranslationDraftSegment,
    bank: &CompiledGaijiBank,
    switch: u16,
    bank_offset: usize,
    interface_offset: usize,
) -> Result<CompiledUnitListStatus> {
    ensure!(
        draft.entries.len() == source.entries.len(),
        "unit list draft population changed"
    );
    let mut entries = source
        .entries
        .iter()
        .zip(&draft.entries)
        .map(|(a, b)| compile_record(a, b, bank))
        .collect::<Result<Vec<_>>>()?;
    let mut bytes = Vec::new();
    for (entry, consumer) in entries.iter_mut().zip(&source.consumers) {
        ensure!(
            entry.line_screen_byte_widths.len() == 1
                && entry.line_screen_byte_widths[0] <= consumer.maximum_screen_bytes,
            "{} exceeds its unit list column",
            entry.id
        );
        entry.file_offset = source.text_start_offset + bytes.len();
        entry.com_address = u16::try_from(entry.file_offset + 0x100)?;
        bytes.extend_from_slice(&entry.bytes);
    }
    let wrapper_offset = source.text_start_offset + bytes.len();
    let wrapper_address = u16::try_from(wrapper_offset + 0x100)?;
    let wrapper = assemble_banked_text_wrapper(
        CodeLocation {
            seg: 0,
            off: wrapper_address,
        },
        0x5728,
        switch,
        bank_offset,
        interface_offset,
    )?;
    ensure!(
        wrapper_offset + wrapper.bytes().len() <= source.text_end_offset,
        "unit list status pool has no room for its bank wrapper"
    );
    let mut sources = BTreeMap::new();
    sources.insert(
        "unit-list-status-bank-wrapper".to_owned(),
        (wrapper_offset, wrapper),
    );
    for (entry, consumer) in entries.iter().zip(&source.consumers) {
        let source = assemble_window_mov(
            consumer.text_instruction_offset,
            Register16::DX,
            entry.com_address,
        )?;
        sources.insert(
            format!("{}-address", entry.id),
            (consumer.text_instruction_offset, source),
        );
        let origin = CodeLocation {
            seg: 0,
            off: u16::try_from(consumer.renderer_call_offset + 0x100)?,
        };
        let mut a = Assembler::new();
        a.emit(Instruction::Call {
            target: CallTarget::Rel16(
                wrapper_address.wrapping_sub(origin.off.wrapping_add(3)) as i16
            ),
        });
        sources.insert(
            format!("{}-banked-render", entry.id),
            (consumer.renderer_call_offset, a.assemble(origin)?),
        );
    }
    Ok(CompiledUnitListStatus {
        entries,
        text_start: source.text_start_offset,
        text_end: source.text_end_offset,
        bytes,
        wrapper_offset,
        sources,
    })
}
pub(in crate::korean_patch) fn add_unit_list_status_plan(
    mut plan: WritePlan,
    original: &[u8],
    compiled: &CompiledUnitListStatus,
    typed: &mut BTreeMap<String, AssembledProgram>,
) -> Result<WritePlan> {
    plan = add_write(
        plan,
        original,
        "unit-list-status-records",
        compiled.text_start,
        compiled.bytes.clone(),
        RegionKind::Data,
        WriteIntent::Data,
    )?;
    let mut tail_start = compiled.wrapper_offset;
    for (id, (offset, source)) in &compiled.sources {
        if *offset == compiled.wrapper_offset {
            tail_start += source.bytes().len();
        }
        ensure!(
            typed.insert(id.clone(), source.clone()).is_none(),
            "duplicate unit list typed source {id}"
        );
        plan = add_write(
            plan,
            original,
            id,
            *offset,
            source.bytes().to_vec(),
            RegionKind::MachineCode,
            WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: id.clone(),
                isa_profile_id: v30::PROFILE_ID.to_owned(),
            }),
        )?;
    }
    if tail_start < compiled.text_end {
        plan = add_write(
            plan,
            original,
            "unit-list-status-tail",
            tail_start,
            vec![0; compiled.text_end - tail_start],
            RegionKind::Data,
            WriteIntent::Data,
        )?;
    }
    Ok(plan)
}
fn add_write(
    plan: WritePlan,
    original: &[u8],
    id: &str,
    offset: usize,
    replacement: Vec<u8>,
    kind: RegionKind,
    intent: WriteIntent,
) -> Result<WritePlan> {
    let range = offset..offset + replacement.len();
    let expected_original = original
        .get(range.clone())
        .ok_or_else(|| anyhow::anyhow!("unit list write outside source"))?
        .to_vec();
    Ok(plan
        .region(ImageRegion {
            id: id.to_owned(),
            range,
            kind,
            reason: "verified unit list text and banked consumer".to_owned(),
        })
        .write(ExpectedWrite {
            id: id.to_owned(),
            owner: "unit-list-status".to_owned(),
            purpose: "render translated list status and restore interface bank".to_owned(),
            offset,
            expected_original,
            replacement,
            intent,
        }))
}
pub(in crate::korean_patch) fn verify_unit_list_status(
    program: &[u8],
    compiled: &CompiledUnitListStatus,
) -> Result<()> {
    ensure!(
        program.get(compiled.text_start..compiled.wrapper_offset)
            == Some(compiled.bytes.as_slice()),
        "unit list text failed readback"
    );
    for entry in &compiled.entries {
        ensure!(
            program.get(entry.file_offset..entry.file_offset + entry.bytes.len())
                == Some(entry.bytes.as_slice()),
            "{} unit list record readback failed",
            entry.id
        );
    }
    for (id, (offset, source)) in &compiled.sources {
        ensure!(
            program.get(*offset..*offset + source.bytes().len()) == Some(source.bytes()),
            "unit list source {id} failed readback"
        );
    }
    Ok(())
}

#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub struct UnitListStatusPatchReport {
    pub entry_count: usize,
    pub bank_file_offset: usize,
    pub wrapper_file_offset: usize,
    pub storage_capacity: usize,
    pub packed_storage_bytes: usize,
    pub lines: Vec<Vec<String>>,
    pub line_screen_byte_widths: Vec<Vec<usize>>,
}
pub(in crate::korean_patch) fn unit_list_status_report(
    compiled: &CompiledUnitListStatus,
    bank_file_offset: usize,
) -> UnitListStatusPatchReport {
    UnitListStatusPatchReport {
        entry_count: compiled.entries.len(),
        bank_file_offset,
        wrapper_file_offset: compiled.wrapper_offset,
        storage_capacity: compiled.text_end - compiled.text_start,
        packed_storage_bytes: compiled.bytes.len(),
        lines: compiled.entries.iter().map(|e| e.lines.clone()).collect(),
        line_screen_byte_widths: compiled
            .entries
            .iter()
            .map(|e| e.line_screen_byte_widths.clone())
            .collect(),
    }
}
