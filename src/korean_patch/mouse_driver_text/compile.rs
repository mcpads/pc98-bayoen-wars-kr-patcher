use anyhow::{Context, Result, ensure};

use super::model::{
    CompiledMouseDriverText, CompiledMouseDriverTextEntry, CompiledMousePointerTable,
};
use super::records::compile_record;
use super::typed_sources::{assemble_mouse_driver_installer, assemble_mouse_driver_sources};
use crate::external_text::{ExternalProgramTextCatalog, MouseDriverRuntimeCatalog};
use crate::game_data::{GaijiCatalog, GaijiReadinessCatalog};
use crate::korean_patch::shared_text::compile_gaiji_bank;
use crate::translation_drafts::TranslationDraftSegment;

const COM_ORIGIN: usize = 0x100;
const FREQUENCY_ROLE: &str = "frequency_label_table";
const COMMAND_MODE_ROLE: &str = "command_mode_label_table";

pub(super) fn compile_mouse_driver_text(
    nmouse_com: &[u8],
    gaiji_com: &[u8],
    gaiji: &GaijiCatalog,
    readiness: &GaijiReadinessCatalog,
    source: &ExternalProgramTextCatalog,
    runtime: MouseDriverRuntimeCatalog,
    draft: &TranslationDraftSegment,
) -> Result<CompiledMouseDriverText> {
    let draft_lines = draft
        .entries
        .iter()
        .flat_map(|entry| entry.korean_text.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let bank = compile_gaiji_bank(gaiji_com, gaiji, readiness, &draft_lines)?;
    let mut entries = source
        .entries
        .iter()
        .zip(&draft.entries)
        .map(|(source, draft)| compile_record(source, draft, &bank))
        .collect::<Result<Vec<_>>>()?;

    let source_file_size = nmouse_com.len();
    let resident_end = usize::from(runtime.resident_paragraph_count) * 16;
    ensure!(
        source_file_size + COM_ORIGIN > resident_end,
        "mouse-driver source has no non-resident file tail"
    );
    let installer_offset = source_file_size;
    let (installer, glyph_records) = assemble_mouse_driver_installer(
        gaiji,
        &bank,
        installer_offset,
        runtime.transient_entry_com_address,
    )?;
    let glyph_records_offset = installer_offset + installer.bytes().len();
    let text_offset = glyph_records_offset + glyph_records.len();
    let mut packed_text = Vec::new();
    for entry in &mut entries {
        entry.file_offset = text_offset + packed_text.len();
        entry.com_address = u16::try_from(entry.file_offset + COM_ORIGIN)
            .context("mouse-driver relocated text address exceeds 16 bits")?;
        packed_text.extend_from_slice(&entry.bytes);
    }
    let output_file_size = text_offset
        .checked_add(packed_text.len())
        .context("mouse-driver output size overflow")?;
    ensure!(
        output_file_size + COM_ORIGIN <= usize::from(u16::MAX),
        "mouse-driver development COM image exceeds its 16-bit segment"
    );

    let pointer_tables = vec![
        compile_pointer_table(
            "mouse-driver-frequency-pointer-table",
            FREQUENCY_ROLE,
            runtime.frequency_pointer_table_offset,
            runtime.frequency_pointer_count,
            &entries,
        )?,
        compile_pointer_table(
            "mouse-driver-command-mode-pointer-table",
            COMMAND_MODE_ROLE,
            runtime.command_mode_pointer_table_offset,
            runtime.command_mode_pointer_count,
            &entries,
        )?,
    ];
    let typed_sources = assemble_mouse_driver_sources(
        nmouse_com,
        runtime,
        installer_offset,
        installer.clone(),
        &entries,
    )?;

    Ok(CompiledMouseDriverText {
        bank,
        runtime,
        source_file_size,
        output_file_size,
        installer_offset,
        installer,
        glyph_records_offset,
        glyph_records,
        text_offset,
        packed_text,
        entries,
        pointer_tables,
        typed_sources,
    })
}

fn compile_pointer_table(
    id: &str,
    role: &str,
    offset: usize,
    count: usize,
    entries: &[CompiledMouseDriverTextEntry],
) -> Result<CompiledMousePointerTable> {
    let mut addresses = Vec::with_capacity(count);
    for index in 0..count {
        let matches = entries
            .iter()
            .filter(|entry| entry.role == role && entry.table_index == Some(index))
            .collect::<Vec<_>>();
        ensure!(
            matches.len() == 1,
            "mouse-driver pointer table {role}[{index}] population changed"
        );
        addresses.push(matches[0].com_address);
    }
    ensure!(
        entries.iter().filter(|entry| entry.role == role).count() == count,
        "mouse-driver pointer table {role} has extra entries"
    );
    let replacement = addresses
        .iter()
        .flat_map(|address| address.to_le_bytes())
        .collect();
    Ok(CompiledMousePointerTable {
        id: id.to_owned(),
        offset,
        addresses,
        replacement,
    })
}
