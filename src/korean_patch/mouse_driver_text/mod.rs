mod compile;
mod model;
mod plans;
mod readback;
mod records;
mod report;
mod typed_sources;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use expected_write::MachineCodeVerifierError;

use self::compile::compile_mouse_driver_text;
use self::model::PatchedMouseDriverTextFiles;
use self::plans::mouse_driver_text_plan;
use self::readback::verify_mouse_driver_text_files;
use self::report::entry_report;
use super::font_rasterizer::font_provenance;
use super::monochrome_text::DevelopmentBuildStatus;
use super::payload_writes::apply_payload_write_plans_with_verifier;
use super::shared_text::SharedGaijiGlyph;
use crate::external_text::{catalog_mouse_driver, catalog_mouse_driver_runtime};
use crate::game_data::catalog_game_data;
use crate::source_disk::SOURCE_DISK_SHA256;
use crate::translation_analysis::load_translation_corpus;
use crate::translation_drafts::{DevelopmentPolicy, TranslationSurface};

const SEGMENT_ID: &str = "external-nmouse";
const NMOUSE_FILE: &str = "NMOUSE.COM";

pub use model::{MouseDriverTextEntryPatchReport, MouseDriverTextPatchReport};

pub(crate) fn build_mouse_driver_text_files(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    runtime_files: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<PatchedMouseDriverTextFiles> {
    let corpus = load_translation_corpus(translations)?;
    ensure!(
        corpus.index.supported_source_sha256 == SOURCE_DISK_SHA256,
        "translation corpus targets a different source disk"
    );
    let mad_com = installer_payload
        .get("MAD.COM")
        .context("verified installer payload is missing MAD.COM")?;
    let gaiji_com = installer_payload
        .get("GAIJI.COM")
        .context("verified installer payload is missing GAIJI.COM")?;
    let menu_com = installer_payload
        .get("MENU.COM")
        .context("verified installer payload is missing MENU.COM")?;
    let nmouse_com = runtime_files
        .get(NMOUSE_FILE)
        .context("verified runtime files are missing NMOUSE.COM")?;
    let game_data = catalog_game_data(mad_com, gaiji_com, menu_com)?;
    let source = catalog_mouse_driver(nmouse_com)?;
    let runtime = catalog_mouse_driver_runtime(nmouse_com)?;
    let segment = corpus
        .segments
        .iter()
        .find(|segment| segment.id == SEGMENT_ID)
        .context("translation corpus is missing external-nmouse")?;
    ensure!(
        segment.surface == TranslationSurface::ExternalProgramText
            && segment.entries.len() == source.entries.len()
            && segment
                .entries
                .iter()
                .all(|entry| entry.development_policy == DevelopmentPolicy::Translate),
        "external-nmouse has the wrong surface, population, or development policy"
    );

    let compiled = compile_mouse_driver_text(
        nmouse_com,
        gaiji_com,
        &game_data.gaiji,
        &game_data.mad_com.gaiji_readiness,
        &source,
        runtime,
        segment,
    )?;
    let plan = mouse_driver_text_plan(nmouse_com, &compiled)?;
    let typed_sources = compiled.typed_sources.clone();
    let verifier = v30::ExpectedWriteVerifier::new(move |source_id: &str| {
        typed_sources.get(source_id).cloned().ok_or_else(|| {
            MachineCodeVerifierError::new(format!(
                "unknown typed V30 mouse-driver source {source_id}"
            ))
        })
    });
    let applied =
        apply_payload_write_plans_with_verifier(runtime_files, vec![plan], Some(&verifier))?;
    verify_mouse_driver_text_files(&applied.files, &compiled)?;

    let font = font_provenance()?;
    let glyphs = compiled
        .bank
        .glyphs
        .iter()
        .enumerate()
        .map(|(index, glyph)| SharedGaijiGlyph {
            character: glyph.character.clone(),
            slot_index: glyph.slot_index,
            shift_jis_code: glyph.shift_jis_code.clone(),
            record_offset: compiled.glyph_records_offset
                + index * super::gaiji_record::GAIJI_RECORD_SIZE,
        })
        .collect();
    Ok(PatchedMouseDriverTextFiles {
        report: MouseDriverTextPatchReport {
            supported_source_sha256: corpus.index.supported_source_sha256,
            translation_status: "needs_human_review".to_owned(),
            build_status: DevelopmentBuildStatus::DevelopmentOnly,
            font_profile: font.profile_id,
            font_sha256: font.font_sha256,
            source_file_size: compiled.source_file_size,
            output_file_size: compiled.output_file_size,
            resident_paragraph_count: compiled.runtime.resident_paragraph_count,
            available_gaiji_slots: compiled.bank.available_slot_count,
            used_gaiji_slots: compiled.bank.glyphs.len(),
            installer_code_bytes: compiled.installer.bytes().len(),
            glyph_record_bytes: compiled.glyph_records.len(),
            packed_text_bytes: compiled.packed_text.len(),
            glyphs,
            entries: compiled.entries.iter().map(entry_report).collect(),
            writes: applied.report,
        },
        files: applied.files,
    })
}
