use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, RegionKind, WriteIntent, WritePlan,
};
use v30::{
    AssembledProgram, Assembler, CodeLocation, Instruction, Operand, Register16, decode_bytes,
};

pub(in crate::korean_patch) struct CompiledWindowTypedWrite {
    pub source_id: String,
    pub write_id: String,
    pub purpose: String,
    pub file_offset: usize,
    pub register: Register16,
    pub output_value: u16,
    pub source: AssembledProgram,
}

pub(in crate::korean_patch) struct CompiledWindowMetadataWrite {
    pub write_id: String,
    pub purpose: String,
    pub file_offset: usize,
    pub output_value: u16,
}

pub(in crate::korean_patch) fn assemble_window_mov(
    file_offset: usize,
    register: Register16,
    value: u16,
) -> Result<AssembledProgram> {
    let mut assembler = Assembler::new();
    assembler.emit(Instruction::Mov {
        dest: Operand::Reg16(register),
        src: Operand::Imm16(value),
    });
    let source = assembler.assemble(CodeLocation {
        seg: 0,
        off: u16::try_from(file_offset + 0x100)
            .context("interface window instruction COM address exceeds 16 bits")?,
    })?;
    ensure!(
        source.bytes().len() == 3,
        "interface window MOV reg16, imm16 changed size"
    );
    Ok(source)
}

pub(in crate::korean_patch) fn add_window_layout_writes(
    mut plan: WritePlan,
    mad_com: &[u8],
    owner: &str,
    typed_writes: &[CompiledWindowTypedWrite],
    metadata_writes: &[CompiledWindowMetadataWrite],
) -> Result<WritePlan> {
    for write in typed_writes {
        let replacement = write.source.bytes().to_vec();
        let range = write.file_offset..write.file_offset + replacement.len();
        let expected_original = mad_com
            .get(range.clone())
            .with_context(|| format!("{} lies outside MAD.COM", write.write_id))?
            .to_vec();
        plan = plan
            .region(ImageRegion {
                id: write.write_id.clone(),
                range,
                kind: RegionKind::MachineCode,
                reason: "one complete typed V30 MOV reg16, imm16 layout instruction".to_owned(),
            })
            .write(ExpectedWrite {
                id: write.write_id.clone(),
                owner: owner.to_owned(),
                purpose: write.purpose.clone(),
                offset: write.file_offset,
                expected_original,
                replacement,
                intent: WriteIntent::MachineCode(MachineCodeProvenance {
                    assembly_source_id: write.source_id.clone(),
                    isa_profile_id: v30::PROFILE_ID.to_owned(),
                }),
            });
    }
    for write in metadata_writes {
        let range = write.file_offset..write.file_offset + 2;
        let expected_original = mad_com
            .get(range.clone())
            .with_context(|| format!("{} lies outside MAD.COM", write.write_id))?
            .to_vec();
        plan = plan
            .region(ImageRegion {
                id: write.write_id.clone(),
                range,
                kind: RegionKind::Metadata,
                reason: "state-indexed interface window-frame word".to_owned(),
            })
            .write(ExpectedWrite {
                id: write.write_id.clone(),
                owner: owner.to_owned(),
                purpose: write.purpose.clone(),
                offset: write.file_offset,
                expected_original,
                replacement: write.output_value.to_le_bytes().to_vec(),
                intent: WriteIntent::Metadata,
            });
    }
    Ok(plan)
}

pub(in crate::korean_patch) fn add_window_layout_sources(
    typed_sources: &mut BTreeMap<String, AssembledProgram>,
    writes: &[CompiledWindowTypedWrite],
) -> Result<()> {
    for write in writes {
        ensure!(
            typed_sources
                .insert(write.source_id.clone(), write.source.clone())
                .is_none(),
            "duplicate interface window typed source {}",
            write.source_id
        );
    }
    Ok(())
}

pub(in crate::korean_patch) fn verify_window_layout_writes(
    mad_com: &[u8],
    typed_writes: &[CompiledWindowTypedWrite],
    metadata_writes: &[CompiledWindowMetadataWrite],
) -> Result<()> {
    for write in typed_writes {
        let decoded = decode_bytes(
            mad_com
                .get(write.file_offset..)
                .with_context(|| format!("{} lies outside MAD.COM", write.write_id))?,
        )?;
        ensure!(
            decoded.instruction
                == Instruction::Mov {
                    dest: Operand::Reg16(write.register),
                    src: Operand::Imm16(write.output_value),
                }
                && decoded.byte_len == write.source.bytes().len()
                && decoded.prefixes.is_empty(),
            "{} failed typed V30 readback",
            write.write_id
        );
    }
    for write in metadata_writes {
        ensure!(
            mad_com.get(write.file_offset..write.file_offset + 2)
                == Some(write.output_value.to_le_bytes().as_slice()),
            "{} failed metadata readback",
            write.write_id
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "writes_tests.rs"]
mod writes_tests;
