mod canvas;
mod difficulty;
mod difficulty_style;
mod model;
mod plans;
mod readback;
mod report;
mod selection;
mod title;
mod title_art;
mod title_art_bounds;
mod title_art_layout;
mod title_art_manifest;
mod title_art_runtime;
mod title_art_transfer_fit;
mod title_background_texture;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use expected_write::MachineCodeVerifierError;

use self::difficulty::compile_difficulty;
use self::model::{AssetPatch, PatchedBakedGraphicsPayload};
use self::plans::baked_asset_plan;
use self::readback::verify_baked_payload;
use self::report::asset_report;
use self::selection::compile_selection;
use self::title::compile_title;
use self::title_art::{TitleArtworkInput, complete_title_artwork_report, prepare_title_artwork};
use self::title_art_runtime::{title_art_runtime_plan, verify_title_art_runtime};
use super::font_catalog::{FontTarget, font_profile_report_for};
use super::monochrome_text::DevelopmentBuildStatus;
use super::payload_writes::{
    PayloadFileWritePlan, PayloadWriteReport, apply_payload_write_plans_with_verifier,
};
use crate::asset_bindings::{BakedTextCatalog, BakedTextUnit, catalog_baked_text};
use crate::localization_assets::{decode_all_streams, encode_single_stream};
use crate::source_disk::SOURCE_DISK_SHA256;
use crate::translation_analysis::load_translation_corpus;
use crate::translation_drafts::{
    DevelopmentPolicy, TranslationDraftEntry, TranslationDraftSegment, TranslationSurface,
};

const TITLE_FILE: &str = "TITLE.DAT";
const DIFFICULTY_FILE: &str = "SEL1.DAT";
const SELECTION_FILE: &str = "SEL3.DAT";

pub use model::{
    BakedGraphicsAssetPatchReport, BakedGraphicsTextPatchReport, TitleArtworkPatchReport,
};

pub(in crate::korean_patch) struct PreparedBakedGraphics {
    supported_source_sha256: String,
    title_artwork: Option<TitleArtworkPatchReport>,
    patches: Vec<AssetPatch>,
    pub(in crate::korean_patch) typed_sources: BTreeMap<String, v30::AssembledProgram>,
}

