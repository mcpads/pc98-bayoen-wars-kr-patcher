mod output;
mod wav;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, ensure};

use crate::asset_bindings::{SAMPLE_RATE_HZ, decode_sample};
use crate::localization_assets::decode_all_streams;
use crate::{AudioAuditEntry, AudioAuditReport};

const SAMPLE_ASSET: &str = "SAMPA";

pub(super) fn write_audio_audit(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    output_directory: &Path,
) -> Result<AudioAuditReport> {
    let packed = installer_payload
        .get(SAMPLE_ASSET)
        .context("verified installer payload is missing SAMPA")?;
    let streams = decode_all_streams(packed)?;
    ensure!(
        streams.len() == 22,
        "SAMPA does not contain 22 sample streams"
    );

    let mut rendered = Vec::with_capacity(streams.len());
    for (index, stream) in streams.into_iter().enumerate() {
        let sample = decode_sample(&stream.output)?;
        let output_file = format!("sample-{index:02}.wav");
        rendered.push(output::RenderedSample {
            entry: AudioAuditEntry {
                stream_index: index,
                packed_offset: stream.packed_offset,
                packed_size: stream.packed_size,
                decoded_size: stream.output.len(),
                header_size: sample.header_size,
                encoded_sample_bytes: sample.nibbles.len().div_ceil(2),
                sample_count: sample.nibbles.len(),
                sample_rate: SAMPLE_RATE_HZ,
                duration_milliseconds: u64::try_from(sample.nibbles.len())? * 1_000
                    / u64::from(SAMPLE_RATE_HZ),
                output_file: output_file.clone(),
            },
            output_file,
            wav: wav::encode_mono_u8(&sample.nibbles, SAMPLE_RATE_HZ)?,
        });
    }

    output::publish_samples(output_directory, rendered)
}
