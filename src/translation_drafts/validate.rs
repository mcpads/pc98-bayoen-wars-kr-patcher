use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};

use super::model::{ProposalBatch, ProposalEntry, ProtectedWorkspace, TranslationSurface};

const RETAINED_AUDIO_SEGMENT: &str = "voice-samples";

pub(super) fn validate_proposals(
    protected: &ProtectedWorkspace,
    proposals: &[ProposalBatch],
) -> Result<()> {
    let batch_ids = proposals
        .iter()
        .map(|batch| batch.batch_id.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        batch_ids.len() == proposals.len(),
        "proposal batch IDs are not unique"
    );
    for batch in proposals {
        for note in &batch.review.notes {
            ensure!(
                !note.chars().any(is_japanese_or_cjk),
                "proposal batch {} review copies Japanese or CJK source text",
                batch.batch_id
            );
        }
    }
    let expected_segments = protected
        .index
        .segments
        .iter()
        .filter(|segment| segment.surface != TranslationSurface::MonochromeGlyphAtlas)
        .map(|segment| segment.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut actual = BTreeMap::<&str, Vec<&ProposalEntry>>::new();
    for entry in proposals.iter().flat_map(|batch| &batch.entries) {
        ensure!(
            expected_segments.contains(entry.segment_id.as_str()),
            "proposal entry {} refers to unknown segment {}",
            entry.id,
            entry.segment_id
        );
        ensure_source_free(entry)?;
        if entry.segment_id == RETAINED_AUDIO_SEGMENT
            || protected.control_only_entry_ids.contains(&entry.id)
        {
            ensure!(
                entry.korean_text.is_empty(),
                "source-preserving entry {} unexpectedly has Korean replacement text",
                entry.id
            );
        } else {
            ensure!(
                !entry.korean_text.is_empty()
                    && entry.korean_text.iter().any(|line| !line.trim().is_empty()),
                "translation entry {} has no Korean draft",
                entry.id
            );
        }
        actual.entry(&entry.segment_id).or_default().push(entry);
    }
    ensure!(
        actual.keys().copied().collect::<BTreeSet<_>>() == expected_segments,
        "reviewed proposals do not cover every translation-bearing segment"
    );
    for segment_id in expected_segments {
        let expected = protected
            .entry_ids_by_segment
            .get(segment_id)
            .expect("protected segment IDs were loaded together");
        let entries = &actual[segment_id];
        let actual_ids = entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>();
        let unique = actual_ids.iter().copied().collect::<BTreeSet<_>>();
        ensure!(
            unique.len() == actual_ids.len(),
            "reviewed proposals duplicate an ID in segment {segment_id}"
        );
        ensure!(
            actual_ids.iter().copied().collect::<BTreeSet<_>>()
                == expected.iter().map(String::as_str).collect::<BTreeSet<_>>(),
            "reviewed proposals do not exactly cover protected segment {segment_id}"
        );
    }
    Ok(())
}

fn ensure_source_free(entry: &ProposalEntry) -> Result<()> {
    for value in entry
        .korean_text
        .iter()
        .map(String::as_str)
        .chain(entry.notes.as_deref())
        .chain(entry.questions.iter().map(String::as_str))
    {
        ensure!(
            !value.chars().any(is_japanese_or_cjk),
            "proposal entry {} copies Japanese or CJK source text",
            entry.id
        );
    }
    Ok(())
}

fn is_japanese_or_cjk(character: char) -> bool {
    matches!(
        character,
        '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}'
    )
}

#[cfg(test)]
#[path = "validate_tests.rs"]
mod validate_tests;
