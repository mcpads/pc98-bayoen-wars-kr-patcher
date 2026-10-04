mod compile;
mod model;
mod plans;
mod readback;
mod records;
pub(in crate::korean_patch) use records::compile_record;
mod report;
mod typed_sources;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use expected_write::MachineCodeVerifierError;

use self::model::PatchedInterfaceTextPayload;
use self::report::entry_report;
use super::font_rasterizer::font_provenance;
use super::monochrome_text::DevelopmentBuildStatus;
use super::payload_writes::apply_payload_write_plans_with_verifier;
use super::shared_text::gaiji_bank_plan;
use crate::game_data::catalog_game_data;
use crate::source_disk::SOURCE_DISK_SHA256;
use crate::translation_analysis::load_translation_corpus;
use crate::translation_drafts::{DevelopmentPolicy, TranslationSurface};

const SEGMENT_ID: &str = "mad-interface";

pub(super) use compile::{compile_interface_text, compile_interface_text_with_bank};
pub(super) use model::{CompiledInterfaceText, CompiledInterfaceTextEntry};
pub use model::{InterfaceTextEntryPatchReport, InterfaceTextPatchReport};
pub(super) use plans::{PlannedInterfaceTextWrites, interface_text_plans};
pub(super) use readback::{
    verify_interface_text_payload, verify_interface_text_records_and_references,
};
pub(super) use report::entry_report as interface_entry_report;

pub(crate) fn build_interface_text_payload(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<PatchedInterfaceTextPayload> {
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
    let source = catalog_game_data(mad_com, gaiji_com, menu_com)?;
    let source_interface = &source.mad_com.interface_text;
    let segment = corpus
        .segments
        .iter()
        .find(|segment| segment.id == SEGMENT_ID)
        .context("translation corpus is missing mad-interface")?;
    ensure!(
        segment.surface == TranslationSurface::InterfaceText
            && segment.entries.len() == source_interface.entries.len()
            && segment
                .entries
                .iter()
                .all(|entry| entry.development_policy == DevelopmentPolicy::Translate),
        "mad-interface has the wrong surface, population, or development policy"
    );

    let compiled = compile_interface_text(gaiji_com, &source.gaiji, &source.mad_com, segment)?;
    let mut planned =
        interface_text_plans(mad_com, &source_interface.reference_catalog, &compiled)?;
    planned.plans.insert(
        0,
        gaiji_bank_plan(
            &compiled.bank,
            "interface-text",
            "interface-text-gaiji-bank",
            "install reviewed interface-text glyphs",
        ),
    );
    let typed_sources = planned.typed_sources;
    let verifier = v30::ExpectedWriteVerifier::new(move |source_id: &str| {
        typed_sources.get(source_id).cloned().ok_or_else(|| {
            MachineCodeVerifierError::new(format!("unknown typed V30 interface source {source_id}"))
        })
    });
    let applied =
        apply_payload_write_plans_with_verifier(installer_payload, planned.plans, Some(&verifier))?;
    verify_interface_text_payload(
        &applied.files,
        &source.gaiji,
        &source_interface.reference_catalog,
        &compiled,
    )?;

    let font = font_provenance()?;
    let available_gaiji_slots = compiled.bank.available_slot_count;
    let storage_capacity = compiled.text_region_end - compiled.text_region_start;

    Ok(PatchedInterfaceTextPayload {
        report: InterfaceTextPatchReport {
            supported_source_sha256: corpus.index.supported_source_sha256,
            translation_status: "needs_human_review".to_owned(),
            build_status: DevelopmentBuildStatus::DevelopmentOnly,
            font_profile: font.profile_id,
            font_sha256: font.font_sha256,
            available_gaiji_slots,
            used_gaiji_slots: compiled.bank.glyphs.len(),
            storage_capacity,
            packed_storage_bytes: compiled.packed_storage_bytes,
            storage_headroom: storage_capacity - compiled.packed_storage_bytes,
            reference_count: source_interface.reference_catalog.reference_count,
            machine_code_reference_count: source_interface
                .reference_catalog
                .machine_code_reference_count,
            metadata_reference_count: source_interface.reference_catalog.metadata_reference_count,
            unreferenced_entry_ids: source_interface
                .reference_catalog
                .unreferenced_entry_ids
                .clone(),
            stage_result_window: super::shared_alert_window::patch_report(
                &compiled.stage_result_window,
            ),
            shared_alert_window: super::shared_alert_window::patch_report(
                &compiled.shared_alert_window,
            ),
            spring_capture_window: super::spring_capture_window::patch_report(
                &compiled.spring_capture_window,
            ),
            spring_recovery_window: super::spring_recovery_window::patch_report(
                &compiled.spring_recovery_window,
            ),
            glyphs: compiled.bank.glyphs.clone(),
            entries: compiled
                .entries
                .iter()
                .map(|entry| entry_report(entry, &source_interface.reference_catalog.references))
                .collect(),
            writes: applied.report,
        },
        files: applied.files,
    })
}
