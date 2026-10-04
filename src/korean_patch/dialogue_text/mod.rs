mod compile;
mod layout;
mod model;
mod plans;
mod punctuation;
mod readback;
mod records;
mod report;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, ensure};

pub(crate) use self::compile::DIALOGUE_GAIJI_PHYSICAL_CAPACITY;
pub(in crate::korean_patch) use self::compile::{
    compile_dialogue_text, compile_scene_dialogue_text,
};
pub(crate) use self::layout::validate_dialogue_rom_layout;
use self::model::PatchedDialogueTextPayload;
use self::plans::dialogue_text_plan;
use self::readback::verify_dialogue_text_payload;
use self::report::entry_report;
use super::font_rasterizer::font_provenance;
use super::monochrome_text::DevelopmentBuildStatus;
use super::payload_writes::apply_payload_write_plans;
use super::shared_text::gaiji_bank_plan;
use crate::game_data::{DialogueCatalog, catalog_game_data};
use crate::source_disk::SOURCE_DISK_SHA256;
use crate::translation_analysis::load_translation_corpus;
use crate::translation_drafts::{DevelopmentPolicy, TranslationDraftSegment, TranslationSurface};

pub(super) use model::CompiledDialogueText;
pub use model::{DialogueTextEntryPatchReport, DialogueTextPatchReport};
pub(crate) use punctuation::{DIALOGUE_GAIJI_PUNCTUATION, dialogue_punctuation_overrides};
pub(super) use readback::verify_dialogue_text_records_and_references;
pub(super) use report::entry_report as dialogue_entry_report;

pub(crate) fn build_dialogue_text_payload(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    translations: &Path,
) -> Result<PatchedDialogueTextPayload> {
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
    let source_dialogue = &source.mad_com.dialogue;
    let segments = dialogue_segments(source_dialogue, &corpus.segments)?;

    let compiled = compile_dialogue_text(
        mad_com,
        gaiji_com,
        &source.gaiji,
        &source.mad_com.gaiji_readiness,
        source_dialogue,
        &segments,
    )?;
    let plans = vec![
        gaiji_bank_plan(
            &compiled.bank,
            "dialogue-text",
            "dialogue-text-gaiji-bank",
            "install reviewed dialogue glyphs",
        ),
        dialogue_text_plan(mad_com, &compiled)?,
    ];
    let applied = apply_payload_write_plans(installer_payload, plans)?;
    verify_dialogue_text_payload(
        &applied.files,
        &source.gaiji,
        &source_dialogue.reference_catalog,
        &compiled,
    )?;

    let font = font_provenance()?;
    let available_gaiji_slots = compiled.bank.available_slot_count;
    let storage_capacity = compiled.text_region_end - compiled.text_region_start;
    let references = &source_dialogue.reference_catalog;
    Ok(PatchedDialogueTextPayload {
        report: DialogueTextPatchReport {
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
            group_count: source_dialogue.group_count,
            entry_count: source_dialogue.entry_count,
            reference_count: references.reference_count,
            machine_code_reference_count: references.machine_code_reference_count,
            group_pointer_reference_count: references.group_pointer_reference_count,
            text_pointer_reference_count: references.text_pointer_reference_count,
            glyphs: compiled.bank.glyphs.clone(),
            entries: compiled.entries.iter().map(entry_report).collect(),
            writes: applied.report,
        },
        files: applied.files,
    })
}

pub(in crate::korean_patch) fn dialogue_segments<'a>(
    source: &DialogueCatalog,
    corpus: &'a [TranslationDraftSegment],
) -> Result<Vec<&'a TranslationDraftSegment>> {
    let dialogue_segment_count = corpus
        .iter()
        .filter(|segment| segment.surface == TranslationSurface::Dialogue)
        .count();
    ensure!(
        dialogue_segment_count == source.groups.len(),
        "translation corpus dialogue group population changed"
    );
    source
        .groups
        .iter()
        .map(|group| {
            let segment = corpus
                .iter()
                .find(|segment| segment.id == group.id)
                .with_context(|| format!("translation corpus is missing {}", group.id))?;
            ensure!(
                segment.surface == TranslationSurface::Dialogue
                    && segment.entries.len() == group.entries.len()
                    && segment
                        .entries
                        .iter()
                        .zip(&group.entries)
                        .all(|(draft, source)| {
                            draft.id == source.id
                                && draft.development_policy == DevelopmentPolicy::Translate
                        }),
                "{} has the wrong surface, population, identity, or development policy",
                group.id
            );
            Ok(segment)
        })
        .collect()
}
