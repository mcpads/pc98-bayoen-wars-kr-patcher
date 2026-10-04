use anyhow::{Context, Result, ensure};
use v30::{
    EffectiveAddressBase, EffectiveAddressDisplacement, Instruction, Operand, OperandSize,
    Register16, decode_bytes,
};

use super::MenuRuntimeInsertReference;
use crate::game_data::menu::MenuTextEntry;

const DYNAMIC_DRIVE_ENTRY_ID: &str = "menu-text-031";
const RUNTIME_INSERT_SPECS: [RuntimeInsertSpec; 2] = [
    RuntimeInsertSpec {
        id: "menu-floppy-position-field",
        role: "reset two-byte \\P position field before rendering",
        entry_byte_offset: 2,
        byte_capacity: 2,
        instruction_offset: 0x0e7a,
        expected_source: [0x00, 0x00],
    },
    RuntimeInsertSpec {
        id: "menu-floppy-drive-label-field",
        role: "write the two-byte runtime drive label into the ?? field",
        entry_byte_offset: 16,
        byte_capacity: 2,
        instruction_offset: 0x0e71,
        expected_source: *b"??",
    },
];

#[derive(Clone, Copy)]
struct RuntimeInsertSpec {
    id: &'static str,
    role: &'static str,
    entry_byte_offset: usize,
    byte_capacity: usize,
    instruction_offset: usize,
    expected_source: [u8; 2],
}

pub(super) fn catalog_runtime_insert_references(
    program: &[u8],
    entries: &[MenuTextEntry],
) -> Result<Vec<MenuRuntimeInsertReference>> {
    let entry = entries
        .iter()
        .find(|entry| entry.id == DYNAMIC_DRIVE_ENTRY_ID)
        .context("MENU dynamic drive-label entry is missing")?;
    let mut references = Vec::new();
    for spec in RUNTIME_INSERT_SPECS {
        ensure!(
            entry.byte_size >= spec.entry_byte_offset + spec.byte_capacity,
            "{} runtime field lies outside its source record",
            spec.id
        );
        let target_com_address = u16::try_from(entry.com_address + spec.entry_byte_offset)
            .context("MENU runtime field address exceeds 16 bits")?;
        let source_offset = entry.file_offset + spec.entry_byte_offset;
        ensure!(
            program.get(source_offset..source_offset + spec.byte_capacity)
                == Some(spec.expected_source.as_slice()),
            "{} source field changed",
            spec.id
        );
        require_direct_ax_store(
            program,
            spec.instruction_offset,
            target_com_address,
            spec.id,
        )?;
        references.push(MenuRuntimeInsertReference {
            id: spec.id.to_owned(),
            role: spec.role.to_owned(),
            target_entry_id: entry.id.clone(),
            entry_byte_offset: spec.entry_byte_offset,
            byte_capacity: spec.byte_capacity,
            instruction_offset: spec.instruction_offset,
            storage_offset: spec.instruction_offset + 1,
            target_com_address,
        });
    }
    references.sort_by_key(|reference| reference.storage_offset);
    Ok(references)
}

fn require_direct_ax_store(
    program: &[u8],
    instruction_offset: usize,
    target_com_address: u16,
    role: &str,
) -> Result<()> {
    let decoded = decode_bytes(
        program
            .get(instruction_offset..)
            .with_context(|| format!("MENU {role} instruction lies outside the file"))?,
    )?;
    let direct = match decoded.instruction {
        Instruction::Mov {
            dest: Operand::Mem(memory),
            src: Operand::Reg16(Register16::AX),
        } => memory,
        _ => anyhow::bail!("MENU {role} is not a typed V30 direct AX store"),
    };
    ensure!(
        decoded.byte_len == 3
            && decoded.prefixes.is_empty()
            && direct.base() == EffectiveAddressBase::Direct
            && direct.displacement() == EffectiveAddressDisplacement::Absolute(target_com_address)
            && direct.size() == OperandSize::Word,
        "MENU {role} direct AX-store encoding changed"
    );
    Ok(())
}
