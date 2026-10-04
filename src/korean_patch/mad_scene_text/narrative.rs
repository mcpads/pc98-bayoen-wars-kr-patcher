use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Result, ensure};

use super::model::{
    MadSceneNarrativePatchReport, MadSystemTextDisposition, PatchedMadSceneNarrativePayload,
};
use super::{complete_mad_scene_text, prepare_mad_scene_text, v30_verifier};
use crate::korean_patch::baked_graphics::{
    complete_baked_graphics, prepare_baked_graphics, prepare_baked_graphics_with_title_artwork,
};
use crate::korean_patch::monochrome_text::{complete_monochrome_text, prepare_monochrome_text};
use crate::korean_patch::payload_writes::{
    PayloadFileWritePlan, PayloadWriteReport, apply_payload_write_plans_with_verifier,
    combine_payload_write_plans,
};

pub(crate) fn build_mad_scene_narrative_payload(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<PatchedMadSceneNarrativePayload> {
    build_mad_scene_narrative_payload_with_optional_title_artwork(
        installer_payload,
        translations,
        None,
        MadSystemTextDisposition::PreservedSource,
    )
}

pub(crate) fn build_mad_scene_narrative_payload_with_title_artwork(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
    manifest_path: &Path,
    source_preview_path: &Path,
    artwork_path: &Path,
) -> Result<PatchedMadSceneNarrativePayload> {
    build_mad_scene_narrative_payload_with_optional_title_artwork(
        installer_payload,
        translations,
        Some((manifest_path, source_preview_path, artwork_path)),
        MadSystemTextDisposition::PreservedSource,
    )
}

pub(crate) fn build_complete_mad_scene_narrative_payload_with_title_artwork(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
    manifest_path: &Path,
    source_preview_path: &Path,
    artwork_path: &Path,
) -> Result<PatchedMadSceneNarrativePayload> {
    build_mad_scene_narrative_payload_with_optional_title_artwork(
        installer_payload,
        translations,
        Some((manifest_path, source_preview_path, artwork_path)),
        MadSystemTextDisposition::TranslatedDevelopment,
    )
}

fn build_mad_scene_narrative_payload_with_optional_title_artwork(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
    title_artwork: Option<(&Path, &Path, &Path)>,
    system_text_disposition: MadSystemTextDisposition,
) -> Result<PatchedMadSceneNarrativePayload> {
    let (scene, scene_planned) =
        prepare_mad_scene_text(installer_payload, translations, system_text_disposition)?;
    let (monochrome, monochrome_plans) = prepare_monochrome_text(installer_payload, translations)?;
    let (baked, baked_plans) = match title_artwork {
        Some((manifest, source_preview, artwork)) => prepare_baked_graphics_with_title_artwork(
            installer_payload,
            translations,
            manifest,
            source_preview,
            artwork,
        )?,
        None => prepare_baked_graphics(installer_payload, translations)?,
    };

    let scene_keys = write_keys(&scene_planned.plans);
    let monochrome_keys = write_keys(&monochrome_plans);
    let baked_keys = write_keys(&baked_plans);
    ensure_disjoint_keys(
        &scene_keys,
        &monochrome_keys,
        "scene text",
        "monochrome text",
    )?;
    ensure_disjoint_keys(&scene_keys, &baked_keys, "scene text", "baked graphics")?;
    ensure_disjoint_keys(
        &monochrome_keys,
        &baked_keys,
        "monochrome text",
        "baked graphics",
    )?;

    let mut plans = scene_planned.plans;
    plans.extend(monochrome_plans);
    plans.extend(baked_plans);
    let plans = combine_payload_write_plans(plans)?;
    let mut typed_sources = scene_planned.typed_sources;
    for (id, source) in &baked.typed_sources {
        ensure!(
            typed_sources.insert(id.clone(), source.clone()).is_none(),
            "scene text and baked graphics duplicate typed V30 source {id}"
        );
    }
    let verifier = v30_verifier(typed_sources);
    let applied =
        apply_payload_write_plans_with_verifier(installer_payload, plans, Some(&verifier))?;

    let scene_writes = component_writes(&applied.report, &scene_keys, "scene text")?;
    let monochrome_writes = component_writes(&applied.report, &monochrome_keys, "monochrome text")?;
    let baked_writes = component_writes(&applied.report, &baked_keys, "baked graphics")?;
    let scene_text = complete_mad_scene_text(&applied.files, &scene, scene_writes)?;
    let monochrome = complete_monochrome_text(
        installer_payload,
        &applied.files,
        &monochrome,
        monochrome_writes,
    )?;
    let baked = complete_baked_graphics(installer_payload, &applied.files, &baked, baked_writes)?;

    Ok(PatchedMadSceneNarrativePayload {
        files: applied.files,
        report: MadSceneNarrativePatchReport {
            scene_text,
            monochrome,
            baked,
            writes: applied.report,
        },
    })
}

fn write_keys(plans: &[PayloadFileWritePlan]) -> BTreeSet<(String, String)> {
    plans
        .iter()
        .flat_map(|file| {
            file.plan
                .writes
                .iter()
                .map(move |write| (file.file_name.to_owned(), write.id.clone()))
        })
        .collect()
}

fn ensure_disjoint_keys(
    first: &BTreeSet<(String, String)>,
    second: &BTreeSet<(String, String)>,
    first_name: &str,
    second_name: &str,
) -> Result<()> {
    ensure!(
        first.is_disjoint(second),
        "{first_name} and {second_name} duplicate an Expected Write identity"
    );
    Ok(())
}

fn component_writes(
    writes: &[PayloadWriteReport],
    keys: &BTreeSet<(String, String)>,
    component: &str,
) -> Result<Vec<PayloadWriteReport>> {
    let selected = writes
        .iter()
        .filter(|write| keys.contains(&(write.file_name.clone(), write.id.clone())))
        .cloned()
        .collect::<Vec<_>>();
    ensure!(
        selected.len() == keys.len(),
        "{component} Expected Write report population changed during composition"
    );
    Ok(selected)
}