pub(crate) fn build_baked_graphics_payload(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<PatchedBakedGraphicsPayload> {
    let (prepared, plans) =
        prepare_baked_graphics_with_optional_title_artwork(installer_payload, translations, None)?;
    let verifier = baked_graphics_verifier(prepared.typed_sources.clone());
    let applied =
        apply_payload_write_plans_with_verifier(installer_payload, plans, Some(&verifier))?;
    let report =
        complete_baked_graphics(installer_payload, &applied.files, &prepared, applied.report)?;
    Ok(PatchedBakedGraphicsPayload {
        report,
        files: applied.files,
    })
}

pub(crate) fn build_baked_graphics_payload_with_title_artwork(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
    manifest_path: &Path,
    source_preview_path: &Path,
    artwork_path: &Path,
) -> Result<PatchedBakedGraphicsPayload> {
    let input = TitleArtworkInput {
        manifest_path,
        source_preview_path,
        artwork_path,
    };
    let (prepared, plans) = prepare_baked_graphics_with_optional_title_artwork(
        installer_payload,
        translations,
        Some(input),
    )?;
    let verifier = baked_graphics_verifier(prepared.typed_sources.clone());
    let applied =
        apply_payload_write_plans_with_verifier(installer_payload, plans, Some(&verifier))?;
    let report =
        complete_baked_graphics(installer_payload, &applied.files, &prepared, applied.report)?;
    Ok(PatchedBakedGraphicsPayload {
        report,
        files: applied.files,
    })
}

pub(in crate::korean_patch) fn prepare_baked_graphics(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<(PreparedBakedGraphics, Vec<PayloadFileWritePlan>)> {
    prepare_baked_graphics_with_optional_title_artwork(installer_payload, translations, None)
}

pub(in crate::korean_patch) fn prepare_baked_graphics_with_title_artwork(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
    manifest_path: &Path,
    source_preview_path: &Path,
    artwork_path: &Path,
) -> Result<(PreparedBakedGraphics, Vec<PayloadFileWritePlan>)> {
    prepare_baked_graphics_with_optional_title_artwork(
        installer_payload,
        translations,
        Some(TitleArtworkInput {
            manifest_path,
            source_preview_path,
            artwork_path,
        }),
    )
}

fn prepare_baked_graphics_with_optional_title_artwork(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
    title_artwork_input: Option<TitleArtworkInput<'_>>,
) -> Result<(PreparedBakedGraphics, Vec<PayloadFileWritePlan>)> {
    let corpus = load_translation_corpus(translations)?;
    ensure!(
        corpus.index.supported_source_sha256 == SOURCE_DISK_SHA256,
        "translation corpus targets a different source disk"
    );
    let mad_com = installer_payload
        .get("MAD.COM")
        .context("verified installer payload is missing MAD.COM")?;
    let catalog = catalog_baked_text(mad_com, installer_payload)?;
    let title_segment = require_segment(&corpus.segments, "title-baked-text")?;
    let difficulty_segment = require_segment(&corpus.segments, "difficulty-baked-text")?;
    let selection_segment = require_segment(&corpus.segments, "selection-baked-text")?;
    let title_units = bind_segment(&catalog, title_segment)?;
    let difficulty_units = bind_segment(&catalog, difficulty_segment)?;
    let selection_units = bind_segment(&catalog, selection_segment)?;

    let title_source = decode_asset(installer_payload, TITLE_FILE)?;
    let difficulty_source = decode_asset(installer_payload, DIFFICULTY_FILE)?;
    let selection_source = decode_asset(installer_payload, SELECTION_FILE)?;
    let (title_decoded, title_artwork_evidence, title_runtime_palette_rgb4) =
        match title_artwork_input {
            Some(input) => {
                let source_packed = installer_payload
                    .get(TITLE_FILE)
                    .context("verified installer payload is missing TITLE.DAT")?;
                let title_animation_source = decode_asset(installer_payload, "TITLE2.DAT")?;
                let prepared = prepare_title_artwork(
                    source_packed,
                    &title_source,
                    mad_com,
                    &title_animation_source,
                    &title_segment.draft_sha256,
                    input,
                )?;
                let (decoded, evidence, runtime_palette_rgb4) = prepared.into_parts();
                (decoded, Some(evidence), Some(runtime_palette_rgb4))
            }
            None => (compile_title(&title_source, &title_units)?, None, None),
        };
    let title = make_patch(
        TITLE_FILE,
        "title-baked-text-producer",
        &title_units,
        title_decoded,
    )?;
    let title_artwork = title_artwork_evidence
        .map(|evidence| complete_title_artwork_report(&title, evidence))
        .transpose()?;
    let difficulty = make_patch(
        DIFFICULTY_FILE,
        "difficulty-baked-text-producer",
        &difficulty_units,
        compile_difficulty(&difficulty_source, &difficulty_units)?,
    )?;
    let (name_units, heading_units): (Vec<_>, Vec<_>) = selection_units
        .iter()
        .copied()
        .partition(|(unit, _)| unit.id.starts_with("character-name-"));
    ensure!(
        heading_units.len() == 1,
        "selection segment must contain one stage heading"
    );
    let selection = make_patch(
        SELECTION_FILE,
        "selection-baked-text-producer",
        &selection_units,
        compile_selection(&selection_source, &name_units, heading_units[0])?,
    )?;
    let patches = vec![title, difficulty, selection];
    let mut plans = patches
        .iter()
        .map(|patch| baked_asset_plan(installer_payload, patch))
        .collect::<Vec<_>>();
    let mut typed_sources = BTreeMap::new();
    if let Some(palette) = &title_runtime_palette_rgb4 {
        let runtime = title_art_runtime_plan(installer_payload, palette)?;
        plans.push(runtime.plan);
        for (id, source) in runtime.typed_sources {
            ensure!(
                typed_sources.insert(id.clone(), source).is_none(),
                "duplicate typed V30 baked-graphics source {id}"
            );
        }
    }
    Ok((
        PreparedBakedGraphics {
            supported_source_sha256: corpus.index.supported_source_sha256,
            title_artwork,
            patches,
            typed_sources,
        },
        plans,
    ))
}

fn baked_graphics_verifier(
    typed_sources: BTreeMap<String, v30::AssembledProgram>,
) -> v30::ExpectedWriteVerifier<
    impl Fn(&str) -> std::result::Result<v30::AssembledProgram, MachineCodeVerifierError>,
> {
    v30::ExpectedWriteVerifier::new(move |source_id: &str| {
        typed_sources.get(source_id).cloned().ok_or_else(|| {
            MachineCodeVerifierError::new(format!(
                "unknown typed V30 baked-graphics source {source_id}"
            ))
        })
    })
}

pub(in crate::korean_patch) fn complete_baked_graphics(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    files: &BTreeMap<String, Vec<u8>>,
    prepared: &PreparedBakedGraphics,
    writes: Vec<PayloadWriteReport>,
) -> Result<BakedGraphicsTextPatchReport> {
    verify_baked_payload(files, &prepared.patches)?;
    if let Some(title_artwork) = &prepared.title_artwork {
        verify_title_art_runtime(files, title_artwork)?;
    }
    Ok(BakedGraphicsTextPatchReport {
        supported_source_sha256: prepared.supported_source_sha256.clone(),
        translation_status: "needs_human_review".to_owned(),
        build_status: DevelopmentBuildStatus::DevelopmentOnly,
        fonts: if prepared.title_artwork.is_some() {
            vec![FontTarget::BakedDisplay16, FontTarget::Difficulty32]
        } else {
            vec![
                FontTarget::BakedDisplay16,
                FontTarget::Difficulty32,
                FontTarget::TitlePrimary16,
            ]
        }
        .into_iter()
        .map(font_profile_report_for)
        .collect::<Result<Vec<_>>>()?,
        title_artwork: prepared.title_artwork.clone(),
        assets: prepared
            .patches
            .iter()
            .map(|patch| asset_report(installer_payload, patch))
            .collect(),
        writes,
    })
}

fn require_segment<'a>(
    segments: &'a [TranslationDraftSegment],
    id: &str,
) -> Result<&'a TranslationDraftSegment> {
    let segment = segments
        .iter()
        .find(|segment| segment.id == id)
        .with_context(|| format!("translation corpus is missing {id}"))?;
    ensure!(
        segment.surface == TranslationSurface::BakedGraphicsText
            && segment
                .entries
                .iter()
                .all(|entry| entry.development_policy == DevelopmentPolicy::Translate),
        "translation segment {id} has the wrong surface or development policy"
    );
    Ok(segment)
}

