use anyhow::{Context, Result, ensure};

use super::layout::{paragraph_aligned_end, reconstruct_resident_extension};
use super::model::CompiledSystemLoaderText;
use super::plans::apply_resident_layout;
use super::records::compile_record;
use super::typed_sources::{
    assemble_post_stack_trampoline, assemble_system_loader_installer,
    assemble_system_loader_sources,
};
use crate::external_text::{ExternalProgramTextCatalog, SystemLoaderRuntimeCatalog};
use crate::game_data::{GaijiCatalog, GaijiReadinessCatalog};
use crate::korean_patch::shared_text::compile_gaiji_bank;
use crate::source_disk::sha256_hex;
use crate::translation_drafts::TranslationDraftSegment;

pub(super) struct SystemLoaderCompileInput<'a> {
    pub source_file: &'a [u8],
    pub source_gaiji_file: &'a [u8],
    pub gaiji: &'a GaijiCatalog,
    pub readiness: &'a GaijiReadinessCatalog,
    pub source_text: &'a ExternalProgramTextCatalog,
    pub runtime: SystemLoaderRuntimeCatalog,
    pub draft: &'a TranslationDraftSegment,
}

pub(super) fn compile_system_loader_text(
    input: SystemLoaderCompileInput<'_>,
) -> Result<CompiledSystemLoaderText> {
    let SystemLoaderCompileInput {
        source_file,
        source_gaiji_file,
        gaiji,
        readiness,
        source_text,
        runtime,
        draft,
    } = input;
    let draft_lines = draft
        .entries
        .iter()
        .flat_map(|entry| entry.korean_text.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let bank = compile_gaiji_bank(source_gaiji_file, gaiji, readiness, &draft_lines)?;
    let mut entries = source_text
        .entries
        .iter()
        .zip(&draft.entries)
        .map(|(source, draft)| compile_record(source, draft, &bank))
        .collect::<Result<Vec<_>>>()?;

    let installer_offset = runtime.tail_offset;
    let (placeholder_installer, placeholder_records) =
        assemble_system_loader_installer(gaiji, &bank, installer_offset, 0)?;
    let glyph_records_offset = installer_offset + placeholder_installer.bytes().len();
    let text_offset = glyph_records_offset + placeholder_records.len();
    let mut packed_text = Vec::new();
    for entry in &mut entries {
        let offset = text_offset
            .checked_add(packed_text.len())
            .context("MEGDOS.SYS packed text offset overflow")?;
        entry.resident_offset =
            u16::try_from(offset).context("MEGDOS.SYS text address exceeds one segment")?;
        packed_text.extend_from_slice(&entry.bytes);
    }
    let trampoline_offset = text_offset
        .checked_add(packed_text.len())
        .context("MEGDOS.SYS trampoline offset overflow")?;
    let trampoline = assemble_post_stack_trampoline(source_file, &runtime, trampoline_offset)?;
    let minimum_resident_end = trampoline_offset
        .checked_add(trampoline.bytes().len())
        .context("MEGDOS.SYS resident layout end overflow")?;
    let output_resident_end = paragraph_aligned_end(minimum_resident_end)?;
    let output_resident_byte_count = u16::try_from(output_resident_end)
        .context("MEGDOS.SYS resident block exceeds one segment")?;
    let resident_extension_byte_count = output_resident_end
        .checked_sub(runtime.tail_offset)
        .context("MEGDOS.SYS resident layout did not extend the source")?;
    ensure!(
        resident_extension_byte_count > 0,
        "MEGDOS.SYS Korean resident layout has no extension"
    );

    let (installer, glyph_records) = assemble_system_loader_installer(
        gaiji,
        &bank,
        installer_offset,
        u16::try_from(trampoline_offset).context("MEGDOS.SYS trampoline exceeds one segment")?,
    )?;
    ensure!(
        installer.bytes().len() == placeholder_installer.bytes().len()
            && glyph_records == placeholder_records,
        "MEGDOS.SYS final installer changed its resident layout"
    );
    let reconstructed_source =
        reconstruct_resident_extension(source_file, runtime.tail_offset, output_resident_end)?;
    let typed_sources = assemble_system_loader_sources(
        source_file,
        &runtime,
        output_resident_byte_count,
        installer_offset,
        installer.clone(),
        &entries,
        trampoline.clone(),
    )?;
    let shifted_tail_sha256 = sha256_hex(&source_file[runtime.tail_offset..]);

    let mut compiled = CompiledSystemLoaderText {
        bank,
        runtime,
        source_bytes: source_file.to_vec(),
        reconstructed_source,
        output_bytes: Vec::new(),
        output_resident_byte_count,
        resident_extension_byte_count,
        shifted_tail_sha256,
        installer_offset,
        installer,
        glyph_records_offset,
        glyph_records,
        text_offset,
        packed_text,
        entries,
        trampoline_offset,
        trampoline,
        typed_sources,
        resident_writes: Vec::new(),
    };
    let (output_bytes, resident_writes) = apply_resident_layout(&compiled)?;
    compiled.output_bytes = output_bytes;
    compiled.resident_writes = resident_writes;
    Ok(compiled)
}
