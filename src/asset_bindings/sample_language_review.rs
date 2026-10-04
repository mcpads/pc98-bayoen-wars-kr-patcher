use anyhow::{Result, ensure};
use serde::Serialize;

use super::sample_playback::SamplePlaybackCatalog;

const REVIEWED_STREAM_COUNT: usize = 22;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleLanguageClassification {
    JapaneseVoicePerformance,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleTranscriptStatus {
    NeedsIndependentReview,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SampleLanguageReview {
    pub id: String,
    pub stream_index: usize,
    pub classification: SampleLanguageClassification,
    pub transcript_status: SampleTranscriptStatus,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SampleLanguageReviewCatalog {
    pub source_asset: String,
    pub reviewed_stream_count: usize,
    pub japanese_voice_stream_count: usize,
    pub sample_rate_hz: u32,
    pub evidence: String,
    pub streams: Vec<SampleLanguageReview>,
}

pub(super) fn catalog_sample_language_review(
    playback: &SamplePlaybackCatalog,
) -> Result<SampleLanguageReviewCatalog> {
    ensure!(
        playback.stream_count == REVIEWED_STREAM_COUNT,
        "SAMPA language review does not cover the complete playback stream population"
    );
    let streams = (0..REVIEWED_STREAM_COUNT)
        .map(|stream_index| SampleLanguageReview {
            id: format!("voice-sample-{:02}", stream_index + 1),
            stream_index,
            classification: SampleLanguageClassification::JapaneseVoicePerformance,
            transcript_status: SampleTranscriptStatus::NeedsIndependentReview,
        })
        .collect::<Vec<_>>();

    Ok(SampleLanguageReviewCatalog {
        source_asset: playback.source_asset.clone(),
        reviewed_stream_count: streams.len(),
        japanese_voice_stream_count: streams.len(),
        sample_rate_hz: playback.sample_rate_hz,
        evidence: "all 22 distinct 9,600 Hz playback streams contain Japanese spoken phrases or character vocalizations; exact wording remains outside this language-presence review"
            .to_owned(),
        streams,
    })
}

#[cfg(test)]
#[path = "sample_language_review_tests.rs"]
mod sample_language_review_tests;
