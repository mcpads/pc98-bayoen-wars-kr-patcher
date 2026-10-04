mod compile;
mod model;
mod plans;
mod report;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use expected_write::MachineCodeVerifierError;

use self::model::PatchedMadSystemInterfaceTextPayload;
use super::font_rasterizer::font_provenance;
use super::interface_text::{interface_text_plans, verify_interface_text_payload};
use super::mad_system_text::{mad_system_text_plans, verify_mad_system_text_payload};
use super::payload_writes::apply_payload_write_plans_with_verifier;
use crate::game_data::catalog_game_data;
use crate::source_disk::SOURCE_DISK_SHA256;
use crate::translation_analysis::{TranslationCorpus, load_translation_corpus};
use crate::translation_drafts::{DevelopmentPolicy, TranslationDraftSegment, TranslationSurface};

const SYSTEM_SEGMENT_ID: &str = "mad-system";
const INTERFACE_SEGMENT_ID: &str = "mad-interface";
pub(in crate::korean_patch) use compile::{
    CompiledMadSystemInterfaceText, compile_mad_system_interface_text,
};
pub use model::{
    MadSystemInterfaceTextPatchReport, PatchedInterfaceComponentReport,
    PatchedSystemComponentReport,
};
pub(in crate::korean_patch) use plans::{
    PlannedMadSystemInterfaceTextWrites, combine_mad_system_interface_text_plans,
};
pub(in crate::korean_patch) use report::combined_report;

pub(crate) fn build_mad_system_interface_text_payload(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<PatchedMadSystemInterfaceTextPayload> {
    let corpus = load_translation_corpus(translations)?;
    ensure!(
        corpus.index.supported_source_sha256 == SOURCE_DISK_SHA256,
        "translation corpus targets a different source disk"
    );
    let mad_com = required(installer_payload, "MAD.COM")?;
    let gaiji_com = required(installer_payload, "GAIJI.COM")?;
    let menu_com = required(installer_payload, "MENU.COM")?;
    let source = catalog_game_data(mad_com, gaiji_com, menu_com)?;
    let system_segment = require_segment(
        &corpus,
        SYSTEM_SEGMENT_ID,
        TranslationSurface::DosSystemText,
        source.mad_com.system_text.entries.len(),
    )?;
    let interface_segment = require_segment(
        &corpus,
        INTERFACE_SEGMENT_ID,
        TranslationSurface::InterfaceText,
        source.mad_com.interface_text.entries.len(),
    )?;

    let compiled = compile_mad_system_interface_text(
        gaiji_com,
        &source.mad_com,
        &source.gaiji,
        system_segment,
        interface_segment,
    )?;

    let interface_plans = interface_text_plans(
        mad_com,
        &source.mad_com.interface_text.reference_catalog,
        &compiled.interface,
    )?;
    let system_plans =
        mad_system_text_plans(mad_com, &source.mad_com.system_runtime, &compiled.system)?;
    let planned =
        combine_mad_system_interface_text_plans(&compiled.bank, system_plans, interface_plans)?;
    let typed_sources = planned.typed_sources;
    let verifier = v30::ExpectedWriteVerifier::new(move |source_id: &str| {
        typed_sources.get(source_id).cloned().ok_or_else(|| {
            MachineCodeVerifierError::new(format!(
                "unknown typed V30 MAD shared-text source {source_id}"
            ))
        })
    });
    let applied =
        apply_payload_write_plans_with_verifier(installer_payload, planned.plans, Some(&verifier))?;
    verify_mad_system_text_payload(
        &applied.files,
        &source.gaiji,
        &source.mad_com.system_runtime,
        &compiled.system,
    )?;
    verify_interface_text_payload(
        &applied.files,
        &source.gaiji,
        &source.mad_com.interface_text.reference_catalog,
        &compiled.interface,
    )?;

    Ok(PatchedMadSystemInterfaceTextPayload {
        files: applied.files,
        report: combined_report(
            corpus.index.supported_source_sha256,
            font_provenance()?,
            &source.mad_com,
            &source.mad_com.system_runtime,
            &compiled.system,
            &compiled.interface,
            applied.report,
        ),
    })
}

pub(in crate::korean_patch) fn require_segment<'a>(
    corpus: &'a TranslationCorpus,
    id: &str,
    surface: TranslationSurface,
    entry_count: usize,
) -> Result<&'a TranslationDraftSegment> {
    let segment = corpus
        .segments
        .iter()
        .find(|segment| segment.id == id)
        .with_context(|| format!("translation corpus is missing {id}"))?;
    ensure!(
        segment.surface == surface
            && segment.entries.len() == entry_count
            && segment
                .entries
                .iter()
                .all(|entry| entry.development_policy == DevelopmentPolicy::Translate),
        "{id} has the wrong surface, population, or development policy"
    );
    Ok(segment)
}

fn required<'a>(files: &'a BTreeMap<String, Vec<u8>>, name: &str) -> Result<&'a [u8]> {
    files
        .get(name)
        .map(Vec::as_slice)
        .with_context(|| format!("verified installer payload is missing {name}"))
}
