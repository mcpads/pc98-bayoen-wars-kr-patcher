mod composition;
mod model;
mod report;
mod targets;

use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

use self::composition::{LocalizationComponentCandidate, compose_file_family};
use self::model::PatchedIntegratedLocalizationFiles;
use self::report::{component_report, patched_file_reports, preserved_file_report};
use self::targets::{
    ARLE_CHARACTER_FILES, MENU_FILES, MOUSE_DRIVER_FILES, PLAYBACK_DRIVER_FILES,
    RETAINED_SOURCE_AUDIO_FILES, SAMPLING_DRIVER_FILES, SCENE_FILES, SHELL_FILES,
    SYSTEM_LOADER_FILES,
};
use super::character_assets::{
    ResolvedArleCharacterAssetSelection, build_arle_character_asset_payload,
};
use super::font_catalog::{FontTarget, font_profile_report_for};
use super::mad_scene_text::build_complete_mad_scene_narrative_payload_with_title_artwork;
use super::menu_text::build_menu_text_files;
use super::monochrome_text::DevelopmentBuildStatus;
use super::mouse_driver_text::build_mouse_driver_text_files;
use super::playback_driver_text::build_playback_driver_text_payload;
use super::sampling_driver_text::build_sampling_driver_text_files;
use super::shell_text::build_shell_text_files;
use super::system_loader_text::build_system_loader_text_files;
use crate::IntegratedLocalizationDevelopmentInputs;
use crate::source_disk::SOURCE_DISK_SHA256;
use crate::translation_analysis::load_translation_corpus;

pub use model::{
    ArleCharacterAssetBuildReport, IntegratedLocalizationComponentReport,
    IntegratedLocalizationFileDisposition, IntegratedLocalizationFileReport,
    IntegratedLocalizationPatchReport,
};

