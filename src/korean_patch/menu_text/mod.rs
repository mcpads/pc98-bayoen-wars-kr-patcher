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

use self::compile::compile_menu_text;
use self::model::PatchedMenuTextFiles;
use self::plans::menu_text_plan;
use self::readback::verify_menu_text_files;
use self::report::entry_report;
use super::font_rasterizer::font_provenance;
use super::monochrome_text::DevelopmentBuildStatus;
use super::payload_writes::apply_payload_write_plans_with_verifier;
use super::shared_text::SharedGaijiGlyph;
use crate::game_data::catalog_game_data;
use crate::source_disk::SOURCE_DISK_SHA256;
use crate::translation_analysis::load_translation_corpus;
use crate::translation_drafts::{DevelopmentPolicy, TranslationSurface};

const SEGMENT_ID: &str = "menu";
const MENU_FILE: &str = "MENU.COM";

pub use model::{MenuTextEntryPatchReport, MenuTextPatchReport};

pub(crate) fn build_menu_text_files(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<PatchedMenuTextFiles> {
    let corpus = load_translation_corpus(translations)?;
    ensure!(
        corpus.index.supported_source_sha256 == SOURCE_DISK_SHA256,
        "translation corpus targets a different source disk"
    );
    let mad_com = required(installer_payload, "MAD.COM")?;
    let gaiji_com = required(installer_payload, "GAIJI.COM")?;
    let menu_com = required(installer_payload, MENU_FILE)?;
    let source = catalog_game_data(mad_com, gaiji_com, menu_com)?;
    let segment = corpus
        .segments
        .iter()
        .find(|segment| segment.id == SEGMENT_ID)
        .context("translation corpus is missing menu")?;
    ensure!(
        segment.surface == TranslationSurface::MenuText
            && segment.entries.len() == source.menu.text.entries.len()
            && segment.entries.iter().all(|entry| {
                matches!(
                    entry.development_policy,
                    DevelopmentPolicy::Translate | DevelopmentPolicy::PreserveSourceControl
                )
            }),
        "menu has the wrong surface, population, or development policy"
    );

    let compiled = compile_menu_text(
        menu_com,
        gaiji_com,
        &source.gaiji,
        &source.mad_com.gaiji_readiness,
        &source.menu,
        segment,
    )?;
    let plan = menu_text_plan(menu_com, &source.menu.runtime, &compiled)?;
    let typed_sources = compiled.typed_sources.clone();
    let verifier = v30::ExpectedWriteVerifier::new(move |source_id: &str| {
        typed_sources.get(source_id).cloned().ok_or_else(|| {
            MachineCodeVerifierError::new(format!("unknown typed V30 MENU source {source_id}"))
        })
    });
    let applied =
        apply_payload_write_plans_with_verifier(installer_payload, vec![plan], Some(&verifier))?;
    verify_menu_text_files(&applied.files, &source.menu.runtime, &compiled)?;

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
    Ok(PatchedMenuTextFiles {
        report: MenuTextPatchReport {
            supported_source_sha256: corpus.index.supported_source_sha256,
            translation_status: "needs_human_review".to_owned(),
            build_status: DevelopmentBuildStatus::DevelopmentOnly,
            font_profile: font.profile_id,
            font_sha256: font.font_sha256,
            source_file_size: compiled.source_file_size,
            output_file_size: compiled.output_file_size,
            available_gaiji_slots: compiled.bank.available_slot_count,
            used_gaiji_slots: compiled.bank.glyphs.len(),
            installer_code_bytes: compiled.installer.bytes().len(),
            glyph_record_bytes: compiled.glyph_records.len(),
            trampoline_code_bytes: compiled.trampoline.bytes().len(),
            packed_text_bytes: compiled.packed_text.len(),
            semantic_reference_count: source.menu.runtime.semantic_reference_count,
            storage_reference_count: source.menu.runtime.storage_reference_count,
            machine_code_reference_count: source.menu.runtime.machine_code_reference_count,
            metadata_reference_count: source.menu.runtime.metadata_reference_count,
            runtime_insert_reference_count: source.menu.runtime.runtime_insert_reference_count,
            glyphs,
            entries: compiled
                .entries
                .iter()
                .map(|entry| entry_report(entry, &source.menu.text))
                .collect(),
            writes: applied.report,
        },
        files: applied.files,
    })
}

fn required<'a>(files: &'a BTreeMap<String, Vec<u8>>, name: &str) -> Result<&'a [u8]> {
    files
        .get(name)
        .map(Vec::as_slice)
        .with_context(|| format!("verified installer payload is missing {name}"))
}
