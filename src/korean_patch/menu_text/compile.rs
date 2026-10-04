use anyhow::{Context, Result, ensure};

use super::model::CompiledMenuText;
use super::records::compile_records;
use super::typed_sources::{assemble_menu_sources, assemble_original_entry_trampoline};
use crate::game_data::{GaijiCatalog, GaijiReadinessCatalog, MenuProgramCatalog};
use crate::korean_patch::shared_text::{
    InstallerInterruptPolicy, assemble_embedded_gaiji_installer, compile_gaiji_bank,
};
use crate::translation_drafts::TranslationDraftSegment;

const COM_ORIGIN: usize = 0x100;

pub(super) fn compile_menu_text(
    menu_com: &[u8],
    gaiji_com: &[u8],
    gaiji: &GaijiCatalog,
    readiness: &GaijiReadinessCatalog,
    source: &MenuProgramCatalog,
    draft: &TranslationDraftSegment,
) -> Result<CompiledMenuText> {
    ensure!(
        menu_com.len() == source.file_size,
        "MENU source size changed before compilation"
    );
    let lines = draft
        .entries
        .iter()
        .flat_map(|entry| entry.korean_text.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let bank = compile_gaiji_bank(gaiji_com, gaiji, readiness, &lines)?;
    let mut entries = compile_records(menu_com, &source.text, draft, &bank)?;

    let source_file_size = menu_com.len();
    let installer_offset = source_file_size;
    let (placeholder_installer, placeholder_records) = assemble_embedded_gaiji_installer(
        gaiji,
        &bank,
        installer_offset,
        source.runtime.entry.resume_com_address,
        InstallerInterruptPolicy::Inherit,
        "MENU.COM",
    )?;
    let glyph_records_offset = installer_offset + placeholder_installer.bytes().len();
    let trampoline_offset = glyph_records_offset + placeholder_records.len();
    let trampoline = assemble_original_entry_trampoline(
        trampoline_offset,
        source.runtime.entry.original_call_target_com_address,
        source.runtime.entry.resume_com_address,
    )?;
    let trampoline_com_address = u16::try_from(trampoline_offset + COM_ORIGIN)
        .context("MENU entry trampoline address exceeds 16 bits")?;
    let (installer, glyph_records) = assemble_embedded_gaiji_installer(
        gaiji,
        &bank,
        installer_offset,
        trampoline_com_address,
        InstallerInterruptPolicy::Inherit,
        "MENU.COM",
    )?;
    ensure!(
        installer.bytes().len() == placeholder_installer.bytes().len()
            && glyph_records == placeholder_records,
        "MENU installer handoff changed its code or GAIJI record layout"
    );

    let text_offset = trampoline_offset + trampoline.bytes().len();
    let mut packed_text = Vec::new();
    for entry in &mut entries {
        entry.file_offset = text_offset + packed_text.len();
        entry.com_address = u16::try_from(entry.file_offset + COM_ORIGIN)
            .context("MENU relocated text address exceeds 16 bits")?;
        packed_text.extend_from_slice(&entry.bytes);
    }
    let output_file_size = text_offset
        .checked_add(packed_text.len())
        .context("MENU output size overflow")?;
    ensure!(
        output_file_size + COM_ORIGIN <= usize::from(u16::MAX) + 1,
        "MENU development COM image exceeds its 16-bit segment"
    );
    let typed_sources = assemble_menu_sources(
        menu_com,
        &source.runtime,
        installer_offset,
        installer.clone(),
        trampoline.clone(),
        &entries,
    )?;

    Ok(CompiledMenuText {
        bank,
        source_file_size,
        output_file_size,
        installer_offset,
        installer,
        glyph_records_offset,
        glyph_records,
        trampoline_offset,
        trampoline,
        text_offset,
        packed_text,
        entries,
        typed_sources,
    })
}
