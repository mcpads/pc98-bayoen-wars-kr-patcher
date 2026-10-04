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

use self::compile::compile_mad_system_text;
use self::model::PatchedMadSystemTextPayload;
use self::report::entry_report;
use super::font_rasterizer::font_provenance;
use super::monochrome_text::DevelopmentBuildStatus;
use super::payload_writes::apply_payload_write_plans_with_verifier;
use super::shared_text::gaiji_bank_plan;
use crate::game_data::catalog_game_data;
use crate::source_disk::SOURCE_DISK_SHA256;
use crate::translation_analysis::load_translation_corpus;
use crate::translation_drafts::{DevelopmentPolicy, TranslationSurface};

const SEGMENT_ID: &str = "mad-system";

pub(super) use compile::{compile_mad_system_text_with_bank, preserve_mad_system_text_with_bank};
pub(super) use model::CompiledMadSystemText;
pub use model::{MadSystemTextEntryPatchReport, MadSystemTextPatchReport};
pub(super) use plans::{PlannedMadSystemTextWrites, mad_system_text_plans};
pub(super) use readback::{
    verify_mad_system_text_payload, verify_mad_system_text_records_and_references,
};
pub(super) use report::entry_report as mad_system_entry_report;

pub(crate) fn build_mad_system_text_payload(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<PatchedMadSystemTextPayload> {
    let corpus = load_translation_corpus(translations)?;
    ensure!(
        corpus.index.supported_source_sha256 == SOURCE_DISK_SHA256,
        "translation corpus targets a different source disk"
    );
    let mad_com = required(installer_payload, "MAD.COM")?;
    let gaiji_com = required(installer_payload, "GAIJI.COM")?;
    let menu_com = required(installer_payload, "MENU.COM")?;
    let source = catalog_game_data(mad_com, gaiji_com, menu_com)?;
    let segment = corpus
        .segments
        .iter()
        .find(|segment| segment.id == SEGMENT_ID)
        .context("translation corpus is missing mad-system")?;
    ensure!(
        segment.surface == TranslationSurface::DosSystemText
            && segment.entries.len() == source.mad_com.system_text.entries.len()
            && segment
                .entries
                .iter()
                .all(|entry| entry.development_policy == DevelopmentPolicy::Translate),
        "mad-system has the wrong surface, population, or development policy"
    );

    let compiled = compile_mad_system_text(
        gaiji_com,
        &source.gaiji,
        &source.mad_com.gaiji_readiness,
        &source.mad_com.system_text,
        segment,
    )?;
    let mut planned = mad_system_text_plans(mad_com, &source.mad_com.system_runtime, &compiled)?;
    planned.plans.insert(
        0,
        gaiji_bank_plan(
            &compiled.bank,
            "mad-system-text",
            "mad-system-text-gaiji-bank",
            "install reviewed MAD system-text glyphs",
        ),
    );
    let typed_sources = planned.typed_sources;
    let verifier = v30::ExpectedWriteVerifier::new(move |source_id: &str| {
        typed_sources.get(source_id).cloned().ok_or_else(|| {
            MachineCodeVerifierError::new(format!(
                "unknown typed V30 MAD system source {source_id}"
            ))
        })
    });
    let applied =
        apply_payload_write_plans_with_verifier(installer_payload, planned.plans, Some(&verifier))?;
    verify_mad_system_text_payload(
        &applied.files,
        &source.gaiji,
        &source.mad_com.system_runtime,
        &compiled,
    )?;

    let font = font_provenance()?;
    let storage_capacity = compiled.text_region_end - compiled.text_region_start;
    Ok(PatchedMadSystemTextPayload {
        files: applied.files,
        report: MadSystemTextPatchReport {
            supported_source_sha256: corpus.index.supported_source_sha256,
            translation_status: "needs_human_review".to_owned(),
            build_status: DevelopmentBuildStatus::DevelopmentOnly,
            font_profile: font.profile_id,
            font_sha256: font.font_sha256,
            available_gaiji_slots: compiled.bank.available_slot_count,
            used_gaiji_slots: compiled.bank.glyphs.len(),
            storage_capacity,
            packed_storage_bytes: compiled.packed_storage_bytes,
            storage_headroom: storage_capacity - compiled.packed_storage_bytes,
            semantic_reference_count: source.mad_com.system_runtime.semantic_reference_count,
            storage_reference_count: source.mad_com.system_runtime.storage_reference_count,
            machine_code_reference_count: source
                .mad_com
                .system_runtime
                .machine_code_reference_count,
            metadata_reference_count: source.mad_com.system_runtime.metadata_reference_count,
            runtime_insert_reference_count: source
                .mad_com
                .system_runtime
                .runtime_insert_reference_count,
            glyphs: compiled.bank.glyphs.clone(),
            entries: compiled
                .entries
                .iter()
                .map(|entry| entry_report(entry, &source.mad_com.system_runtime.references))
                .collect(),
            writes: applied.report,
        },
    })
}

fn required<'a>(files: &'a BTreeMap<String, Vec<u8>>, name: &str) -> Result<&'a [u8]> {
    files
        .get(name)
        .map(Vec::as_slice)
        .with_context(|| format!("verified installer payload is missing {name}"))
}
