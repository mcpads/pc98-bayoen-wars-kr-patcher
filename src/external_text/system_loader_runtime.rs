use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use v30::{
    EffectiveAddressBase, EffectiveAddressDisplacement, Instruction, JmpTarget, Operand,
    OperandSize, Register16, SegmentRegister, decode_bytes,
};

use super::ExternalProgramTextCatalog;

const ENTRY_JUMP_OFFSET: usize = 0x0000;
const INITIAL_ENTRY_ADDRESS: u16 = 0x0100;
const RESIDENT_SIZE_LOAD_OFFSET: usize = 0x0100;
const RESIDENT_COPY_COUNT_LOAD_OFFSET: usize = 0x0120;
const ORIGINAL_RESIDENT_BYTE_COUNT: u16 = 0x09a0;
const RELOCATED_ENTRY_ADDRESS: u16 = 0x012c;
const POST_STACK_HOOK_OFFSET: usize = 0x0150;
const POST_STACK_RESUME_ADDRESS: u16 = 0x0155;
const TAIL_DECLARED_COPY_BYTE_COUNT: u16 = 0x3a2d;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct SystemLoaderRuntimeCatalog {
    pub source_file_size: usize,
    pub entry_jump_offset: usize,
    pub initial_entry_address: u16,
    pub resident_size_load_offset: usize,
    pub resident_copy_count_load_offset: usize,
    pub resident_byte_count: u16,
    pub relocated_entry_address: u16,
    pub post_stack_hook_offset: usize,
    pub post_stack_hook_byte_count: usize,
    pub post_stack_resume_address: u16,
    pub tail_offset: usize,
    pub tail_byte_count: usize,
    pub tail_declared_copy_byte_count: u16,
    pub tail_entry_target: u16,
    pub references: Vec<SystemLoaderTextReference>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct SystemLoaderTextReference {
    pub entry_id: String,
    pub role: String,
    pub instruction_offset: usize,
    pub storage_offset: usize,
    pub original_runtime_address: u16,
}

pub(crate) fn catalog_system_loader_runtime(
    bytes: &[u8],
    text: &ExternalProgramTextCatalog,
) -> Result<SystemLoaderRuntimeCatalog> {
    let entry = decode_at(bytes, ENTRY_JUMP_OFFSET, "initial entry jump")?;
    let displacement = match entry.instruction {
        Instruction::Jmp {
            target: JmpTarget::Rel16(displacement),
        } if entry.byte_len == 3 && entry.prefixes.is_empty() => displacement,
        _ => anyhow::bail!("MEGDOS.SYS initial entry is not a typed near jump"),
    };
    ensure!(
        i32::try_from(entry.byte_len)? + i32::from(displacement)
            == i32::from(INITIAL_ENTRY_ADDRESS),
        "MEGDOS.SYS initial entry target changed"
    );

    let resident_byte_count = require_mov_immediate(
        bytes,
        RESIDENT_SIZE_LOAD_OFFSET,
        Register16::AX,
        "resident-size load",
    )?;
    let copy_byte_count = require_mov_immediate(
        bytes,
        RESIDENT_COPY_COUNT_LOAD_OFFSET,
        Register16::CX,
        "resident-copy count",
    )?;
    ensure!(
        resident_byte_count == ORIGINAL_RESIDENT_BYTE_COUNT
            && copy_byte_count == resident_byte_count
            && resident_byte_count % 16 == 0,
        "MEGDOS.SYS resident size or copy count changed"
    );
    require_bytes(
        bytes,
        RESIDENT_SIZE_LOAD_OFFSET + 3,
        &[0xb1, 0x04, 0xd3, 0xe8],
        "resident paragraph conversion",
    )?;
    require_bytes(
        bytes,
        RESIDENT_COPY_COUNT_LOAD_OFFSET + 3,
        &[0xfc, 0xf3, 0xa4],
        "forward resident byte copy",
    )?;
    require_bytes(
        bytes,
        0x0126,
        &[0x06, 0xb8, 0x2c, 0x01, 0x50, 0xcb],
        "relocated far handoff",
    )?;
    require_bytes(
        bytes,
        0x012c,
        &[
            0x8c, 0xd8, 0x2e, 0x03, 0x06, 0xe2, 0x07, 0x8e, 0xd8, 0x33, 0xf6, 0x2e, 0x8e, 0x06,
            0x03, 0x00, 0x8b, 0xfe, 0x2e, 0x8c, 0x06, 0xde, 0x07, 0x8b, 0x0e, 0x03, 0x00, 0xf3,
            0xa4,
        ],
        "relocated tail handoff",
    )?;
    require_bytes(
        bytes,
        0x0149,
        &[0xfa, 0x0e, 0x17, 0xbc, 0x00, 0x01, 0xfb],
        "relocated stack initialization",
    )?;
    require_post_stack_load(bytes)?;

    let tail_offset = usize::from(resident_byte_count);
    let tail_byte_count = bytes
        .len()
        .checked_sub(tail_offset)
        .context("MEGDOS.SYS resident block exceeds the file")?;
    ensure!(
        read_word(bytes, tail_offset + 3, "tail copy size")? == TAIL_DECLARED_COPY_BYTE_COUNT,
        "MEGDOS.SYS tail copy size changed"
    );
    let tail_entry = decode_at(bytes, tail_offset, "tail entry jump")?;
    let tail_entry_displacement = match tail_entry.instruction {
        Instruction::Jmp {
            target: JmpTarget::Rel16(displacement),
        } if tail_entry.byte_len == 3 && tail_entry.prefixes.is_empty() => displacement,
        _ => anyhow::bail!("MEGDOS.SYS tail entry is not a typed near jump"),
    };
    let tail_entry_target =
        i32::try_from(tail_offset + tail_entry.byte_len)? + i32::from(tail_entry_displacement);
    ensure!(
        tail_entry_target >= i32::try_from(tail_offset)?
            && tail_entry_target < i32::try_from(bytes.len())?,
        "MEGDOS.SYS tail entry target lies outside the source file"
    );

    let references = catalog_text_references(bytes, text, tail_offset)?;
    Ok(SystemLoaderRuntimeCatalog {
        source_file_size: bytes.len(),
        entry_jump_offset: ENTRY_JUMP_OFFSET,
        initial_entry_address: INITIAL_ENTRY_ADDRESS,
        resident_size_load_offset: RESIDENT_SIZE_LOAD_OFFSET,
        resident_copy_count_load_offset: RESIDENT_COPY_COUNT_LOAD_OFFSET,
        resident_byte_count,
        relocated_entry_address: RELOCATED_ENTRY_ADDRESS,
        post_stack_hook_offset: POST_STACK_HOOK_OFFSET,
        post_stack_hook_byte_count: 5,
        post_stack_resume_address: POST_STACK_RESUME_ADDRESS,
        tail_offset,
        tail_byte_count,
        tail_declared_copy_byte_count: TAIL_DECLARED_COPY_BYTE_COUNT,
        tail_entry_target: u16::try_from(tail_entry_target)
            .context("MEGDOS.SYS tail entry target exceeds 16 bits")?,
        references,
    })
}

fn catalog_text_references(
    bytes: &[u8],
    text: &ExternalProgramTextCatalog,
    resident_end: usize,
) -> Result<Vec<SystemLoaderTextReference>> {
    let mut references = Vec::new();
    for entry in &text.entries {
        let target = u16::try_from(entry.runtime_address)
            .context("MEGDOS.SYS text address exceeds 16 bits")?;
        for reference in &entry.references {
            ensure!(
                reference.table_index.is_none(),
                "MEGDOS.SYS text unexpectedly uses a pointer table"
            );
            ensure!(
                require_mov_immediate(
                    bytes,
                    reference.consumer_offset,
                    Register16::DX,
                    &reference.role,
                )? == target,
                "MEGDOS.SYS {} target changed",
                reference.role
            );
            references.push(SystemLoaderTextReference {
                entry_id: entry.id.clone(),
                role: reference.role.clone(),
                instruction_offset: reference.consumer_offset,
                storage_offset: reference.consumer_offset + 1,
                original_runtime_address: target,
            });
        }
    }
    references.sort_by_key(|reference| reference.storage_offset);
    ensure!(
        references.len() == 8,
        "MEGDOS.SYS text reference population changed"
    );

    let targets = text
        .entries
        .iter()
        .map(|entry| Ok((u16::try_from(entry.runtime_address)?, entry.id.as_str())))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let occurrences = scan_target_occurrences(&bytes[..resident_end], &targets);
    let reference_offsets = references
        .iter()
        .map(|reference| reference.storage_offset)
        .collect::<BTreeSet<_>>();
    ensure!(
        occurrences
            .iter()
            .map(|(offset, _, _)| *offset)
            .collect::<BTreeSet<_>>()
            == reference_offsets,
        "MEGDOS.SYS has an unclassified target-like text address"
    );
    Ok(references)
}

fn require_post_stack_load(bytes: &[u8]) -> Result<()> {
    let decoded = decode_at(bytes, POST_STACK_HOOK_OFFSET, "post-stack parameter load")?;
    let source = match decoded.instruction {
        Instruction::Lds {
            dest: Register16::SI,
            src,
        } => src,
        _ => anyhow::bail!("MEGDOS.SYS post-stack hook is not the verified typed LDS"),
    };
    ensure!(
        decoded.byte_len == 5
            && source.segment() == Some(SegmentRegister::SS)
            && source.base() == EffectiveAddressBase::Direct
            && source.displacement() == EffectiveAddressDisplacement::Absolute(0x0005)
            && source.size() == OperandSize::Word,
        "MEGDOS.SYS post-stack LDS encoding changed"
    );
    Ok(())
}

fn require_mov_immediate(
    bytes: &[u8],
    offset: usize,
    register: Register16,
    role: &str,
) -> Result<u16> {
    let decoded = decode_at(bytes, offset, role)?;
    let immediate = match decoded.instruction {
        Instruction::Mov {
            dest: Operand::Reg16(actual),
            src: Operand::Imm16(immediate),
        } if actual == register && decoded.byte_len == 3 && decoded.prefixes.is_empty() => {
            immediate
        }
        _ => anyhow::bail!(
            "MEGDOS.SYS {role} is not a typed MOV {}, imm16",
            register.name()
        ),
    };
    Ok(immediate)
}

fn decode_at(bytes: &[u8], offset: usize, role: &str) -> Result<v30::DecodedInstruction> {
    Ok(decode_bytes(bytes.get(offset..).with_context(|| {
        format!("MEGDOS.SYS {role} lies outside the file")
    })?)?)
}

fn require_bytes(bytes: &[u8], offset: usize, expected: &[u8], role: &str) -> Result<()> {
    ensure!(
        bytes.get(offset..offset + expected.len()) == Some(expected),
        "MEGDOS.SYS {role} changed"
    );
    Ok(())
}

fn read_word(bytes: &[u8], offset: usize, role: &str) -> Result<u16> {
    let raw: [u8; 2] = bytes
        .get(offset..offset + 2)
        .with_context(|| format!("MEGDOS.SYS {role} lies outside the file"))?
        .try_into()
        .expect("a two-byte range converts to an array");
    Ok(u16::from_le_bytes(raw))
}

fn scan_target_occurrences<'a>(
    bytes: &[u8],
    targets: &BTreeMap<u16, &'a str>,
) -> Vec<(usize, u16, &'a str)> {
    bytes
        .windows(2)
        .enumerate()
        .filter_map(|(offset, pair)| {
            let word = u16::from_le_bytes([pair[0], pair[1]]);
            targets.get(&word).map(|entry_id| (offset, word, *entry_id))
        })
        .collect()
}

#[cfg(test)]
#[path = "system_loader_runtime_tests.rs"]
mod system_loader_runtime_tests;
