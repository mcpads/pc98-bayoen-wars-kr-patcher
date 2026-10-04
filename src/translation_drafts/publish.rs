use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::model::{
    DevelopmentPolicy, DraftStatus, ProposalBatch, ProtectedWorkspace, TranslationDraftEntry,
    TranslationDraftIndex, TranslationDraftReport, TranslationDraftReview, TranslationDraftSegment,
    TranslationDraftSegmentReference, TranslationSurface,
};
use crate::audit_directory::publish_new_directory;
use crate::source_disk::sha256_hex;

const RETAINED_AUDIO_SEGMENT: &str = "voice-samples";

pub(super) fn publish_drafts(
    protected: &ProtectedWorkspace,
    proposals: &[ProposalBatch],
    output_directory: &Path,
) -> Result<TranslationDraftReport> {
    let mut proposal_entries = BTreeMap::new();
    for batch in proposals {
        for entry in &batch.entries {
            proposal_entries.insert(
                (entry.segment_id.as_str(), entry.id.as_str()),
                (entry, &batch.review),
            );
        }
    }
    let protected_index_bytes = fs::read(protected.root.join("index.json"))?;
    let producer_evidence_segments = protected
        .index
        .segments
        .iter()
        .filter(|segment| segment.surface == TranslationSurface::MonochromeGlyphAtlas)
        .map(|segment| segment.id.clone())
        .collect::<Vec<_>>();
    let mut references = Vec::new();
    let mut translated_text_unit_count = 0;
    let mut preserved_source_control_unit_count = 0;
    let mut retained_source_audio_unit_count = 0;
    let mut unresolved_question_count = 0;

    publish_new_directory(output_directory, |staged| {
        let segment_directory = staged.join("segments");
        fs::create_dir(&segment_directory)?;
        for protected_segment in protected
            .index
            .segments
            .iter()
            .filter(|segment| segment.surface != TranslationSurface::MonochromeGlyphAtlas)
        {
            let mut segment_review = None;
            let entries = protected.entry_ids_by_segment[&protected_segment.id]
                .iter()
                .map(|id| {
                    let (proposal, review) =
                        proposal_entries[&(protected_segment.id.as_str(), id.as_str())];
                    segment_review.get_or_insert_with(|| review.clone());
                    unresolved_question_count += proposal.questions.len();
                    let development_policy = if protected_segment.id == RETAINED_AUDIO_SEGMENT {
                        retained_source_audio_unit_count += 1;
                        DevelopmentPolicy::RetainSourceAudio
                    } else if protected.control_only_entry_ids.contains(id) {
                        preserved_source_control_unit_count += 1;
                        DevelopmentPolicy::PreserveSourceControl
                    } else {
                        translated_text_unit_count += 1;
                        DevelopmentPolicy::Translate
                    };
                    TranslationDraftEntry {
                        id: id.clone(),
                        korean_text: proposal.korean_text.clone(),
                        development_policy,
                        status: DraftStatus::NeedsHumanReview,
                        notes: proposal.notes.clone(),
                        questions: proposal.questions.clone(),
                    }
                })
                .collect::<Vec<_>>();
            let draft_bytes = serde_json::to_vec(&entries)?;
            let draft_sha256 = sha256_hex(&draft_bytes);
            let review = segment_review.expect("validated segment has proposals");
            let document = TranslationDraftSegment {
                id: protected_segment.id.clone(),
                surface: protected_segment.surface,
                protected_segment_sha256: protected_segment.content_sha256.clone(),
                draft_sha256: draft_sha256.clone(),
                entries,
                review: TranslationDraftReview {
                    reviewed_draft_sha256: draft_sha256.clone(),
                    method: review.method,
                    outcome: review.outcome,
                    notes: review.notes,
                },
            };
            let file_name = format!("{}.json", protected_segment.id);
            write_json(&segment_directory.join(&file_name), &document)?;
            references.push(TranslationDraftSegmentReference {
                id: protected_segment.id.clone(),
                surface: protected_segment.surface,
                path: format!("segments/{file_name}"),
                entry_count: document.entries.len(),
                protected_segment_sha256: protected_segment.content_sha256.clone(),
                draft_sha256,
            });
        }
        let index = TranslationDraftIndex {
            supported_source_sha256: protected.index.supported_source_sha256.clone(),
            protected_index_sha256: sha256_hex(&protected_index_bytes),
            status: DraftStatus::NeedsHumanReview,
            translation_unit_count: translated_text_unit_count
                + preserved_source_control_unit_count
                + retained_source_audio_unit_count,
            translated_text_unit_count,
            preserved_source_control_unit_count,
            retained_source_audio_unit_count,
            producer_evidence_segments: producer_evidence_segments.clone(),
            segments: references,
        };
        write_json(&staged.join("index.json"), &index)
    })?;

    Ok(TranslationDraftReport {
        output_directory: output_directory.to_path_buf(),
        supported_source_sha256: protected.index.supported_source_sha256.clone(),
        segment_count: protected
            .index
            .segments
            .iter()
            .filter(|segment| segment.surface != TranslationSurface::MonochromeGlyphAtlas)
            .count(),
        translation_unit_count: translated_text_unit_count
            + preserved_source_control_unit_count
            + retained_source_audio_unit_count,
        translated_text_unit_count,
        preserved_source_control_unit_count,
        retained_source_audio_unit_count,
        unresolved_question_count,
    })
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))
}
