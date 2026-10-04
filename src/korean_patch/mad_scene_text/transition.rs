use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use v30::{
    AssembledProgram, Assembler, CallTarget, CodeLocation, Condition, Instruction, JmpTarget,
    Operand, Register16, SegmentRegister,
};

use crate::game_data::{
    FixedGaijiTextRuntimeCatalog, MadSceneTransitionCall, MadSceneTransitionCatalog,
};

const COM_ORIGIN: usize = 0x100;
const GAIJI_FILE_NAME: &[u8] = b"GAIJI.COM\0";
pub(super) const GAIJI_RECORD_BYTE_SIZE: usize = 34;
pub(super) const WRAPPER_SOURCE_ID: &str = "mad-scene-gaiji-bank-wrapper";
pub(super) const STARTUP_ENTRY_SOURCE_ID: &str = "mad-startup-interface-bank-entry";
pub(super) const SELECTION_ENTRY_SOURCE_ID: &str = "mad-selection-fixed-bank-entry";

pub(super) struct CompiledSceneTransition {
    pub wrapper_offset: usize,
    pub wrapper_capacity: usize,
    pub wrapper: AssembledProgram,
    pub file_name_offset: usize,
    pub file_name: Vec<u8>,
    pub buffer_offset: usize,
    pub call_sources: BTreeMap<String, AssembledProgram>,
    pub startup_entry_source: AssembledProgram,
    pub selection_entry_source: AssembledProgram,
    pub startup_entry_com_address: u16,
    pub fixed_entry_com_address: u16,
    pub switch_bank_com_address: u16,
    pub switch_status_bank_com_address: u16,
    pub status_record_count: usize,
}

pub(super) struct SceneTransitionLayout {
    pub wrapper_offset: usize,
    pub wrapper_capacity: usize,
    pub buffer_offset: usize,
    pub buffer_capacity: usize,
    pub interface_bank_file_offset: usize,
    pub dialogue_bank_file_offset: usize,
    pub fixed_bank_file_offset: usize,
    pub gaiji_record_count: usize,
    pub status_record_count: usize,
}

struct BankWrapperAddresses {
    file_name: u16,
    buffer: u16,
    dialogue_call_displacement: i16,
    startup_call_displacement: i16,
    failure_exit_displacement: i16,
    selection_tail_displacement: i16,
}

