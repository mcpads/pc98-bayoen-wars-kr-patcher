use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use super::sample_format::decode_sample;
use super::sample_language_review::{
    SampleLanguageClassification, SampleLanguageReview, SampleLanguageReviewCatalog,
    SampleTranscriptStatus,
};
use super::sample_playback::SamplePlaybackCatalog;
use crate::byte_string::encode_lower_hex;
use crate::localization_assets::{DecodedStream, decode_all_streams};
use crate::source_disk::sha256_hex;

const SAMPLE_ASSET: &str = "SAMPA";

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct VoiceSampleCatalog {
    pub source_asset: String,
    pub sample_rate_hz: u32,
    pub streams: Vec<VoiceSampleUnit>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct VoiceSampleUnit {
    pub id: String,
    pub stream_index: usize,
    pub packed_offset: usize,
    pub packed_size: usize,
    pub packed_sha256: String,
    pub packed_raw_hex: String,
    pub decoded_size: usize,
    pub decoded_sha256: String,
    pub decoded_raw_hex: String,
    pub header_size: usize,
    pub encoded_sample_bytes: usize,
    pub sample_count: usize,
    pub duration_milliseconds: u64,
    pub classification: SampleLanguageClassification,
    pub transcript_status: SampleTranscriptStatus,
}

pub(super) fn catalog_voice_samples(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    playback: &SamplePlaybackCatalog,
    language: &SampleLanguageReviewCatalog,
) -> Result<VoiceSampleCatalog> {
    ensure!(
        playback.source_asset == SAMPLE_ASSET && language.source_asset == SAMPLE_ASSET,
        "voice sample catalogs do not refer to SAMPA"
    );
    let packed_asset = installer_payload
        .get(SAMPLE_ASSET)
        .context("verified installer payload is missing SAMPA")?;
    let decoded = decode_all_streams(packed_asset)?;
    ensure!(
        decoded.len() == playback.stream_count && decoded.len() == language.streams.len(),
        "voice sample metadata does not cover every SAMPA stream"
    );

    let streams = decoded
        .iter()
        .zip(&language.streams)
        .enumerate()
        .map(|(index, (stream, review))| {
            bind_voice_sample(index, packed_asset, stream, review, playback.sample_rate_hz)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(VoiceSampleCatalog {
        source_asset: SAMPLE_ASSET.to_owned(),
        sample_rate_hz: playback.sample_rate_hz,
        streams,
    })
}

fn bind_voice_sample(
    index: usize,
    packed_asset: &[u8],
    stream: &DecodedStream,
    review: &SampleLanguageReview,
    sample_rate_hz: u32,
) -> Result<VoiceSampleUnit> {
    ensure!(
        review.stream_index == index,
        "voice sample review index does not match stream order"
    );
    let packed = packed_asset
        .get(stream.packed_offset..stream.packed_offset + stream.packed_size)
        .with_context(|| format!("{} packed range lies outside SAMPA", review.id))?;
    let sample = decode_sample(&stream.output)?;
    let sample_count = sample.nibbles.len();

    Ok(VoiceSampleUnit {
        id: review.id.clone(),
        stream_index: index,
        packed_offset: stream.packed_offset,
        packed_size: stream.packed_size,
        packed_sha256: sha256_hex(packed),
        packed_raw_hex: encode_lower_hex(packed),
        decoded_size: stream.output.len(),
        decoded_sha256: sha256_hex(&stream.output),
        decoded_raw_hex: encode_lower_hex(&stream.output),
        header_size: sample.header_size,
        encoded_sample_bytes: sample_count.div_ceil(2),
        sample_count,
        duration_milliseconds: u64::try_from(sample.nibbles.len())? * 1_000
            / u64::from(sample_rate_hz),
        classification: review.classification,
        transcript_status: review.transcript_status,
    })
}

#[cfg(test)]
#[path = "voice_samples_tests.rs"]
mod voice_samples_tests;
