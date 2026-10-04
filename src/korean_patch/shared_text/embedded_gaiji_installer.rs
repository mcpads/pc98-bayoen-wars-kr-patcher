use anyhow::{Context, Result, ensure};
use v30::{
    AssembledProgram, Assembler, CodeLocation, Instruction, JmpTarget, Operand, Register8,
    Register16, SegmentRegister,
};

use super::{CompiledGaijiBank, collect_embedded_gaiji_records};
use crate::game_data::GaijiCatalog;

const COM_ORIGIN: usize = 0x100;
const BIOS_FUNCTION: u8 = 0x1a;
const BIOS_INTERRUPT: u8 = 0x18;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InstallerInterruptPolicy {
    Inherit,
    EnableAndRestore,
}

pub(crate) fn assemble_embedded_gaiji_installer(
    source_gaiji: &GaijiCatalog,
    bank: &CompiledGaijiBank,
    installer_offset: usize,
    handoff_com_address: u16,
    interrupt_policy: InstallerInterruptPolicy,
    owner: &str,
) -> Result<(AssembledProgram, Vec<u8>)> {
    assemble_gaiji_installer_for_address_bias(
        source_gaiji,
        bank,
        installer_offset,
        COM_ORIGIN,
        handoff_com_address,
        interrupt_policy,
        owner,
    )
}

pub(crate) fn assemble_segment_gaiji_installer(
    source_gaiji: &GaijiCatalog,
    bank: &CompiledGaijiBank,
    installer_offset: usize,
    handoff_segment_offset: u16,
    interrupt_policy: InstallerInterruptPolicy,
    owner: &str,
) -> Result<(AssembledProgram, Vec<u8>)> {
    assemble_gaiji_installer_for_address_bias(
        source_gaiji,
        bank,
        installer_offset,
        0,
        handoff_segment_offset,
        interrupt_policy,
        owner,
    )
}

fn assemble_gaiji_installer_for_address_bias(
    source_gaiji: &GaijiCatalog,
    bank: &CompiledGaijiBank,
    installer_offset: usize,
    address_bias: usize,
    handoff_segment_address: u16,
    interrupt_policy: InstallerInterruptPolicy,
    owner: &str,
) -> Result<(AssembledProgram, Vec<u8>)> {
    let embedded = collect_embedded_gaiji_records(source_gaiji, bank)?;
    let placeholder_addresses = vec![0_u16; bank.glyphs.len()];
    let installer_origin = segment_location(installer_offset, address_bias, owner)?;
    let placeholder = assemble_installer(
        installer_origin,
        &placeholder_addresses,
        &embedded.character_codes,
        handoff_segment_address,
        interrupt_policy,
        owner,
    )?;
    let records_offset = installer_offset + placeholder.bytes().len();
    let record_addresses = embedded.segment_addresses(records_offset, address_bias)?;
    let installer = assemble_installer(
        installer_origin,
        &record_addresses,
        &embedded.character_codes,
        handoff_segment_address,
        interrupt_policy,
        owner,
    )?;
    ensure!(
        installer.bytes().len() == placeholder.bytes().len(),
        "{owner} installer address placement changed its code size"
    );
    Ok((installer, embedded.bytes))
}

fn assemble_installer(
    origin: CodeLocation,
    record_addresses: &[u16],
    character_codes: &[u16],
    handoff_com_address: u16,
    interrupt_policy: InstallerInterruptPolicy,
    owner: &str,
) -> Result<AssembledProgram> {
    ensure!(
        !record_addresses.is_empty() && record_addresses.len() == character_codes.len(),
        "{owner} installer has a mismatched or empty GAIJI population"
    );
    let mut assembler = installer_body(record_addresses, character_codes, interrupt_policy);
    assembler.emit(Instruction::Jmp {
        target: JmpTarget::Rel16(0),
    });
    let placeholder = assembler.assemble(origin)?;
    let next = i32::from(origin.off) + i32::try_from(placeholder.bytes().len())?;
    let displacement = i16::try_from(i32::from(handoff_com_address) - next)
        .with_context(|| format!("{owner} installer handoff is outside a near jump"))?;

    let mut assembler = installer_body(record_addresses, character_codes, interrupt_policy);
    assembler.emit(Instruction::Jmp {
        target: JmpTarget::Rel16(displacement),
    });
    Ok(assembler.assemble(origin)?)
}

fn installer_body(
    record_addresses: &[u16],
    character_codes: &[u16],
    interrupt_policy: InstallerInterruptPolicy,
) -> Assembler {
    let mut assembler = Assembler::new();
    if interrupt_policy == InstallerInterruptPolicy::EnableAndRestore {
        assembler.emit(Instruction::Pushf).emit(Instruction::Sti);
    }
    for register in [
        Register16::AX,
        Register16::BX,
        Register16::CX,
        Register16::DX,
    ] {
        assembler.emit(Instruction::Push {
            src: Operand::Reg16(register),
        });
    }
    for (&record_address, &character_code) in record_addresses.iter().zip(character_codes.iter()) {
        assembler
            .emit(Instruction::Mov {
                dest: Operand::Reg16(Register16::BX),
                src: Operand::Sreg(SegmentRegister::CS),
            })
            .emit(Instruction::Mov {
                dest: Operand::Reg16(Register16::CX),
                src: Operand::Imm16(record_address),
            })
            .emit(Instruction::Mov {
                dest: Operand::Reg16(Register16::DX),
                src: Operand::Imm16(character_code),
            })
            .emit(Instruction::Mov {
                dest: Operand::Reg8(Register8::AH),
                src: Operand::Imm8(BIOS_FUNCTION),
            })
            .emit(Instruction::Int {
                vector: BIOS_INTERRUPT,
            });
    }
    for register in [
        Register16::DX,
        Register16::CX,
        Register16::BX,
        Register16::AX,
    ] {
        assembler.emit(Instruction::Pop {
            dest: Operand::Reg16(register),
        });
    }
    if interrupt_policy == InstallerInterruptPolicy::EnableAndRestore {
        assembler.emit(Instruction::Popf);
    }
    assembler
}

fn segment_location(file_offset: usize, address_bias: usize, owner: &str) -> Result<CodeLocation> {
    Ok(CodeLocation {
        seg: 0,
        off: u16::try_from(file_offset + address_bias)
            .with_context(|| format!("{owner} installer segment address exceeds 16 bits"))?,
    })
}

#[cfg(test)]
#[path = "embedded_gaiji_installer_tests.rs"]
mod embedded_gaiji_installer_tests;
