use anyhow::{Context, Result, ensure};

use super::model::CompiledSamplingDriverText;
use super::records::compile_record;
use super::typed_sources::{assemble_embedded_installer, assemble_sampling_driver_sources};
use crate::external_text::{ExternalProgramTextCatalog, SamplingDriverLifetimeCatalog};
use crate::game_data::{GaijiCatalog, GaijiReadinessCatalog};
use crate::korean_patch::shared_text::compile_gaiji_bank;
use crate::translation_drafts::TranslationDraftSegment;

const COM_ORIGIN: usize = 0x100;

pub(super) fn compile_sampling_driver_text(
    bsamp_com: &[u8],
    gaiji_com: &[u8],
    gaiji: &GaijiCatalog,
    readiness: &GaijiReadinessCatalog,
    source: &ExternalProgramTextCatalog,
    lifetime: SamplingDriverLifetimeCatalog,
    draft: &TranslationDraftSegment,
) -> Result<CompiledSamplingDriverText> {
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

    let source_file_size = bsamp_com.len();
    ensure!(
        usize::from(lifetime.resident_end_com_address) < source_file_size + COM_ORIGIN,
        "sampling-driver source has no non-resident transient tail"
    );
    let installer_offset = source_file_size;
    let (installer, glyph_records) = assemble_embedded_installer(
        gaiji,
        &bank,
        installer_offset,
        lifetime.transient_entry_com_address,
    )?;
    let glyph_records_offset = installer_offset + installer.bytes().len();
    let text_offset = glyph_records_offset + glyph_records.len();
    let mut packed_text = Vec::new();
    for entry in &mut entries {
        entry.file_offset = text_offset + packed_text.len();
        entry.com_address = u16::try_from(entry.file_offset + COM_ORIGIN)
            .context("sampling-driver relocated text address exceeds 16 bits")?;
        packed_text.extend_from_slice(&entry.bytes);
    }
    let output_file_size = text_offset
        .checked_add(packed_text.len())
        .context("sampling-driver output size overflow")?;
    ensure!(
        output_file_size + COM_ORIGIN <= usize::from(u16::MAX),
        "sampling-driver development COM image exceeds its 16-bit segment"
    );
    let typed_sources = assemble_sampling_driver_sources(
        bsamp_com,
        lifetime,
        installer_offset,
        installer.clone(),
        &entries,
    )?;

    Ok(CompiledSamplingDriverText {
        bank,
        lifetime,
        source_file_size,
        output_file_size,
        installer_offset,
        installer,
        glyph_records_offset,
        glyph_records,
        text_offset,
        packed_text,
        entries,
        typed_sources,
    })
}
