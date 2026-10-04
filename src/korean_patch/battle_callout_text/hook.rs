use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use v30::{
    AssembledProgram, Assembler, CallTarget, CodeLocation, Instruction, Operand, Register16,
};

use super::model::CompiledBattleCalloutHook;
use crate::game_data::{BattleCalloutCall, BattleCalloutCatalog};

const COM_ORIGIN: usize = 0x100;
pub(in crate::korean_patch) const BATTLE_WRAPPER_SOURCE_ID: &str =
    "mad-battle-callout-bank-wrapper";

pub(in crate::korean_patch) struct BattleCalloutHookLayout {
    pub wrapper_offset: usize,
    pub wrapper_capacity: usize,
    pub switch_bank_com_address: u16,
    pub interface_bank_file_offset: usize,
    pub battle_bank_file_offset: usize,
}

pub(in crate::korean_patch) fn compile_battle_callout_hook(
    source: &BattleCalloutCatalog,
    layout: BattleCalloutHookLayout,
) -> Result<CompiledBattleCalloutHook> {
    let origin = CodeLocation {
        seg: 0,
        off: u16::try_from(layout.wrapper_offset + COM_ORIGIN)
            .context("battle callout wrapper address exceeds 16 bits")?,
    };
    let wrapper = assemble_banked_text_wrapper(
        origin,
        source.renderer_com_address,
        layout.switch_bank_com_address,
        layout.battle_bank_file_offset,
        layout.interface_bank_file_offset,
    )?;
    ensure!(
        wrapper.bytes().len() <= layout.wrapper_capacity,
        "battle callout bank wrapper needs {} bytes but its verified text tail holds {}",
        wrapper.bytes().len(),
        layout.wrapper_capacity
    );
    let mut call_sources = BTreeMap::new();
    for call in &source.calls {
        let id = call_source_id(call);
        let assembled = assemble_call_redirect(call, origin.off)?;
        ensure!(
            call_sources.insert(id.clone(), assembled).is_none(),
            "duplicate battle callout source {id}"
        );
    }
    ensure!(
        call_sources.len() == source.call_site_count,
        "battle callout hook source population changed"
    );
    Ok(CompiledBattleCalloutHook {
        wrapper_offset: layout.wrapper_offset,
        wrapper_capacity: layout.wrapper_capacity,
        wrapper,
        call_sources,
    })
}

pub(in crate::korean_patch) fn call_source_id(call: &BattleCalloutCall) -> String {
    format!("{}-banked", call.id)
}

pub(in crate::korean_patch) fn assemble_banked_text_wrapper(
    origin: CodeLocation,
    renderer_com_address: u16,
    switch_bank_com_address: u16,
    text_bank_file_offset: usize,
    interface_bank_file_offset: usize,
) -> Result<AssembledProgram> {
    let mut assembler = Assembler::new();
    assembler
        .emit(mov_si(u16::try_from(text_bank_file_offset)?))
        .emit(Instruction::Call {
            target: CallTarget::Rel16(relative_displacement(
                origin.off.wrapping_add(3),
                switch_bank_com_address,
            )),
        })
        .emit(Instruction::Call {
            target: CallTarget::Rel16(relative_displacement(
                origin.off.wrapping_add(6),
                renderer_com_address,
            )),
        })
        .emit(mov_si(u16::try_from(interface_bank_file_offset)?))
        .emit(Instruction::Call {
            target: CallTarget::Rel16(relative_displacement(
                origin.off.wrapping_add(12),
                switch_bank_com_address,
            )),
        })
        .emit(Instruction::Ret { pop: 0 });
    let assembled = assembler.assemble(origin)?;
    ensure!(
        assembled.bytes().len() == 16,
        "battle callout bank wrapper instruction width changed"
    );
    Ok(assembled)
}

fn assemble_call_redirect(
    call: &BattleCalloutCall,
    wrapper_address: u16,
) -> Result<AssembledProgram> {
    let origin = CodeLocation {
        seg: 0,
        off: call.com_address,
    };
    let mut assembler = Assembler::new();
    assembler.emit(Instruction::Call {
        target: CallTarget::Rel16(relative_displacement(origin.off, wrapper_address)),
    });
    let assembled = assembler.assemble(origin)?;
    ensure!(
        assembled.bytes().len() == call.byte_size,
        "battle callout redirect {} changed instruction width",
        call.id
    );
    Ok(assembled)
}

fn mov_si(value: u16) -> Instruction {
    Instruction::Mov {
        dest: Operand::Reg16(Register16::SI),
        src: Operand::Imm16(value),
    }
}

fn relative_displacement(instruction_address: u16, target_address: u16) -> i16 {
    target_address.wrapping_sub(instruction_address.wrapping_add(3)) as i16
}

#[cfg(test)]
#[path = "hook_tests.rs"]
mod hook_tests;
