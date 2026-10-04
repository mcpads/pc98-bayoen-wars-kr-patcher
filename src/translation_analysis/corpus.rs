use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};

use crate::source_disk::sha256_hex;
use crate::translation_drafts::{
    DevelopmentPolicy, DraftStatus, TranslationDraftIndex, TranslationDraftSegment,
    TranslationDraftSegmentReference, TranslationSurface,
};

pub(crate) struct TranslationCorpus {
    pub(crate) index: TranslationDraftIndex,
    pub(crate) segments: Vec<TranslationDraftSegment>,
}

pub(crate) fn load_translation_corpus(directory: &Path) -> Result<TranslationCorpus> {
    let index: TranslationDraftIndex = read_json(&directory.join("index.json"))?;
    ensure!(
        index.status == DraftStatus::NeedsHumanReview,
        "translation corpus is not a needs_human_review development input"
    );
    let mut segment_ids = BTreeSet::new();
    let mut segments = Vec::with_capacity(index.segments.len());
    for reference in &index.segments {
        ensure!(
            segment_ids.insert(reference.id.as_str()),
            "translation corpus index duplicates segment {}",
            reference.id
        );
        let segment: TranslationDraftSegment = read_json(&directory.join(&reference.path))?;
        validate_segment(reference, &segment)?;
        segments.push(segment);
    }
    let translated_text_unit_count = segments
        .iter()
        .flat_map(|segment| &segment.entries)
        .filter(|entry| entry.development_policy == DevelopmentPolicy::Translate)
        .count();
    let preserved_source_control_unit_count = segments
        .iter()
        .flat_map(|segment| &segment.entries)
        .filter(|entry| entry.development_policy == DevelopmentPolicy::PreserveSourceControl)
        .count();
    let retained_source_audio_unit_count = segments
        .iter()
        .flat_map(|segment| &segment.entries)
        .filter(|entry| entry.development_policy == DevelopmentPolicy::RetainSourceAudio)
        .count();
    ensure!(
        translated_text_unit_count == index.translated_text_unit_count
            && preserved_source_control_unit_count == index.preserved_source_control_unit_count
            && retained_source_audio_unit_count == index.retained_source_audio_unit_count,
        "translation corpus policy counts differ from its index"
    );
    ensure!(
        translated_text_unit_count
            + preserved_source_control_unit_count
            + retained_source_audio_unit_count
            == index.translation_unit_count,
        "translation corpus unit count differs from its policy counts"
    );
    Ok(TranslationCorpus { index, segments })
}

fn validate_segment(
    reference: &TranslationDraftSegmentReference,
    segment: &TranslationDraftSegment,
) -> Result<()> {
    ensure!(
        segment.id == reference.id
            && segment.surface == reference.surface
            && segment.protected_segment_sha256 == reference.protected_segment_sha256,
        "translation segment {} identity differs from its index",
        reference.id
    );
    ensure!(
        segment.entries.len() == reference.entry_count,
        "translation segment {} entry count differs from its index",
        reference.id
    );
    let draft_sha256 = sha256_hex(&serde_json::to_vec(&segment.entries)?);
    ensure!(
        draft_sha256 == reference.draft_sha256
            && draft_sha256 == segment.draft_sha256
            && draft_sha256 == segment.review.reviewed_draft_sha256,
        "translation segment {} no longer matches its reviewed draft hash",
        reference.id
    );
    let entry_ids = segment
        .entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        entry_ids.len() == segment.entries.len(),
        "translation segment {} duplicates an entry ID",
        reference.id
    );
    for entry in &segment.entries {
        ensure!(
            entry.status == DraftStatus::NeedsHumanReview,
            "translation entry {} is not marked needs_human_review",
            entry.id
        );
        match entry.development_policy {
            DevelopmentPolicy::Translate => ensure!(
                entry.korean_text.iter().any(|line| !line.trim().is_empty()),
                "translation entry {} has no visible draft text",
                entry.id
            ),
            DevelopmentPolicy::PreserveSourceControl => ensure!(
                segment.surface != TranslationSurface::JapaneseVoicePerformance
                    && entry.korean_text.is_empty(),
                "control-only entry {} has replacement text or the wrong surface",
                entry.id
            ),
            DevelopmentPolicy::RetainSourceAudio => ensure!(
                segment.surface == TranslationSurface::JapaneseVoicePerformance
                    && entry.korean_text.is_empty(),
                "retained audio entry {} has replacement text or the wrong surface",
                entry.id
            ),
        }
    }
    Ok(())
}

fn read_json<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> Result<T> {
    serde_json::from_slice(
        &fs::read(path).with_context(|| format!("failed to read {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", path.display()))
}

#[cfg(test)]
#[path = "corpus_tests.rs"]
mod corpus_tests;
