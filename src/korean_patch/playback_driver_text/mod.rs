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

use self::compile::{PlaybackDriverCompileInput, compile_playback_driver};
use self::model::PatchedPlaybackDriverTextFiles;
use self::plans::packed_driver_plan;
use self::readback::verify_playback_driver_files;
use self::report::driver_report;
use super::font_rasterizer::font_provenance;
use super::monochrome_text::DevelopmentBuildStatus;
use super::payload_writes::apply_payload_write_plans;
use crate::external_text::{catalog_packed_sound_driver_runtime, catalog_playback_driver};
use crate::game_data::catalog_game_data;
use crate::source_disk::SOURCE_DISK_SHA256;
use crate::translation_analysis::load_translation_corpus;
use crate::translation_drafts::{DevelopmentPolicy, TranslationSurface};

const DRIVER_SEGMENTS: [(&str, &str); 2] = [
    ("BPLAY6.COM", "external-bplay6"),
    ("FPLAY6.COM", "external-fplay6"),
];

pub use model::{
    PlaybackDriverPatchReport, PlaybackDriverTextEntryPatchReport, PlaybackDriverTextPatchReport,
};

pub(crate) fn build_playback_driver_text_payload(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<PatchedPlaybackDriverTextFiles> {
    let corpus = load_translation_corpus(translations)?;
    ensure!(
        corpus.index.supported_source_sha256 == SOURCE_DISK_SHA256,
        "translation corpus targets a different source disk"
    );
    let mad_com = required(installer_payload, "MAD.COM")?;
    let gaiji_com = required(installer_payload, "GAIJI.COM")?;
    let menu_com = required(installer_payload, "MENU.COM")?;
    let game_data = catalog_game_data(mad_com, gaiji_com, menu_com)?;

    let mut compiled = Vec::new();
    for (file_name, segment_id) in DRIVER_SEGMENTS {
        let packed = required(installer_payload, file_name)?;
        let source = catalog_playback_driver(file_name, packed)?;
        let runtime = catalog_packed_sound_driver_runtime(packed, &source)?;
        let segment = corpus
            .segments
            .iter()
            .find(|segment| segment.id == segment_id)
            .with_context(|| format!("translation corpus is missing {segment_id}"))?;
        ensure!(
            segment.surface == TranslationSurface::ExternalProgramText
                && segment.entries.len() == source.entries.len()
                && segment.entries.iter().all(|entry| {
                    matches!(
                        entry.development_policy,
                        DevelopmentPolicy::Translate | DevelopmentPolicy::PreserveSourceControl
                    )
                }),
            "{segment_id} has the wrong surface, population, or development policy"
        );
        compiled.push(compile_playback_driver(PlaybackDriverCompileInput {
            file_name,
            packed_source: packed,
            gaiji_com,
            gaiji: &game_data.gaiji,
            readiness: &game_data.mad_com.gaiji_readiness,
            source: &source,
            runtime,
            draft: segment,
        })?);
    }

    let plans = compiled
        .iter()
        .map(|driver| {
            packed_driver_plan(
                driver.file_name,
                installer_payload
                    .get(driver.file_name)
                    .expect("compiled driver came from the installer payload"),
                &driver.output_packed,
            )
        })
        .collect();
    let applied = apply_payload_write_plans(installer_payload, plans)?;
    verify_playback_driver_files(&applied.files, &compiled)?;

    let font = font_provenance()?;
    Ok(PatchedPlaybackDriverTextFiles {
        report: PlaybackDriverTextPatchReport {
            supported_source_sha256: corpus.index.supported_source_sha256,
            translation_status: "needs_human_review".to_owned(),
            build_status: DevelopmentBuildStatus::DevelopmentOnly,
            font_profile: font.profile_id,
            font_sha256: font.font_sha256,
            drivers: compiled.iter().map(driver_report).collect(),
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