pub(super) fn compile_scene_transition(
    catalog: &MadSceneTransitionCatalog,
    fixed_runtime: &FixedGaijiTextRuntimeCatalog,
    layout: SceneTransitionLayout,
) -> Result<CompiledSceneTransition> {
    ensure!(
        layout.buffer_capacity >= GAIJI_RECORD_BYTE_SIZE,
        "MAD scene transition buffer holds {}, needs {GAIJI_RECORD_BYTE_SIZE}",
        layout.buffer_capacity
    );
    let storage_tail_end = layout
        .buffer_offset
        .checked_add(layout.buffer_capacity)
        .context("MAD scene system-text tail exceeds MAD.COM")?;
    let origin = com_location(layout.wrapper_offset, "MAD scene bank wrapper")?;
    let placeholder = assemble_wrapper(
        origin,
        &layout,
        catalog,
        BankWrapperAddresses {
            file_name: 0,
            buffer: com_address(layout.buffer_offset, "MAD scene bank buffer")?,
            dialogue_call_displacement: 0,
            startup_call_displacement: 0,
            failure_exit_displacement: 0,
            selection_tail_displacement: 0,
        },
    )?;
    let file_name_offset = layout
        .buffer_offset
        .checked_add(GAIJI_RECORD_BYTE_SIZE)
        .context("MAD scene bank file name offset exceeds MAD.COM")?;
    let file_name_end = file_name_offset
        .checked_add(GAIJI_FILE_NAME.len())
        .context("MAD scene bank file name end exceeds MAD.COM")?;
    ensure!(
        file_name_end <= storage_tail_end,
        "MAD scene system-text tail cannot preserve both the record buffer and bank file name"
    );
    let file_name_address = com_address(file_name_offset, "MAD scene bank file name")?;
    let dialogue_call_address = placeholder
        .label_location("dialogue-call")
        .context("MAD scene wrapper lost its dialogue-call label")?
        .off;
    let failure_jump_address = placeholder
        .label_location("failure-jump")
        .context("MAD scene wrapper lost its failure-jump label")?
        .off;
    let startup_call_address = placeholder
        .label_location("startup-call")
        .context("MAD scene wrapper lost its startup-call label")?
        .off;
    let startup_entry_com_address = placeholder
        .label_location("startup-entry")
        .context("MAD scene wrapper lost its startup-bank entry label")?
        .off;
    let selection_tail_jump_address = placeholder
        .label_location("selection-tail-jump")
        .context("MAD scene wrapper lost its selection tail-jump label")?
        .off;
    let fixed_entry_com_address = placeholder
        .label_location("fixed-entry")
        .context("MAD scene wrapper lost its fixed-bank entry label")?
        .off;
    let switch_bank_com_address = placeholder
        .label_location("switch-bank")
        .context("MAD scene wrapper lost its switch-bank label")?
        .off;
    let switch_status_bank_com_address = placeholder
        .label_location("switch-status-bank")
        .context("missing status bank entry")?
        .off;
    let wrapper = assemble_wrapper(
        origin,
        &layout,
        catalog,
        BankWrapperAddresses {
            file_name: file_name_address,
            buffer: com_address(layout.buffer_offset, "MAD scene bank buffer")?,
            dialogue_call_displacement: relative_displacement(
                dialogue_call_address,
                catalog.dialogue_routine_com_address,
            ),
            startup_call_displacement: relative_displacement(
                startup_call_address,
                catalog.startup_call.target_com_address,
            ),
            failure_exit_displacement: relative_displacement(
                failure_jump_address,
                catalog.failure_exit_com_address,
            ),
            selection_tail_displacement: relative_displacement(
                selection_tail_jump_address,
                fixed_runtime.selection_entry_call.target_com_address,
            ),
        },
    )?;
    ensure!(
        wrapper.bytes().len() == placeholder.bytes().len(),
        "MAD scene wrapper address resolution changed its code size"
    );
    let used = wrapper.bytes().len();
    ensure!(
        used <= layout.wrapper_capacity,
        "MAD scene wrapper needs {used} bytes but the verified dialogue tail holds {}",
        layout.wrapper_capacity
    );

    let mut call_sources = BTreeMap::new();
    let wrapper_address = origin.off;
    for call in &catalog.calls {
        let source = assemble_call_redirect(call, wrapper_address)?;
        let id = call_source_id(call);
        ensure!(
            call_sources.insert(id.clone(), source).is_none(),
            "duplicate MAD scene transition source {id}"
        );
    }
    ensure!(
        call_sources.len() == catalog.call_site_count,
        "MAD scene transition source population changed"
    );
    let startup_entry_source = assemble_near_call_redirect(
        catalog.startup_call.file_offset,
        catalog.startup_call.byte_size,
        STARTUP_ENTRY_SOURCE_ID,
        startup_entry_com_address,
    )?;
    let selection_entry_source = assemble_near_call_redirect(
        fixed_runtime.selection_entry_call.file_offset,
        fixed_runtime.selection_entry_call.byte_size,
        SELECTION_ENTRY_SOURCE_ID,
        fixed_entry_com_address,
    )?;

    Ok(CompiledSceneTransition {
        wrapper_offset: layout.wrapper_offset,
        wrapper_capacity: layout.wrapper_capacity,
        wrapper,
        file_name_offset,
        file_name: GAIJI_FILE_NAME.to_vec(),
        buffer_offset: layout.buffer_offset,
        call_sources,
        startup_entry_source,
        selection_entry_source,
        startup_entry_com_address,
        fixed_entry_com_address,
        switch_bank_com_address,
        switch_status_bank_com_address,
        status_record_count: layout.status_record_count,
    })
}

pub(super) fn call_source_id(call: &MadSceneTransitionCall) -> String {
    format!("mad-scene-{}-banked-call", call.id)
}

