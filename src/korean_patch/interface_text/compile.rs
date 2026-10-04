use anyhow::{Context, Result, ensure};

use super::model::CompiledInterfaceText;
use super::records::compile_record;
use crate::game_data::{
    GaijiCatalog, InterfaceTextCatalog, MadProgramCatalog, SharedAlertWindowCatalog,
    SpringCaptureWindowCatalog, SpringRecoveryWindowCatalog,
};
use crate::korean_patch::shared_text::{CompiledGaijiBank, compile_gaiji_bank};
use crate::translation_drafts::TranslationDraftSegment;

const COM_ORIGIN: usize = 0x100;

pub(in crate::korean_patch) fn compile_interface_text(
    gaiji_com: &[u8],
    gaiji: &GaijiCatalog,
    mad: &MadProgramCatalog,
    draft: &TranslationDraftSegment,
) -> Result<CompiledInterfaceText> {
    let lines = draft
        .entries
        .iter()
        .flat_map(|entry| entry.korean_text.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let bank = compile_gaiji_bank(gaiji_com, gaiji, &mad.gaiji_readiness, &lines)?;
    compile_interface_text_with_bank(
        &mad.interface_text,
        &mad.shared_alert_window,
        &mad.stage_result_window,
        &mad.spring_capture_window,
        &mad.spring_recovery_window,
        draft,
        bank,
    )
}

pub(in crate::korean_patch) fn compile_interface_text_with_bank(
    source: &InterfaceTextCatalog,
    shared_alert_window: &SharedAlertWindowCatalog,
    stage_result_window: &SharedAlertWindowCatalog,
    spring_capture_window: &SpringCaptureWindowCatalog,
    spring_recovery_window: &SpringRecoveryWindowCatalog,
    draft: &TranslationDraftSegment,
    bank: CompiledGaijiBank,
) -> Result<CompiledInterfaceText> {
    let mut entries = source
        .entries
        .iter()
        .zip(&draft.entries)
        .map(|(source, draft)| compile_record(source, draft, &bank))
        .collect::<Result<Vec<_>>>()?;

    let storage_capacity = source
        .text_end_offset
        .checked_sub(source.text_start_offset)
        .context("interface text storage boundaries are reversed")?;
    let packed_storage_bytes = entries.iter().map(|entry| entry.bytes.len()).sum::<usize>();
    ensure!(
        packed_storage_bytes <= storage_capacity,
        "interface text needs {packed_storage_bytes} bytes but the verified region holds {storage_capacity}"
    );
    let mut text_region_replacement = vec![0; storage_capacity];
    let mut cursor = source.text_start_offset;
    for entry in &mut entries {
        let relative = cursor - source.text_start_offset;
        let end = relative + entry.bytes.len();
        text_region_replacement[relative..end].copy_from_slice(&entry.bytes);
        entry.file_offset = cursor;
        entry.com_address = u16::try_from(cursor + COM_ORIGIN)
            .context("relocated interface text address exceeds 16 bits")?;
        cursor += entry.bytes.len();
    }
    ensure!(
        cursor == source.text_start_offset + packed_storage_bytes,
        "interface text packer cursor changed"
    );

    let stage_result_window = super::super::shared_alert_window::compile_centered_text_window(
        stage_result_window,
        &entries,
        "stage-result",
        2,
    )?;
    let shared_alert_window = super::super::shared_alert_window::compile_shared_alert_window(
        shared_alert_window,
        &entries,
    )?;
    let spring_capture_window = super::super::spring_capture_window::compile_spring_capture_window(
        spring_capture_window,
        &entries,
    )?;
    let spring_recovery_window =
        super::super::spring_recovery_window::compile_spring_recovery_window(
            spring_recovery_window,
            &entries,
        )?;
    Ok(CompiledInterfaceText {
        bank,
        text_region_start: source.text_start_offset,
        text_region_end: source.text_end_offset,
        packed_storage_bytes,
        populated_storage_bytes: packed_storage_bytes,
        text_region_replacement,
        entries,
        shared_alert_window,
        stage_result_window,
        spring_capture_window,
        spring_recovery_window,
    })
}
