mod atlas;
mod model;
mod pages;
mod plans;
mod readback;
mod report;

use std::collections::BTreeMap;
use std::path::Path;

use self::atlas::compile_atlas;
use self::model::{PatchedMonochromeTextPayload, SequencePatch};
use self::pages::compile_pages;
use self::plans::{mad_page_plan, packed_atlas_plan};
use self::readback::verify_patched_payload;
use self::report::sequence_report;
use super::font_catalog::{FontTarget, font_provenance_for};
use super::font_rasterizer::FontProvenance;
use super::payload_writes::{PayloadFileWritePlan, PayloadWriteReport, apply_payload_write_plans};
use crate::asset_bindings::{
    MonochromeTextSequence, catalog_monochrome_sprites, catalog_monochrome_text,
};
use crate::localization_assets::encode_single_stream;
use crate::source_disk::SOURCE_DISK_SHA256;
use crate::translation_analysis::load_translation_corpus;
use crate::translation_drafts::{DevelopmentPolicy, TranslationDraftSegment, TranslationSurface};
use anyhow::{Context, Result, ensure};

const MAD_FILE: &str = "MAD.COM";
const OPENING_FILE: &str = "OPM.DAT";
const ENDING_FILE: &str = "EDM.DAT";

pub use model::{
    DevelopmentBuildStatus, MonochromeGlyphSlot, MonochromeSequencePatchReport,
    MonochromeTextPatchReport,
};

pub(in crate::korean_patch) struct PreparedMonochromeText {
    supported_source_sha256: String,
    font: FontProvenance,
    opening: SequencePatch,
    ending: SequencePatch,
}

pub(crate) fn build_monochrome_text_payload(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<PatchedMonochromeTextPayload> {
    let (prepared, plans) = prepare_monochrome_text(installer_payload, translations)?;
    let applied = apply_payload_write_plans(installer_payload, plans)?;
    let report =
        complete_monochrome_text(installer_payload, &applied.files, &prepared, applied.report)?;
    Ok(PatchedMonochromeTextPayload {
        report,
        files: applied.files,
    })
}

pub(in crate::korean_patch) fn prepare_monochrome_text(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<(PreparedMonochromeText, Vec<PayloadFileWritePlan>)> {
    let corpus = load_translation_corpus(translations)?;
    ensure!(
        corpus.index.supported_source_sha256 == SOURCE_DISK_SHA256,
        "translation corpus targets a different source disk"
    );
    let mad_com = installer_payload
        .get(MAD_FILE)
        .context("verified installer payload is missing MAD.COM")?;
    let sprites = catalog_monochrome_sprites(mad_com, installer_payload)?;
    let source_catalog = catalog_monochrome_text(mad_com, installer_payload, &sprites)?;

    let opening_segment = require_segment(
        &corpus.segments,
        "opening-pages",
        TranslationSurface::OpeningGlyphIndexPages,
    )?;
    let ending_segment = require_segment(
        &corpus.segments,
        "ending-pages",
        TranslationSurface::EndingGlyphIndexPages,
    )?;
    let opening = compile_sequence(
        OPENING_FILE,
        opening_segment,
        &source_catalog.opening,
        installer_payload,
    )?;
    let ending = compile_sequence(
        ENDING_FILE,
        ending_segment,
        &source_catalog.ending,
        installer_payload,
    )?;

    let plans = vec![
        packed_atlas_plan(installer_payload, &opening),
        packed_atlas_plan(installer_payload, &ending),
        mad_page_plan(
            mad_com,
            &source_catalog.opening,
            &opening.pages,
            &source_catalog.ending,
            &ending.pages,
        )?,
    ];
    let font = font_provenance_for(FontTarget::Narrative32)?;
    Ok((
        PreparedMonochromeText {
            supported_source_sha256: corpus.index.supported_source_sha256,
            font,
            opening,
            ending,
        },
        plans,
    ))
}

pub(in crate::korean_patch) fn complete_monochrome_text(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    files: &BTreeMap<String, Vec<u8>>,
    prepared: &PreparedMonochromeText,
    writes: Vec<PayloadWriteReport>,
) -> Result<MonochromeTextPatchReport> {
    verify_patched_payload(files, &prepared.opening, &prepared.ending)?;
    Ok(MonochromeTextPatchReport {
        supported_source_sha256: prepared.supported_source_sha256.clone(),
        translation_status: "needs_human_review".to_owned(),
        build_status: DevelopmentBuildStatus::DevelopmentOnly,
        font_profile: prepared.font.profile_id.clone(),
        font_sha256: prepared.font.font_sha256.clone(),
        font_version: prepared.font.font_version.clone(),
        font_source: prepared.font.source.clone(),
        font_upstream_revision: prepared.font.upstream_revision.clone(),
        sequences: vec![
            sequence_report(installer_payload, &prepared.opening),
            sequence_report(installer_payload, &prepared.ending),
        ],
        writes,
    })
}

fn require_segment<'a>(
    segments: &'a [TranslationDraftSegment],
    id: &str,
    surface: TranslationSurface,
) -> Result<&'a TranslationDraftSegment> {
    let segment = segments
        .iter()
        .find(|segment| segment.id == id)
        .with_context(|| format!("translation corpus is missing {id}"))?;
    ensure!(
        segment.surface == surface
            && segment
                .entries
                .iter()
                .all(|entry| entry.development_policy == DevelopmentPolicy::Translate),
        "translation segment {id} has the wrong surface or development policy"
    );
    Ok(segment)
}

fn compile_sequence(
    file_name: &'static str,
    segment: &TranslationDraftSegment,
    source: &MonochromeTextSequence,
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<SequencePatch> {
    ensure!(
        source.source_asset == file_name,
        "{} source asset binding changed",
        source.id
    );
    let lines = segment
        .entries
        .iter()
        .flat_map(|entry| entry.korean_text.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let atlas = compile_atlas(&lines, source.glyph_capacity)?;
    let pages = compile_pages(source, &segment.entries, &atlas.glyph_indices)?;
    let packed_atlas = encode_single_stream(&atlas.decoded)?;
    ensure!(
        installer_payload.contains_key(file_name),
        "verified installer payload is missing {file_name}"
    );
    Ok(SequencePatch {
        id: source.id.clone(),
        file_name,
        atlas,
        pages,
        packed_atlas,
    })
}

#[cfg(test)]
#[path = "monochrome_text_tests.rs"]
mod monochrome_text_tests;
