mod bank_file;
mod compile;
mod model;
mod narrative;
mod plans;
mod readback;
mod report;
mod transition;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use expected_write::MachineCodeVerifierError;

use self::compile::compile_mad_scene_text;
use self::model::PatchedMadSceneTextPayload;
use self::plans::{PlannedMadSceneTextWrites, mad_scene_text_plans};
use self::readback::verify_mad_scene_text_payload;
use self::report::mad_scene_report;
use super::font_rasterizer::{FontProvenance, font_provenance};
use super::payload_writes::{PayloadWriteReport, apply_payload_write_plans_with_verifier};
use crate::game_data::{GameDataCatalog, catalog_game_data};
use crate::source_disk::SOURCE_DISK_SHA256;
use crate::translation_analysis::load_translation_corpus;

pub use model::{
    MadSceneNarrativePatchReport, MadSceneTextPatchReport, MadSceneTransitionPatchReport,
    MadSystemTextDisposition, PatchedDialogueComponentReport,
};
pub(crate) use narrative::{
    build_complete_mad_scene_narrative_payload_with_title_artwork,
    build_mad_scene_narrative_payload, build_mad_scene_narrative_payload_with_title_artwork,
};

struct PreparedMadSceneText {
    supported_source_sha256: String,
    font: FontProvenance,
    source_gaiji_file_size: usize,
    source: GameDataCatalog,
    compiled: compile::CompiledMadSceneText,
}

pub(crate) fn build_mad_scene_text_payload(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<PatchedMadSceneTextPayload> {
    let (prepared, planned) = prepare_mad_scene_text(
        installer_payload,
        translations,
        MadSystemTextDisposition::TranslatedDevelopment,
    )?;
    let typed_sources = planned.typed_sources;
    let verifier = v30_verifier(typed_sources);
    let applied =
        apply_payload_write_plans_with_verifier(installer_payload, planned.plans, Some(&verifier))?;
    let report = complete_mad_scene_text(&applied.files, &prepared, applied.report)?;
    Ok(PatchedMadSceneTextPayload {
        files: applied.files,
        report,
    })
}

fn prepare_mad_scene_text(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
    system_text_disposition: MadSystemTextDisposition,
) -> Result<(PreparedMadSceneText, PlannedMadSceneTextWrites)> {
    let corpus = load_translation_corpus(translations)?;
    ensure!(
        corpus.index.supported_source_sha256 == SOURCE_DISK_SHA256,
        "translation corpus targets a different source disk"
    );
    let mad_com = required(installer_payload, "MAD.COM")?;
    let gaiji_com = required(installer_payload, "GAIJI.COM")?;
    let menu_com = required(installer_payload, "MENU.COM")?;
    let source = catalog_game_data(mad_com, gaiji_com, menu_com)?;
    let compiled = compile_mad_scene_text(
        mad_com,
        gaiji_com,
        &source,
        &corpus,
        system_text_disposition,
    )?;
    let planned = mad_scene_text_plans(mad_com, gaiji_com, &source, &compiled)?;
    Ok((
        PreparedMadSceneText {
            supported_source_sha256: corpus.index.supported_source_sha256,
            font: font_provenance()?,
            source_gaiji_file_size: gaiji_com.len(),
            source,
            compiled,
        },
        planned,
    ))
}

fn v30_verifier(
    typed_sources: BTreeMap<String, v30::AssembledProgram>,
) -> v30::ExpectedWriteVerifier<
    impl Fn(&str) -> std::result::Result<v30::AssembledProgram, MachineCodeVerifierError>,
> {
    v30::ExpectedWriteVerifier::new(move |source_id: &str| {
        typed_sources.get(source_id).cloned().ok_or_else(|| {
            MachineCodeVerifierError::new(format!(
                "unknown typed V30 MAD scene-text source {source_id}"
            ))
        })
    })
}

fn complete_mad_scene_text(
    files: &BTreeMap<String, Vec<u8>>,
    prepared: &PreparedMadSceneText,
    writes: Vec<PayloadWriteReport>,
) -> Result<MadSceneTextPatchReport> {
    verify_mad_scene_text_payload(files, &prepared.source, &prepared.compiled)?;
    Ok(mad_scene_report(
        prepared.supported_source_sha256.clone(),
        prepared.font.clone(),
        &prepared.source,
        prepared.source_gaiji_file_size,
        &prepared.compiled,
        writes,
    ))
}

fn required<'a>(files: &'a BTreeMap<String, Vec<u8>>, name: &str) -> Result<&'a [u8]> {
    files
        .get(name)
        .map(Vec::as_slice)
        .with_context(|| format!("verified installer payload is missing {name}"))
}