fn bind_segment<'a>(
    catalog: &'a BakedTextCatalog,
    segment: &'a TranslationDraftSegment,
) -> Result<Vec<(&'a BakedTextUnit, &'a TranslationDraftEntry)>> {
    segment
        .entries
        .iter()
        .map(|entry| {
            let unit = catalog
                .units
                .iter()
                .find(|unit| unit.id == entry.id)
                .with_context(|| format!("baked consumer catalog is missing {}", entry.id))?;
            Ok((unit, entry))
        })
        .collect()
}

fn decode_asset(installer_payload: &BTreeMap<String, Vec<u8>>, file_name: &str) -> Result<Vec<u8>> {
    let streams = decode_all_streams(
        installer_payload
            .get(file_name)
            .with_context(|| format!("verified installer payload is missing {file_name}"))?,
    )?;
    ensure!(
        streams.len() == 1,
        "{file_name} does not have exactly one Compile LZ stream"
    );
    Ok(streams.into_iter().next().unwrap().output)
}

fn make_patch(
    file_name: &'static str,
    producer_id: &'static str,
    units: &[(&BakedTextUnit, &TranslationDraftEntry)],
    decoded: Vec<u8>,
) -> Result<AssetPatch> {
    let packed = encode_single_stream(&decoded)?;
    Ok(AssetPatch {
        file_name,
        producer_id,
        unit_ids: units.iter().map(|(unit, _)| unit.id.clone()).collect(),
        decoded,
        packed,
    })
}

#[cfg(test)]
#[path = "baked_graphics_tests.rs"]
mod baked_graphics_tests;