pub(crate) fn build_integrated_localization_files(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    boot_files: &BTreeMap<String, Vec<u8>>,
    runtime_files: &BTreeMap<String, Vec<u8>>,
    inputs: IntegratedLocalizationDevelopmentInputs<'_>,
) -> Result<PatchedIntegratedLocalizationFiles> {
    let scene = build_complete_mad_scene_narrative_payload_with_title_artwork(
        installer_payload,
        inputs.translations,
        inputs.title_artwork_manifest,
        inputs.title_source_preview,
        inputs.title_artwork,
    )?;
    ensure!(
        scene.report.baked.title_artwork.is_some(),
        "integrated localization silently omitted the declared title artwork"
    );
    let playback = build_playback_driver_text_payload(installer_payload, inputs.translations)?;
    let menu = build_menu_text_files(installer_payload, inputs.translations)?;
    let shell = build_shell_text_files(installer_payload, boot_files, inputs.translations)?;
    let system_loader =
        build_system_loader_text_files(installer_payload, boot_files, inputs.translations)?;
    let sampling =
        build_sampling_driver_text_files(installer_payload, runtime_files, inputs.translations)?;
    let mouse =
        build_mouse_driver_text_files(installer_payload, runtime_files, inputs.translations)?;
    let arle = match inputs.arle_character_assets.resolve()? {
        ResolvedArleCharacterAssetSelection::PreserveOriginal => None,
        ResolvedArleCharacterAssetSelection::Replace(asset_set) => {
            let patched = build_arle_character_asset_payload(
                installer_payload,
                &asset_set.manifest,
                &asset_set.artwork,
            )?;
            ensure!(
                patched.report.approval_status == "needs_human_review"
                    && patched.report.build_status == DevelopmentBuildStatus::DevelopmentOnly,
                "integrated Arle character asset crossed its development or approval boundary"
            );
            Some(patched)
        }
    };
    let replaces_arle = arle.is_some();

    let mut installer_candidates = vec![
        LocalizationComponentCandidate {
            id: "mad-scene-narrative",
            files: &scene.files,
            writes: &scene.report.writes,
            modified_files: SCENE_FILES,
        },
        LocalizationComponentCandidate {
            id: "playback-driver-text",
            files: &playback.files,
            writes: &playback.report.writes,
            modified_files: PLAYBACK_DRIVER_FILES,
        },
        LocalizationComponentCandidate {
            id: "menu-text",
            files: &menu.files,
            writes: &menu.report.writes,
            modified_files: MENU_FILES,
        },
    ];
    if let Some(arle) = &arle {
        installer_candidates.push(LocalizationComponentCandidate {
            id: "arle-character-asset",
            files: &arle.files,
            writes: &arle.report.writes,
            modified_files: ARLE_CHARACTER_FILES,
        });
    }
    let installer = compose_file_family(installer_payload, &installer_candidates)?;
    let boot = compose_file_family(
        boot_files,
        &[
            LocalizationComponentCandidate {
                id: "shell-text",
                files: &shell.files,
                writes: &shell.report.writes,
                modified_files: SHELL_FILES,
            },
            LocalizationComponentCandidate {
                id: "system-loader-text",
                files: &system_loader.files,
                writes: &system_loader.report.writes,
                modified_files: SYSTEM_LOADER_FILES,
            },
        ],
    )?;
    let runtime = compose_file_family(
        runtime_files,
        &[
            LocalizationComponentCandidate {
                id: "sampling-driver-text",
                files: &sampling.files,
                writes: &sampling.report.writes,
                modified_files: SAMPLING_DRIVER_FILES,
            },
            LocalizationComponentCandidate {
                id: "mouse-driver-text",
                files: &mouse.files,
                writes: &mouse.report.writes,
                modified_files: MOUSE_DRIVER_FILES,
            },
        ],
    )?;

    let corpus = load_translation_corpus(inputs.translations)?;
    ensure!(
        corpus.index.supported_source_sha256 == SOURCE_DISK_SHA256,
        "translation corpus targets a different source disk"
    );
    let component_statuses = [
        scene.report.scene_text.translation_status.as_str(),
        shell.report.translation_status.as_str(),
        system_loader.report.translation_status.as_str(),
        sampling.report.translation_status.as_str(),
        mouse.report.translation_status.as_str(),
        playback.report.translation_status.as_str(),
        menu.report.translation_status.as_str(),
    ];
    ensure!(
        component_statuses
            .iter()
            .all(|status| *status == "needs_human_review"),
        "integrated localization component review states differ"
    );

    let components = vec![
        component_report(
            "mad-scene-narrative",
            SCENE_FILES,
            scene.report.writes.len(),
            0,
        ),
        component_report("shell-text", SHELL_FILES, shell.report.writes.len(), 0),
        component_report(
            "system-loader-text",
            SYSTEM_LOADER_FILES,
            system_loader.report.writes.len(),
            system_loader.report.resident_writes.len(),
        ),
        component_report(
            "sampling-driver-text",
            SAMPLING_DRIVER_FILES,
            sampling.report.writes.len(),
            0,
        ),
        component_report(
            "mouse-driver-text",
            MOUSE_DRIVER_FILES,
            mouse.report.writes.len(),
            0,
        ),
        component_report(
            "playback-driver-text",
            PLAYBACK_DRIVER_FILES,
            playback.report.writes.len(),
            playback
                .report
                .drivers
                .iter()
                .map(|driver| driver.unpacked_writes.len())
                .sum(),
        ),
        component_report("menu-text", MENU_FILES, menu.report.writes.len(), 0),
        match &arle {
            Some(arle) => component_report(
                "arle-character-asset",
                ARLE_CHARACTER_FILES,
                arle.report.writes.len(),
                arle.report.writes.len(),
            ),
            None => component_report("arle-character-asset", &[], 0, 0),
        },
    ];

    let mut files =
        patched_file_reports(installer_payload, &installer.files, &installer.producers)?;
    files.extend(patched_file_reports(
        boot_files,
        &boot.files,
        &boot.producers,
    )?);
    files.extend(patched_file_reports(
        runtime_files,
        &runtime.files,
        &runtime.producers,
    )?);
    if !replaces_arle {
        for &file_name in ARLE_CHARACTER_FILES {
            files.push(preserved_file_report(
                installer_payload,
                &installer.files,
                file_name,
                IntegratedLocalizationFileDisposition::PreservedOriginalAsset,
                "original-arle-asset-policy",
            )?);
        }
    }
    for &file_name in RETAINED_SOURCE_AUDIO_FILES {
        files.push(preserved_file_report(
            installer_payload,
            &installer.files,
            file_name,
            IntegratedLocalizationFileDisposition::RetainedSourceAudio,
            "retained-source-audio-policy",
        )?);
    }
    files.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    let declared_target_files = SCENE_FILES
        .iter()
        .chain(SHELL_FILES)
        .chain(SYSTEM_LOADER_FILES)
        .chain(SAMPLING_DRIVER_FILES)
        .chain(MOUSE_DRIVER_FILES)
        .chain(PLAYBACK_DRIVER_FILES)
        .chain(MENU_FILES)
        .chain(ARLE_CHARACTER_FILES)
        .chain(RETAINED_SOURCE_AUDIO_FILES)
        .copied()
        .collect::<BTreeSet<_>>();
    let reported_target_files = files
        .iter()
        .map(|file| file.file_name.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        declared_target_files.len() == 17 && reported_target_files == declared_target_files,
        "integrated localization report does not cover the 17-file target population"
    );

    let mut composition_writes = installer.writes;
    composition_writes.extend(boot.writes);
    composition_writes.extend(runtime.writes);
    composition_writes.sort_by(|left, right| {
        left.file_name
            .cmp(&right.file_name)
            .then(left.id.cmp(&right.id))
    });
    let expected_modified_file_count = if replaces_arle { 16 } else { 14 };
    ensure!(
        composition_writes.len() == expected_modified_file_count,
        "integrated localization composition does not own the selected {expected_modified_file_count}-file target population"
    );
    let font_profiles = [
        FontTarget::Body16,
        FontTarget::Narrative32,
        FontTarget::BakedDisplay16,
        FontTarget::Difficulty32,
    ]
    .into_iter()
    .map(font_profile_report_for)
    .collect::<Result<Vec<_>>>()?;

    Ok(PatchedIntegratedLocalizationFiles {
        boot_files: boot.files,
        runtime_files: runtime.files,
        installer_payload: installer.files,
        report: IntegratedLocalizationPatchReport {
            supported_source_sha256: corpus.index.supported_source_sha256,
            translation_status: "needs_human_review".to_owned(),
            build_status: DevelopmentBuildStatus::DevelopmentOnly,
            translation_unit_count: corpus.index.translation_unit_count,
            translated_text_unit_count: corpus.index.translated_text_unit_count,
            preserved_source_control_unit_count: corpus.index.preserved_source_control_unit_count,
            retained_source_audio_unit_count: corpus.index.retained_source_audio_unit_count,
            target_file_count: files.len(),
            modified_file_count: composition_writes.len(),
            retained_source_audio_file_count: RETAINED_SOURCE_AUDIO_FILES.len(),
            font_profiles,
            title_artwork: scene.report.baked.title_artwork.clone(),
            arle_character_asset: match arle {
                Some(arle) => ArleCharacterAssetBuildReport::Replaced {
                    report: Box::new(arle.report),
                },
                None => ArleCharacterAssetBuildReport::OriginalPreserved,
            },
            components,
            files,
            composition_writes,
        },
    })
}
