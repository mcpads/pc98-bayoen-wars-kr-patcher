use anyhow::{Result, ensure};

use super::super::interface_text::{CompiledInterfaceText, compile_interface_text_with_bank};
use super::super::mad_system_text::{CompiledMadSystemText, compile_mad_system_text_with_bank};
use super::super::shared_text::{CompiledGaijiBank, compile_gaiji_bank};
use crate::game_data::{GaijiCatalog, MadProgramCatalog};
use crate::translation_drafts::TranslationDraftSegment;

const VERIFIED_SHARED_GAIJI_CAPACITY: usize = 170;

pub(in crate::korean_patch) struct CompiledMadSystemInterfaceText {
    pub bank: CompiledGaijiBank,
    pub system: CompiledMadSystemText,
    pub interface: CompiledInterfaceText,
}

pub(in crate::korean_patch) fn compile_mad_system_interface_text(
    gaiji_com: &[u8],
    mad: &MadProgramCatalog,
    gaiji: &GaijiCatalog,
    system_segment: &TranslationDraftSegment,
    interface_segment: &TranslationDraftSegment,
) -> Result<CompiledMadSystemInterfaceText> {
    let lines = interface_segment
        .entries
        .iter()
        .chain(&system_segment.entries)
        .flat_map(|entry| entry.korean_text.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let bank = compile_gaiji_bank(gaiji_com, gaiji, &mad.gaiji_readiness, &lines)?;
    ensure!(
        bank.available_slot_count == VERIFIED_SHARED_GAIJI_CAPACITY
            && bank.glyphs.len() <= VERIFIED_SHARED_GAIJI_CAPACITY,
        "MAD system and interface text exceeds or changed the verified shared bank"
    );
    let interface = compile_interface_text_with_bank(
        &mad.interface_text,
        &mad.shared_alert_window,
        &mad.stage_result_window,
        &mad.spring_capture_window,
        &mad.spring_recovery_window,
        interface_segment,
        bank.clone(),
    )?;
    let system = compile_mad_system_text_with_bank(&mad.system_text, system_segment, bank.clone())?;
    Ok(CompiledMadSystemInterfaceText {
        bank,
        system,
        interface,
    })
}