fn assemble_wrapper(
    origin: CodeLocation,
    layout: &SceneTransitionLayout,
    catalog: &MadSceneTransitionCatalog,
    addresses: BankWrapperAddresses,
) -> Result<AssembledProgram> {
    let mut assembler = Assembler::new();
    assembler
        .emit(mov_reg_imm(
            Register16::SI,
            u16::try_from(layout.dialogue_bank_file_offset)?,
        ))
        .emit_call_near("switch-bank")
        .label("dialogue-call")
        .emit(Instruction::Call {
            target: CallTarget::Rel16(addresses.dialogue_call_displacement),
        })
        .emit(mov_reg_imm(
            Register16::SI,
            u16::try_from(layout.interface_bank_file_offset)?,
        ))
        .emit_call_near("switch-bank")
        .emit(Instruction::Ret { pop: 0 })
        .label("fixed-entry")
        .emit(mov_reg_imm(
            Register16::SI,
            u16::try_from(layout.fixed_bank_file_offset)?,
        ))
        .emit_call_near("switch-bank")
        .label("selection-tail-jump")
        .emit(Instruction::Jmp {
            target: JmpTarget::Rel16(addresses.selection_tail_displacement),
        })
        .label("startup-entry")
        .emit(mov_reg_imm(
            Register16::SI,
            u16::try_from(layout.interface_bank_file_offset)?,
        ))
        .emit_call_near("switch-bank")
        .label("startup-call")
        .emit(Instruction::Call {
            target: CallTarget::Rel16(addresses.startup_call_displacement),
        })
        .emit(Instruction::Ret { pop: 0 })
        .label("switch-bank")
        .emit(Instruction::Pushf)
        .emit(Instruction::Sti)
        .emit(Instruction::Pusha)
        .emit(Instruction::Push {
            src: Operand::Sreg(SegmentRegister::DS),
        })
        .emit(Instruction::Push {
            src: Operand::Sreg(SegmentRegister::CS),
        })
        .emit(Instruction::Pop {
            dest: Operand::Sreg(SegmentRegister::DS),
        })
        .emit(mov_reg_imm(
            Register16::CX,
            u16::try_from(layout.gaiji_record_count)?,
        ))
        .emit_jump_near("switch-records")
        .label("switch-status-bank")
        .emit(Instruction::Pushf)
        .emit(Instruction::Sti)
        .emit(Instruction::Pusha)
        .emit(Instruction::Push {
            src: Operand::Sreg(SegmentRegister::DS),
        })
        .emit(Instruction::Push {
            src: Operand::Sreg(SegmentRegister::CS),
        })
        .emit(Instruction::Pop {
            dest: Operand::Sreg(SegmentRegister::DS),
        })
        .emit(mov_reg_imm(
            Register16::CX,
            u16::try_from(layout.status_record_count)?,
        ))
        .label("switch-records")
        .emit(Instruction::Push {
            src: Operand::Reg16(Register16::CX),
        })
        .emit(mov_reg_imm(Register16::DX, addresses.file_name))
        .emit(mov_reg_imm(Register16::AX, 0x3d00))
        .emit(Instruction::Int { vector: 0x21 })
        .emit_branch(Condition::B, "failure")
        .emit(Instruction::Xchg {
            a: Operand::Reg16(Register16::AX),
            b: Operand::Reg16(Register16::BX),
        })
        .emit(Instruction::Xor {
            dest: Operand::Reg16(Register16::CX),
            src: Operand::Reg16(Register16::CX),
        })
        .emit(Instruction::Mov {
            dest: Operand::Reg16(Register16::DX),
            src: Operand::Reg16(Register16::SI),
        })
        .emit(mov_reg_imm(Register16::AX, 0x4200))
        .emit(Instruction::Int { vector: 0x21 })
        .emit_branch(Condition::B, "close-failure")
        .emit(Instruction::Pop {
            dest: Operand::Reg16(Register16::SI),
        })
        .emit(mov_reg_imm(Register16::DI, 0x7621))
        .label("record-loop")
        .emit(Instruction::Mov {
            dest: Operand::Reg8(v30::Register8::AH),
            src: Operand::Imm8(0x3f),
        })
        .emit(mov_reg_imm(
            Register16::CX,
            u16::try_from(GAIJI_RECORD_BYTE_SIZE)?,
        ))
        .emit(mov_reg_imm(Register16::DX, addresses.buffer))
        .emit(Instruction::Int { vector: 0x21 })
        .emit_branch(Condition::B, "close-failure")
        .emit(Instruction::Cmp {
            a: Operand::Reg16(Register16::AX),
            b: Operand::Imm16(u16::try_from(GAIJI_RECORD_BYTE_SIZE)?),
        })
        .emit_branch(Condition::Ne, "close-failure")
        .emit(Instruction::Pusha)
        .emit(Instruction::Mov {
            dest: Operand::Reg16(Register16::BX),
            src: Operand::Sreg(SegmentRegister::CS),
        })
        .emit(mov_reg_imm(Register16::CX, addresses.buffer))
        .emit(Instruction::Mov {
            dest: Operand::Reg16(Register16::DX),
            src: Operand::Reg16(Register16::DI),
        })
        .emit(Instruction::Mov {
            dest: Operand::Reg8(v30::Register8::AH),
            src: Operand::Imm8(0x1a),
        })
        .emit(Instruction::Int { vector: 0x18 })
        .emit(Instruction::Popa)
        .emit(Instruction::Cmp {
            a: Operand::Reg16(Register16::DI),
            b: Operand::Imm16(0x767e),
        })
        .emit_branch(Condition::Ne, "advance-code")
        .emit(mov_reg_imm(Register16::DI, 0x7720))
        .label("advance-code")
        .emit(Instruction::Inc {
            dest: Operand::Reg16(Register16::DI),
        })
        .emit(Instruction::Dec {
            dest: Operand::Reg16(Register16::SI),
        })
        .emit_branch(Condition::Ne, "record-loop")
        .emit(Instruction::Mov {
            dest: Operand::Reg8(v30::Register8::AH),
            src: Operand::Imm8(0x3e),
        })
        .emit(Instruction::Int { vector: 0x21 })
        .emit_branch(Condition::B, "failure")
        .emit(Instruction::Pop {
            dest: Operand::Sreg(SegmentRegister::DS),
        })
        .emit(Instruction::Popa)
        .emit(Instruction::Popf)
        .emit(Instruction::Ret { pop: 0 })
        .label("close-failure")
        .emit(Instruction::Mov {
            dest: Operand::Reg8(v30::Register8::AH),
            src: Operand::Imm8(0x3e),
        })
        .emit(Instruction::Int { vector: 0x21 })
        .label("failure")
        .label("failure-jump")
        .emit(Instruction::Jmp {
            target: JmpTarget::Rel16(addresses.failure_exit_displacement),
        });
    let assembled = assembler.assemble(origin)?;
    ensure!(
        assembled
            .placed_instructions()
            .iter()
            .any(|placed| matches!(placed.instruction, Instruction::Int { vector: 0x18 })),
        "MAD scene wrapper lost its typed PC-98 GAIJI BIOS call"
    );
    ensure!(
        catalog.call_site_count == 2,
        "MAD scene wrapper requires both verified dialogue entry paths"
    );
    ensure!(
        layout.fixed_bank_file_offset > layout.dialogue_bank_file_offset,
        "MAD fixed-text bank no longer follows the dialogue bank"
    );
    Ok(assembled)
}

