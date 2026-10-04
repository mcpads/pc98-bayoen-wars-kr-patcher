mod compile;
mod layout;
mod model;
mod plans;
mod readback;
mod records;
mod report;
mod typed_sources;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, ensure};

use self::compile::{SystemLoaderCompileInput, compile_system_loader_text};
use self::model::PatchedSystemLoaderTextFiles;
use self::plans::system_loader_file_plan;
use self::readback::verify_system_loader_text_files;
use self::report::{entry_report, glyph_reports};
use super::font_rasterizer::font_provenance;
use super::monochrome_text::DevelopmentBuildStatus;
use super::payload_writes::apply_payload_write_plans;
use crate::external_text::{catalog_system_loader, catalog_system_loader_runtime};
use crate::game_data::catalog_game_data;
use crate::source_disk::{SOURCE_DISK_SHA256, sha256_hex};
use crate::translation_analysis::load_translation_corpus;
use crate::translation_drafts::{DevelopmentPolicy, TranslationSurface};

const SEGMENT_ID: &str = "external-megdos";
const SYSTEM_LOADER_FILE: &str = "MEGDOS.SYS";

pub use model::{SystemLoaderTextEntryPatchReport, SystemLoaderTextPatchReport};

pub(crate) fn build_system_loader_text_files(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    boot_files: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<PatchedSystemLoaderTextFiles> {
    let corpus = load_translation_corpus(translations)?;
    ensure!(
        corpus.index.supported_source_sha256 == SOURCE_DISK_SHA256,
        "translation corpus targets a different source disk"
    );
    let mad_com = required(installer_payload, "MAD.COM")?;
    let gaiji_com = required(installer_payload, "GAIJI.COM")?;
    let menu_com = required(installer_payload, "MENU.COM")?;
    let system_loader = required(boot_files, SYSTEM_LOADER_FILE)?;
    let game_data = catalog_game_data(mad_com, gaiji_com, menu_com)?;
    let source_text = catalog_system_loader(system_loader)?;
    let runtime = catalog_system_loader_runtime(system_loader, &source_text)?;
    let segment = corpus
        .segments
        .iter()
        .find(|segment| segment.id == SEGMENT_ID)
        .context("translation corpus is missing external-megdos")?;
    ensure!(
        segment.surface == TranslationSurface::ExternalProgramText
            && segment.entries.len() == source_text.entries.len()
            && segment
                .entries
                .iter()
                .all(|entry| entry.development_policy == DevelopmentPolicy::Translate),
        "external-megdos has the wrong surface, population, or development policy"
    );

    let compiled = compile_system_loader_text(SystemLoaderCompileInput {
        source_file: system_loader,
        source_gaiji_file: gaiji_com,
        gaiji: &game_data.gaiji,
        readiness: &game_data.mad_com.gaiji_readiness,
        source_text: &source_text,
        runtime,
        draft: segment,
    })?;
    let plan = system_loader_file_plan(system_loader, &compiled.output_bytes);
    let applied = apply_payload_write_plans(boot_files, vec![plan])?;
    verify_system_loader_text_files(&applied.files, &compiled)?;

    let font = font_provenance()?;
    let report = SystemLoaderTextPatchReport {
        supported_source_sha256: corpus.index.supported_source_sha256,
        translation_status: "needs_human_review".to_owned(),
        build_status: DevelopmentBuildStatus::DevelopmentOnly,
        font_profile: font.profile_id,
        font_sha256: font.font_sha256,
        source_file_size: compiled.source_bytes.len(),
        output_file_size: compiled.output_bytes.len(),
        source_resident_byte_count: compiled.runtime.resident_byte_count,
        output_resident_byte_count: compiled.output_resident_byte_count,
        resident_extension_byte_count: compiled.resident_extension_byte_count,
        shifted_tail_byte_count: compiled.runtime.tail_byte_count,
        shifted_tail_sha256: compiled.shifted_tail_sha256.clone(),
        available_gaiji_slots: compiled.bank.available_slot_count,
        used_gaiji_slots: compiled.bank.glyphs.len(),
        installer_code_bytes: compiled.installer.bytes().len(),
        glyph_record_bytes: compiled.glyph_records.len(),
        packed_text_bytes: compiled.packed_text.len(),
        trampoline_code_bytes: compiled.trampoline.bytes().len(),
        reference_count: compiled.runtime.references.len(),
        glyphs: glyph_reports(&compiled),
        entries: compiled
            .entries
            .iter()
            .map(|entry| entry_report(entry, &compiled))
            .collect(),
        resident_writes: compiled.resident_writes.clone(),
        writes: applied.report,
    };
    ensure!(
        sha256_hex(&compiled.source_bytes) == sha256_hex(system_loader),
        "MEGDOS.SYS source changed during compilation"
    );
    Ok(PatchedSystemLoaderTextFiles {
        files: applied.files,
        report,
    })
}

fn required<'a>(files: &'a BTreeMap<String, Vec<u8>>, name: &str) -> Result<&'a [u8]> {
    files
        .get(name)
        .map(Vec::as_slice)
        .with_context(|| format!("verified source files are missing {name}"))
}