fn assemble_call_redirect(
    call: &MadSceneTransitionCall,
    wrapper_address: u16,
) -> Result<AssembledProgram> {
    assemble_near_call_redirect(call.file_offset, call.byte_size, &call.id, wrapper_address)
}

fn assemble_near_call_redirect(
    file_offset: usize,
    byte_size: usize,
    id: &str,
    target_address: u16,
) -> Result<AssembledProgram> {
    let origin = com_location(file_offset, id)?;
    let mut assembler = Assembler::new();
    assembler.emit(Instruction::Call {
        target: CallTarget::Rel16(relative_displacement(origin.off, target_address)),
    });
    let assembled = assembler.assemble(origin)?;
    ensure!(
        assembled.bytes().len() == byte_size,
        "MAD scene transition {id} changed instruction width"
    );
    Ok(assembled)
}

fn mov_reg_imm(register: Register16, value: u16) -> Instruction {
    Instruction::Mov {
        dest: Operand::Reg16(register),
        src: Operand::Imm16(value),
    }
}

fn relative_displacement(instruction_address: u16, target_address: u16) -> i16 {
    target_address.wrapping_sub(instruction_address.wrapping_add(3)) as i16
}

fn com_address(file_offset: usize, role: &str) -> Result<u16> {
    u16::try_from(file_offset + COM_ORIGIN)
        .with_context(|| format!("{role} COM address exceeds 16 bits"))
}

fn com_location(file_offset: usize, role: &str) -> Result<CodeLocation> {
    Ok(CodeLocation {
        seg: 0,
        off: com_address(file_offset, role)?,
    })
}

#[cfg(test)]
#[path = "transition_tests.rs"]
mod transition_tests;
